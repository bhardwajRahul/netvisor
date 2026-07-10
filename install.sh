#!/bin/bash
set -e

REPO="scanopy/scanopy"
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')

case "$PLATFORM" in
    mingw*|msys*|cygwin*)
        echo "Windows detected. This install script is for Linux and macOS."
        echo ""
        echo "To install on Windows, go to the Scanopy web UI and create a daemon — it will"
        echo "generate the correct PowerShell download and run commands for you."
        exit 1
        ;;
esac

ARCH=$(uname -m)

# Map architecture names to match release binaries
case "$ARCH" in
    x86_64)
        ARCH="amd64"
        ;;
    aarch64|arm64)
        ARCH="arm64"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        echo "Supported architectures: x86_64 (amd64), aarch64/arm64"
        exit 1
        ;;
esac

BINARY_NAME="scanopy-daemon-${PLATFORM}-${ARCH}"

echo "Installing Scanopy daemon..."
echo "Platform: $PLATFORM"
echo "Architecture: $ARCH"
echo "Binary: $BINARY_NAME"
echo ""

# Download latest binary
BINARY_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}"
echo "Downloading from: $BINARY_URL"

if ! curl -fL "$BINARY_URL" -o scanopy-daemon; then
    echo "Error: Failed to download binary from $BINARY_URL"
    echo "Please check:"
    echo "  1. Your internet connection"
    echo "  2. That a release exists for your platform"
    echo "  3. GitHub releases: https://github.com/${REPO}/releases/latest"
    exit 1
fi

chmod +x scanopy-daemon

# Install to system
echo "Installing to /usr/local/bin (may require sudo)..."
if [ -w "/usr/local/bin" ]; then
    mv scanopy-daemon /usr/local/bin/
else
    sudo mv scanopy-daemon /usr/local/bin/ || {
        echo "Error: Failed to install scanopy-daemon. Please check sudo permissions."
        rm -f scanopy-daemon
        exit 1
    }
fi

# Verify installation
if [ ! -f "/usr/local/bin/scanopy-daemon" ]; then
    echo "Error: Installation verification failed."
    exit 1
fi

echo ""
echo "✓ Scanopy daemon binary installed successfully!"
echo ""

# Register the background service (systemd on Linux, launchd on macOS, rc.d on FreeBSD) via the
# daemon's own `install` subcommand, which also writes config.json. It generates a fully-formed
# service definition — no hand-editing required.
echo "To register the daemon as a background service, run:"
echo "  sudo scanopy-daemon install --server-url YOUR_SERVER_URL --daemon-api-key YOUR_API_KEY"
echo ""
echo "The Scanopy web UI (Daemons → Add daemon) generates this command with your values"
echo "filled in. To run in the foreground without a service, use \`scanopy-daemon\` directly."
echo ""
echo "Need help? Visit: https://github.com/${REPO}#readme"
