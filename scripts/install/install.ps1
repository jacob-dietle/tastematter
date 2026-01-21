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

$fileSize = (Get-Item $binaryPath).Length / 1MB
Write-Host "[tastematter] Downloaded $([math]::Round($fileSize, 1)) MB"

# Add to PATH if needed
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
    Write-Host "[tastematter] Added $InstallDir to PATH" -ForegroundColor Green
    Write-Host "[tastematter] Restart your terminal for PATH changes to take effect" -ForegroundColor Yellow
} else {
    Write-Host "[tastematter] $InstallDir already in PATH"
}

Write-Host ""
Write-Host "[tastematter] Installation complete!" -ForegroundColor Green
Write-Host "  Run 'tastematter --help' to get started"
Write-Host "  (Restart terminal first if PATH was updated)"
