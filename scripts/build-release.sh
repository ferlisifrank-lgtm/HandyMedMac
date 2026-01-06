#!/bin/bash

# Handy Release Build Script
# Builds app, extracts signatures, and prepares files for GitHub Release

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Handy Release Build Script          ║${NC}"
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo ""

# Check for required environment variables
if [ -z "$TAURI_SIGNING_PRIVATE_KEY" ]; then
    echo -e "${RED}Error: TAURI_SIGNING_PRIVATE_KEY environment variable not set${NC}"
    echo ""
    echo "Please add this to your ~/.zshrc or ~/.bash_profile:"
    echo ""
    echo 'export TAURI_SIGNING_PRIVATE_KEY="dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5UXV6QXRWU0J4d2ZGaWNnc0VORkxsdmJKczhiWE1nL0Y3b1R4R0Nyc0RlSUFBQkFBQUFBQUFBQUFBQUlBQUFBQTE2SlRLb1J2aExtejNGSTVTM1FwOWp5VURKc2krR2R4MTlwWHVCRGxjSzFwODV2L284MVhFNHVFckhhVzZWTzRDcit5bWJwTXQxYkNjM3ZFRXcwYlQyT05jNnp5TWdKWUk3RXFYc0hZenVZMENGQnlWTVB3TzFYUVFYWGhmVVl0NGpzNDBxMUc4ZlE9Cg=="'
    echo 'export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""'
    echo ""
    echo "Then run: source ~/.zshrc"
    exit 1
fi

# Set empty password if not set
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

# Set CMAKE policy minimum for macOS compatibility
export CMAKE_POLICY_VERSION_MINIMUM=3.5

# Get project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Read current version from tauri.conf.json
CURRENT_VERSION=$(grep -o '"version": "[^"]*"' src-tauri/tauri.conf.json | cut -d'"' -f4)
echo -e "${BLUE}Current version:${NC} $CURRENT_VERSION"

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Increment patch version
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="${MAJOR}.${MINOR}.${NEW_PATCH}"

echo -e "${GREEN}New version:${NC} $NEW_VERSION"
echo ""

# Ask for confirmation
read -p "$(echo -e ${YELLOW}Build version $NEW_VERSION? [Y/n]:${NC} )" -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ ! -z $REPLY ]]; then
    echo "Build cancelled."
    exit 0
fi

# Update version in tauri.conf.json
echo -e "${BLUE}Updating version in tauri.conf.json...${NC}"
sed -i.bak "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json
rm src-tauri/tauri.conf.json.bak

# Read release notes
RELEASE_NOTES_FILE="$SCRIPT_DIR/RELEASE_NOTES.md"
if [ -f "$RELEASE_NOTES_FILE" ]; then
    RELEASE_NOTES=$(cat "$RELEASE_NOTES_FILE")
else
    RELEASE_NOTES="- Bug fixes and improvements"
fi

# Build the app
echo -e "${BLUE}Building Handy $NEW_VERSION...${NC}"
echo ""
bun run tauri build --target aarch64-apple-darwin

# Check if build succeeded
if [ ! -d "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Handy.app" ]; then
    echo -e "${RED}Build failed! Handy.app not found${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Build completed successfully${NC}"
echo ""

# Create release-output directory
OUTPUT_DIR="$PROJECT_ROOT/release-output"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Copy DMG file
echo -e "${BLUE}Copying release files...${NC}"
DMG_FILE="src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/Handy_${NEW_VERSION}_aarch64.dmg"
if [ -f "$DMG_FILE" ]; then
    cp "$DMG_FILE" "$OUTPUT_DIR/"
    echo -e "${GREEN}✓ Copied DMG file${NC}"
else
    echo -e "${RED}Error: DMG file not found at $DMG_FILE${NC}"
    exit 1
fi

# Check if gh CLI is available
if ! command -v gh &> /dev/null; then
    echo -e "${YELLOW}Warning: GitHub CLI (gh) not found${NC}"
    echo -e "${YELLOW}Install with: brew install gh${NC}"
    SKIP_UPLOAD=true
else
    # Check if user is authenticated
    if ! gh auth status &> /dev/null; then
        echo -e "${YELLOW}Warning: Not authenticated with GitHub CLI${NC}"
        echo -e "${YELLOW}Run: gh auth login${NC}"
        SKIP_UPLOAD=true
    else
        SKIP_UPLOAD=false
    fi
fi

# Ask if user wants to upload to GitHub
if [ "$SKIP_UPLOAD" = false ]; then
    echo ""
    read -p "$(echo -e ${YELLOW}Upload to GitHub and publish release? [Y/n]:${NC} )" -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        echo -e "${BLUE}Creating GitHub release...${NC}"

        # Create release and upload files
        cd "$OUTPUT_DIR"

        # Check if tag already exists
        if gh release view "v${NEW_VERSION}" &> /dev/null; then
            echo -e "${YELLOW}Release v${NEW_VERSION} already exists. Deleting...${NC}"
            gh release delete "v${NEW_VERSION}" -y
        fi

        # Create release with notes
        gh release create "v${NEW_VERSION}" \
            --title "Handy v${NEW_VERSION}" \
            --notes "$RELEASE_NOTES" \
            --repo ferlisifrank-lgtm/HandyMedMac \
            "Handy_${NEW_VERSION}_aarch64.dmg"

        if [ $? -eq 0 ]; then
            echo ""
            echo -e "${GREEN}✓ Release published successfully!${NC}"
            echo -e "${BLUE}View release:${NC} https://github.com/ferlisifrank-lgtm/HandyMedMac/releases/tag/v${NEW_VERSION}"
            echo ""
            echo -e "${GREEN}✓ Users can download the DMG from the release page${NC}"
            echo ""
            echo -e "${YELLOW}Note: Users will see a security warning when opening the DMG.${NC}"
            echo -e "${YELLOW}Instructions: Right-click DMG → Open, or use System Settings → Privacy & Security → Open Anyway${NC}"
            UPLOADED=true
        else
            echo -e "${RED}Failed to create release${NC}"
            UPLOADED=false
        fi

        cd "$PROJECT_ROOT"
    else
        UPLOADED=false
    fi
else
    UPLOADED=false
fi

# Create upload instructions if not uploaded
if [ "$UPLOADED" = false ]; then
cat > "$OUTPUT_DIR/UPLOAD_INSTRUCTIONS.txt" << 'INSTRUCTIONS'
═══════════════════════════════════════════════════════════════
 GitHub Release Upload Instructions
═══════════════════════════════════════════════════════════════

INSTRUCTIONS

cat >> "$OUTPUT_DIR/UPLOAD_INSTRUCTIONS.txt" << EOF

1. Go to GitHub Releases:
   https://github.com/ferlisifrank-lgtm/HandyMedMac/releases/new

2. Create new release:
   - Tag: v${NEW_VERSION}
   - Title: Handy v${NEW_VERSION}
   - Description: (copy from RELEASE_NOTES.md or customize)

3. Upload the DMG file from release-output/:
   ✓ Handy_${NEW_VERSION}_aarch64.dmg

4. Click "Publish release"

5. Users will download the DMG file manually and install it

═══════════════════════════════════════════════════════════════
 Important Notes
═══════════════════════════════════════════════════════════════

- Auto-updates are disabled to avoid macOS Gatekeeper issues
  with unsigned apps

- Users need to download and install the DMG manually

- For signed builds with auto-updates, enroll in Apple Developer
  Program ($99/year) and configure Developer ID signing

- Make sure to publish the release (not just save as draft)
  for the updater to detect it

EOF

    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   Build Complete!                      ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BLUE}Version:${NC} $NEW_VERSION"
    echo -e "${BLUE}Output directory:${NC} $OUTPUT_DIR"
    echo ""
    echo -e "${BLUE}Files ready for upload:${NC}"
    ls -lh "$OUTPUT_DIR" | tail -n +2 | awk '{printf "  - %-50s %10s\n", $9, $5}'
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "  1. Review files in: $OUTPUT_DIR"
    echo "  2. Read: $OUTPUT_DIR/UPLOAD_INSTRUCTIONS.txt"
    echo "  3. Upload to GitHub Releases"
    echo ""
else
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   Build & Release Complete!            ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""
fi
