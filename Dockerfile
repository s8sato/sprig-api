FROM rust:1.48 AS base
RUN cargo install diesel_cli --no-default-features --features postgres

FROM base AS dev
RUN cargo install cargo-watch
WORKDIR /usr/local/src
# keep binaries out of volume
ENV CARGO_TARGET_DIR=/tmp/target
# build dependencies
COPY Cargo.toml Cargo.lock ./
RUN echo 'fn main() {}' >dummy.rs
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
RUN rm dummy.rs
#
COPY . .
CMD if [ -f "___migration___" ]; then \
  diesel migration run; \
  rm ___migration___; \
  fi && \
  cargo watch -x run

FROM base AS build
WORKDIR /usr/local/src
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
RUN apt-get update
RUN apt-get install libpq-dev -y
COPY --from=build /usr/local/src/target/release/api /usr/local/bin/
CMD ["api"]
