#!/usr/bin/env bash
# Local CI/CD: build release binaries and publish them to GitHub Releases.
#
# Usage:
#     scripts/publish-release.sh <tag>            # build everything supported, publish
#     TARGETS="x86_64-apple-darwin ..." scripts/publish-release.sh <tag>
#     DRY_RUN=1 scripts/publish-release.sh <tag>  # build + package, skip upload + notarization
#     SIGN=0    scripts/publish-release.sh <tag>  # skip code signing + notarization
#
# Targets supported out of the box:
#     x86_64-apple-darwin        rustup target (native on Intel macOS)
#     aarch64-apple-darwin       rustup target (native on Apple Silicon)
#
# This fork ships macOS-only binaries. To build other targets, override TARGETS and install
# the relevant cross toolchains (e.g. cargo-zigbuild + zig for Linux, cargo-xwin for Windows).
#
# macOS binaries are Developer ID signed (hardened runtime) and notarized so they pass
# Gatekeeper even when downloaded through a quarantine-setting path (browser/AirDrop). The
# signing identity and App Store Connect notary key are read from ~/.config/macos-codesign/
# (see the macos-codesign skill); if neither is present the build still succeeds but ships
# unsigned, with a warning. Bare CLI binaries can't be stapled, so notarization is recognized
# online (our curl|tar installer sets no quarantine, so it runs regardless).
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
SIGN="${SIGN:-1}"
CODESIGN_CFG="$HOME/.config/macos-codesign"
NOTARY_PROFILE="macos-codesign-notary"

ALL_TARGETS=(
    x86_64-apple-darwin
    aarch64-apple-darwin
)
TARGETS_ARR=(${TARGETS:-${ALL_TARGETS[@]}})

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/$TAG"
mkdir -p "$DIST_DIR"
# Signed binaries are copied here so a single notary submission covers every target. Dot-prefixed
# so the `"$DIST_DIR"/*` upload glob never picks it up.
NOTARIZE_STAGE="$DIST_DIR/.notarize"

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

# Resolve the Developer ID Application identity: explicit override, then the skill's identity.env,
# then whatever is in the keychain. Empty output means "no identity available".
resolve_codesign_identity() {
    if [[ -n "${CODESIGN_IDENTITY:-}" ]]; then
        printf '%s' "$CODESIGN_IDENTITY"
        return 0
    fi
    if [[ -f "$CODESIGN_CFG/identity.env" ]]; then
        # shellcheck disable=SC1090
        . "$CODESIGN_CFG/identity.env"
    fi
    if [[ -n "${CODESIGN_IDENTITY:-}" ]]; then
        printf '%s' "$CODESIGN_IDENTITY"
        return 0
    fi
    security find-identity -v -p codesigning 2>/dev/null \
        | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1
}

# Developer ID sign uv/uvx in a staged directory with the hardened runtime + secure timestamp
# (both prerequisites for notarization). Best-effort: warns and ships unsigned if no identity.
sign_stage() {
    local stage="$1" identity binary
    [[ "$SIGN" == 1 ]] || { warn "SIGN=0 — shipping unsigned"; return 0; }
    [[ "$(uname)" == Darwin ]] || return 0
    identity="$(resolve_codesign_identity)"
    if [[ -z "$identity" ]]; then
        warn "no Developer ID Application identity found — shipping UNSIGNED (set up ~/.config/macos-codesign or pass SIGN=0)"
        return 0
    fi
    for binary in "$stage/uv" "$stage/uvx"; do
        [[ -f "$binary" ]] || continue
        say "Signing $(basename "$binary") with $identity"
        codesign --force --timestamp --options runtime --sign "$identity" "$binary"
        codesign --verify --strict "$binary"
    done
    mkdir -p "$NOTARIZE_STAGE/$(basename "$stage")"
    cp "$stage/uv" "$NOTARIZE_STAGE/$(basename "$stage")/" 2>/dev/null || true
    [[ -f "$stage/uvx" ]] && cp "$stage/uvx" "$NOTARIZE_STAGE/$(basename "$stage")/" || true
}

# Submit every signed binary to Apple's notary service in one zip and wait for acceptance. The
# tarballs already contain these exact (signed) binaries, so registering their cdhashes notarizes
# what we ship. Bare Mach-O binaries cannot be stapled, so acceptance is recognized online.
notarize_artifacts() {
    [[ "$SIGN" == 1 ]] || return 0
    [[ "$(uname)" == Darwin ]] || return 0
    [[ -n "$DRY_RUN" ]] && { say "DRY_RUN set — skipping notarization"; return 0; }
    [[ -d "$NOTARIZE_STAGE" ]] && find "$NOTARIZE_STAGE" -type f | grep -q . || {
        warn "nothing signed to notarize — skipping"
        return 0
    }
    if ! xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
        if [[ -f "$CODESIGN_CFG/notary_key_id" && -f "$CODESIGN_CFG/notary_issuer" ]] \
            && ls "$CODESIGN_CFG"/AuthKey_*.p8 >/dev/null 2>&1; then
            say "Registering notary profile from $CODESIGN_CFG"
            xcrun notarytool store-credentials "$NOTARY_PROFILE" \
                --key "$(ls "$CODESIGN_CFG"/AuthKey_*.p8 | head -1)" \
                --key-id "$(cat "$CODESIGN_CFG/notary_key_id")" \
                --issuer "$(cat "$CODESIGN_CFG/notary_issuer")" >/dev/null
        else
            warn "no notary credentials in $CODESIGN_CFG — binaries are signed but NOT notarized"
            return 0
        fi
    fi
    local zip="$DIST_DIR/.notarize-bundle.zip"
    rm -f "$zip"
    (cd "$NOTARIZE_STAGE" && zip -rq "$zip" .)
    say "Submitting to Apple notary service (waits for acceptance)…"
    xcrun notarytool submit "$zip" --keychain-profile "$NOTARY_PROFILE" --wait
    rm -f "$zip"
    rm -rf "$NOTARIZE_STAGE"
    say "Notarization accepted — shipped binaries are signed + notarized."
}

build_target() {
    local target="$1"
    say "Building $target"
    install_rustup_target "$target"

    local build_cmd=()
    case "$target" in
        *-apple-darwin)
            build_cmd=(cargo build --release --locked --features self-update -p uv --target "$target")
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
            build_cmd=(cargo zigbuild --release --locked --features self-update -p uv --target "$target")
            ;;
        *-pc-windows-msvc)
            if ! has_cmd cargo-xwin; then
                warn "skipping $target: cargo-xwin not installed (\`cargo install cargo-xwin\`)"
                return 0
            fi
            build_cmd=(cargo xwin build --release --locked --features self-update -p uv --target "$target")
            ;;
        *)
            warn "skipping $target: no build strategy configured"
            return 0
            ;;
    esac

    # `UV_FORK_VERSION` is read at build time via `option_env!` in self_update.rs — stamping the
    # tag in means the resulting binary identifies itself as that release, so `uv self update`
    # doesn't falsely report "update available" against the tag it's already on.
    if ! UV_FORK_VERSION="$TAG" "${build_cmd[@]}"; then
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

    # Sign before archiving so the tarball carries the Developer ID signature; notarization of the
    # cdhash happens once for all targets after the build loop.
    if [[ "$target" == *-apple-darwin* ]]; then
        sign_stage "$stage"
    fi

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
}

publish_release() {
    local body_file
    body_file="$(mktemp)"
    cat > "$body_file" <<EOF
Prebuilt binaries for the \`jmpnop/uv\` fork, built locally with \`scripts/publish-release.sh\`.

## Install

macOS (Intel, Apple Silicon):
\`\`\`
curl -LsSf https://github.com/${REPO}/releases/download/${TAG}/uv-installer.sh | sh
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

    notarize_artifacts

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
