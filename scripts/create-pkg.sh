#!/bin/bash

# Create a .pkg installer that removes quarantine automatically

set -e

VERSION="0.6.9"
APP_PATH="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Handy.app"
PKG_NAME="Handy_${VERSION}_aarch64.pkg"
OUTPUT_DIR="release-output"

if [ ! -d "$APP_PATH" ]; then
    echo "Error: App not found at $APP_PATH"
    echo "Please build the app first: bun run tauri build"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
mkdir -p /tmp/handy-pkg-root/Applications

# Copy app to temporary location
cp -R "$APP_PATH" /tmp/handy-pkg-root/Applications/

# Create postinstall script that removes quarantine
mkdir -p /tmp/handy-pkg-scripts
cat > /tmp/handy-pkg-scripts/postinstall << 'EOF'
#!/bin/bash
# Remove quarantine attribute after installation
xattr -cr "/Applications/Handy.app" 2>/dev/null || true
exit 0
EOF

chmod +x /tmp/handy-pkg-scripts/postinstall

# Build the package
pkgbuild --root /tmp/handy-pkg-root \
         --scripts /tmp/handy-pkg-scripts \
         --identifier "com.pais.handy" \
         --version "$VERSION" \
         --install-location / \
         "$OUTPUT_DIR/$PKG_NAME"

# Clean up
rm -rf /tmp/handy-pkg-root
rm -rf /tmp/handy-pkg-scripts

echo "✅ Package created: $OUTPUT_DIR/$PKG_NAME"
echo "This package will automatically remove quarantine attributes during installation"
