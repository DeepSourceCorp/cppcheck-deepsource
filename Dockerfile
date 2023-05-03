FROM rust:slim-bookworm AS rs_builder

RUN mkdir -p /code
ADD . /code
WORKDIR /code

RUN cargo b --release

FROM us.gcr.io/deepsource-dev/sleipnir_build_env:dev

RUN mkdir -p /toolbox
COPY --from=rs_builder /code/target/release/cppcheck-deepsource /toolbox/

RUN apt install -y cppcheck
