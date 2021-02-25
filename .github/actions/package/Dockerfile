ARG BASE_IMAGE=rust:1.50.0-buster
FROM $BASE_IMAGE
WORKDIR /linkerd

RUN apt-get update && \
    apt-get install -y jq && \
    apt-get install -y musl-tools && \
    apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-musl-gcc.sh \
    CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc \
    CXX_aarch64_unknown_linux_musl=aarch64-linux-musl-g++

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER=arm-linux-musleabihf-gcc \
    CC_armv7_unknown_linux_musleabihf=arm-linux-musleabihf-gcc \
    CXX_armv7_unknown_linux_musleabihf=arm-linux-musleabihf-g++

RUN rustup target add aarch64-unknown-linux-musl && \
    rustup target add armv7-unknown-linux-muslabihf
