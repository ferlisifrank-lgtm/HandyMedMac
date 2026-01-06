# Quick Release Guide

This is a quick reference for creating releases. See [UPDATE_SERVER_SETUP.md](UPDATE_SERVER_SETUP.md) for full details.

## First Time Setup

```bash
# 1. Install and authenticate GitHub CLI
brew install gh
gh auth login

# 2. Add to ~/.zshrc or ~/.bash_profile
export TAURI_SIGNING_PRIVATE_KEY="dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5UXV6QXRWU0J4d2ZGaWNnc0VORkxsdmJKczhiWE1nL0Y3b1R4R0Nyc0RlSUFBQkFBQUFBQUFBQUFBQUlBQUFBQTE2SlRLb1J2aExtejNGSTVTM1FwOWp5VURKc2krR2R4MTlwWHVCRGxjSzFwODV2L284MVhFNHVFckhhVzZWTzRDcit5bWJwTXQxYkNjM3ZFRXcwYlQyT05jNnp5TWdKWUk3RXFYc0hZenVZMENGQnlWTVB3TzFYUVFYWGhmVVl0NGpzNDBxMUc4ZlE9Cg=="
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""

# 3. Reload shell
source ~/.zshrc

# 4. Make script executable
chmod +x scripts/build-release.sh
```

## Create a Release

```bash
# 1. (Optional) Edit release notes
nano scripts/RELEASE_NOTES.md

# 2. Build and publish (fully automated!)
./scripts/build-release.sh

# The script will:
# - Build the app
# - Auto-increment version
# - Ask to upload to GitHub (press Y)
# - Automatically publish release

# 3. Test
# Open Handy and check for updates
```

## What Gets Built

- `Handy_X.X.X_aarch64.app.tar.gz` - Update file (required)
- `Handy_X.X.X_aarch64.app.tar.gz.sig` - Signature (required)
- `latest.json` - Update manifest (required)
- `Handy_X.X.X_aarch64.dmg` - Direct download (optional)

## Troubleshooting

**"TAURI_SIGNING_PRIVATE_KEY environment variable not set"**

- Run: `echo $TAURI_SIGNING_PRIVATE_KEY` to check
- If empty, add export to ~/.zshrc and run `source ~/.zshrc`

**"Build failed"**

- Make sure Silero VAD model exists: `ls src-tauri/resources/models/silero_vad_v4.onnx`
- Download if missing: `curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx`

**"App doesn't detect update"**

- Make sure you published the release (not just saved as draft)
- Check that all 3 files (.tar.gz, .sig, latest.json) are uploaded
- Wait a few minutes for GitHub's CDN to update

**"Update downloads but won't install"**

- Signature mismatch - make sure you're using the correct signing key
- Check that public key in `src-tauri/tauri.conf.json` matches the private key
