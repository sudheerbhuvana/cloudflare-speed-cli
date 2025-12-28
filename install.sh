#!/usr/bin/env sh
set -eu

VERSION="0.1.0"

detect_os() {
  case "$(uname -s)" in
    Linux) OS=unknown-linux-musl ;;
    Darwin) OS=apple-darwin ;;
    *) echo "Unsupported OS" >&2; exit 1 ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64) ARCH=x86_64 ;;
    arm64|aarch64) ARCH=aarch64 ;;
    *) echo "Unsupported ARCH" >&2; exit 1 ;;
  esac
}

detect_os
detect_arch

BIN="cloudflare-speed-cli"
FILE="${BIN}_${ARCH}-${OS}.tar.xz"
URL="https://github.com/kavehtehrani/cloudflare-speed-cli/releases/download/v${VERSION}/${FILE}"

tmpdir=$(mktemp -d)
cd "$tmpdir"

curl -fsSLO "$URL"
curl -fsSLO "${URL}.sha256"

sha256sum -c "${FILE}.sha256"

tar -xJf "$FILE"

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
install -m 0755 "$BIN" "$INSTALL_DIR/$BIN"
echo "Installed to $INSTALL_DIR"
echo "Add $INSTALL_DIR to PATH if needed"
