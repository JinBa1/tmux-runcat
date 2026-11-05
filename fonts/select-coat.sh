#!/usr/bin/env bash
# Select an installed coat in Windows Terminal; no fonts are rebuilt.
set -euo pipefail
coat_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -z "${WSL_DISTRO_NAME:-}" ]]; then
    printf 'Automatic coat selection currently requires Windows Terminal on WSL.\n' >&2
    exit 1
fi
coat_powershell='/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe'
if [[ ! -x "$coat_powershell" ]] || ! command -v wslpath >/dev/null 2>&1; then
    printf 'Automatic coat selection requires WSL Windows interoperability.\n' >&2
    exit 1
fi
exec "$coat_powershell" -NoProfile -ExecutionPolicy Bypass \
    -File "$(wslpath -w "$coat_dir/select-coat.ps1")" -Coat "${1:-random}"
