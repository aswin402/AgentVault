#!/bin/bash
set -euo pipefail

REPO="aswin402/AgentVault"
BINARY="vault"

echo "============================================="
echo " Installing AgentVault CLI v0.1.0"
echo "============================================="

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64)
                TARGET="x86_64-unknown-linux-gnu"
                ;;
            aarch64|arm64)
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64)
                TARGET="x86_64-apple-darwin"
                ;;
            aarch64|arm64)
                TARGET="aarch64-apple-darwin"
                ;;
            *)
                echo "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    MINGW*|MSYS*|CYGWIN*)
        TARGET="x86_64-pc-windows-msvc"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "✓ Detected Platform: $OS ($ARCH)"
echo "✓ Target Release Build: $TARGET"

# Make temporary download directory
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

URL="https://github.com/$REPO/releases/download/v0.1.0/$BINARY-$TARGET.tar.gz"
if [ "$TARGET" = "x86_64-pc-windows-msvc" ]; then
    URL="https://github.com/$REPO/releases/download/v0.1.0/$BINARY-$TARGET.zip"
fi

echo "✓ Downloading release package..."
if [ "$TARGET" = "x86_64-pc-windows-msvc" ]; then
    curl -fsSL "$URL" -o "$TMP_DIR/vault.zip"
    unzip -q "$TMP_DIR/vault.zip" -d "$TMP_DIR"
else
    curl -fsSL "$URL" -o "$TMP_DIR/vault.tar.gz"
    tar -xzf "$TMP_DIR/vault.tar.gz" -C "$TMP_DIR"
fi

# Determine installation directory
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

if [ "$TARGET" = "x86_64-pc-windows-msvc" ]; then
    mv "$TMP_DIR/vault.exe" "$INSTALL_DIR/vault.exe"
    chmod +x "$INSTALL_DIR/vault.exe"
    echo "✓ Installed vault to $INSTALL_DIR/vault.exe"
else
    mv "$TMP_DIR/vault" "$INSTALL_DIR/vault"
    chmod +x "$INSTALL_DIR/vault"
    echo "✓ Installed vault to $INSTALL_DIR/vault"
fi

echo "---------------------------------------------"
echo " Make sure $INSTALL_DIR is in your PATH."
echo " Try running 'vault init' to get started!"
echo "============================================="
