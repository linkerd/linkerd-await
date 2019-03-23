ARG RUST_IMAGE=registry.gitlab.com/rust_musl_docker/image:stable-1.33.0
ARG RUNTIME_IMAGE=scratch

FROM $RUST_IMAGE as build
WORKDIR /usr/src/linkerd-await
RUN mkdir -p src && touch src/lib.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked
COPY src src
RUN cargo build --frozen --release --target=x86_64-unknown-linux-musl

FROM $RUNTIME_IMAGE as runtime
COPY --from=build \
    /usr/src/linkerd-await/target/x86_64-unknown-linux-musl/release/linkerd-await \
    /linkerd-await
ENTRYPOINT ["/linkerd-await", "--"]
