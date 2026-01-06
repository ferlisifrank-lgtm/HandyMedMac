═══════════════════════════════════════════════════════════
  Handy - Speech-to-Text for macOS
═══════════════════════════════════════════════════════════

To install Handy without security warnings:

METHOD 1 (Recommended - Terminal):
  Open Terminal and run:

  xattr -cr "/Volumes/Handy/Handy.app"

  Then drag Handy.app to Applications

METHOD 2 (Right-click):
  1. Right-click on Handy.app
  2. Select "Open"
  3. Click "Open" in the confirmation dialog
  4. The app will launch (future launches won't show warnings)

METHOD 3 (System Settings):
  1. Try to open Handy.app
  2. Go to System Settings → Privacy & Security
  3. Click "Open Anyway"

─────────────────────────────────────────────────────────

Why do I see a security warning?

macOS shows warnings for apps downloaded from the internet.
Handy is properly signed with Apple's code signing, but it's
not notarized (which requires a paid Apple Developer account).

The warning only appears once. After approving, the app works
normally without any issues.

─────────────────────────────────────────────────────────

Questions? Visit: https://github.com/ferlisifrank-lgtm/HandyMedMac
