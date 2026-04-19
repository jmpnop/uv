"""Build a Python flavor from source: ``configure`` + ``make`` + ``make install``.

Each flavor's artifacts land in ``<source>/build_<flavor>`` (configure + make)
and then a staged install tree at ``<dist_dir>/stage/<flavor>`` (``make install
DESTDIR=…``). The stage dir is what ``_pack.py`` tars up.

Builds are cumulative: re-running ``build.py jit`` re-runs make inside the jit
build dir without re-configuring unless the build dir is missing. Run
``release.py clean`` to wipe everything and start over.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

from ._schema import Config, Flavor


def build_flavor(cfg: Config, flavor: Flavor, *, jobs: int | None = None) -> Path:
    """Build ``flavor`` from source, returning the staged install-tree path."""
    source = Path(cfg.python.source_dir).expanduser().resolve()
    if not (source / "configure").exists():
        sys.exit(f"error: no `configure` in source_dir={source!s}")

    build_dir = source / f"build_{flavor.name}"
    dist_dir = Path(cfg.python.dist_dir).expanduser().resolve()
    stage_dir = dist_dir / "stage" / flavor.name

    build_dir.mkdir(parents=True, exist_ok=True)
    stage_dir.mkdir(parents=True, exist_ok=True)

    _configure_if_needed(source, build_dir, flavor)
    _make(build_dir, jobs=jobs)
    _make_install(build_dir, stage_dir)

    _say(f"{flavor.name}: staged install tree at {stage_dir}")
    return stage_dir


def _configure_if_needed(source: Path, build_dir: Path, flavor: Flavor) -> None:
    if (build_dir / "Makefile").exists():
        _say(f"{flavor.name}: reusing existing build dir {build_dir}")
        return

    _say(f"{flavor.name}: ./configure {' '.join(flavor.configure_flags)}")
    cmd = [str(source / "configure"), *flavor.configure_flags]
    subprocess.run(cmd, cwd=build_dir, check=True)


def _make(build_dir: Path, *, jobs: int | None) -> None:
    if jobs is None:
        jobs = os.cpu_count() or 4
    _say(f"make -j{jobs} (in {build_dir})")
    subprocess.run(["make", f"-j{jobs}"], cwd=build_dir, check=True)


def _make_install(build_dir: Path, stage_dir: Path) -> None:
    # Wipe the stage so stale artifacts from an earlier flavor don't linger.
    if stage_dir.exists():
        shutil.rmtree(stage_dir)
    stage_dir.mkdir(parents=True, exist_ok=True)

    _say(f"make install (DESTDIR={stage_dir})")
    # Install into `stage_dir/usr/local` (the default --prefix). `pack.py` hoists
    # the tree up to `stage_dir/` root later.
    subprocess.run(
        ["make", "install", f"DESTDIR={stage_dir}"], cwd=build_dir, check=True
    )


def _say(msg: str) -> None:
    print(f"\033[1;36m==>\033[0m {msg}", flush=True)
