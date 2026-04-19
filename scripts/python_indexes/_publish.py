"""Upload packaged tarballs and ``versions.json`` to a GitHub Release via ``gh``.

A single rolling tag (default ``python-builds``) keeps the URL of
``versions.json`` stable so the ``[[python-indexes]]`` entry users put in their
``uv.toml`` never needs to change.

No GitHub Actions are involved.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

from ._schema import Config


def publish(cfg: Config, paths: list[Path], *, dry_run: bool = False) -> None:
    if cfg.publish is None:
        sys.exit("error: config has no [publish] section")

    if not shutil.which("gh"):
        sys.exit("error: `gh` (GitHub CLI) not found; `brew install gh`")

    if dry_run:
        _say("DRY RUN — skipping upload. Would publish:")
        for p in paths:
            print(f"  - {p}")
        return

    tag = cfg.publish.release_tag
    repo = cfg.publish.repo

    if not _release_exists(repo, tag):
        _say(f"creating release {tag} on {repo}")
        subprocess.run(
            [
                "gh",
                "release",
                "create",
                tag,
                "--repo",
                repo,
                "--title",
                "Custom Python builds",
                "--notes",
                _release_notes(cfg),
            ],
            check=True,
        )
    else:
        _say(f"release {tag} already exists on {repo} — uploading (replacing) assets")

    cmd = [
        "gh",
        "release",
        "upload",
        tag,
        "--repo",
        repo,
        "--clobber",
        *[str(p) for p in paths],
    ]
    subprocess.run(cmd, check=True)
    _say(f"published: https://github.com/{repo}/releases/tag/{tag}")


def _release_exists(repo: str, tag: str) -> bool:
    return (
        subprocess.run(
            ["gh", "release", "view", tag, "--repo", repo],
            capture_output=True,
        ).returncode
        == 0
    )


def _release_notes(cfg: Config) -> str:
    assert cfg.publish is not None
    return (
        "Custom CPython builds published by `scripts/python_indexes/release.py`.\n"
        "\n"
        "Consume from `uv.toml`:\n"
        "\n"
        "```toml\n"
        "[[python-indexes]]\n"
        f'name = "{cfg.publish.repo.split("/")[0]}-python"\n'
        f'url  = "{cfg.publish.url_prefix}versions.json"\n'
        "```\n"
    )


def _say(msg: str) -> None:
    print(f"\033[1;36m==>\033[0m {msg}", flush=True)
