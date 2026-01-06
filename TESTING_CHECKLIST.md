# Ephemeral Mode Testing Checklist

**App Status:** ✅ Running in dev mode
**Build Time:** 14.47s
**Compilation:** ✅ Success (warnings about unused history code are expected)

---

## Manual Testing Steps

### 1. Verify Privacy Notice Appears ✅

- [ ] Open the app settings
- [ ] Go to "General" tab
- [ ] **Expected:** Blue informational banner at the top saying "Privacy-First Ephemeral Mode"
- [ ] **Expected:** Banner explains:
  - Audio recordings are not saved to disk
  - Transcriptions are not saved to disk
  - Text is processed in-memory and pasted
  - All processing happens locally
  - Warning about clipboard managers

---

### 2. Verify Medical Mode Default Changed ✅

- [ ] Go to Advanced settings (or wherever medical mode toggle is)
- [ ] **Expected:** Medical mode should be **OFF by default** (was ON before)
- [ ] This requires explicit user opt-in now (PIPEDA compliance)

---

### 3. Test Basic Transcription Flow ✅

- [ ] Click the microphone or press transcription shortcut
- [ ] Say a few words (e.g., "Testing ephemeral mode")
- [ ] Stop recording
- [ ] **Expected:** Text should be pasted to active application
- [ ] **Expected:** No errors in console

---

### 4. Verify No Files Created (CRITICAL) ✅

After doing a transcription, run these commands:

```bash
# Check recordings directory
ls -la ~/Library/Application\ Support/com.pais.handy/recordings/

# Expected: Directory is empty or doesn't exist
# If files exist, they're from before the ephemeral mode implementation
```

```bash
# Check for history database
ls -la ~/Library/Application\ Support/com.pais.handy/history.db

# Expected: File doesn't exist or has old timestamp
# New transcriptions should NOT update this file
```

```bash
# Check database modification time (if exists)
stat -f "%Sm" ~/Library/Application\ Support/com.pais.handy/history.db

# Expected: Timestamp should be BEFORE today (old data)
```

---

### 5. Verify Log Level Default ✅

- [ ] Go to Debug settings
- [ ] Check log level setting
- [ ] **Expected:** Default should be **WARN** (not DEBUG)
- [ ] This prevents sensitive transcription text from being logged

---

### 6. Test Medical Vocabulary (if enabled) ✅

- [ ] Enable medical mode in settings
- [ ] Transcribe medical terms (e.g., "patient has hypertension")
- [ ] **Expected:** Medical vocabulary processing works
- [ ] **Expected:** Still no files created on disk
- [ ] **Expected:** Text pasted correctly

---

### 7. Test Chinese Conversion (if applicable) ✅

- [ ] Set language to Simplified or Traditional Chinese
- [ ] Transcribe Chinese speech
- [ ] **Expected:** Conversion works
- [ ] **Expected:** Still no files created

---

### 8. Verify History UI Disabled ✅

- [ ] Look for "History" section in sidebar
- [ ] **Expected:** History section may still appear (not removed from UI yet)
- [ ] **Expected:** If you click it, it should show no data
- [ ] **Note:** Full UI removal is in "Future Work"

---

### 9. Check Console for Errors ✅

While app is running, check terminal for:

- [ ] **No errors** related to HistoryManager
- [ ] **No errors** about missing history database
- [ ] Warnings about unused history functions are **OK** (expected)

---

### 10. Verify Performance ✅

- [ ] Time from speech end to paste completion
- [ ] **Expected:** Should feel slightly faster (no I/O overhead)
- [ ] **Expected:** No noticeable lag

---

### 11. Test Multiple Transcriptions ✅

- [ ] Do 3-5 transcriptions in a row
- [ ] Check recordings directory after each
- [ ] **Expected:** No accumulation of files
- [ ] **Expected:** Memory usage stable (no memory leak)

---

### 12. Verify Clipboard Behavior ✅

- [ ] Before transcription, copy some text to clipboard
- [ ] Do a transcription
- [ ] After paste, check clipboard content
- [ ] **Expected:** Original clipboard content should be restored
- [ ] **Note:** This is existing behavior, not ephemeral mode specific

---

## Automated Verification Script

Run this after testing:

```bash
#!/bin/bash

echo "=== Ephemeral Mode Verification ==="
echo ""

APP_DATA_DIR=~/Library/Application\ Support/com.pais.handy

# Check recordings directory
echo "1. Checking recordings directory..."
if [ -d "$APP_DATA_DIR/recordings" ]; then
    FILE_COUNT=$(ls -1 "$APP_DATA_DIR/recordings" 2>/dev/null | wc -l)
    if [ "$FILE_COUNT" -eq 0 ]; then
        echo "   ✅ No recording files (ephemeral mode working)"
    else
        echo "   ⚠️  Found $FILE_COUNT files (may be old files from before ephemeral mode)"
        ls -lh "$APP_DATA_DIR/recordings"
    fi
else
    echo "   ✅ Recordings directory doesn't exist (perfect!)"
fi

# Check history database
echo ""
echo "2. Checking history database..."
if [ -f "$APP_DATA_DIR/history.db" ]; then
    MOD_TIME=$(stat -f "%Sm" "$APP_DATA_DIR/history.db")
    echo "   ⚠️  Database exists, last modified: $MOD_TIME"
    echo "   (If today, ephemeral mode may not be working)"
else
    echo "   ✅ No history database (perfect!)"
fi

# Check for any new WAV files
echo ""
echo "3. Checking for WAV files created today..."
TODAY=$(date +%Y-%m-%d)
if [ -d "$APP_DATA_DIR/recordings" ]; then
    NEW_FILES=$(find "$APP_DATA_DIR/recordings" -name "*.wav" -newermt "$TODAY" 2>/dev/null)
    if [ -z "$NEW_FILES" ]; then
        echo "   ✅ No new WAV files created today"
    else
        echo "   ❌ ERROR: New WAV files found!"
        echo "$NEW_FILES"
    fi
else
    echo "   ✅ No recordings directory"
fi

echo ""
echo "=== Verification Complete ==="
```

Save as `verify_ephemeral.sh` and run: `bash verify_ephemeral.sh`

---

## Expected Build Warnings (OK to ignore)

These warnings are **expected** and **harmless**:

```
warning: function `get_history_entries` is never used
warning: function `toggle_history_entry_saved` is never used
warning: function `delete_history_entry` is never used
warning: static `MIGRATIONS` is never used
warning: struct `HistoryManager` is never constructed
```

**Reason:** History system is disabled but code still exists (commented out or unused). These can be cleaned up later.

---

## Success Criteria

**Ephemeral mode is working correctly if:**

✅ App launches without errors
✅ Privacy notice appears in General Settings
✅ Medical mode defaults to OFF
✅ Transcriptions paste successfully
✅ **NO new files in recordings/ directory**
✅ **NO updates to history.db**
✅ Log level defaults to WARN
✅ Performance is same or better

---

## If You Find Issues

### Issue: Files are being created

**Fix:** Check that you're running the latest build

```bash
# Rebuild from scratch
bun run tauri build
# Or restart dev server
pkill -f "tauri dev"
bun run tauri dev
```

### Issue: Privacy notice doesn't appear

**Fix:** Check React dev console for errors

- Press Cmd+Option+I (macOS) or F12 (Windows/Linux)
- Look for component errors

### Issue: App crashes on transcription

**Fix:** Check console logs

- Look for errors related to HistoryManager
- May need to clear old app data

### Issue: History UI shows errors

**Fix:** This is expected - History UI hasn't been removed yet

- Just don't use History section
- Future work will clean this up

---

## Clean Up Old Data (Optional)

If you want to remove all old recordings:

```bash
# DANGER: This deletes all old transcriptions and recordings
rm -rf ~/Library/Application\ Support/com.pais.handy/recordings/
rm ~/Library/Application\ Support/com.pais.handy/history.db

# Then restart the app
```

---

## Next Steps After Testing

Once verification passes:

1. **Optional:** Remove History UI components
2. **Optional:** Add medical mode consent dialog
3. **Optional:** Create privacy policy document
4. **Ready:** Deploy to production

---

**Testing Date:** **\*\*\*\***\_**\*\*\*\***
**Tester:** **\*\*\*\***\_**\*\*\*\***
**Result:** ⬜ PASS ⬜ FAIL
**Notes:**
