ARG RUST_VERSION=1.60.0

FROM docker.io/rust:${RUST_VERSION}-bullseye as cargo-deny
ARG CARGO_DENY_VERSION=0.11.1
RUN curl --proto '=https' --tlsv1.3 -vsSfL "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar zvxf - --strip-components=1 -C /usr/local/bin "cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl/cargo-deny"

FROM docker.io/rust:${RUST_VERSION}-bullseye
RUN rustup component add clippy rustfmt rust-analysis rust-std

COPY --from=cargo-deny /usr/local/bin/cargo-deny /usr/local/bin/cargo-deny

ENV DEBIAN_FRONTEND=noninteractive
RUN apt update && apt upgrade -y
RUN apt install -y --no-install-recommends \
    jo \
    jq \
    locales \
    lsb-release \
    sudo \
    time

RUN sed -i 's/^# *\(en_US.UTF-8\)/\1/' /etc/locale.gen && locale-gen

ARG USER=code
ARG USER_UID=1000
ARG USER_GID=1000
RUN groupadd --gid=$USER_GID $USER \
    && useradd --uid=$USER_UID --gid=$USER_GID -m $USER \
    && echo "$USER ALL=(root) NOPASSWD:ALL" >/etc/sudoers.d/$USER \
    && chmod 0440 /etc/sudoers.d/$USER
USER $USER
