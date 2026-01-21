# Fix PATH for tastematter CLI
$binPath = "$env:USERPROFILE\.context-os\bin"
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

if ($currentPath -notlike "*$binPath*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$binPath", "User")
    Write-Host "[INFO] Added $binPath to PATH" -ForegroundColor Green
    Write-Host "[WARN] Restart your terminal for changes to take effect" -ForegroundColor Yellow
} else {
    Write-Host "[INFO] PATH already contains $binPath" -ForegroundColor Green
}

# Verify
Write-Host ""
Write-Host "Current wrapper location:"
if (Test-Path "$binPath\tastematter.cmd") {
    Write-Host "  $binPath\tastematter.cmd" -ForegroundColor Cyan
} else {
    Write-Host "  [WARNING] tastematter.cmd not found at $binPath" -ForegroundColor Red
}
