#!/bin/bash

# Handy Quarantine Removal Script
# Run this before opening Handy.app to avoid Gatekeeper warnings

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_PATH="${SCRIPT_DIR}/Handy.app"

if [ ! -d "$APP_PATH" ]; then
    echo "❌ Handy.app not found at: $APP_PATH"
    echo "Please run this script from the same folder as Handy.app"
    exit 1
fi

echo "🔓 Removing quarantine attribute from Handy.app..."
xattr -cr "$APP_PATH"

echo "✅ Quarantine removed! You can now open Handy.app without warnings."
echo "Opening Handy.app..."
open "$APP_PATH"
