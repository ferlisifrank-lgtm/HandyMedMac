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

**Platform:** macOS (Metal), Windows (Vulkan), Linux (OpenBLAS+Vulkan, limited Wayland)
