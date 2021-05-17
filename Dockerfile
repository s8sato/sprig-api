FROM rust:1.48 AS base
RUN cargo install diesel_cli --no-default-features --features postgres

FROM base AS dev
RUN cargo install cargo-watch
WORKDIR /usr/local/src
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

FROM base AS prod
WORKDIR /usr/local/src
COPY . .
RUN cargo build --release
RUN cp target/release/api /usr/local/bin/
# TODO diesel migration run
CMD ["api"]
