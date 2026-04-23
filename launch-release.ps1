# Build and run the release binary in THIS repo's target\release\ (same path the installer uses).
#
# After build, the .exe is copied to %LOCALAPPDATA%\AyrAudioLink\ and launched from there so
# Windows does not treat \\wsl.localhost\... as the Internet zone (SmartScreen "can't verify"
# prompt on every launch). The copy under target\release\ is left intact for installer packaging.
#
# Usage:  powershell -ExecutionPolicy Bypass -File .\launch-release.ps1

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
Set-Location $Root

$env:CARGO_TARGET_DIR = Join-Path $Root "target"

Write-Host "==> cargo build --release" -ForegroundColor Cyan
cargo build --release

$Exe = Join-Path $Root "target\release\ayr-audio-link.exe"
if (-not (Test-Path $Exe)) {
    Write-Error "Missing $Exe - build failed."
}

$Cache = Join-Path $env:LOCALAPPDATA "AyrAudioLink"
if (-not (Test-Path $Cache)) {
    New-Item -ItemType Directory -Path $Cache | Out-Null
}

Write-Host "==> Stopping running AYR Audio Link (if any)" -ForegroundColor Cyan
$cacheFull = (Resolve-Path $Cache).Path
foreach ($p in Get-Process -ErrorAction SilentlyContinue) {
    try {
        $exePath = $p.MainModule.FileName
    } catch {
        continue
    }
    if (-not $exePath) { continue }
    $dir = [IO.Path]::GetDirectoryName($exePath)
    $leaf = [IO.Path]::GetFileName($exePath)
    if ($dir.Equals($cacheFull, [StringComparison]::OrdinalIgnoreCase) -and ($leaf -like 'ayr-audio-link*.exe')) {
        Write-Host ('    Stopping PID ' + $p.Id + ' ' + $leaf) -ForegroundColor DarkGray
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    }
}
Start-Sleep -Milliseconds 400

$LocalExe = Join-Path $Cache "ayr-audio-link.exe"
Copy-Item -Path $Exe -Destination $LocalExe -Force
try { Unblock-File -Path $LocalExe -ErrorAction Stop } catch {}

Write-Host ('==> Starting ' + $LocalExe) -ForegroundColor Cyan
Start-Process -FilePath $LocalExe -WorkingDirectory $Cache -WindowStyle Normal
