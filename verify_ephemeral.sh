#!/bin/bash

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║         Handy Ephemeral Mode Verification Script              ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

APP_DATA_DIR=~/Library/Application\ Support/com.pais.handy
PASS=0
FAIL=0

# Check recordings directory
echo "📁 Checking recordings directory..."
if [ -d "$APP_DATA_DIR/recordings" ]; then
    FILE_COUNT=$(ls -1 "$APP_DATA_DIR/recordings" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$FILE_COUNT" -eq 0 ]; then
        echo "   ✅ PASS: No recording files found (ephemeral mode working)"
        PASS=$((PASS + 1))
    else
        echo "   ⚠️  WARN: Found $FILE_COUNT files"
        echo "   These may be old files from before ephemeral mode:"
        ls -lht "$APP_DATA_DIR/recordings" | head -5
        echo ""
        echo "   Check modification dates - files from today indicate a problem!"
        FAIL=$((FAIL + 1))
    fi
else
    echo "   ✅ PASS: Recordings directory doesn't exist (perfect!)"
    PASS=$((PASS + 1))
fi

echo ""

# Check history database
echo "💾 Checking history database..."
if [ -f "$APP_DATA_DIR/history.db" ]; then
    MOD_TIME=$(stat -f "%Sm" "$APP_DATA_DIR/history.db" 2>/dev/null || stat -c "%y" "$APP_DATA_DIR/history.db" 2>/dev/null)
    TODAY=$(date +%Y-%m-%d)

    if echo "$MOD_TIME" | grep -q "$TODAY"; then
        echo "   ❌ FAIL: Database modified TODAY!"
        echo "   Last modified: $MOD_TIME"
        echo "   Ephemeral mode may not be working correctly."
        FAIL=$((FAIL + 1))
    else
        echo "   ✅ PASS: Database exists but not modified today"
        echo "   Last modified: $MOD_TIME (old data)"
        PASS=$((PASS + 1))
    fi
else
    echo "   ✅ PASS: No history database found (perfect!)"
    PASS=$((PASS + 1))
fi

echo ""

# Check for newly created WAV files
echo "🎙️  Checking for WAV files created in last hour..."
if [ -d "$APP_DATA_DIR/recordings" ]; then
    NEW_FILES=$(find "$APP_DATA_DIR/recordings" -name "*.wav" -mmin -60 2>/dev/null)
    if [ -z "$NEW_FILES" ]; then
        echo "   ✅ PASS: No new WAV files in last hour"
        PASS=$((PASS + 1))
    else
        echo "   ❌ FAIL: New WAV files found!"
        echo "$NEW_FILES"
        echo ""
        echo "   This indicates ephemeral mode is NOT working!"
        FAIL=$((FAIL + 1))
    fi
else
    echo "   ✅ PASS: No recordings directory exists"
    PASS=$((PASS + 1))
fi

echo ""

# Check if app is running
echo "🚀 Checking if Handy is running..."
if pgrep -x "handy" > /dev/null; then
    echo "   ✅ PASS: Handy process is running"
    PASS=$((PASS + 1))
else
    echo "   ⚠️  INFO: Handy is not currently running"
    echo "   Run: bun run tauri dev"
fi

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
echo "══════════════════════════════════════════════════════════════"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ ALL CHECKS PASSED! Ephemeral mode is working correctly."
    echo ""
    echo "Next steps:"
    echo "1. Test transcription manually"
    echo "2. Run this script again after transcription"
    echo "3. Verify the privacy notice appears in General Settings"
    exit 0
else
    echo "❌ SOME CHECKS FAILED"
    echo ""
    echo "Troubleshooting:"
    echo "1. Make sure you rebuilt the app after changes"
    echo "2. Check if you're running the dev version"
    echo "3. Look for errors in the console"
    echo "4. See TESTING_CHECKLIST.md for detailed steps"
    exit 1
fi
