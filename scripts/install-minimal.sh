#!/bin/bash
APP_NAME="que"
BINARY_URL="https://github.com/AT-290690/que-script/releases/latest/download/que"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    chmod +x "/tmp/$APP_NAME"
    sudo mv "/tmp/$APP_NAME" "$INSTALL_PATH"
    echo "Installed binary: $INSTALL_PATH"
else
    echo "❌ Error: Could not download binary."
    echo "Check that you have a RELEASE and an ASSET named 'que' at:"
    echo "https://github.com/AT-290690/que-script/releases"
    exit 1
fi

echo "✅ Success! Installed binary only (no std library)."
echo "To install std separately, run: scripts/install-std-only.sh"
