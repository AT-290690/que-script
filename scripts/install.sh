#!/bin/bash
APP_NAME="que"
USER="AT-290690"
REPO="rust-lisp"

BINARY_URL="https://github.com$USER/$REPO/releases/latest/download/que"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    chmod +x "/tmp/$APP_NAME"
    sudo mv "/tmp/$APP_NAME" "$INSTALL_PATH"
    echo "✅ Success! You can now run: $APP_NAME your_file.que"
else
    echo "❌ Error: Could not download binary."
    echo "Check that you have a RELEASE and an ASSET named 'que' at:"
    echo "https://github.com$USER/$REPO/releases"
    exit 1
fi