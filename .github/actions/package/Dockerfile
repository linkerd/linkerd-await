FROM ghcr.io/linkerd/dev:v32-rust
WORKDIR /linkerd

RUN apt-get update && \
    apt-get install -y musl-tools docker.io && \
    apt-get install -y binutils-x86-64-linux-gnu binutils-aarch64-linux-gnu binutils-arm-linux-gnueabihf && \
    apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/

RUN cargo install cross
