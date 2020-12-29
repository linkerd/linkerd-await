# linkerd-await

A command-wrapper that polls Linkerd for readiness until it becomes ready and only then executes a command.

## Usage

```
linkerd-await 0.2.0
Wait for linkerd to become ready before running a program

USAGE:
    linkerd-await [OPTIONS] [CMD]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --backoff <backoff>    Time to wait after a failed readiness check [default: 1s]
    -p, --port <port>          The port of the local Linkerd proxy admin server [default: 4191]

ARGS:
    <CMD>...
```

## Examples

### Dockerfile

The `linkerd-await` container image contains only a static binary, so it's
possible to use this utility in `scratch` images:

```dockerfile
ARG LINKERD_AWAIT_VERSION=v0.1.3

FROM scratch
RUN curl -vsLO https://github.com/olix0r/linkerd-await/releases/download/release/${LINKERD_AWAIT_VERSION}/linkerd-await
# ... install myapp ..
ENTRYPOINT ["/linkerd-await", "--"]
CMD ["/myapp", "-flags"]
```

In a multi-stage build, `linkerd-await` can be downloaded in a previous stage as follows:

```dockerfile
FROM node:alpine as builder
WORKDIR /app
RUN apk add --update curl && rm -rf /var/cache/apk/*
COPY package*.json ./
RUN npm install --production
COPY . .
ARG LINKERD_AWAIT_VERSION=v0.2.0
RUN curl -vsLO https://github.com/olix0r/linkerd-await/releases/download/release/${LINKERD_AWAIT_VERSION}/linkerd-await && \
  chmod +x linkerd-await

FROM node:alpine
WORKDIR /app
COPY --from=builder /app .
USER 10001
ENTRYPOINT ["./linkerd-await", "--"]
CMD  ["node", "index.js"]
```

Note that the `LINKERD_DISABLED` flag can be set to bypass `linkerd-await`'s
readiness checks. This way, `linkerd-await` may be controlled by overriding a
default environment variable:

```yaml
    # ...
    spec:
      containers:
        - name: myapp
          env:
            - name: LINKERD_DISABLED
              value: "Linkerd is disabled ;("
          # ...
```

## License

linkerd-await is copyright 2019 the Linkerd authors. All rights reserved.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use
these files except in compliance with the License. You may obtain a copy of the
License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
