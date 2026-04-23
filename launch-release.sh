#!/usr/bin/env bash
# Drive the Windows native build/launch of AYR Audio Link from a WSL shell.
#
# Invokes Windows PowerShell against the repo on \\wsl$\… so the binary is
# target\release\ayr-audio-link.exe — the same path installer\build-installer.ps1 packages.
#
# Usage (from repo root, inside WSL):
#   ./launch-release.sh
#
# Requirements on Windows (same as Meter):
#   Rust MSVC toolchain, Visual Studio Build Tools / Windows SDK for linking.

set -euo pipefail

if ! command -v powershell.exe >/dev/null 2>&1; then
  echo "error: powershell.exe not on PATH - are you inside WSL with Windows interop enabled?" >&2
  exit 1
fi

if ! command -v wslpath >/dev/null 2>&1; then
  echo "error: wslpath missing - this script must run inside WSL." >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PS1_WIN="$(wslpath -w "$SCRIPT_DIR/launch-release.ps1")"

exec powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \
  "\$env:Path = \"\$env:USERPROFILE\\.cargo\\bin;\$env:Path\"; & '$PS1_WIN'"
