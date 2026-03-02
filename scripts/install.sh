#!/bin/bash
# setup.sh

APP_NAME="que"
USER="AT-290690"
REPO="rust-lisp"

# Here is the fix: We use the variables $USER and $REPO inside the string
BINARY_URL="https://github.com{USER}/${REPO}/releases/latest/download/que"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

# 1. Download the RAW binary (fails if URL is wrong)
if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    # 2. Make it executable
    chmod +x "/tmp/$APP_NAME"
    
    # 3. Move it to the global path
    sudo mv "/tmp/$APP_NAME" "$INSTALL_PATH"

    echo "✅ Success! You can now run: $APP_NAME your_file.que"
else
    echo "❌ Error: Could not download binary."
    echo "Check that you have a RELEASE and an ASSET named 'que' at:"
    echo "https://github.comUSER/$REPO/releases"
    exit 1
fi