#!/bin/bash
# setup.sh

APP_NAME="que"
# This URL always points to the latest binary in your GitHub Releases
BINARY_URL="https://github.com"
INSTALL_PATH="/usr/local/bin/$APP_NAME"

echo "Installing $APP_NAME..."

# Download to a temporary location
curl -L "$BINARY_URL" -o "/tmp/$APP_NAME"

# Grant execution rights and move to global path
chmod +x "/tmp/$APP_NAME"
mv "/tmp/$APP_NAME" "$INSTALL_PATH"

echo "Success! You can now run: $APP_NAME your_file.que"