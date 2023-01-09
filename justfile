# See https://just.systems/man/en

#
# Configuration
#

export RUST_BACKTRACE := env_var_or_default("RUST_BACKTRACE", "short")

# By default we compile in development mode mode because it's faster.
build_type := if env_var_or_default("CARGO_RELEASE", "") == "" { "debug" } else { "release" }

toolchain := ""
_cargo := env_var_or_default("CARGO", "cargo" + if toolchain != "" { " +" + toolchain } else { "" })

# The version name to use for packages.
package_version := env_var_or_default("PACKAGE_VERSION", `git rev-parse --short HEAD`)

# The architecture name to use for packages. Either 'amd64', 'arm64', or 'arm'.
package_arch := env_var_or_default("ARCH", "amd64")

# If a `package_arch` is specified, then we change the default cargo `--target`
# to support cross-compilation. Otherwise, we use `rustup` to find the default.
_cargo_target := if package_arch == "amd64" {
        "x86_64-unknown-linux-gnu"
    } else if package_arch == "arm64" {
        "aarch64-unknown-linux-gnu"
    } else if package_arch == "arm" {
        "armv7-unknown-linux-gnueabihf"
    } else {
        `rustup show | sed -n 's/^Default host: \(.*\)/\1/p'`
    }

# Support cross-compilation when `package_arch` changes.
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER := "aarch64-linux-gnu-gcc"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER := "arm-linux-gnueabihf-gcc"
_strip := if package_arch == "arm64" { "aarch64-linux-gnu-strip" } else if package_arch == "arm" { "arm-linux-gnueabihf-strip" } else { "strip" }

_target_dir := "target" / _cargo_target / build_type
_target_bin := _target_dir / "linkerd-await"
_package_name := "linkerd-await-" + package_version + "-" + package_arch
_package_dir := "target/package" / _package_name
_shasum := "shasum -a 256"

# If we're running in Github Actions and cargo-action-fmt is installed, then add
# a command suffix that formats errors.
_fmt := if env_var_or_default("GITHUB_ACTIONS", "") != "true" { "" } else {
    ```
    if command -v cargo-action-fmt >/dev/null 2>&1; then
        echo "--message-format=json | cargo-action-fmt"
    fi
    ```
}

#
# Recipes
#

# Run all tests and build linkerd-await
default: fetch lint test build

# Fetch dependencies
fetch:
    {{ _cargo }} fetch --locked

fmt:
    {{ _cargo }} fmt

# Fails if the code does not match the expected format (via rustfmt).
check-fmt:
    {{ _cargo }} fmt -- --check

# Run all lints
lint: clippy doc check-fmt md-lint actionlint actions-dev-versions

md-lint:
    markdownlint-cli2 '**/*.md' '!**/node_modules' '!**/target'

# Format actionlint output for Github Actions if running in CI.
_actionlint-fmt := if env_var_or_default("GITHUB_ACTIONS", "") != "true" { "" } else {
  '{{range $err := .}}::error file={{$err.Filepath}},line={{$err.Line}},col={{$err.Column}}::{{$err.Message}}%0A```%0A{{replace $err.Snippet "\\n" "%0A"}}%0A```\n{{end}}'
}

# Lints all GitHub Actions workflows
actionlint:
    actionlint {{ if _actionlint-fmt != '' { "-format '" + _actionlint-fmt + "'" } else { "" } }} .github/workflows/*

# Ensure all devcontainer versions are in sync
actions-dev-versions:
    #!/usr/bin/env bash
    set -euo pipefail
    IMAGE=$(j5j .devcontainer/devcontainer.json |jq -r '.image')
    check_image() {
        if [ "$1" != "$IMAGE" ]; then
            # Report all line numbers with the unexpected image.
            for n in $(grep -nF "$1" "$2" | cut -d: -f1) ; do
                if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
                    echo "::error file=${2},line=${n}::Expected image '${IMAGE}'; found '${1}'">&2
                else
                    echo "${2}:${n}: Expected image '${IMAGE}'; found '${1}'" >&2
                fi
            done
            return 1
        fi
    }
    EX=0
    # Check workflows for devcontainer images
    for f in .github/workflows/* ; do
        # Find all container images that look like our dev image, dropping the
        # `-suffix` from the tag.
        for i in $(yq '.jobs.* |  (.container | select(.) // .container.image | select(.)) | match("ghcr.io/linkerd/dev:v[0-9]+").string' < "$f") ; do
            if ! check_image "$i" "$f" ; then
                EX=$((EX+1))
                break
            fi
        done
    done
    # Check actions for devcontainer images
    while IFS= read -r f ; do
        for i in $(awk 'toupper($1) ~ "FROM" { print $2 }' "$f" \
                    | sed -Ene 's,(ghcr\.io/linkerd/dev:v[0-9]+).*,\1,p')
        do
            if ! check_image "$i" "$f" ; then
                EX=$((EX+1))
                break
            fi
        done
    done < <(find .github/actions -name Dockerfile\*)
    exit $EX

check *flags:
    {{ _cargo }} check --all-targets --frozen {{ flags }} {{ _fmt }}

clippy *flags:
    {{ _cargo }} clippy --all-targets --frozen {{ flags }} {{ _fmt }}

doc *flags:
    {{ _cargo }} doc --no-deps --frozen {{ flags }} {{ _fmt }}

test-build:
    {{ _cargo }} nextest run --no-run --frozen \
        {{ if build_type == "release" { "--release" } else { "" } }} \
        {{ _fmt }}

# Run all tests
test *flags:
    {{ _cargo }} nextest run --frozen \
        {{ if build_type == "release" { "--release" } else { "" } }} \
        {{ flags }}

# Build linkerd-await
build:
    {{ _cargo }} build --frozen --target={{ _cargo_target }} \
        {{ if build_type == "release" { "--release" } else { "" } }} \
        {{ _fmt }}

release: fetch build
    @mkdir -p release
    cp {{ _target_bin }} release/{{ _package_name }}
    {{ _strip }} release/{{ _package_name }}
    {{ _shasum }} release/{{ _package_name }} >release/{{ _package_name }}.shasum

# Display the git history minus dependabot updates
history *paths='.':
    @git log --oneline --graph --invert-grep --author="dependabot" -- {{ paths }}

# vim: set ft=make :
