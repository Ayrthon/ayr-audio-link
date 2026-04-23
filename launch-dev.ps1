# Fast iteration: debug build + staged launch under %LOCALAPPDATA%\AyrAudioLink\
# (avoids SmartScreen nagging about \\wsl.localhost\ paths).
#
# Optional: -SkipBuild  (used by launch-dev-watch.ps1 after cargo watch already ran `cargo build`)
#
# Usage:  powershell -ExecutionPolicy Bypass -File .\launch-dev.ps1

param([switch]$SkipBuild)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
Set-Location $Root

$DevTargetRoot = Join-Path $env:LOCALAPPDATA "AyrAudioLink\cargo-target-dev"
$env:CARGO_TARGET_DIR = $DevTargetRoot
Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue

if (-not $SkipBuild) {
    Write-Host "==> cargo build (debug) -> $DevTargetRoot" -ForegroundColor Cyan
    # Optional: stamp the in-app version chip (same idea as Meter’s AUDIO_METER_BUILD), e.g.
    #   $env:AYR_LINK_BUILD = "$(git rev-parse --short HEAD)"
    cargo build
}

$Exe = Join-Path $DevTargetRoot "debug\ayr-audio-link.exe"
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

# Wait until staged copies are gone so Copy-Item does not race an in-use exe.
$deadline = (Get-Date).AddSeconds(8)
while ((Get-Date) -lt $deadline) {
    $still = @()
    foreach ($p in Get-Process -ErrorAction SilentlyContinue) {
        try { $exePath = $p.MainModule.FileName } catch { continue }
        if (-not $exePath) { continue }
        $dir = [IO.Path]::GetDirectoryName($exePath)
        $leaf = [IO.Path]::GetFileName($exePath)
        if ($dir.Equals($cacheFull, [StringComparison]::OrdinalIgnoreCase) -and ($leaf -like 'ayr-audio-link*.exe')) {
            $still += $p
        }
    }
    if ($still.Count -eq 0) { break }
    Start-Sleep -Milliseconds 250
}
if ((Get-Date) -ge $deadline) {
    Write-Warning "Timed out waiting for old AYR Audio Link to exit; copy/start may fail if the exe is locked."
}

$LocalExe = Join-Path $Cache "ayr-audio-link.exe"
Copy-Item -Path $Exe -Destination $LocalExe -Force
try { Unblock-File -Path $LocalExe -ErrorAction Stop } catch {}

$built = (Get-Item $Exe).LastWriteTimeUtc.ToString("yyyy-MM-dd HH:mm:ss 'UTC'")
$staged = (Get-Item $LocalExe).LastWriteTimeUtc.ToString("yyyy-MM-dd HH:mm:ss 'UTC'")
Write-Host ("==> Staged exe timestamp: {0} (build output was {1})" -f $staged, $built) -ForegroundColor DarkGray

Write-Host ('==> Starting ' + $LocalExe) -ForegroundColor Cyan
$started = Start-Process -FilePath $LocalExe -WorkingDirectory $Cache -WindowStyle Normal -PassThru
Start-Sleep -Milliseconds 600
if ($started.HasExited) {
    Write-Error ("AYR Audio Link exited immediately (code " + $started.ExitCode + "). Check log or run from a console for errors.")
}
