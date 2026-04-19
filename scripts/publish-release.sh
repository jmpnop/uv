#!/usr/bin/env bash
# Local CI/CD: build release binaries and publish them to GitHub Releases.
#
# Usage:
#     scripts/publish-release.sh <tag>            # build everything supported, publish
#     TARGETS="x86_64-apple-darwin ..." scripts/publish-release.sh <tag>
#     DRY_RUN=1 scripts/publish-release.sh <tag>  # build + package, skip upload
#
# Targets supported out of the box (skipped gracefully if toolchain missing):
#     x86_64-apple-darwin        rustup target (native on Intel macOS)
#     aarch64-apple-darwin       rustup target (native on Apple Silicon)
#     x86_64-unknown-linux-gnu   cargo-zigbuild
#     aarch64-unknown-linux-gnu  cargo-zigbuild
#     x86_64-pc-windows-msvc     cargo-xwin
#
# Optional toolchains:
#     cargo install cargo-zigbuild         # linux cross-compile from macOS
#     cargo install cargo-xwin             # windows cross-compile from macOS
#     brew install zig                     # required by cargo-zigbuild
#
# No GitHub Actions involved. `gh release create` uploads the tarballs + installer scripts.

set -euo pipefail

if [[ $# -lt 1 ]]; then
    printf 'usage: %s <tag>\n' "$0" >&2
    exit 2
fi

TAG="$1"
REPO="${REPO:-jmpnop/uv}"
DRY_RUN="${DRY_RUN:-}"

ALL_TARGETS=(
    x86_64-apple-darwin
    aarch64-apple-darwin
    x86_64-unknown-linux-gnu
    aarch64-unknown-linux-gnu
    x86_64-pc-windows-msvc
)
TARGETS_ARR=(${TARGETS:-${ALL_TARGETS[@]}})

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/$TAG"
mkdir -p "$DIST_DIR"

say()  { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarn:\033[0m %s\n' "$*" >&2; }
err()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

has_cmd() { command -v "$1" >/dev/null 2>&1; }

rustup_target_installed() {
    rustup target list --installed 2>/dev/null | grep -qx "$1"
}

install_rustup_target() {
    local t="$1"
    if ! rustup_target_installed "$t"; then
        say "Installing rustup target $t"
        rustup target add "$t"
    fi
}

build_target() {
    local target="$1"
    say "Building $target"
    install_rustup_target "$target"

    local build_cmd=()
    case "$target" in
        *-apple-darwin)
            build_cmd=(cargo build --release --locked -p uv --target "$target")
            ;;
        *-unknown-linux-gnu)
            if ! has_cmd cargo-zigbuild; then
                warn "skipping $target: cargo-zigbuild not installed (\`cargo install cargo-zigbuild\`)"
                return 0
            fi
            if ! has_cmd zig; then
                warn "skipping $target: zig not installed (\`brew install zig\`)"
                return 0
            fi
            build_cmd=(cargo zigbuild --release --locked -p uv --target "$target")
            ;;
        *-pc-windows-msvc)
            if ! has_cmd cargo-xwin; then
                warn "skipping $target: cargo-xwin not installed (\`cargo install cargo-xwin\`)"
                return 0
            fi
            build_cmd=(cargo xwin build --release --locked -p uv --target "$target")
            ;;
        *)
            warn "skipping $target: no build strategy configured"
            return 0
            ;;
    esac

    if ! "${build_cmd[@]}"; then
        warn "build failed for $target"
        return 0
    fi

    package_target "$target"
}

package_target() {
    local target="$1"
    local stage="$DIST_DIR/uv-$target"
    rm -rf "$stage"
    mkdir -p "$stage"

    local src="$ROOT_DIR/target/$target/release"
    local ext=""
    if [[ "$target" == *windows* ]]; then
        ext=".exe"
    fi

    if [[ ! -x "$src/uv$ext" ]]; then
        warn "missing binary: $src/uv$ext"
        rm -rf "$stage"
        return 0
    fi
    cp "$src/uv$ext" "$stage/uv$ext"
    [[ -x "$src/uvx$ext" ]] && cp "$src/uvx$ext" "$stage/uvx$ext" || true

    local archive_name=""
    if [[ "$target" == *windows* ]]; then
        archive_name="uv-$target.zip"
        (cd "$DIST_DIR" && zip -rq "$archive_name" "uv-$target")
    else
        archive_name="uv-$target.tar.gz"
        (cd "$DIST_DIR" && tar czf "$archive_name" "uv-$target")
    fi
    rm -rf "$stage"

    (cd "$DIST_DIR" && shasum -a 256 "$archive_name" > "$archive_name.sha256")
    say "Packaged $DIST_DIR/$archive_name"
}

stage_installers() {
    cp "$ROOT_DIR/install.sh"  "$DIST_DIR/uv-installer.sh"
    cp "$ROOT_DIR/install.ps1" "$DIST_DIR/uv-installer.ps1"
}

publish_release() {
    local body_file
    body_file="$(mktemp)"
    cat > "$body_file" <<EOF
Prebuilt binaries for the \`jmpnop/uv\` fork, built locally with \`scripts/publish-release.sh\`.

## Install

macOS / Linux:
\`\`\`
curl -LsSf https://github.com/${REPO}/releases/download/${TAG}/uv-installer.sh | sh
\`\`\`

Windows (PowerShell):
\`\`\`
powershell -ExecutionPolicy ByPass -c "irm https://github.com/${REPO}/releases/download/${TAG}/uv-installer.ps1 | iex"
\`\`\`
EOF

    if ! has_cmd gh; then
        err "gh (GitHub CLI) not found; install it to upload (\`brew install gh\`)"
    fi

    if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
        say "Release $TAG already exists — uploading (replacing) assets"
        gh release upload "$TAG" --repo "$REPO" --clobber "$DIST_DIR"/*
    else
        say "Creating release $TAG"
        gh release create "$TAG" \
            --repo "$REPO" \
            --title "$TAG" \
            --notes-file "$body_file" \
            "$DIST_DIR"/*
    fi
    rm -f "$body_file"
}

main() {
    has_cmd rustup || err "rustup not found (install from https://rustup.rs/)"
    has_cmd cargo  || err "cargo not found"

    say "Target set: ${TARGETS_ARR[*]}"
    for t in "${TARGETS_ARR[@]}"; do
        build_target "$t"
    done

    stage_installers

    say "Artifacts in $DIST_DIR:"
    ls -la "$DIST_DIR"

    if [[ -n "$DRY_RUN" ]]; then
        say "DRY_RUN set — skipping GitHub upload"
        return 0
    fi

    publish_release
    say "Done. Release: https://github.com/${REPO}/releases/tag/${TAG}"
}

main "$@"
