#!/bin/bash
set -e

REPO="ericwiley/SteamStats"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="steam-stats"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  arm64)
    TARGET="aarch64-apple-darwin"
    ;;
  x86_64)
    TARGET="x86_64-apple-darwin"
    ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

# Determine latest release tag if not specified
if [ -z "$VERSION" ]; then
  VERSION=$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
fi

if [ -z "$VERSION" ]; then
  echo "Failed to determine latest release version" >&2
  exit 1
fi

BINARY_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}-${TARGET}.tar.gz"

echo "Installing steam-stats ${VERSION} for ${TARGET}..."

# Download and extract
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -sSfL "$BINARY_URL" | tar xz -C "$TMPDIR"

# Install binary
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPDIR/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
else
  sudo mv "$TMPDIR/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
fi

chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "steam-stats installed to ${INSTALL_DIR}/${BINARY_NAME}"
echo "Run 'steam-stats' to get started."
