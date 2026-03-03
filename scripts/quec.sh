#!/bin/bash
APP_NAME="quec"

BINARY_URL="https://github.com/AT-290690/que-script/releases/latest/download/quec"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    chmod +x "/tmp/$APP_NAME"
    sudo mv "/tmp/$APP_NAME" "$INSTALL_PATH"
    echo "✅ Success! You can now run: $APP_NAME yourfile.que"
else
    echo "❌ Error: Could not download binary."
    echo "Check that you have a RELEASE and an ASSET named 'quec' at:"
    echo "https://github.com/AT-290690/que-script/releases"
    exit 1
fi