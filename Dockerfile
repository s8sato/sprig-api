# ARG cmd_help_dir
# ARG work_dir



FROM rust:1.52 AS base
ARG work_dir
ENV WORK_DIR $work_dir
WORKDIR $WORK_DIR
RUN cargo install diesel_cli --no-default-features --features postgres



FROM base AS build
# build dependencies
COPY Cargo.toml ./
RUN mkdir src
RUN echo 'fn main() {}' >src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/api*
# build executable
COPY . .
RUN cargo build --release
#
CMD diesel migration run



FROM debian:buster-slim AS prod
RUN apt update
RUN apt install -y libpq-dev ca-certificates libssl-dev
ARG cmd_help_dir
ARG work_dir
COPY --from=build ${work_dir}/target/release/api /usr/local/bin/
COPY --from=build ${work_dir}/src/handlers/app/_cmd_help ${cmd_help_dir}
CMD ["api"]



FROM base AS dev
RUN cargo install cargo-watch
# build dependencies
COPY Cargo.toml ./
RUN mkdir src
RUN echo 'fn main() {}' >src/main.rs
RUN cargo build
RUN rm -f target/debug/deps/api*
#
COPY . .
CMD cargo watch -x run
