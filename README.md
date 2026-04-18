# uv

[![uv](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/uv/main/assets/badge/v0.json)](https://github.com/astral-sh/uv)
[![image](https://img.shields.io/pypi/v/uv.svg)](https://pypi.python.org/pypi/uv)
[![image](https://img.shields.io/pypi/l/uv.svg)](https://pypi.python.org/pypi/uv)
[![image](https://img.shields.io/pypi/pyversions/uv.svg)](https://pypi.python.org/pypi/uv)
[![Actions status](https://github.com/astral-sh/uv/actions/workflows/ci.yml/badge.svg)](https://github.com/astral-sh/uv/actions)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/astral-sh)

An extremely fast Python package and project manager, written in Rust.

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/astral-sh/uv/assets/1309177/03aa9163-1c79-4a87-a31d-7a9311ed9310">
    <source media="(prefers-color-scheme: light)" srcset="https://github.com/astral-sh/uv/assets/1309177/629e59c0-9c6e-4013-9ad4-adb2bcf5080d">
    <img alt="Shows a bar chart with benchmark results." src="https://github.com/astral-sh/uv/assets/1309177/629e59c0-9c6e-4013-9ad4-adb2bcf5080d">
  </picture>
</p>

<p align="center">
  <i>Installing <a href="https://trio.readthedocs.io/">Trio</a>'s dependencies with a warm cache.</i>
</p>

## Highlights

- A single tool to replace `pip`, `pip-tools`, `pipx`, `poetry`, `pyenv`, `twine`, `virtualenv`, and
  more.
- [10-100x faster](https://github.com/astral-sh/uv/blob/main/BENCHMARKS.md) than `pip`.
- Provides [comprehensive project management](#projects), with a
  [universal lockfile](https://docs.astral.sh/uv/concepts/projects/layout#the-lockfile).
- [Runs scripts](#scripts), with support for
  [inline dependency metadata](https://docs.astral.sh/uv/guides/scripts#declaring-script-dependencies).
- [Installs and manages](#python-versions) Python versions.
- [Runs and installs](#tools) tools published as Python packages.
- Includes a [pip-compatible interface](#the-pip-interface) for a performance boost with a familiar
  CLI.
- Supports Cargo-style [workspaces](https://docs.astral.sh/uv/concepts/projects/workspaces) for
  scalable projects.
- Disk-space efficient, with a [global cache](https://docs.astral.sh/uv/concepts/cache) for
  dependency deduplication.
- Installable without Rust or Python via `curl` or `pip`.
- Supports macOS, Linux, and Windows.

uv is backed by [Astral](https://astral.sh), the creators of
[Ruff](https://github.com/astral-sh/ruff) and [ty](https://github.com/astral-sh/ty).

## Extra features in this fork

<p align="center">
  <img alt="Fork logo" src="assets/fork-logo.webp" width="640">
</p>

This repository is a fork of [`astral-sh/uv`](https://github.com/astral-sh/uv) with additional
features layered on top. Everything from upstream continues to work unchanged — the sections below
describe only what's been added here.

### Custom Python indexes (`[[python-indexes]]`)

uv's managed-Python feature (`uv python install`) ships with a hard-coded list of distributions from
[`python-build-standalone`](https://github.com/astral-sh/python-build-standalone). This fork lets
users point uv at **additional** JSON manifests that follow the same schema, either to _augment_ the
built-in list with custom builds or to _replace_ it entirely.

#### Configuration surface

**TOML** (`uv.toml` or `pyproject.toml`'s `[tool.uv]`):

```toml
[[python-indexes]]
name = "mycorp"
url = "https://python.mycorp.example.com/versions.json"
# default = true  # set to replace the built-in list
```

**Environment variable** (equivalent to one `[[python-indexes]]` entry):

```bash
export UV_PYTHON_INDEX="https://python.mycorp.example.com/versions.json"
```

**CLI flag** (repeatable, available on `uv python list`, `find`, `install`, `upgrade`, `pin`):

```bash
uv python install 3.14 --python-index https://experimental.example.com/jit/versions.json
```

#### Use cases

1. **Corporate fork / internally-signed builds.** Platform teams that build and sign their own
   Python distributions can publish a manifest alongside the binaries and have every developer's
   `uv python install` pick them up transparently without changing build scripts or docs.
2. **Experimental Python builds.** Evaluating a JIT, free-threaded, or LTO-tuned build that hasn't
   landed upstream yet. Point a dev workstation at the experimental index for the duration of the
   evaluation; remove or toggle `default = true` when finished.
3. **Air-gapped CI.** CI runners that cannot reach `github.com/releases/...` host the same
   distributions on an internal file or HTTP server. Configure the index URL once in shared
   `uv.toml` and every subsequent install hits the internal mirror.
4. **One-off version override.** Project X needs a patched `3.12.3` build. Set the override in the
   project's `uv.toml` only; other projects on the same machine continue using upstream.
5. **Multi-tier layering.** A system-wide `uv.toml` defines `name = "mycorp"` at the corporate
   mirror. A user's home config redefines `name = "mycorp"` to their own fork during local
   development without touching the system config.

#### Semantics

- **Layered config.** Higher-priority layers (CLI > env > project > user > system) override
  lower-priority layers _by name_. A `name = "mycorp"` entry in the project config replaces the user
  or system `name = "mycorp"` entry; distinct names coexist.
- **Merge with built-in.** By default, custom indexes _add_ to the built-in list. Entries with the
  same `PythonInstallationKey` (implementation + version + platform) as a built-in override it.
  Entries with a distinct key coexist.
- **Full replacement.** A single `[[python-indexes]]` entry with `default = true` suppresses the
  built-in list entirely. Useful for air-gapped environments.
- **Precedence on `find()`.** After merging sources, `find()` returns the highest-versioned entry
  matching a request regardless of which source contributed it — a lower-versioned custom entry
  never shadows a higher-versioned built-in one.

#### Safety invariants

- **HTTPS required.** Index JSON must be served over HTTPS. Plain HTTP is rejected unless the host
  is loopback (`localhost`, `127.0.0.0/8`, or `::1`), which emits a one-shot warning for local
  testing.
- **Per-entry sha256 required.** Every entry must carry a 64-character hex `sha256`. Missing or
  malformed hashes fail fast at load time with a clear error instead of surfacing later as an opaque
  `HashMismatch` at extraction time.
- **Scheme allow-list.** Only `http`, `https`, and `file` schemes are accepted. Typos like `ftp://`
  or `mailto:` are rejected up-front rather than silently interpreted as filesystem paths.
- **Reserved `$` prefix.** Names starting with `$` are reserved for internally-synthesized entries
  (`$env` from the env var, `$cli-0`, `$cli-1`, etc. from CLI flags). User-supplied TOML names
  starting with `$` are rejected at deserialization.
- **At-most-one default.** Multiple `[[python-indexes]]` with `default = true` across the merged
  configuration is an error — exactly one index may fully replace the built-in list.
- **Per-file uniqueness.** Duplicate names within a single config file are errors; cross-file
  duplicates are resolved by higher-layer-wins.
- **Offline respect.** In `--offline` mode, HTTP sources are skipped with a visible
  `warn_user_once!` warning naming each skipped index — so `uv run` against an already-installed
  interpreter still succeeds without the user being confused about why their index "vanished."
- **`--only-system` / `--only-installed` skip fetching.** `uv python list --only-system` and
  `--only-installed` don't consult downloads, so the remote index isn't fetched in those modes.

#### Error UX

Every error variant names the offending index (by `name`) so the user knows which entry to fix:

- `UnsupportedIndexScheme { name, scheme }` — scheme outside `http/https/file`.
- `CustomIndexInsecureScheme { name, url }` — plain HTTP to a non-loopback host.
- `CustomIndexMissingHash { name, key }` — entry has no `sha256`.
- `CustomIndexInvalidHash { name, key, value }` — `sha256` is the wrong length or non-hex.
- `MultipleDefaultPythonIndexes(count, names)` — more than one `default = true`.
- `DuplicatePythonIndexName(name)` — same name twice in one config file.
- `InvalidFileUrl(url)` — malformed `file://` URL.

Names starting with `$` fail at TOML parse time via a serde `Error::custom` message rather than a
dedicated enum variant — the TOML diagnostic points at the offending `[[python-indexes]]` block and
reads: "Python index name `$x` uses the reserved `$` prefix; `$`-prefixed names are synthesized
internally (e.g. for `UV_PYTHON_INDEX` or `--python-index`)."

#### Index format

Identical to `python-build-standalone`'s `download-metadata.json`. Example entry:

```json
{
  "cpython-3.14.0-linux-x86_64-gnu": {
    "name": "cpython",
    "arch": { "family": "x86_64", "variant": null },
    "os": "linux",
    "libc": "gnu",
    "major": 3,
    "minor": 14,
    "patch": 0,
    "prerelease": "",
    "url": "https://python.mycorp.example.com/cpython-3.14.0-linux-x86_64-gnu.tar.gz",
    "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
    "variant": null,
    "build": "20260101"
  }
}
```

#### Test coverage

- 23+ integration tests in `crates/uv/tests/it/python_list.rs` and `python_find.rs` covering:
  happy-path merge, same-key override, higher-version protection, multiple defaults, missing /
  malformed / null sha256, unsupported scheme, plain-HTTP rejection, loopback exception
  (IPv4+localhost), IPv6 loopback, file:// URLs, file-not-found, duplicate names, unknown TOML
  fields, reserved `$` prefix, CLI+config merge, env var, offline skip, malformed URLs.
- Unit tests for `PythonInstallMirrors::combine` (layer dedup) and `is_loopback_http` (all host
  kinds).

#### Relation to existing options

- `python-install-mirror` / `UV_PYTHON_INSTALL_MIRROR` — swaps just the hostname of built-in
  `python-build-standalone` download URLs. Narrower scope.
- `python-downloads-json-url` / `UV_PYTHON_DOWNLOADS_JSON_URL` — replaces the built-in manifest with
  a single JSON URL. Effectively equivalent to a single `[[python-indexes]]` entry with
  `default = true`. Kept for backwards compatibility.

---

## Installation

This fork ships **source only** — the extra features (e.g. `[[python-indexes]]`) are compiled into
the `uv` binary you build locally. If you don't need the fork's extras, install upstream uv from
[docs.astral.sh/uv/getting-started/installation](https://docs.astral.sh/uv/getting-started/installation/)
instead — it's prebuilt and downloads in seconds.

All build methods below require a Rust toolchain. If you don't have one:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### `cargo install` from the fork

The one-liner. Recommended if you don't want a working clone:

```bash
cargo install --git https://github.com/jmpnop/uv --locked uv
```

The binary lands at `~/.cargo/bin/uv`. Make sure `~/.cargo/bin` is on your `PATH`.

Pin to a specific commit for reproducibility:

```bash
cargo install --git https://github.com/jmpnop/uv --rev <sha> --locked uv
```

### Build from a local clone

Best if you want to modify the source or run the test suite:

```bash
git clone https://github.com/jmpnop/uv
cd uv
cargo install --locked --path crates/uv
```

### Release build without `cargo install`

Produces a release-optimized binary at `target/release/uv` without publishing it to `~/.cargo/bin`:

```bash
cargo build --release --locked -p uv
./target/release/uv --version
```

Copy the binary wherever you want it on your `PATH`.

### Docker / CI

Build once in a multi-stage image, copy the static-linked binary into your runtime stage:

```dockerfile
FROM rust:1-bookworm AS builder
WORKDIR /src
RUN git clone https://github.com/jmpnop/uv . \
    && cargo build --release --locked -p uv

FROM debian:bookworm-slim
COPY --from=builder /src/target/release/uv /usr/local/bin/uv
ENTRYPOINT ["uv"]
```

### Upgrading

`uv self update` is **disabled** in source-built installs — the self-updater looks for signed
release artifacts that this fork doesn't publish. To upgrade, re-run your install command with
`--force`:

```bash
cargo install --git https://github.com/jmpnop/uv --locked --force uv
```

Or, from a clone: `git pull && cargo install --locked --force --path crates/uv`.

### Shell autocompletion

For `uv`:

| Shell             | Command                                                                                                              |
| ----------------- | -------------------------------------------------------------------------------------------------------------------- |
| Bash              | `echo 'eval "$(uv generate-shell-completion bash)"' >> ~/.bashrc`                                                    |
| Zsh               | `echo 'eval "$(uv generate-shell-completion zsh)"' >> ~/.zshrc`                                                      |
| fish              | `echo 'uv generate-shell-completion fish \| source' > ~/.config/fish/completions/uv.fish`                            |
| Elvish            | `echo 'eval (uv generate-shell-completion elvish \| slurp)' >> ~/.elvish/rc.elv`                                     |
| PowerShell / pwsh | `Add-Content -Path $PROFILE -Value '(& uv generate-shell-completion powershell) \| Out-String \| Invoke-Expression'` |

For `uvx`, substitute `uvx --generate-shell-completion <shell>` for the
`uv generate-shell-completion <shell>` form above.

### Uninstallation

Remove the installed binary:

```bash
# If installed via `cargo install`:
cargo uninstall uv

# Otherwise, delete the binary from wherever you placed it.
rm -f ~/.cargo/bin/uv  # or your chosen location
```

Clean up uv's managed state (optional — wipes the cache, installed Pythons, and installed tools):

```bash
uv cache clean
rm -rf "$(uv python dir)"
rm -rf "$(uv tool dir)"
```

See the upstream
[installation documentation](https://docs.astral.sh/uv/getting-started/installation/) for how to
install prebuilt upstream uv via Homebrew, WinGet, PyPI, or the standalone installer.

## Documentation

uv's documentation is available at [docs.astral.sh/uv](https://docs.astral.sh/uv).

Additionally, the command line reference documentation can be viewed with `uv help`.

## Features

### Projects

uv manages project dependencies and environments, with support for lockfiles, workspaces, and more,
similar to `rye` or `poetry`:

```console
$ uv init example
Initialized project `example` at `/home/user/example`

$ cd example

$ uv add ruff
Creating virtual environment at: .venv
Resolved 2 packages in 170ms
   Built example @ file:///home/user/example
Prepared 2 packages in 627ms
Installed 2 packages in 1ms
 + example==0.1.0 (from file:///home/user/example)
 + ruff==0.5.0

$ uv run ruff check
All checks passed!

$ uv lock
Resolved 2 packages in 0.33ms

$ uv sync
Resolved 2 packages in 0.70ms
Checked 1 package in 0.02ms
```

See the [project documentation](https://docs.astral.sh/uv/guides/projects/) to get started.

uv also supports building and publishing projects, even if they're not managed with uv. See the
[publish guide](https://docs.astral.sh/uv/guides/publish/) to learn more.

### Scripts

uv manages dependencies and environments for single-file scripts.

Create a new script and add inline metadata declaring its dependencies:

```console
$ echo 'import requests; print(requests.get("https://astral.sh"))' > example.py

$ uv add --script example.py requests
Updated `example.py`
```

Then, run the script in an isolated virtual environment:

```console
$ uv run example.py
Reading inline script metadata from: example.py
Installed 5 packages in 12ms
<Response [200]>
```

See the [scripts documentation](https://docs.astral.sh/uv/guides/scripts/) to get started.

### Tools

uv executes and installs command-line tools provided by Python packages, similar to `pipx`.

Run a tool in an ephemeral environment using `uvx` (an alias for `uv tool run`):

```console
$ uvx pycowsay 'hello world!'
Resolved 1 package in 167ms
Installed 1 package in 9ms
 + pycowsay==0.0.0.2
  """

  ------------
< hello world! >
  ------------
   \   ^__^
    \  (oo)\_______
       (__)\       )\/\
           ||----w |
           ||     ||
```

Install a tool with `uv tool install`:

```console
$ uv tool install ruff
Resolved 1 package in 6ms
Installed 1 package in 2ms
 + ruff==0.5.0
Installed 1 executable: ruff

$ ruff --version
ruff 0.5.0
```

See the [tools documentation](https://docs.astral.sh/uv/guides/tools/) to get started.

### Python versions

uv installs Python and allows quickly switching between versions.

Install multiple Python versions:

```console
$ uv python install 3.12 3.13 3.14
Installed 3 versions in 972ms
 + cpython-3.12.12-macos-aarch64-none (python3.12)
 + cpython-3.13.9-macos-aarch64-none (python3.13)
 + cpython-3.14.0-macos-aarch64-none (python3.14)

```

Download Python versions as needed:

```console
$ uv venv --python 3.12.0
Using Python 3.12.0
Creating virtual environment at: .venv
Activate with: source .venv/bin/activate

$ uv run --python pypy@3.8 -- python --version
Python 3.8.16 (a9dbdca6fc3286b0addd2240f11d97d8e8de187a, Dec 29 2022, 11:45:30)
[PyPy 7.3.11 with GCC Apple LLVM 13.1.6 (clang-1316.0.21.2.5)] on darwin
Type "help", "copyright", "credits" or "license" for more information.
>>>>
```

Use a specific Python version in the current directory:

```console
$ uv python pin 3.11
Pinned `.python-version` to `3.11`
```

See the [Python installation documentation](https://docs.astral.sh/uv/guides/install-python/) to get
started.

### The pip interface

uv provides a drop-in replacement for common `pip`, `pip-tools`, and `virtualenv` commands.

uv extends their interfaces with advanced features, such as dependency version overrides,
platform-independent resolutions, reproducible resolutions, alternative resolution strategies, and
more.

Migrate to uv without changing your existing workflows — and experience a 10-100x speedup — with the
`uv pip` interface.

Compile requirements into a platform-independent requirements file:

```console
$ uv pip compile requirements.in \
   --universal \
   --output-file requirements.txt
Resolved 43 packages in 12ms
```

Create a virtual environment:

```console
$ uv venv
Using Python 3.12.3
Creating virtual environment at: .venv
Activate with: source .venv/bin/activate
```

Install the locked requirements:

```console
$ uv pip sync requirements.txt
Resolved 43 packages in 11ms
Installed 43 packages in 208ms
 + babel==2.15.0
 + black==24.4.2
 + certifi==2024.7.4
 ...
```

See the [pip interface documentation](https://docs.astral.sh/uv/pip/index/) to get started.

## Contributing

We are passionate about supporting contributors of all levels of experience and would love to see
you get involved in the project. See the
[contributing guide](https://github.com/astral-sh/uv?tab=contributing-ov-file#contributing) to get
started.

## FAQ

#### How do you pronounce uv?

It's pronounced as "you - vee" ([`/juː viː/`](https://en.wikipedia.org/wiki/Help:IPA/English#Key))

#### How should I stylize uv?

Just "uv", please. See the [style guide](./STYLE.md#styling-uv) for details.

#### What platforms does uv support?

See uv's [platform support](https://docs.astral.sh/uv/reference/platforms/) document.

#### Is uv ready for production?

Yes, uv is stable and widely used in production. See uv's
[versioning policy](https://docs.astral.sh/uv/reference/versioning/) document for details.

## Acknowledgements

uv's dependency resolver uses [PubGrub](https://github.com/pubgrub-rs/pubgrub) under the hood. We're
grateful to the PubGrub maintainers, especially [Jacob Finkelman](https://github.com/Eh2406), for
their support.

uv's Git implementation is based on [Cargo](https://github.com/rust-lang/cargo).

Some of uv's optimizations are inspired by the great work we've seen in [pnpm](https://pnpm.io/),
[Orogene](https://github.com/orogene/orogene), and [Bun](https://github.com/oven-sh/bun). We've also
learned a lot from Nathaniel J. Smith's [Posy](https://github.com/njsmith/posy) and adapted its
[trampoline](https://github.com/njsmith/posy/tree/main/src/trampolines/windows-trampolines/posy-trampoline)
for Windows support.

## License

uv is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in uv
by you, as defined in the Apache-2.0 license, shall be dually licensed as above, without any
additional terms or conditions.

<div align="center">
  <a target="_blank" href="https://astral.sh" style="background:none">
    <img src="https://raw.githubusercontent.com/astral-sh/uv/main/assets/svg/Astral.svg" alt="Made by Astral">
  </a>
</div>
