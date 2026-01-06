# Platform-specific build configuration setup for Handy (Windows)
# This script copies the Windows .cargo/config.toml template

Write-Host ""
Write-Host "Setting up Windows build configuration..." -ForegroundColor Cyan

# Ensure .cargo directory exists
$cargoDir = Join-Path $PSScriptRoot ".cargo"
if (-not (Test-Path $cargoDir)) {
    New-Item -ItemType Directory -Force -Path $cargoDir | Out-Null
}

# Copy Windows config template
$templatePath = Join-Path $PSScriptRoot ".cargo\config.toml.windows"
$targetPath = Join-Path $PSScriptRoot ".cargo\config.toml"

if (-not (Test-Path $templatePath)) {
    Write-Host "❌ Error: Template file not found: $templatePath" -ForegroundColor Red
    Write-Host "Make sure you've run 'git pull' to get the latest changes." -ForegroundColor Yellow
    exit 1
}

Copy-Item -Path $templatePath -Destination $targetPath -Force
Write-Host "✓ Windows configuration applied (.cargo\config.toml created)" -ForegroundColor Green

Write-Host ""
Write-Host "Platform setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Create dummy Vulkan SDK (see WINDOWS_BUILD.md for details):" -ForegroundColor White
Write-Host "   New-Item -ItemType Directory -Force -Path 'C:\VulkanSDK\Lib'" -ForegroundColor Gray
Write-Host "   New-Item -ItemType Directory -Force -Path 'C:\VulkanSDK\Include'" -ForegroundColor Gray
Write-Host "   New-Item -ItemType File -Force -Path 'C:\VulkanSDK\Lib\vulkan-1.lib'" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Build the app:" -ForegroundColor White
Write-Host "   bun run tauri build" -ForegroundColor Gray
Write-Host ""
