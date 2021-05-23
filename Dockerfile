FROM rust:1.52 AS base
RUN cargo install diesel_cli --no-default-features --features postgres



FROM base AS dev
ARG bind_mnt
ENV BIND_MNT $bind_mnt
RUN cargo install cargo-watch
WORKDIR $BIND_MNT
# build dependencies
COPY Cargo.toml Cargo.lock ./
RUN echo 'fn main() {}' >dummy.rs
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
RUN rm dummy.rs
#
COPY . .
CMD diesel migration run && \
    cargo watch -x run



FROM base AS build
# build dependencies
COPY Cargo.toml Cargo.lock ./
RUN echo 'fn main() {}' >dummy.rs
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
RUN rm dummy.rs
# build executable
COPY . .
RUN cargo build --release
#
CMD diesel migration run



FROM debian:buster-slim AS prod
RUN apt update
RUN apt install -y libpq-dev ca-certificates libssl-dev
COPY --from=build /target/release/api /usr/local/bin/
COPY --from=build /${CMD_HELP_DIR} /usr/local/share/help
CMD ["api"]
