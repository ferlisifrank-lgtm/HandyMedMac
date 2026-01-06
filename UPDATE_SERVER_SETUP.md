# Custom Update Server Setup

This document explains how to set up and deploy your custom update server for Handy.

## Quick Start: GitHub Releases (Recommended)

The simplest way to distribute updates is using GitHub Releases. This requires no external server and is completely free.

### One-Time Setup

1. **Install GitHub CLI** (if not already installed):

   ```bash
   brew install gh
   ```

2. **Authenticate with GitHub**:

   ```bash
   gh auth login
   ```

   Follow the prompts to authenticate.

3. **Set environment variables** (add to `~/.zshrc` or `~/.bash_profile`):

   ```bash
   export TAURI_SIGNING_PRIVATE_KEY="dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5UXV6QXRWU0J4d2ZGaWNnc0VORkxsdmJKczhiWE1nL0Y3b1R4R0Nyc0RlSUFBQkFBQUFBQUFBQUFBQUlBQUFBQTE2SlRLb1J2aExtejNGSTVTM1FwOWp5VURKc2krR2R4MTlwWHVCRGxjSzFwODV2L284MVhFNHVFckhhVzZWTzRDcit5bWJwTXQxYkNjM3ZFRXcwYlQyT05jNnp5TWdKWUk3RXFYc0hZenVZMENGQnlWTVB3TzFYUVFYWGhmVVl0NGpzNDBxMUc4ZlE9Cg=="
   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
   ```

   Then reload: `source ~/.zshrc`

4. **Make build script executable**:
   ```bash
   chmod +x scripts/build-release.sh
   ```

### Creating a Release

1. **Edit release notes** (optional):

   ```bash
   nano scripts/RELEASE_NOTES.md
   ```

2. **Run build script**:

   ```bash
   ./scripts/build-release.sh
   ```

   The script will:
   - Auto-increment version (0.6.8 → 0.6.9)
   - Build the app for Apple Silicon
   - Extract signatures
   - Generate `latest.json`
   - **Automatically create GitHub release and upload files**

3. **Confirm upload** when prompted:
   - Script asks: "Upload to GitHub and publish release? [Y/n]"
   - Press Enter or Y to automatically upload
   - Files are published to GitHub instantly

4. **Test**:
   - Open previous version of Handy
   - App should automatically detect and install new version v0.6.9

**That's it!** The entire process is automated - just run the script and confirm.

### How It Works

The app is configured to check:

```
https://github.com/ferlisifrank-lgtm/HandyMedMac/releases/latest/download/latest.json
```

GitHub's "latest" endpoint automatically serves the most recent release, so you don't need to update any URLs - just publish new releases!

---

## Alternative: External Update Server

If you need more control (analytics, staged rollouts, etc.), you can use an external server.

## Overview

Handy now uses a custom update system with its own signing keys. This prevents your custom builds from reverting to the original repository when users update.

## Generated Keys

**Private Key** (keep secret!):

```
dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5UXV6QXRWU0J4d2ZGaWNnc0VORkxsdmJKczhiWE1nL0Y3b1R4R0Nyc0RlSUFBQkFBQUFBQUFBQUFBQUlBQUFBQTE2SlRLb1J2aExtejNGSTVTM1FwOWp5VURKc2krR2R4MTlwWHVCRGxjSzFwODV2L284MVhFNHVFckhhVzZWTzRDcit5bWJwTXQxYkNjM3ZFRXcwYlQyT05jNnp5TWdKWUk3RXFYc0hZenVZMENGQnlWTVB3TzFYUVFYWGhmVVl0NGpzNDBxMUc4ZlE9Cg==
```

**Public Key** (already added to tauri.conf.json):

```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQwMjkxMThERjdFRjRGNDAKUldSQVQrLzNqUkVwUUhZV0V5ck8vYTJpVktLMGRhelBsMWNUanRjd3JSOVZxdnBpOWt2ZjR1OTAK
```

## Deployment Options

### Option 1: Vercel/Netlify (Recommended for simplicity)

1. Deploy the example update server to Vercel:

```bash
npm install -g vercel
cd /path/to/your/update-server
vercel deploy
```

2. Update your custom domain in `src-tauri/tauri.conf.json`:

```json
"endpoints": [
  "https://your-vercel-app.vercel.app/handy/latest.json"
]
```

### Option 2: Railway/Render (Node.js hosting)

1. Push your update server to GitHub
2. Connect Railway/Render to your repository
3. Set environment variable `PORT` (usually 3000)
4. Update endpoint in `tauri.conf.json`

### Option 3: AWS S3 + CloudFront (Static hosting)

1. Create S3 bucket and enable static website hosting
2. Upload `latest.json` and release artifacts
3. Set up CloudFront for HTTPS
4. Update endpoint in `tauri.conf.json`

## Update Server Implementation

The example server is already created at `update-server-example.js`. Here's what you need to customize:

```javascript
const latestVersion = {
  version: "0.6.9", // Update this with each release
  notes: "- Your release notes here",
  pub_date: new Date().toISOString(),
  platforms: {
    "darwin-aarch64": {
      signature: "YOUR_SIGNATURE_HERE", // Generated during build
      url: "https://yourdomain.com/releases/Handy_0.6.9_aarch64.app.tar.gz",
    },
    "darwin-x86_64": {
      signature: "YOUR_SIGNATURE_HERE",
      url: "https://yourdomain.com/releases/Handy_0.6.9_x64.app.tar.gz",
    },
  },
};
```

## Building and Signing Releases

### Environment Variables

Set these before building:

```bash
export TAURI_SIGNING_PRIVATE_KEY="dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5UXV6QXRWU0J4d2ZGaWNnc0VORkxsdmJKczhiWE1nL0Y3b1R4R0Nyc0RlSUFBQkFBQUFBQUFBQUFBQUlBQUFBQTE2SlRLb1J2aExtejNGSTVTM1FwOWp5VURKc2krR2R4MTlwWHVCRGxjSzFwODV2L284MVhFNHVFckhhVzZWTzRDcit5bWJwTXQxYkNjM3ZFRXcwYlQyT05jNnp5TWdKWUk3RXFYc0hZenVZMENGQnlWTVB3TzFYUVFYWGhmVVl0NGpzNDBxMUc4ZlE9Cg=="
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""  # Empty since we generated without password
```

### Build Process

1. Update version in `src-tauri/tauri.conf.json`:

```json
"version": "0.6.9"
```

2. Build the release:

```bash
bun run tauri build
```

3. Find the generated signature files:

```bash
cat src-tauri/target/release/bundle/macos/Handy.app.tar.gz.sig
```

4. Update your `latest.json` with:
   - New version number
   - New signatures from `.sig` files
   - URLs to your hosted release artifacts

5. Upload artifacts to your hosting:
   - `Handy_0.6.9_aarch64.app.tar.gz`
   - `Handy_0.6.9_x64.app.tar.gz`

### Automated GitHub Actions Workflow

Create `.github/workflows/release.yml`:

```yaml
name: Release
on:
  push:
    tags:
      - "v*"

jobs:
  build:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - name: Install dependencies
        run: bun install

      - name: Build app
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ""
        run: bun run tauri build

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: macos-release
          path: src-tauri/target/release/bundle/macos/*.tar.gz*

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            src-tauri/target/release/bundle/macos/*.tar.gz
            src-tauri/target/release/bundle/macos/*.tar.gz.sig
```

Add your private key as a GitHub secret:

1. Go to Settings → Secrets and variables → Actions
2. Update `TAURI_SIGNING_PRIVATE_KEY` with the new private key shown above
3. Ensure `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` is set to an empty string (or remove it)

The existing [.github/workflows/release.yml](.github/workflows/release.yml) already uses these secrets and will automatically sign your builds.

## Testing Updates

1. Build and install version 0.6.8
2. Deploy your update server with version 0.6.9
3. Launch the app and check for updates
4. Verify it downloads and installs from your custom endpoint

## Security Notes

- **Keep your private key secure!** Never commit it to version control
- Store it in environment variables or secret management systems
- If compromised, generate new keys and rebuild all clients
- Use HTTPS for your update endpoint

## Troubleshooting

**Update check fails:**

- Verify endpoint URL is correct and accessible
- Check browser console for CORS errors
- Ensure `latest.json` is valid JSON

**Signature verification fails:**

- Ensure public key in `tauri.conf.json` matches your build
- Verify signatures were generated with the correct private key
- Check that `.tar.gz` files haven't been modified

**App won't install update:**

- Ensure version number in `latest.json` is higher than current
- Verify download URLs are accessible
- Check app has write permissions

## Next Steps

1. Choose a hosting provider (Vercel recommended for simplicity)
2. Deploy the update server
3. Update the endpoint URL in `tauri.conf.json`
4. Build a new release (0.6.9) with the signing keys
5. Test the update flow
