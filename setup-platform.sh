#!/bin/bash
# Platform-specific build configuration setup for Handy
# This script copies the appropriate .cargo/config.toml template for your platform

set -e

# Detect platform
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')

echo "Detected platform: $PLATFORM"

# Ensure .cargo directory exists
mkdir -p .cargo

# Copy appropriate config template
if [[ "$PLATFORM" == "darwin" ]]; then
    echo "Setting up macOS build configuration..."
    cp .cargo/config.toml.macos .cargo/config.toml
    echo "✓ macOS configuration applied (.cargo/config.toml created)"
elif [[ "$PLATFORM" == "linux" ]]; then
    echo "Setting up Linux build configuration..."
    cp .cargo/config.toml.macos .cargo/config.toml  # Use same as macOS for now
    echo "✓ Linux configuration applied (.cargo/config.toml created)"
else
    echo "❌ Unknown platform: $PLATFORM"
    echo "Expected 'darwin' (macOS) or 'linux'"
    exit 1
fi

echo ""
echo "Platform setup complete!"
echo "You can now run: bun run tauri dev"
