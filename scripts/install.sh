#!/bin/bash
# setup.sh

APP_NAME="que"
# Change these two lines!
USER="AT-290690"
REPO="rust-lisp"

# This is the magic URL for direct binary downloads
BINARY_URL="https://github.com"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

# 1. Download (the -f flag fails if the URL is wrong, preventing HTML downloads)
if curl -fsSL "$BINARY_URL" -o "/tmp/$APP_NAME"; then
    # 2. Grant execution rights
    chmod +x "/tmp/$APP_NAME"
    
    # 3. Move to global path (sudo is usually needed for /usr/local/bin)
    # If running via 'curl | sudo bash', the 'mv' doesn't need sudo inside
    mv "/tmp/$APP_NAME" "$INSTALL_PATH"

    echo "Success! You can now run: $APP_NAME your-script.que"
else
    echo "Error: Could not download binary. Check if the Release exists on GitHub."
    exit 1
fi
