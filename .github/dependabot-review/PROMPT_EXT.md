<!-- markdownlint-disable-file MD041 -->
<!-- Appended to PROMPT.md under "## Repository extension (supplied)". -->

## About this repo

`linkerd-await` is a single binary (`src/main.rs`) used as a container
entrypoint. It polls the Linkerd proxy admin server's `/ready` endpoint, then
`exec`s the wrapped command, or with `--shutdown` forks it, forwards SIGTERM to
it, and POSTs `/shutdown` to the proxy when it exits. Release builds are static
musl binaries for linux amd64 and arm64, and a mingw build for windows amd64.
Users drop the binary into `scratch` images, so any new runtime dependency on
libc, OpenSSL, or files in the image breaks them.

## Critical paths in this repo

- **HTTP client.** `hyper`, `hyper-util` (`client::legacy`, `TokioExecutor`),
  `http`, and `http-body-util` make the plain-HTTP/1 `/ready` and `/shutdown`
  calls to the proxy. A change to the legacy client, connection handling, or
  the HTTP/1 feature set can stop readiness from ever succeeding or drop the
  shutdown request, which leaves Jobs hanging.
- **Async runtime and process handling.** `tokio` (current-thread runtime,
  `process`, `signal`, `time`, `select!`) spawns the child, waits on it, and
  handles SIGTERM on unix and console signals on windows. A change to child
  reaping, signal registration, or timer behavior affects every user.
- **Signal forwarding.** `nix` (`process`, `signal` features) sends SIGTERM to
  the child. `nix` is pre-1.0, so a minor bump can rename or move these APIs.
- **CLI.** `clap` (derive, `env`) parses flags and the `LINKERD_AWAIT_VERBOSE`
  environment variable. Users bake these flags into Dockerfiles and pod specs,
  so a change in how flags, defaults, trailing arguments (`CMD`, `ARGS`), or
  `--timeout-fatal[=BOOL]` parse is a user-visible break.
- **Toolchain and build targets.** `rust-toolchain.toml` pins Rust. A
  dependency that raises its MSRV above that pin, or stops building for
  `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, or
  `x86_64-pc-windows-gnu`, breaks the release. CI does not build the release
  targets on dependency PRs, so check the upstream for target or MSRV changes.
- **Supply-chain policy.** `deny.toml` allows only Apache-2.0 and MIT licenses
  and denies duplicate versions. A new transitive dependency with another
  license, or a second copy of a crate, will fail `cargo deny`.
- **Release workflow actions.** `.github/workflows/release.yml` builds and
  publishes the release binaries with `actions/upload-artifact`,
  `actions/download-artifact`, and `softprops/action-gh-release`. A breaking
  change in these runs only on release tags, so PR CI does not catch it.
