#!/bin/bash
set -euo pipefail

APP_NAME="quelsp"
RELEASE_BASE="https://github.com/AT-290690/que-script/releases/latest/download"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "${arch}" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)
            echo "Unsupported architecture: ${arch}" >&2
            exit 1
            ;;
    esac

    case "${os}" in
        Linux) echo "${arch}-unknown-linux-gnu" ;;
        Darwin) echo "${arch}-apple-darwin" ;;
        *)
            echo "Unsupported operating system: ${os}" >&2
            exit 1
            ;;
    esac
}

TARGET="$(detect_target)"
BINARY_URL="${RELEASE_BASE}/${APP_NAME}-${TARGET}"

echo "Installing $APP_NAME..."
echo "Resolved target: $TARGET"

if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    chmod +x "/tmp/$APP_NAME"
    sudo mv "/tmp/$APP_NAME" "$INSTALL_PATH"
    echo "✅ Success! You can add $APP_NAME in your editor"
else
    echo "❌ Error: Could not download binary."
    echo "Check that you have a RELEASE and an ASSET named '${APP_NAME}-${TARGET}' at:"
    echo "https://github.com/AT-290690/que-script/releases"
    exit 1
fi
