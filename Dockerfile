ARG cmd_help_dir
ARG work_dir



FROM rust:1.52 AS base
ENV WORK_DIR $work_dir
WORKDIR $WORK_DIR
RUN cargo install diesel_cli --no-default-features --features postgres



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
COPY --from=build ${work_dir}/target/release/api /usr/local/bin/
COPY --from=build ${work_dir}/src/handlers/app/_cmd_help ${cmd_help_dir}
CMD ["api"]



FROM base AS dev
RUN cargo install cargo-watch
# build dependencies
COPY Cargo.toml Cargo.lock ./
RUN echo 'fn main() {}' >dummy.rs
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
RUN rm dummy.rs
#
COPY . .
CMD cargo watch -x run
