#!/usr/bin/env bash
# Build and install the default or user-selected fonts without pip or network.
set -euo pipefail

FONT_SETUP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FONT_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/tmux-runcat"
FONT_BUILD_ONLY=false
FONT_BASE="${RUNCAT_BASE_FONT:-}"
FONT_BOLD="${RUNCAT_BOLD_FONT:-}"
FONT_ITALIC="${RUNCAT_ITALIC_FONT:-}"
FONT_BOLD_ITALIC="${RUNCAT_BOLD_ITALIC_FONT:-}"
FONT_FAMILY="${RUNCAT_FONT_FAMILY:-}"
FONT_FUR="${RUNCAT_FUR:-mono}"
usage() {
    cat <<'USAGE'
Usage: fonts/setup.sh [--build-only] [--base-font FILE] [--family NAME]
                      [--bold-font FILE] [--italic-font FILE] [--bold-italic-font FILE]
                      [--fur mono|tabby|orange-tabby|silver-tabby|black|tuxedo|calico]

Use static monospaced TrueType files. Optional faces require --base-font.
Without --base-font, use the bundled JetBrainsMono Nerd Font family.
Environment: RUNCAT_BASE_FONT, RUNCAT_BOLD_FONT, RUNCAT_ITALIC_FONT,
             RUNCAT_BOLD_ITALIC_FONT, RUNCAT_FONT_FAMILY, RUNCAT_FUR.
USAGE
}
while (($#)); do
    case "$1" in
        --build-only) FONT_BUILD_ONLY=true; shift ;;
        --help|-h) usage; exit 0 ;;
        --base-font|--bold-font|--italic-font|--bold-italic-font|--family|--fur)
            if (($# < 2)) || [[ -z "$2" || "$2" == --* ]]; then
                printf 'Missing value for %s\n' "$1" >&2
                exit 2
            fi
            case "$1" in
                --base-font) FONT_BASE="$2" ;;
                --bold-font) FONT_BOLD="$2" ;;
                --italic-font) FONT_ITALIC="$2" ;;
                --bold-italic-font) FONT_BOLD_ITALIC="$2" ;;
                --family) FONT_FAMILY="$2" ;;
                --fur) FONT_FUR="$2" ;;
            esac
            shift 2 ;;
        *) usage >&2; exit 2 ;;
    esac
done
if [[ -z "$FONT_BASE" && -n "$FONT_BOLD$FONT_ITALIC$FONT_BOLD_ITALIC" ]]; then
    printf 'Optional font faces require --base-font or RUNCAT_BASE_FONT.\n' >&2
    exit 2
fi
FONT_BUILD_ARGS=(--fur "$FONT_FUR")
[[ -z "$FONT_BASE" ]] || FONT_BUILD_ARGS+=(--base-font "$FONT_BASE")
[[ -z "$FONT_BOLD" ]] || FONT_BUILD_ARGS+=(--bold-font "$FONT_BOLD")
[[ -z "$FONT_ITALIC" ]] || FONT_BUILD_ARGS+=(--italic-font "$FONT_ITALIC")
[[ -z "$FONT_BOLD_ITALIC" ]] || FONT_BUILD_ARGS+=(--bold-italic-font "$FONT_BOLD_ITALIC")
[[ -z "$FONT_FAMILY" ]] || FONT_BUILD_ARGS+=(--family "$FONT_FAMILY")

if ! command -v python3 >/dev/null 2>&1 || ! python3 -c 'import sys; sys.exit(sys.version_info < (3, 11))'; then
    printf 'RunCat setup requires Python 3.11 or newer on PATH.\n' >&2
    exit 1
fi

(
    cd "$FONT_SETUP_DIR/vendor"
    sha256sum --check SHA256SUMS
)
mkdir -p "$FONT_DATA_DIR/source" "$FONT_DATA_DIR/generated"
if [[ -z "$FONT_BASE" ]]; then
    tar -xJf "$FONT_SETUP_DIR/vendor/JetBrainsMono-v3.5.1.tar.xz" -C "$FONT_DATA_DIR/source"
fi
PYTHONPATH="$FONT_SETUP_DIR/vendor/fonttools-4.67.0-py3-none-any.whl:$FONT_SETUP_DIR/vendor/pypng-0.20220715.0-py3-none-any.whl" \
    PYTHONDONTWRITEBYTECODE=1 python3 "$FONT_SETUP_DIR/build-runcat-font.py" \
    "$FONT_DATA_DIR/source" "$FONT_DATA_DIR/generated" "${FONT_BUILD_ARGS[@]}"
FONT_FAMILY="$(cat "$FONT_DATA_DIR/generated/font-family.txt")"

if "$FONT_BUILD_ONLY"; then
    printf 'Built RunCat fonts in %s\n' "$FONT_DATA_DIR/generated"
    exit 0
fi

if [[ -n "${WSL_DISTRO_NAME:-}" ]]; then
    # The terminal renders on Windows. Font installation inside WSL alone
    # cannot supply its glyphs to Windows Terminal.
    FONT_POWERSHELL='/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe'
    if [[ ! -x "$FONT_POWERSHELL" ]] || ! command -v wslpath >/dev/null 2>&1; then
        printf 'RunCat requires WSL Windows interoperability and wslpath to install its Windows fonts.\n' >&2
        exit 1
    fi
    "$FONT_POWERSHELL" -NoProfile -ExecutionPolicy Bypass \
        -File "$(wslpath -w "$FONT_SETUP_DIR/install-windows.ps1")" \
        -FontDirectory "$(wslpath -w "$FONT_DATA_DIR/generated")"
else
    FONT_INSTALL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/fonts/tmux-runcat"
    mkdir -p "$FONT_INSTALL_DIR"
    # Read the current manifest, excluding any stale fonts from an older build.
    mapfile -t FONT_FILES < <(python3 -c 'import json,sys; print("\n".join(json.load(open(sys.argv[1]))))' \
        "$FONT_DATA_DIR/generated/font-names.json")
    for font_name in "${FONT_FILES[@]}"; do
        font_file="$FONT_DATA_DIR/generated/$font_name"
        ln -sfn "$font_file" "$FONT_INSTALL_DIR/$(basename "$font_file")"
    done
    if command -v fc-cache >/dev/null 2>&1; then
        fc-cache -f "$FONT_INSTALL_DIR"
    fi
    printf 'Installed RunCat fonts. Select "%s" in your terminal.\n' "$FONT_FAMILY"
fi
