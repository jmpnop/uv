# Contributing

## Finding ways to help

We label issues that would be good for a first time contributor as
[`good first issue`](https://github.com/astral-sh/uv/issues?q=is%3Aopen+is%3Aissue+label%3A%22good+first+issue%22).
These usually do not require significant experience with Rust or the uv code base.

We label issues that we think are a good opportunity for subsequent contributions as
[`help wanted`](https://github.com/astral-sh/uv/issues?q=is%3Aopen+is%3Aissue+label%3A%22help+wanted%22).
These require varying levels of experience with Rust and uv. Often, we want to accomplish these
tasks but do not have the resources to do so ourselves.

You don't need our permission to start on an issue we have labeled as appropriate for community
contribution as described above. However, it's a good idea to indicate that you are going to work on
an issue to avoid concurrent attempts to solve the same problem.

Please check in with us before starting work on an issue that has not been labeled as appropriate
for community contribution. We're happy to receive contributions for other issues, but it's
important to make sure we have consensus on the solution to the problem first.

Outside of issues with the labels above, issues labeled as
[`bug`](https://github.com/astral-sh/uv/issues?q=is%3Aopen+is%3Aissue+label%3A%22bug%22) are the
best candidates for contribution. In contrast, issues labeled with `needs-decision` or
`needs-design` are _not_ good candidates for contribution. Please do not open pull requests for
issues with these labels.

Please do not open pull requests for new features without prior discussion. While we appreciate
exploration of new features, we will almost always close these pull requests immediately. Adding a
new feature to uv creates a long-term maintenance burden and requires strong consensus from the uv
team before it is appropriate to begin work on an implementation.

## Use of AI

We **require all use of AI in contributions to follow our
[AI Policy](https://github.com/astral-sh/.github/blob/main/AI_POLICY.md)**.

If your contribution does not follow the policy, it will be closed.

## Setup

[Rust](https://rustup.rs/) (and a C compiler) are required to build uv.

On Ubuntu and other Debian-based distributions, you can install a C compiler with:

```shell
sudo apt install build-essential
```

On Fedora-based distributions, you can install a C compiler with:

```shell
sudo dnf install gcc
```

On Windows, [NASM](https://www.nasm.us/) is required for building the TLS backend (`aws-lc-sys`). If
it is not present, a prebuilt blob provided by `aws-lc-sys` will be used instead. WinGet can be used
to install NASM:

```shell
winget install NASM.NASM
```

After installation, add `C:\Program Files\NASM` to your `PATH`. While the prebuilt blob will not be
used when NASM is found, you can guarantee this behavior by setting `AWS_LC_SYS_PREBUILT_NASM=0`.

## Testing

For running tests, we recommend [nextest](https://nexte.st/).

To run a specific test by name:

```shell
cargo nextest run -E 'test(test_name)'
```

To run all tests and accept snapshot changes:

```shell
cargo insta test --accept --test-runner nextest
```

To update snapshots for a specific test:

```shell
cargo insta test --accept --test-runner nextest -- <test_name>
```

### Python

Testing uv requires multiple specific Python versions; they can be installed with:

```shell
cargo run python install
```

The storage directory can be configured with `UV_PYTHON_INSTALL_DIR`. (It must be an absolute path.)

### Snapshot testing

uv uses [insta](https://insta.rs/) for snapshot testing. It's recommended (but not necessary) to use
`cargo-insta` for a better snapshot review experience. See the
[installation guide](https://insta.rs/docs/cli/) for more information.

In tests, you can use `uv_snapshot!` macro to simplify creating snapshots for uv commands. For
example:

```rust
#[test]
fn test_add() {
    let context = TestContext::new("3.12");
    uv_snapshot!(context.filters(), context.add().arg("requests"), @"");
}
```

To run and review a specific snapshot test:

```shell
cargo test --package <package> --test <test> -- <test_name> -- --exact
cargo insta review
```

A script is available to update the snapshots based on results in CI. This is useful for updating
snapshots without re-running the test suite and for updating platform-specific snapshots.

```shell
./scripts/apply-ci-snapshots.sh
```

### Git and Git LFS

A subset of uv tests require both [Git](https://git-scm.com) and [Git LFS](https://git-lfs.com/) to
execute properly.

These tests can be disabled by turning off either `git` or `git-lfs` uv features.

### Local testing

You can invoke your development version of uv with `cargo run -- <args>`. For example:

```shell
cargo run -- venv
cargo run -- pip install requests
```

## Formatting

```shell
# Rust
cargo fmt --all

# Python
uvx ruff format .

# Markdown, YAML, and other files (requires Node.js)
npx prettier --write .
# or in Docker
docker run --rm -v .:/src/ -w /src/ node:alpine npx prettier --write .
```

## Linting

Linting requires [shellcheck](https://github.com/koalaman/shellcheck) and
[cargo-shear](https://github.com/Boshen/cargo-shear) to be installed separately.

```shell
# Rust
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Python
uvx ruff check .

# Python type checking
uvx ty check python/uv

# Shell scripts
shellcheck <script>

# Spell checking
uvx typos

# Unused Rust dependencies
cargo shear
```

### Compiling for Windows from Unix

To run clippy for a Windows target from Linux or macOS, you can use
[cargo-xwin](https://github.com/rust-cross/cargo-xwin):

```shell
# Install cargo-xwin
cargo install cargo-xwin --locked

# Add the Windows target
rustup target add x86_64-pc-windows-msvc

# Run clippy for Windows
cargo xwin clippy --workspace --all-targets --all-features --locked -- -D warnings
```

## Crate structure

Rust does not allow circular dependencies between crates. To visualize the crate hierarchy, install
[cargo-depgraph](https://github.com/jplatte/cargo-depgraph) and graphviz, then run:

```shell
cargo depgraph --dedup-transitive-deps --workspace-only | dot -Tpng > graph.png
```

## Running inside a Docker container

Source distributions can run arbitrary code on build and can make unwanted modifications to your
system
(["Someone's Been Messing With My Subnormals!" on Blogspot](https://moyix.blogspot.com/2022/09/someones-been-messing-with-my-subnormals.html),
["nvidia-pyindex" on PyPI](https://pypi.org/project/nvidia-pyindex/)), which can even occur when
just resolving requirements. To prevent this, there's a Docker container you can run commands in:

```console
$ docker build -t uv-builder -f crates/uv-dev/builder.dockerfile --load .
# Build for musl to avoid glibc errors, might not be required with your OS version
cargo build --target x86_64-unknown-linux-musl --profile profiling
docker run --rm -it -v $(pwd):/app uv-builder /app/target/x86_64-unknown-linux-musl/profiling/uv-dev resolve-many --cache-dir /app/cache-docker /app/scripts/popular_packages/pypi_10k_most_dependents.txt
```

We recommend using this container if you don't trust the dependency tree of the package(s) you are
trying to resolve or install.

## Profiling and Benchmarking

Please refer to Ruff's
[Profiling Guide](https://github.com/astral-sh/ruff/blob/main/CONTRIBUTING.md#profiling-projects),
it applies to uv, too.

We provide diverse sets of requirements for testing and benchmarking the resolver in
`test/requirements` and for the installer in `test/requirements/compiled`.

You can use `scripts/benchmark` to benchmark predefined workloads between uv versions and with other
tools, e.g., from the `scripts/benchmark` directory:

```shell
uv run resolver \
    --uv-pip \
    --poetry \
    --benchmark \
    resolve-cold \
    ../test/requirements/trio.in
```

### Analyzing concurrency

You can use [tracing-durations-export](https://github.com/konstin/tracing-durations-export) to
visualize parallel requests and find any spots where uv is CPU-bound. Example usage, with `uv` and
`uv-dev` respectively:

```shell
RUST_LOG=uv=info TRACING_DURATIONS_FILE=target/traces/jupyter.ndjson cargo run --features tracing-durations-export --profile profiling -- pip compile test/requirements/jupyter.in
```

```shell
RUST_LOG=uv=info TRACING_DURATIONS_FILE=target/traces/jupyter.ndjson cargo run --features tracing-durations-export --bin uv-dev --profile profiling -- resolve jupyter
```

### Trace-level logging

You can enable `trace` level logging using the `RUST_LOG` environment variable, i.e.

```shell
RUST_LOG=trace uv
```

## Documentation

To preview any changes to the documentation locally:

1. Install the [Rust toolchain](https://www.rust-lang.org/tools/install).

2. Run `cargo dev generate-all`, to update any auto-generated documentation.

3. Run the development server with:

   ```shell
   uv run --only-group docs mkdocs serve -f mkdocs.yml
   ```

The documentation should then be available locally at
[http://127.0.0.1:8000/uv/](http://127.0.0.1:8000/uv/).

Documentation is deployed automatically on release by publishing to the
[Astral documentation](https://github.com/astral-sh/docs) repository, which itself deploys via
Cloudflare Pages.

After making changes to the documentation, [format the markdown files](#formatting) using Prettier.

## Development code signing on macOS

This section covers signing a _local test build_ so the macOS keychain approves it across
recompiles. It is independent of **release** signing, which uses this fork's Developer ID identity
and is automated in `scripts/publish-release.sh` (see [Releases](#releases)).

Code signing on macOS can improve developer experience when running tests, e.g., when running tests
that access the macOS keychain, a signed binary can be approved once but an unsigned binary will
need to be approved on each re-compile.

### Acquiring a development certificate

1. Generate a
   [request for the certificate](https://developer.apple.com/help/account/certificates/create-a-certificate-signing-request)
2. Create a certificate in the
   [Apple Developer portal](https://developer.apple.com/account/resources/certificates/list)
3. Download and install the certificate to your login keychain

   ```shell
   security import ~/Downloads/mac_development.cer -k ~/Library/Keychains/login.keychain-db
   ```

4. Identify your code signing identity

   ```shell
   security find-identity -v -p codesigning
   ```

5. If the above fails to find your identity, install the intermediate certificates

   ```shell
   curl -sLO "https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer"
   security import AppleWWDRCAG3.cer -k ~/Library/Keychains/login.keychain-db
   rm AppleWWDRCAG3.cer
   ```

6. Set `UV_TEST_CODESIGN_IDENTITY`

   ```shell
   export UV_TEST_CODESIGN_IDENTITY="Mac Developer: Your Name (TEAM_ID)"
   ```

Note `UV_TEST_CODESIGN_IDENTITY` is only supported via `nextest`.

## Releases

> This fork releases locally — there is no GitHub Actions release workflow. The upstream
> `scripts/release.sh` / `release.yml` flow does not apply here.

The fork ships **macOS-only** binaries (Intel + Apple Silicon) from a maintainer's Mac via
[`scripts/publish-release.sh`](scripts/publish-release.sh).

### Tagging convention

Releases are tagged `vX.Y.Z-N`, where `X.Y.Z` is the upstream uv version this fork is based on and
`-N` is the fork build number (a [PEP 440](https://peps.python.org/pep-0440/) post-release, so
`v0.11.24-2` > `v0.11.24-1` > `v0.11.24`). Bump `-N` whenever you cut a new build of the same
upstream base — `uv self update` keys off the version, so a same-version re-upload is **not** picked
up by existing installs.

### Cutting a release

```shell
# 1. Commit your changes, then tag (annotated):
git tag -a v0.11.24-4 -m "v0.11.24-4"
git push fork v0.11.24-4

# 2. Build, sign, notarize, and publish both macOS targets:
./scripts/publish-release.sh v0.11.24-4
```

`publish-release.sh` builds each target with `--features self-update`, stamps the tag into the
binary via `UV_FORK_VERSION`, **signs** `uv`/`uvx` with the Developer ID Application identity
(hardened runtime + secure timestamp), **notarizes** all targets in a single `notarytool`
submission, packages tarballs + checksums, and uploads them with `gh release create`. Useful
environment variables:

- `DRY_RUN=1` — build + package locally, skip upload and notarization.
- `SIGN=0` — skip signing and notarization (ships unsigned).
- `TARGETS="..."` — override the target list.

Signing requires the Developer ID identity and App Store Connect notary key in
`~/.config/macos-codesign/`; without them the build still succeeds but warns and ships unsigned.
Because bare CLI binaries cannot have a notarization ticket stapled, notarization is recognized
online — fine for the `curl | sh` installer, which extracts with `tar` and sets no quarantine.
