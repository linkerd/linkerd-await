# linkerd-await

A command-wrapper that polls Linkerd for readiness until it becomes ready and
only then executes a command.

## Usage

```text
linkerd-await 0.2.9
Wait for linkerd to become ready before running a program

Usage: linkerd-await [OPTIONS] [CMD] [ARGS]...

Arguments:
  [CMD]      The command to run after linkerd is ready
  [ARGS]...  Arguments to pass to CMD if specified

Options:
  -p, --port <PORT>
          The port of the local Linkerd proxy admin server [default: 4191]
  -b, --backoff <BACKOFF>
          Time to wait after a failed readiness check [default: 1s]
  -S, --shutdown
          Forks the program and triggers proxy shutdown on completion
  -v, --verbose
          Causes linkerd-await to print an error message when disabled [env: LINKERD_AWAIT_VERBOSE=]
  -t, --timeout <TIMEOUT>
          Causes linked-await to fail when the timeout elapses before the proxy becomes ready
      --timeout-fatal[=<TIMEOUT_FATAL>]
          Controls whether a readiness timeout failure prevents CMD from running [default: true] [possible values: true, false]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Examples

### Dockerfile

```dockerfile
# Create a base layer with linkerd-await from a recent release.
FROM docker.io/curlimages/curl:latest as linkerd
ARG LINKERD_AWAIT_VERSION=v0.3.0
RUN curl -sSLo /tmp/linkerd-await https://github.com/linkerd/linkerd-await/releases/download/release%2F${LINKERD_AWAIT_VERSION}/linkerd-await-${LINKERD_AWAIT_VERSION}-amd64 && \
    chmod 755 /tmp/linkerd-await

# Build your application with whatever environment makes sense.
FROM myapp-build as app
WORKDIR /app
RUN make build

# Package the application wrapped by linkerd-await. Note that the binary is
# static so it can be used in `scratch` images:
FROM scratch
COPY --from=linkerd /tmp/linkerd-await /linkerd-await
COPY --from=app /app/myapp /myapp
# In this case, we configure the proxy to be shutdown after `myapp` completes
# running. This is only really needed for jobs where the application is
# expected to complete on its own (namely, `Jobs` and `Cronjobs`)
ENTRYPOINT ["/linkerd-await", "--shutdown", "--"]
CMD  ["/myapp"]
```

### Disabling `linkerd-await` at runtime

The `LINKERD_AWAIT_DISABLED` (or `LINKERD_DISABLED`) environment variable can
be set to bypass `linkerd-await`'s readiness checks. This way,
`linkerd-await` may be controlled by overriding a default environment
variable:

```yaml
    # ...
    spec:
      containers:
        - name: myapp
          env:
            - name: LINKERD_AWAIT_DISABLED
              value: "Linkerd is disabled ;("
          # ...
```

<!-- markdownlint-configure-file { "MD013": { "code_blocks": false } } -->
