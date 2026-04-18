#!/bin/sh
# uv fork installer (jmpnop/uv)
#
# Usage:
#   curl -LsSf https://github.com/jmpnop/uv/releases/latest/download/uv-installer.sh | sh
#
# Options (via env vars):
#   UV_INSTALL_DIR  directory to install into (default: $HOME/.local/bin)
#   UV_VERSION      release tag to install (default: latest)

set -eu

REPO="jmpnop/uv"
INSTALL_DIR="${UV_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${UV_VERSION:-latest}"

err() { printf 'error: %s\n' "$*" >&2; exit 1; }
say() { printf '%s\n' "$*"; }

need() { command -v "$1" >/dev/null 2>&1 || err "missing required tool: $1"; }

need uname
need mkdir
need tar
if command -v curl >/dev/null 2>&1; then
    DL="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    DL="wget -qO-"
else
    err "need either curl or wget"
fi

os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)

case "$os" in
    linux)   platform_os="unknown-linux-gnu" ;;
    darwin)  platform_os="apple-darwin" ;;
    *)       err "unsupported OS: $os" ;;
esac

case "$arch" in
    x86_64|amd64)   platform_arch="x86_64" ;;
    aarch64|arm64)  platform_arch="aarch64" ;;
    *)              err "unsupported architecture: $arch" ;;
esac

target="${platform_arch}-${platform_os}"
archive="uv-${target}.tar.gz"

if [ "$VERSION" = "latest" ]; then
    url="https://github.com/${REPO}/releases/latest/download/${archive}"
else
    url="https://github.com/${REPO}/releases/download/${VERSION}/${archive}"
fi

say "Downloading ${archive}..."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if ! $DL "$url" > "$tmp/$archive"; then
    err "failed to download $url"
fi

tar xzf "$tmp/$archive" -C "$tmp"

mkdir -p "$INSTALL_DIR"
cp "$tmp/uv-${target}/uv" "$INSTALL_DIR/uv"
chmod +x "$INSTALL_DIR/uv"
if [ -f "$tmp/uv-${target}/uvx" ]; then
    cp "$tmp/uv-${target}/uvx" "$INSTALL_DIR/uvx"
    chmod +x "$INSTALL_DIR/uvx"
fi

say "Installed uv to $INSTALL_DIR/uv"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) say "Note: $INSTALL_DIR is not on your PATH. Add it to your shell config:"
       say "    export PATH=\"\$PATH:$INSTALL_DIR\"" ;;
esac
