ARG RUST_IMAGE=rust:1.33-slim-stretch
FROM $RUST_IMAGE as build
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src/linkerd-await
RUN mkdir -p src && touch src/lib.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked
COPY src src
RUN cargo build --frozen --release --target=x86_64-unknown-linux-musl

FROM scratch
COPY --from=build \
    /usr/src/linkerd-await/target/x86_64-unknown-linux-musl/release/linkerd-await \
    /linkerd-await
ENTRYPOINT ["/linkerd-await", "--"]
