# Custom Python index pipeline

Scripts for building patched / experimental CPython flavors, packaging them as
python-build-standalone-compatible tarballs, and publishing them as a custom `[[python-indexes]]`
target alongside the uv fork.

**No GitHub Actions** — every step runs locally. `gh` is used only as a file-upload channel to a
single rolling Release (default tag: `python-builds`).

## One-liner

Edit `config.toml`, then:

```bash
uv run scripts/python_indexes/release.py all
```

This does: build every flavor → pack each into an install-only tarball → generate `versions.json` →
upload everything to the `python-builds` Release.

## Subcommands

| Command        | Purpose                                                           |
| -------------- | ----------------------------------------------------------------- |
| `build [F...]` | Run `configure` + `make` + `make install DESTDIR=…` for a flavor. |
| `pack [F...]`  | Take the staged tree for a flavor and produce install tarballs.   |
| `index`        | Scan `dist/python/` and emit `versions.json` in uv's schema.      |
| `publish`      | Upload tarballs + `versions.json` to the configured Release tag.  |
| `all`          | `build` → `pack` → `index` → `publish`, every flavor.             |
| `clean`        | `rm -rf dist/python/` (staged trees and packaged tarballs).       |

Each command accepts `--help` for full flag documentation. All commands take a `--config PATH`
option; defaults to `config.toml` alongside the script.

## How it plugs into the pipeline

1. **Build**: per-flavor `build_<name>/` dir next to the CPython source tree;
   `make install DESTDIR=<dist_dir>/stage/<name>` produces a staged install.
2. **Pack**: the staged tree is tarred into
   `cpython-<version>[+variant]-<os>-<arch>-<libc>-install_only.tar.gz` in `<dist_dir>`, with a
   matching `.sha256` sidecar.
3. **Index**: `_index.py` regex-matches every tarball in `<dist_dir>`, maps it back to a
   `JsonPythonDownload` (see `_schema.py`), and writes `versions.json`. The `url` field uses
   `publish.url_prefix` from the config.
4. **Publish**: creates (or reuses) the configured Release tag on `publish.repo` via
   `gh release create` / `gh release upload --clobber`.

`versions.json` is the exact shape uv's `ManagedPythonDownloadList::new` consumes — the same schema
as `python-build-standalone`'s `download-metadata.json`.

## Consuming the index

Once `publish` completes, users point their `uv.toml` at the published JSON:

```toml
[[python-indexes]]
name = "jmpnop-python"
url  = "https://github.com/jmpnop/uv/releases/download/python-builds/versions.json"
```

Then:

```bash
uv python install cpython@3.14.4               # default flavor (no variant)
uv python install cpython@3.14.4+jit           # jit flavor
uv python install cpython@3.14.4+freethreaded  # no-GIL flavor
```

## Requirements

- Python 3.11+ (for `tomllib`)
- `gh` (GitHub CLI): `brew install gh`
- CPython build deps. On macOS: Xcode CLT, `brew install openssl@3 xz gdbm mpdecimal pkg-config`.
- Optional: `rustup` isn't involved here (no Rust in the Python pipeline).

## File layout

```
scripts/python_indexes/
├── README.md          this file
├── config.toml        declarative flavor + publish config
├── release.py         CLI entry point (subcommands: all / build / pack / …)
├── _schema.py         dataclasses mirroring uv's JsonPythonDownload
├── _build.py          configure + make + staged install
├── _pack.py           tar a staged tree into an install_only.tar.gz
├── _index.py          emit versions.json
└── _publish.py        upload via `gh release create` / `gh release upload`
```

`dist/python/` (created at runtime) is where staged trees and packaged tarballs live. It's
gitignored by the repo root's `.gitignore` via the `dist/` rule.
