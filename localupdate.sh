#!/bin/bash
set -euo pipefail

# localupdate.sh - Build and update AgentVault locally to your global PATH

echo "============================================="
echo "  AgentVault - Local Update Script"
echo "============================================="

# Ensure we are in the root directory of the project
if [ ! -f "Cargo.toml" ] || [ ! -d "crates/vault-cli" ]; then
    echo "❌ Error: This script must be run from the root of the AgentVault repository."
    exit 1
fi

# Detect OS
OS="$(uname -s)"
BINARY_NAME="vault"
if [[ "$OS" == MINGW* || "$OS" == MSYS* || "$OS" == CYGWIN* ]]; then
    BINARY_NAME="vault.exe"
fi

# Step 1: Optimize resources for check/build
# Detect CPU cores and limit parallel cargo jobs to conserve CPU/RAM (max 4 jobs).
CORES=$(nproc 2>/dev/null || echo 4)
if [ "$CORES" -gt 4 ]; then
    JOBS=4
elif [ "$CORES" -gt 1 ]; then
    JOBS=$(( CORES / 2 ))
else
    JOBS=1
fi
echo "✓ Optimizing resources: limiting cargo to $JOBS jobs (detected $CORES cores)..."
export CARGO_BUILD_JOBS=$JOBS

# Step 2: Run code checks
echo "✓ Formatting code..."
cargo fmt --all

echo "✓ Checking lints..."
cargo clippy --workspace --all-targets -- -D warnings

# Step 3: Build release binary
echo "✓ Building release binary..."
cargo build --release --bin vault

# Step 4: Verify target binary exists
TARGET_PATH="target/release/$BINARY_NAME"
if [ ! -f "$TARGET_PATH" ]; then
    echo "❌ Error: Build succeeded but binary was not found at $TARGET_PATH."
    exit 1
fi

# Step 5: Install to ~/.local/bin
INSTALL_DIR="$HOME/.local/bin"
echo "✓ Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"

# Copy binary to path
cp "$TARGET_PATH" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo "✓ Successfully installed $BINARY_NAME to $INSTALL_DIR/$BINARY_NAME"

# Step 6: Verify installation works
echo "---------------------------------------------"
if command -v vault >/dev/null 2>&1; then
    INSTALLED_VER="$(vault --version)"
    echo "✓ Verification successful: $INSTALLED_VER is accessible in your PATH."
else
    echo "⚠️  Warning: $INSTALL_DIR is not currently in your system PATH."
    echo "   Please add the following line to your ~/.bashrc or ~/.zshrc:"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
echo "============================================="
