ARG BASE_IMAGE=rust:1.50.0-buster
FROM $BASE_IMAGE
WORKDIR /linkerd

RUN apt-get update && \
    apt-get install -y jq && \
    apt-get install -y musl-tools docker.io && \
    apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/

RUN cargo install cross
