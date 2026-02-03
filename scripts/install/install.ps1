#Requires -Version 5.1
<#
.SYNOPSIS
    Installs the tastematter CLI.
.DESCRIPTION
    Downloads and installs the tastematter CLI binary for Windows.
    Automatically adds to PATH.
.PARAMETER Version
    Version to install (default: latest)
.PARAMETER InstallDir
    Installation directory (default: ~/.local/bin)
.EXAMPLE
    irm https://install.tastematter.dev/install.ps1 | iex
#>
param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.local\bin"
)

$ErrorActionPreference = "Stop"
$BaseUrl = "https://install.tastematter.dev"
$BinaryName = "tastematter"

Write-Host "[tastematter] Installing..." -ForegroundColor Cyan

# Get version
if ($Version -eq "latest") {
    try {
        $Version = (Invoke-RestMethod "$BaseUrl/latest.txt" -UseBasicParsing).Trim()
    } catch {
        Write-Host "[tastematter] Error: Could not fetch latest version" -ForegroundColor Red
        Write-Host "  Check your internet connection or try specifying -Version" -ForegroundColor Yellow
        exit 1
    }
}
Write-Host "[tastematter] Version: $Version"

# Download URL
$downloadUrl = "$BaseUrl/releases/$Version/$BinaryName-windows-x86_64.exe"
$binaryPath = Join-Path $InstallDir "$BinaryName.exe"

# Create install directory
if (-not (Test-Path $InstallDir)) {
    Write-Host "[tastematter] Creating $InstallDir"
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Stop any running tastematter processes (required for Windows to overwrite binary)
$runningProcesses = Get-Process -Name "tastematter" -ErrorAction SilentlyContinue
if ($runningProcesses) {
    Write-Host "[tastematter] Stopping running processes for update..." -ForegroundColor Yellow
    $runningProcesses | Stop-Process -Force
    Start-Sleep -Seconds 1  # Brief pause to ensure file handles are released
    Write-Host "[tastematter] Stopped existing processes"
}

# Download binary
Write-Host "[tastematter] Downloading from $downloadUrl"
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $binaryPath -UseBasicParsing
} catch {
    Write-Host "[tastematter] Error: Download failed" -ForegroundColor Red
    Write-Host "  URL: $downloadUrl" -ForegroundColor Yellow
    exit 1
}

# Verify download
if (-not (Test-Path $binaryPath)) {
    Write-Host "[tastematter] Error: Binary not found after download" -ForegroundColor Red
    exit 1
}

$fileSize = (Get-Item $binaryPath).Length
$fileSizeMB = $fileSize / 1MB
Write-Host "[tastematter] Downloaded $([math]::Round($fileSizeMB, 1)) MB"

# Verify download is not truncated (binary should be at least 10 MB)
$minSize = 10 * 1MB
if ($fileSize -lt $minSize) {
    Write-Host "[tastematter] Error: Download appears truncated ($([math]::Round($fileSizeMB, 1)) MB < 10 MB minimum)" -ForegroundColor Red
    Write-Host "  This usually means the download was interrupted." -ForegroundColor Yellow
    Write-Host "  Please try running the install script again." -ForegroundColor Yellow
    Remove-Item $binaryPath -Force
    exit 1
}

# Add to PATH if needed
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
    Write-Host "[tastematter] Added $InstallDir to PATH" -ForegroundColor Green
    Write-Host "[tastematter] Restart your terminal for PATH changes to take effect" -ForegroundColor Yellow
} else {
    Write-Host "[tastematter] $InstallDir already in PATH"
}

# Register daemon to run on login (best-effort, warn on failure)
Write-Host "[tastematter] Setting up background sync..."
try {
    $daemonResult = & $binaryPath daemon install --interval 30 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[tastematter] Background sync registered (runs on login)" -ForegroundColor Green
    } else {
        Write-Host "[tastematter] Warning: Could not register background sync" -ForegroundColor Yellow
        Write-Host "  Run 'tastematter daemon install' manually to enable" -ForegroundColor Yellow
    }
} catch {
    Write-Host "[tastematter] Warning: Could not register background sync" -ForegroundColor Yellow
    Write-Host "  Run 'tastematter daemon install' manually to enable" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "[tastematter] Installation complete!" -ForegroundColor Green
Write-Host "  Run 'tastematter --help' to get started"
Write-Host "  Background sync will start on next login"
Write-Host "  Check status with: tastematter daemon status"
Write-Host "  (Restart terminal first if PATH was updated)"
