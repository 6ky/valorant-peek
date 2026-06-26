# Peek one-line installer for Windows.
# Usage:  irm https://raw.githubusercontent.com/6ky/valorant-peek/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$repo = "6ky/valorant-peek"

Write-Host "Fetching the latest Peek release..."
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers @{ "User-Agent" = "peek-installer" }
$asset = $release.assets | Where-Object { $_.name -like "*-setup.exe" } | Select-Object -First 1
if (-not $asset) { throw "No setup .exe found in the latest release." }

$out = Join-Path $env:TEMP $asset.name
$sizeMb = [math]::Round($asset.size / 1MB, 1)
Write-Host "Downloading $($asset.name) ($sizeMb MB)..."
Invoke-WebRequest $asset.browser_download_url -OutFile $out -UseBasicParsing

Write-Host "Installing..."
Start-Process -FilePath $out -ArgumentList "/S" -Wait
Remove-Item $out -ErrorAction SilentlyContinue

Write-Host "Peek $($release.tag_name) installed. Launch it from the Start menu."
