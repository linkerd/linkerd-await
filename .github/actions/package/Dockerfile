ARG BASE_IMAGE=rust:1.53.0-buster
FROM $BASE_IMAGE
WORKDIR /linkerd

RUN apt-get update && \
    apt-get install -y jq && \
    apt-get install -y musl-tools docker.io && \
    apt-get install -y binutils-x86-64-linux-gnu binutils-aarch64-linux-gnu binutils-arm-linux-gnueabihf && \
    apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/

RUN cargo install cross
