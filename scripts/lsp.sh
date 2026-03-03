#!/bin/bash
APP_NAME="quelsp"

BINARY_URL="https://github.com/AT-290690/que-script/releases/latest/download/quelsp"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    chmod +x "/tmp/$APP_NAME"
    sudo mv "/tmp/$APP_NAME" "$INSTALL_PATH"
    echo "✅ Success! You can add $APP_NAME in your editor"
else
    echo "❌ Error: Could not download binary."
    echo "Check that you have a RELEASE and an ASSET named 'que' at:"
    echo "https://github.com/AT-290690/que-script/releases"
    exit 1
fi