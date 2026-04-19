"""Dataclasses matching uv's `JsonPythonDownload` JSON schema.

uv's `ManagedPythonDownloadList::new` deserializes a `HashMap<String, JsonPythonDownload>`
from each configured `[[python-indexes]]` URL. The key is the installation identifier
(e.g. ``cpython-3.14.4-darwin-aarch64-none``) and the value is the metadata below.

Keep this file in sync with
``crates/uv-python/src/downloads.rs :: JsonPythonDownload``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True, slots=True)
class JsonArch:
    family: str
    variant: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {"family": self.family, "variant": self.variant}


@dataclass(frozen=True, slots=True)
class JsonPythonDownload:
    name: str  # "cpython" | "pypy" | "graalpy" | ...
    arch: JsonArch
    os: str  # "darwin" | "linux" | "windows"
    libc: str  # "gnu" | "musl" | "none"
    major: int
    minor: int
    patch: int
    prerelease: str = ""  # "" for stable, "rc1" etc.
    url: str = ""
    sha256: str | None = None
    variant: str | None = None  # "" / "freethreaded" / "jit" / etc.
    build: str | None = None  # opaque build tag

    def installation_key(self) -> str:
        """Produce the dictionary key uv uses for this entry.

        Format: ``<name>-<major>.<minor>.<patch>[<prerelease>][+<variant>]-<os>-<arch>-<libc>``

        uv's `PythonInstallationKey` spells `darwin` for macOS builds.
        """
        version = f"{self.major}.{self.minor}.{self.patch}{self.prerelease}"
        if self.variant:
            version = f"{version}+{self.variant}"
        return f"{self.name}-{version}-{self.os}-{self.arch.family}-{self.libc}"

    def to_dict(self) -> dict[str, Any]:
        # uv's deserializer expects every field, even when empty. Keep `None` for
        # `sha256` / `variant` / `build` — serde `Option` reads `null` as `None`.
        return {
            "name": self.name,
            "arch": self.arch.to_dict(),
            "os": self.os,
            "libc": self.libc,
            "major": self.major,
            "minor": self.minor,
            "patch": self.patch,
            "prerelease": self.prerelease,
            "url": self.url,
            "sha256": self.sha256,
            "variant": self.variant if self.variant else None,
            "build": self.build,
        }


@dataclass(frozen=True, slots=True)
class Flavor:
    """A build flavor declared in ``config.toml``."""

    name: str
    configure_flags: tuple[str, ...]
    variant: str  # "" for the default (plain) flavor
    build: str

    @classmethod
    def from_toml(cls, name: str, raw: dict[str, Any]) -> "Flavor":
        return cls(
            name=name,
            configure_flags=tuple(raw.get("configure_flags", ())),
            variant=raw.get("variant", "") or "",
            build=raw.get("build", "") or "",
        )


@dataclass(frozen=True, slots=True)
class PythonCfg:
    version: str
    source_dir: str
    arches: tuple[str, ...]
    os: str
    libc: str
    dist_dir: str

    @property
    def major_minor_patch(self) -> tuple[int, int, int]:
        parts = self.version.split(".")
        if len(parts) != 3:
            raise ValueError(f"expected MAJOR.MINOR.PATCH, got {self.version!r}")
        return int(parts[0]), int(parts[1]), int(parts[2])


@dataclass(frozen=True, slots=True)
class PublishCfg:
    repo: str
    release_tag: str
    url_prefix: str


@dataclass(frozen=True, slots=True)
class Config:
    python: PythonCfg
    flavors: dict[str, Flavor] = field(default_factory=dict)
    publish: PublishCfg | None = None

    @classmethod
    def load(cls, path: str) -> "Config":
        import tomllib
        from pathlib import Path

        raw = tomllib.loads(Path(path).expanduser().read_text())

        py = raw["python"]
        python = PythonCfg(
            version=py["version"],
            source_dir=py["source_dir"],
            arches=tuple(py["arches"]),
            os=py["os"],
            libc=py["libc"],
            dist_dir=py["dist_dir"],
        )

        flavors = {
            name: Flavor.from_toml(name, body)
            for name, body in (raw.get("flavors") or {}).items()
        }

        pub = raw.get("publish") or None
        publish = (
            PublishCfg(
                repo=pub["repo"],
                release_tag=pub["release_tag"],
                url_prefix=pub["url_prefix"],
            )
            if pub
            else None
        )

        return cls(python=python, flavors=flavors, publish=publish)
