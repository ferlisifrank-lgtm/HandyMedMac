# CLAUDE.md

**Stack:** Tauri 2.x desktop app - Rust backend + React/TypeScript frontend. Cross-platform speech-to-text using Whisper/Parakeet models.

**Pipeline:** Audio capture → VAD (Silero) → Whisper/Parakeet → Text → Clipboard/Paste

## Quick Start

Prerequisites: [Rust](https://rustup.rs/), [Bun](https://bun.sh/)

```bash
bun install
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
bun run tauri dev  # macOS cmake error? CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev
```

## Key File Locations

| Path                           | Purpose                                       |
| ------------------------------ | --------------------------------------------- |
| `src-tauri/src/lib.rs`         | Tauri entry, manager init                     |
| `src-tauri/src/managers/`      | Audio, Model, Transcription, History managers |
| `src-tauri/src/audio_toolkit/` | Low-level audio + VAD                         |
| `src-tauri/src/commands/`      | Tauri command handlers                        |
| `src/App.tsx`                  | Main component, onboarding                    |
| `src/components/settings/`     | Settings UI (35+ files)                       |
| `src/stores/settingsStore.ts`  | Zustand state                                 |
| `src/bindings.ts`              | Auto-gen Tauri types (tauri-specta)           |
| `src/i18n/locales/`            | Translations (en/es/fr/vi)                    |

## Architecture Patterns

- **Manager Pattern:** Core logic in managers (Audio/Model/Transcription) → Tauri state
- **Communication:** Frontend ↔ Tauri commands/events ↔ Rust backend
- **State:** Zustand → Tauri Command → Rust State → tauri-plugin-store
- **Audio Flow:** Device → Recording → Resampling → VAD → Transcription

## Code Rules

**TypeScript/React:**

- i18next for ALL user strings (ESLint enforced). Add to `src/i18n/locales/en/translation.json`, use `t('key')`
- Strict types (no `any`), functional components, Tailwind CSS
- Path alias: `@/` = `./src/`

**Rust:**

- `cargo fmt` + `cargo clippy` before commit
- Explicit error handling (no unwrap in prod)

**Commits:** Conventional format (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`)

**Commands:**

```bash
bun run lint:fix     # Fix linting
bun run format       # Format TS + Rust
bun run tauri build  # Production build
```

**Debug:** `Cmd+Shift+D` (macOS) / `Ctrl+Shift+D` (Win/Linux)

**Platform:** macOS (Metal), Windows (CPU-only), Linux (OpenBLAS+Vulkan, limited Wayland)

## Cross-Platform Development

### Working Between Mac and Windows

This repo supports primary development on macOS with local builds on Windows hardware using Git for syncing.

#### First-Time Windows Setup

1. **Clone and configure:**
   ```powershell
   git clone https://github.com/ferlisifrank-lgtm/HandyMedMac.git Handy
   cd Handy
   git config core.longpaths true  # Critical for Windows
   ```

2. **Run platform setup:**
   ```powershell
   .\setup-platform.ps1
   ```
   This creates `.cargo/config.toml` with Windows-specific build settings (GPU disabled, dummy Vulkan SDK path).

3. **See [WINDOWS_BUILD.md](WINDOWS_BUILD.md)** for complete Windows build instructions, including:
   - Dummy Vulkan SDK setup (required)
   - Visual Studio Build Tools installation
   - Known issues and solutions
   - Troubleshooting guide

#### Daily Workflow

**Mac (primary development):**
```bash
# Make changes, commit, push
git add .
git commit -m "feat: your changes"
git push origin main
```

**Windows (build and test):**
```powershell
# Pull latest changes
git pull origin main

# Build release
bun run tauri build
```

#### Platform-Specific Files

- **`.cargo/config.toml`** - Gitignored (platform-specific, regenerate with setup scripts)
- **`.cargo/config.toml.windows`** - Windows template (tracked in Git)
- **`.cargo/config.toml.macos`** - macOS template (tracked in Git)
- **Source code** - Platform-specific code uses `#[cfg(target_os = "...")]` attributes

#### Windows Build Notes

- Windows builds use **CPU-only inference** (GPU disabled to avoid Vulkan SDK issues)
- **Parakeet models recommended** for Windows (2x faster CPU inference than Whisper)
- Requires dummy Vulkan SDK structure even though GPU is disabled
- See [WINDOWS_BUILD.md](WINDOWS_BUILD.md) for detailed troubleshooting
