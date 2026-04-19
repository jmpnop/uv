"""Generate the ``versions.json`` that uv's `[[python-indexes]]` consumes.

The JSON is a top-level object keyed by installation-key strings. Every value
matches the shape of ``uv_python::downloads::JsonPythonDownload`` — see
``_schema.py`` for the dataclass mirror.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from ._schema import Config, JsonArch, JsonPythonDownload


# cpython-3.14.4[+variant]-<os>-<arch>-<libc>-install_only.tar.gz
_FILENAME_RE = re.compile(
    r"^cpython-(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)"
    r"(?:(?P<prerelease>(?:a|b|rc)\d+))?"
    r"(?:\+(?P<variant>[A-Za-z0-9_]+))?"
    r"-(?P<os>[a-z0-9]+)-(?P<arch>[a-z0-9_]+)-(?P<libc>[a-z]+)"
    r"-install_only\.tar\.gz$"
)


def generate(cfg: Config) -> Path:
    """Scan ``dist_dir`` for tarballs and produce ``versions.json``.

    Returns the path to the written file.
    """
    if cfg.publish is None:
        raise RuntimeError(
            "config has no [publish] section — cannot compute url_prefix"
        )

    dist_dir = Path(cfg.python.dist_dir).expanduser().resolve()
    entries: dict[str, dict] = {}

    flavor_build = {f.name: f.build for f in cfg.flavors.values()}
    flavor_by_variant = {f.variant: f for f in cfg.flavors.values()}

    for tarball in sorted(dist_dir.glob("*.tar.gz")):
        match = _FILENAME_RE.match(tarball.name)
        if not match:
            continue

        sha256_file = tarball.with_name(tarball.name + ".sha256")
        sha256 = _read_sha256(sha256_file) if sha256_file.exists() else None

        variant = match.group("variant") or ""
        flavor = flavor_by_variant.get(variant)
        build = flavor.build if flavor else None

        download = JsonPythonDownload(
            name="cpython",
            arch=JsonArch(family=match.group("arch")),
            os=match.group("os"),
            libc=match.group("libc"),
            major=int(match.group("major")),
            minor=int(match.group("minor")),
            patch=int(match.group("patch")),
            prerelease=match.group("prerelease") or "",
            url=cfg.publish.url_prefix + tarball.name,
            sha256=sha256,
            variant=variant or None,
            build=build,
        )
        entries[download.installation_key()] = download.to_dict()

    out = dist_dir / "versions.json"
    out.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
    _say(f"wrote {out} ({len(entries)} entries)")
    return out


def _read_sha256(sha_file: Path) -> str | None:
    line = sha_file.read_text(encoding="ascii").strip()
    return line.split()[0] if line else None


def _say(msg: str) -> None:
    print(f"\033[1;36m==>\033[0m {msg}", flush=True)
