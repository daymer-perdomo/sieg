#!/bin/sh
set -eu

REPO="daymer-perdomo/sieg"
BIN_NAME="sieg"
INSTALL_DIR="${SIEG_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

if [ "$os" != "Darwin" ]; then
    echo "error: this install script only supports macOS (got: $os)" >&2
    exit 1
fi

case "$arch" in
    arm64|aarch64)
        asset="${BIN_NAME}-macos-aarch64"
        ;;
    x86_64)
        asset="${BIN_NAME}-macos-x86_64"
        ;;
    *)
        echo "error: unsupported architecture: $arch" >&2
        exit 1
        ;;
esac

url="https://github.com/${REPO}/releases/latest/download/${asset}"

echo "downloading ${asset}..."
mkdir -p "$INSTALL_DIR"
tmp_file="$(mktemp)"
curl -fsSL "$url" -o "$tmp_file"
chmod +x "$tmp_file"
mv "$tmp_file" "$INSTALL_DIR/$BIN_NAME"

echo "installed to $INSTALL_DIR/$BIN_NAME"

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "note: $INSTALL_DIR is not on your PATH. add this to your shell profile:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac

echo ""
echo "run it with: $BIN_NAME"
