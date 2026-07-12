# Installs Marq to a stable per-user location, adds a Start Menu shortcut,
# and launches it (first run registers file associations + autostart).
$ErrorActionPreference = 'Stop'

$src = Join-Path $PSScriptRoot 'src-tauri\target\release\marq.exe'
if (-not (Test-Path $src)) {
    Write-Error "marq.exe not found. Build first:`n  cd $PSScriptRoot\src-tauri; cargo build --release"
}

$dst = "$env:LOCALAPPDATA\Marq"  # same location the NSIS installer uses
New-Item -ItemType Directory -Force $dst | Out-Null

# Stop a running instance so the exe can be replaced
try { Stop-Process -Name marq -Force -ErrorAction Stop; Start-Sleep -Milliseconds 600 } catch {}

Copy-Item $src "$dst\marq.exe" -Force
$loader = Join-Path $PSScriptRoot 'src-tauri\target\release\WebView2Loader.dll'
if (Test-Path $loader) { Copy-Item $loader $dst -Force }

# Start Menu shortcut
$ws = New-Object -ComObject WScript.Shell
$lnk = $ws.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Marq.lnk")
$lnk.TargetPath = "$dst\marq.exe"
$lnk.WorkingDirectory = $dst
$lnk.IconLocation = "$dst\marq.exe,0"
$lnk.Description = 'Marq — instant Markdown viewer'
$lnk.Save()

# First run: registers .md associations (HKCU), enables start-with-Windows, parks in tray
Start-Process "$dst\marq.exe"

Write-Host ""
Write-Host "Marq installed to $dst" -ForegroundColor Green
Write-Host ""
Write-Host "Last step (Windows requires one click from you):"
Write-Host "  Right-click any .md file -> Open with -> Choose another app -> Marq -> Always" -ForegroundColor Yellow
Write-Host "  (or use the tray menu: 'Make default for .md files')"
