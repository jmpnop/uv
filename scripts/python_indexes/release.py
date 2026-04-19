#!/usr/bin/env python3
"""Orchestrator CLI: build / pack / index / publish custom Python flavors.

Usage:
    scripts/python_indexes/release.py all
    scripts/python_indexes/release.py build [FLAVOR ...]
    scripts/python_indexes/release.py pack [FLAVOR ...]
    scripts/python_indexes/release.py index
    scripts/python_indexes/release.py publish
    scripts/python_indexes/release.py clean

If no FLAVOR is given to ``build`` / ``pack``, every flavor from ``config.toml``
is processed. ``all`` is ``build`` → ``pack`` → ``index`` → ``publish`` in one
invocation.
"""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path


# Make relative imports work whether we're invoked as a script or a module.
if __package__ in (None, ""):
    HERE = Path(__file__).resolve().parent
    sys.path.insert(0, str(HERE.parent))
    __package__ = "python_indexes"  # noqa: A001

from python_indexes._build import build_flavor  # noqa: E402
from python_indexes._index import generate as generate_index  # noqa: E402
from python_indexes._pack import pack_flavor  # noqa: E402
from python_indexes._publish import publish as publish_release  # noqa: E402
from python_indexes._schema import Config, Flavor  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--config",
        default=str(Path(__file__).resolve().parent / "config.toml"),
        help="path to the TOML config (default: alongside this script)",
    )

    sub = parser.add_subparsers(dest="cmd", required=True)

    p_build = sub.add_parser(
        "build", help="configure + make + staged install for a flavor"
    )
    p_build.add_argument("flavors", nargs="*", help="flavor names; empty = all")
    p_build.add_argument("-j", "--jobs", type=int, default=None, help="make -j N")

    p_pack = sub.add_parser("pack", help="tar up staged install trees")
    p_pack.add_argument("flavors", nargs="*", help="flavor names; empty = all")

    sub.add_parser("index", help="emit versions.json from dist/python/")

    p_pub = sub.add_parser(
        "publish", help="upload tarballs + versions.json to GitHub Release"
    )
    p_pub.add_argument("--dry-run", action="store_true")

    p_all = sub.add_parser("all", help="build + pack + index + publish (every flavor)")
    p_all.add_argument("-j", "--jobs", type=int, default=None)
    p_all.add_argument("--dry-run", action="store_true")

    sub.add_parser("clean", help="rm -rf dist/python/")

    args = parser.parse_args()
    cfg = Config.load(args.config)

    if args.cmd == "build":
        for f in _flavors(cfg, args.flavors):
            build_flavor(cfg, f, jobs=args.jobs)
        return 0

    if args.cmd == "pack":
        for f in _flavors(cfg, args.flavors):
            stage_dir = _stage_dir(cfg, f)
            if not stage_dir.exists():
                sys.exit(f"error: no staged tree for {f.name} (run `build` first)")
            pack_flavor(cfg, f, stage_dir)
        return 0

    if args.cmd == "index":
        generate_index(cfg)
        return 0

    if args.cmd == "publish":
        paths = _collect_upload_paths(cfg)
        publish_release(cfg, paths, dry_run=args.dry_run)
        return 0

    if args.cmd == "all":
        flavors = list(cfg.flavors.values())
        for f in flavors:
            build_flavor(cfg, f, jobs=args.jobs)
        for f in flavors:
            stage_dir = _stage_dir(cfg, f)
            pack_flavor(cfg, f, stage_dir)
        generate_index(cfg)
        publish_release(cfg, _collect_upload_paths(cfg), dry_run=args.dry_run)
        return 0

    if args.cmd == "clean":
        dist = Path(cfg.python.dist_dir).expanduser().resolve()
        if dist.exists():
            shutil.rmtree(dist)
            print(f"removed {dist}")
        return 0

    parser.error(f"unknown command: {args.cmd}")


def _flavors(cfg: Config, names: list[str]) -> list[Flavor]:
    if not names:
        return list(cfg.flavors.values())
    out = []
    for n in names:
        f = cfg.flavors.get(n)
        if f is None:
            sys.exit(f"error: unknown flavor {n!r} (available: {sorted(cfg.flavors)})")
        out.append(f)
    return out


def _stage_dir(cfg: Config, flavor: Flavor) -> Path:
    return Path(cfg.python.dist_dir).expanduser().resolve() / "stage" / flavor.name


def _collect_upload_paths(cfg: Config) -> list[Path]:
    dist = Path(cfg.python.dist_dir).expanduser().resolve()
    paths = sorted(dist.glob("*.tar.gz"))
    paths += sorted(dist.glob("*.tar.gz.sha256"))
    versions = dist / "versions.json"
    if versions.exists():
        paths.append(versions)
    if not paths:
        sys.exit(f"error: nothing to upload in {dist}")
    return paths


if __name__ == "__main__":
    sys.exit(main())
