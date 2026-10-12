#!/usr/bin/env bash
set -Eeuo pipefail

BIN_PATH="$HOME/.local/bin/platinum"
ICON_DIR="$HOME/Applications/.icons"
ICON_PATH="$ICON_DIR/platinum"
DESKTOP_FILE_PATH="$HOME/.local/share/applications/platinum.desktop"
GNOME_EXTENSION_DIR="$HOME/.local/share/gnome-shell/extensions/platinum-monitor@abcdavk"
NAUTILUS_EXTENSION_DIR="$HOME/.local/share/nautilus-python/extensions"

echo "Removing Platinum..."

rm -f -- "$BIN_PATH" "$ICON_PATH" "$DESKTOP_FILE_PATH"
rm -rf -- "$GNOME_EXTENSION_DIR"
rm -f -- "$NAUTILUS_EXTENSION_DIR"/platinum*.py

if [[ -d "$ICON_DIR" ]]; then
    rmdir --ignore-fail-on-non-empty "$ICON_DIR"
fi

echo "[OK] Platinum binary, icon, desktop entry, and extensions removed."
echo "System packages installed by setup.sh were left unchanged."
echo "AppImages in ~/Applications were left unchanged."
echo "Restart GNOME Shell and Nautilus if necessary."
