"""Pack a staged Python install tree into a python-build-standalone-style tarball.

Given a stage directory produced by ``_build.make_install`` (which installs
into ``<stage>/usr/local``), produce a tarball with this layout::

    python/
      bin/python3.14
      lib/python3.14/
      ...

The tarball is named per python-build-standalone convention:

    cpython-<MAJOR.MINOR.PATCH>[+variant]-<os>-<arch>-<libc>-install_only.tar.gz

Each tarball gets a sibling ``.sha256`` file suitable for uploading alongside.
"""

from __future__ import annotations

import hashlib
import shutil
import sys
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path

from ._schema import Config, Flavor


@dataclass(frozen=True)
class Artifact:
    """A packaged tarball ready to upload."""

    path: Path
    sha256: str
    arch: str
    flavor: Flavor


def pack_flavor(cfg: Config, flavor: Flavor, stage_dir: Path) -> list[Artifact]:
    """Produce one tarball per configured arch.

    Universal2 builds ship a single fat binary that runs on both Apple Silicon
    and Intel. uv's installation-key schema doesn't understand "universal2", so
    we publish *the same tarball* under each arch-specific name. Slightly
    wasteful on disk, but cleaner than teaching uv about universal2.
    """
    dist_dir = Path(cfg.python.dist_dir).expanduser().resolve()
    dist_dir.mkdir(parents=True, exist_ok=True)

    # The `make install` under DESTDIR puts everything under `<stage>/usr/local`.
    # Hoist that up so the tar's top-level is `python/`.
    root = _find_install_root(stage_dir)

    major, minor, patch = cfg.python.major_minor_patch
    variant_suffix = f"+{flavor.variant}" if flavor.variant else ""

    # One *actual* tarball (same bytes) copied per arch name so URLs match
    # what versions.json advertises. We pack once to a temp dir then copy.
    with tempfile.TemporaryDirectory() as tmp:
        tmp_tar = Path(tmp) / "python.tar.gz"
        _create_tar(tmp_tar, root)

        artifacts = []
        for arch in cfg.python.arches:
            version = f"{major}.{minor}.{patch}{variant_suffix}"
            name = (
                f"cpython-{version}-{_os_tag(cfg.python.os)}-{arch}-{cfg.python.libc}"
                f"-install_only.tar.gz"
            )
            out = dist_dir / name
            shutil.copy2(tmp_tar, out)
            digest = _sha256(out)
            out.with_name(out.name + ".sha256").write_text(
                f"{digest}  {out.name}\n", encoding="ascii"
            )
            artifacts.append(
                Artifact(path=out, sha256=digest, arch=arch, flavor=flavor)
            )
            _say(f"packaged {out}")
        return artifacts


def _find_install_root(stage_dir: Path) -> Path:
    """Return the directory that should become the tarball's ``python/`` root.

    For a standard ``DESTDIR=<stage> make install`` of CPython we expect
    ``<stage>/usr/local`` to contain ``bin/``, ``lib/``, ``include/``, ``share/``.
    """
    candidates = [
        stage_dir / "usr" / "local",
        stage_dir / "opt" / "local",
        stage_dir,
    ]
    for c in candidates:
        if (c / "bin").is_dir() and (c / "lib").is_dir():
            return c
    sys.exit(f"error: could not locate python install under {stage_dir}")


def _create_tar(dest: Path, root: Path) -> None:
    """Write ``dest`` as a gzipped tar containing ``python/<root contents>``."""
    with tarfile.open(dest, mode="w:gz") as tf:
        for child in sorted(root.iterdir()):
            tf.add(child, arcname=f"python/{child.name}")


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _os_tag(os_name: str) -> str:
    # uv uses "darwin" for macOS in installation keys.
    return {"macos": "darwin"}.get(os_name, os_name)


def _say(msg: str) -> None:
    print(f"\033[1;36m==>\033[0m {msg}", flush=True)
