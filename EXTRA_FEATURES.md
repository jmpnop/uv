# Extra Features

This document describes features added on top of upstream uv. Each entry covers the user-facing
surface (TOML, env vars, CLI flags), the problem it solves, and concrete use cases.

## Custom Python indexes (`[[python-indexes]]`)

### Summary

uv's managed-Python feature (`uv python install`) ships with a hard-coded list of distributions from
[`python-build-standalone`](https://github.com/astral-sh/python-build-standalone). This feature lets
users point uv at **additional** JSON manifests that follow the same schema, either to _augment_ the
built-in list with custom builds or to _replace_ it entirely.

### Configuration surface

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

### Use cases

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

### Semantics

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

### Safety invariants

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

### Error UX

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

### Index format

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

### Test coverage

- 23+ integration tests in `crates/uv/tests/it/python_list.rs` and `python_find.rs` covering:
  happy-path merge, same-key override, higher-version protection, multiple defaults, missing /
  malformed / null sha256, unsupported scheme, plain-HTTP rejection, loopback exception
  (IPv4+localhost), IPv6 loopback, file:// URLs, file-not-found, duplicate names, unknown TOML
  fields, reserved `$` prefix, CLI+config merge, env var, malformed URLs.
- Unit tests for `PythonInstallMirrors::combine` (layer dedup) and `is_loopback_http` (all host
  kinds).

### Relation to existing options

- `python-install-mirror` / `UV_PYTHON_INSTALL_MIRROR` — swaps just the hostname of built-in
  `python-build-standalone` download URLs. Narrower scope.
- `python-downloads-json-url` / `UV_PYTHON_DOWNLOADS_JSON_URL` — replaces the built-in manifest with
  a single JSON URL. Effectively equivalent to a single `[[python-indexes]]` entry with
  `default = true`. Kept for backwards compatibility.
