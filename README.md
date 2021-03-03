# linkerd-await

A command-wrapper that polls Linkerd for readiness until it becomes ready and only then executes a command.

## Usage

```
linkerd-await 0.2.2
Wait for linkerd to become ready before running a program

USAGE:
    linkerd-await [OPTIONS] [CMD]...

FLAGS:
    -h, --help        Prints help information
    -S, --shutdown    Forks the program and triggers proxy shutdown on completion
    -V, --version     Prints version information

OPTIONS:
    -b, --backoff <backoff>    Time to wait after a failed readiness check [default: 1s]
    -p, --port <port>          The port of the local Linkerd proxy admin server [default: 4191]

ARGS:
    <CMD>...
```

## Examples

### Dockerfile

```dockerfile
# Create a base layer with linkerd-await from a recent release.
FROM docker.io/curlimages/curl:latest as linkerd
ARG LINKERD_AWAIT_VERSION=v0.2.2
RUN curl -sSLo /tmp/linkerd-await https://github.com/olix0r/linkerd-await/releases/download/release%2F${LINKERD_AWAIT_VERSION}/linkerd-await-${LINKERD_AWAIT_VERSION}-amd64 && \
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
ENTRYPOINT ["/linkerd-await", "--shutdown" "--"]
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

## License

linkerd-await is copyright 2021 the Linkerd authors. All rights reserved.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use
these files except in compliance with the License. You may obtain a copy of the
License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
