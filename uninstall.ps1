# Removes Marq completely: process, files, shortcut, registry, autostart.
$ErrorActionPreference = 'SilentlyContinue'

try { Stop-Process -Name marq -Force -ErrorAction Stop; Start-Sleep -Milliseconds 600 } catch {}

# Autostart entry (written by tauri-plugin-autostart)
Remove-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Marq'
Remove-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'marq'

# File associations
Remove-Item 'HKCU:\Software\Classes\Marq.Markdown' -Recurse -Force
foreach ($ext in '.md', '.markdown', '.mdown', '.mkd') {
    Remove-ItemProperty "HKCU:\Software\Classes\$ext\OpenWithProgids" -Name 'Marq.Markdown'
}
Remove-Item 'HKCU:\Software\Marq' -Recurse -Force
Remove-ItemProperty 'HKCU:\Software\RegisteredApplications' -Name 'Marq'

# Files, shortcut, app data
Remove-Item "$env:LOCALAPPDATA\Programs\Marq" -Recurse -Force
Remove-Item "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Marq.lnk" -Force
Remove-Item "$env:APPDATA\com.marq.viewer" -Recurse -Force
Remove-Item "$env:LOCALAPPDATA\com.marq.viewer" -Recurse -Force

Write-Host "Marq removed. If .md files still show a Marq icon, sign out and back in (Explorer icon cache)."
