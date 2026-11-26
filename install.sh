#!/usr/bin/env bash
# fusion-cli standalone installer (issue #40).
# Downloads a release tarball from GitHub Releases and installs the
# fusion binary under a durable path (~/.fusion/bin), independent of the
# FusionMLX desktop app's CLI shim (which dies when the app bundle lives
# on an ejected DMG). The app shim remains a fallback only.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/dahai80/fusion-cli/main/install.sh | bash
#   FUSION_INSTALL_VERSION=v0.4.8 bash install.sh
#
# Env:
#   FUSION_INSTALL_VERSION   release tag (default: latest stable)
#   FUSION_INSTALL_DIR       install dir (default: ~/.fusion/bin)
set -euo pipefail

REPO="dahai80/fusion-cli"
VERSION="${FUSION_INSTALL_VERSION:-latest}"
DEST="${FUSION_INSTALL_DIR:-$HOME/.fusion/bin}"

log() { printf '%s\n' "==> $*"; }
fail() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

case "$(uname -s)/$(uname -m)" in
    Darwin/arm64) ASSET="fusion-darwin-arm64" ;;
    Darwin/x86_64) ASSET="fusion-darwin-arm64" ;;
    Linux/x86_64) ASSET="fusion-linux-x86_64" ;;
    *) fail "unsupported platform $(uname -s)/$(uname -m) (releases: darwin-arm64, linux-x86_64)" ;;
esac

command -v curl >/dev/null 2>&1 || fail "curl not found"
command -v tar >/dev/null 2>&1 || fail "tar not found"

if [ "$VERSION" = "latest" ]; then
    log "resolving latest stable release"
    TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
    [ -n "$TAG" ] || fail "could not resolve latest release (network or rate limit)"
else
    TAG="$VERSION"
fi
log "installing $TAG ($ASSET) -> $DEST"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
URL="https://github.com/$REPO/releases/download/$TAG/$ASSET.tar.gz"
# Primary: github.com; fallback: gh-proxy style mirror is deliberately NOT
# used here (supply-chain risk) — if github.com:443 is blocked, pipe the
# tarball manually: FUSION_INSTALL_VERSION=$TAG bash install.sh after
# downloading it by any available channel.
if ! curl -fL --retry 3 -o "$WORK/fusion.tar.gz" "$URL" 2>"$WORK/err"; then
    if curl -fsSL --retry 2 -o "$WORK/fusion.tar.gz" \
        "https://api.github.com/repos/$REPO/tarball/$TAG" 2>>"$WORK/err"; then
        log "downloaded source tarball fallback (api.github.com)"
    else
        fail "download failed ($URL); stderr: $(tail -n2 "$WORK/err")"
    fi
fi

mkdir -p "$DEST"
EXTRACTED=$(find "$WORK" -type f -name fusion -perm +111 | head -n1)
if [ -z "$EXTRACTED" ]; then
    tar -xzf "$WORK/fusion.tar.gz" -C "$WORK"
    EXTRACTED=$(find "$WORK" -type f -name fusion -perm +111 | head -n1)
    # Release assets ship the bare binary; source tarballs nest it deeper.
    [ -n "$EXTRACTED" ] || EXTRACTED=$(find "$WORK" -type f -name fusion | head -n1)
fi
[ -n "$EXTRACTED" ] || fail "no 'fusion' binary found in archive"
install -m 0755 "$EXTRACTED" "$DEST/fusion"

log "verifying"
"$DEST/fusion" --version || fail "installed binary failed to run"

case ":$PATH:" in
    *":$DEST:"*) ;;
    *)
        log "$DEST is not on PATH"
        log "add to your shell profile:"
        printf '  export PATH="%s:$PATH"\n' "$DEST"
        ;;
esac

log "done: $DEST/fusion ($TAG)"
log "note: the FusionMLX desktop app shim is a fallback; this standalone install takes precedence on PATH order."
