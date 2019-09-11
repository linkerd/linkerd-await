# linkerd-await

A command-wrapper that polls Linkerd for readiness until it becomes ready and only then executes a command.

## Usage

```
linkerd-await 0.1.1
Oliver Gould <ver@buoyant.io>
Wait for linkerd to become ready before running a program.

USAGE:
    linkerd-await [OPTIONS] [CMD]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --backoff <backoff>     [default: 1s]
    -u, --uri <uri>             [default: http://127.0.0.1:4191/ready]

ARGS:
    <CMD>...
```

## Examples

### Dockerfile

The `linkerd-await` container image contains only a static binary, so it's
possible to use this utility in `scratch` images:

```dockerfile
FROM scratch
# ...
COPY --from=olix0r/linkerd-await:v0.1.1 /linkerd-await /
ENTRYPOINT ["/linkerd-await", "--"]
CMD ["/myapp", "-flags"]
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
