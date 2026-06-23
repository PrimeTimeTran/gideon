#!/bin/sh

VERSION="1.0.0"
REPO="primetimetran/gideon"
INSTALL_DIR="/usr/local/bin"

# Detect OS and Arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m | sed 's/x86_64/amd64/' | sed 's/aarch64/arm64/')

# Build Download URL
URL="https://github.com/$REPO/releases/download/v$VERSION/myapp-$OS-$ARCH"

echo "Installing myapp to $INSTALL_DIR..."

# Download and Install
curl -sSL "$URL" -o myapp
chmod +x myapp
sudo mv myapp "$INSTALL_DIR/myapp"

echo "Installation complete!"
