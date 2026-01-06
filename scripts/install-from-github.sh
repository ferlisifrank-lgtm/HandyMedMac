#!/bin/bash

# Handy Installation Script
# Downloads and installs Handy without quarantine warnings

set -e

VERSION="0.6.9"
DMG_URL="https://github.com/ferlisifrank-lgtm/HandyMedMac/releases/download/v${VERSION}/Handy_${VERSION}_aarch64.dmg"
DMG_NAME="Handy_${VERSION}_aarch64.dmg"

echo "📦 Downloading Handy v${VERSION}..."
curl -L -o "/tmp/${DMG_NAME}" "$DMG_URL"

echo "🔓 Removing quarantine attribute..."
xattr -cr "/tmp/${DMG_NAME}"

echo "💿 Mounting DMG..."
MOUNT_POINT=$(hdiutil attach "/tmp/${DMG_NAME}" | grep Volumes | awk '{print $3}')

echo "📲 Installing to /Applications..."
cp -R "${MOUNT_POINT}/Handy.app" /Applications/

echo "🧹 Cleaning up..."
hdiutil detach "$MOUNT_POINT"
rm "/tmp/${DMG_NAME}"

echo "✅ Handy v${VERSION} installed successfully!"
echo "You can now open Handy from /Applications/Handy.app"
