# Windows Build Guide for Handy

This guide documents how to build Handy on Windows, including solutions to common build issues.

## Prerequisites

### Required Software

1. **Git for Windows**
   - Download from: https://git-scm.com/download/win
   - During installation, select "Checkout as-is, commit Unix-style line endings"

2. **Rust**
   - Download from: https://rustup.rs/
   - Install the stable MSVC toolchain (default)
   - Verify: `rustc --version`

3. **Visual Studio Build Tools**
   - Download: https://visualstudio.microsoft.com/downloads/
   - Required components:
     - Desktop development with C++
     - Windows 10/11 SDK
     - MSVC v143 (or latest) build tools

4. **Bun**
   - Download from: https://bun.sh/
   - Verify: `bun --version`

## First-Time Setup

### 1. Clone and Configure Repository

```powershell
# Clone the repository
git clone https://github.com/ferlisifrank-lgtm/HandyMedMac.git Handy
cd Handy

# CRITICAL: Enable long path support (prevents path length errors)
git config core.longpaths true

# Set line endings (if not configured globally)
git config core.autocrlf true
```

### 2. Run Platform Setup

```powershell
# This creates .cargo/config.toml with Windows-specific build settings
.\setup-platform.ps1
```

This script copies [.cargo/config.toml.windows](/.cargo/config.toml.windows) to `.cargo/config.toml`, which:
- Disables GPU acceleration (GGML_VULKAN, CUDA, Metal)
- Points to dummy Vulkan SDK directory
- Forces CPU-only inference (recommended for Windows)

### 3. Create Dummy Vulkan SDK

Windows builds require a minimal Vulkan SDK structure to satisfy whisper-rs linker checks, even though GPU acceleration is disabled.

```powershell
# Create dummy Vulkan SDK directories and files
New-Item -ItemType Directory -Force -Path "C:\VulkanSDK\Lib"
New-Item -ItemType Directory -Force -Path "C:\VulkanSDK\Include"
New-Item -ItemType File -Force -Path "C:\VulkanSDK\Lib\vulkan-1.lib"
```

**Why is this needed?**
- The `whisper-rs-sys` crate checks for Vulkan SDK during build
- Even with `GGML_VULKAN=OFF`, CMake's `FindVulkan` module still searches for SDK files
- Creating empty files satisfies the linker without actually using Vulkan

### 4. Install Dependencies

```powershell
bun install
```

### 5. Download Models

```powershell
# Create models directory
New-Item -ItemType Directory -Force -Path "src-tauri\resources\models"

# Download Silero VAD model (required)
Invoke-WebRequest -Uri "https://blob.handy.computer/silero_vad_v4.onnx" `
  -OutFile "src-tauri\resources\models\silero_vad_v4.onnx"
```

## Building

### Development Build

```powershell
bun run tauri dev
```

### Production Build

```powershell
bun run tauri build
```

Build artifacts will be in:
- Executable: `src-tauri/target/release/Handy.exe`
- Installer: `src-tauri/target/release/bundle/`

## Known Issues and Solutions

### Issue 1: Vulkan SDK Detection Failure

**Symptom:**
```
CMake Error: Could not find Vulkan SDK
```

**Cause:**
whisper-rs-sys build script searches for Vulkan SDK even when GPU is disabled.

**Solution:**
1. Verify `.cargo/config.toml` exists (run `.\setup-platform.ps1` if missing)
2. Create dummy Vulkan SDK (see First-Time Setup step 3)
3. Verify `C:\VulkanSDK\Lib\vulkan-1.lib` exists

**Related commits:** 6d9cf4f, 1d934ac, 7c1d854

---

### Issue 2: Path Too Long Errors

**Symptom:**
```
error: path too long
filename or extension is too long
```

**Cause:**
Windows MAX_PATH limitation (260 characters) affects deep dependency trees.

**Solution:**
```powershell
# Enable long path support in Git (required)
git config core.longpaths true

# Enable long paths in Windows (requires admin, optional)
# Run PowerShell as Administrator:
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
  -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

**Related commit:** 17807c7

---

### Issue 3: GPU Acceleration Build Failures

**Symptom:**
```
error: failed to link Vulkan libraries
DirectML compilation errors
```

**Cause:**
GPU backend support (Vulkan/DirectML) is inconsistent across Windows configurations.

**Solution:**
Use CPU-only builds (default in `.cargo/config.toml.windows`). This is the recommended approach for Windows.

**Performance:**
Windows builds use Parakeet models, which are CPU-optimized and achieve 2x faster inference than Whisper on CPU.

**Related commits:** 33fc0fe, 69d5f4c, 3a7a425

---

### Issue 4: MSVC Linker Errors

**Symptom:**
```
link.exe: command not found
LINK : fatal error LNK1181
```

**Cause:**
Visual Studio Build Tools not installed or not in PATH.

**Solution:**
1. Install "Desktop development with C++" workload in Visual Studio Installer
2. Restart terminal/IDE after installation
3. Verify MSVC is available:
   ```powershell
   # This should show MSVC environment variables
   & "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
   ```

---

### Issue 5: Bun Installation Fails on ARM64

**Symptom:**
```
bun: command not found (on ARM64 Windows)
```

**Cause:**
Bun has limited ARM64 support on Windows.

**Solution:**
Use x64 baseline build of Bun. GitHub Actions workflow uses `--cpu=x64` flag for ARM64 Windows builds.

---

## Development Workflow

### Syncing Changes from Mac

```powershell
# Pull latest changes from Mac development
git pull origin main

# Rebuild (if source code changed)
bun run tauri build
```

### Making Windows-Specific Changes

If you need to make Windows-specific code changes:

```powershell
# Make your changes, then commit and push
git add .
git commit -m "fix: Windows-specific change description"
git push origin main
```

**Note:** Platform-specific code should use Rust's `cfg` attributes:
```rust
#[cfg(target_os = "windows")]
fn windows_specific_function() {
    // Windows-only code
}

#[cfg(target_os = "macos")]
fn macos_specific_function() {
    // macOS-only code
}
```

## Platform-Specific Files

| File | Status | Purpose |
|------|--------|---------|
| `.cargo/config.toml` | **Gitignored** | Platform-specific build settings (regenerate with `setup-platform.ps1`) |
| `.cargo/config.toml.windows` | **Tracked** | Windows template (checked into Git) |
| `.cargo/config.toml.macos` | **Tracked** | macOS template (checked into Git) |
| `src-tauri/Cargo.toml` | **Tracked** | Uses `[target.'cfg(windows)'.dependencies]` for platform deps |
| `src-tauri/src/**/*.rs` | **Tracked** | Uses `#[cfg(target_os = "...")]` for platform code |

## Performance Optimization

### Recommended Models for Windows

Since Windows builds use CPU-only inference, Parakeet models are recommended:

- **Parakeet Tiny Int8**: Fastest, 2x faster than Whisper Tiny on CPU
- **Parakeet Small Int8**: Balanced speed and accuracy
- **Parakeet Medium Int8**: Better accuracy, still faster than equivalent Whisper

These models are automatically marked as "platform_recommended: true" in the model manager.

### Optional: Disable AVX Instructions

For older CPUs without AVX/AVX2 support, uncomment these lines in `.cargo/config.toml`:

```toml
WHISPER_NO_AVX = "ON"
WHISPER_NO_AVX2 = "ON"
```

## Troubleshooting Checklist

Before reporting build issues, verify:

- [ ] Git `core.longpaths` is enabled
- [ ] `.cargo/config.toml` exists (run `.\setup-platform.ps1`)
- [ ] Dummy Vulkan SDK created at `C:\VulkanSDK\Lib\vulkan-1.lib`
- [ ] Visual Studio Build Tools installed with C++ workload
- [ ] Rust stable toolchain installed (`rustc --version`)
- [ ] Bun installed and in PATH (`bun --version`)
- [ ] `bun install` completed successfully
- [ ] Terminal restarted after installing build tools

## Build Verification

After a successful build, verify:

```powershell
# Check executable exists
Test-Path "src-tauri\target\release\Handy.exe"

# Run the application
.\src-tauri\target\release\Handy.exe
```

In the app:
1. Download a Parakeet model (recommended for Windows)
2. Test audio transcription
3. Verify clipboard/paste functionality

## Additional Resources

- [Main README](README.md) - Project overview
- [CLAUDE.md](CLAUDE.md) - Development guide and architecture
- [Tauri Documentation](https://tauri.app/) - Tauri framework docs
- [Whisper-rs](https://github.com/tazz4843/whisper-rs) - Rust Whisper bindings

## Getting Help

If you encounter issues not covered here:

1. Check recent commits for related fixes (search for "windows", "build", "vulkan")
2. Review GitHub Actions logs in [.github/workflows/build.yml](.github/workflows/build.yml)
3. File an issue on GitHub with:
   - Error message
   - Output of `rustc --version`, `bun --version`
   - Whether dummy Vulkan SDK is created
   - Contents of `.cargo/config.toml`
