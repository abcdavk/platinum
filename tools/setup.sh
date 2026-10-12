
#!/usr/bin/env bash
set -Eeuo pipefail

APP_NAME="Platinum"
BINARY_URL="https://github.com/abcdavk/platinum/releases/download/v0.2.0/platinum_x86-64"
EXTENSION_REPO="https://github.com/abcdavk/platinum-extension"
ICON_URL="https://raw.githubusercontent.com/abcdavk/platinum/refs/heads/main/assets/icons/platinum.svg"
DESKTOP_FILE_URL="https://raw.githubusercontent.com/abcdavk/platinum/refs/heads/main/platinum.desktop"

BIN_DIR="$HOME/.local/bin"
BINARY_PATH="$BIN_DIR/platinum"
ICON_DIR="$HOME/Applications/.icons"
ICON_PATH="$ICON_DIR/platinum"
APPLICATIONS_DIR="$HOME/.local/share/applications"
DESKTOP_FILE_PATH="$APPLICATIONS_DIR/platinum.desktop"

# Ask for confirmation using the user's terminal.
confirm() {
    local answer

    if [[ ! -r /dev/tty ]]; then
        echo "Cannot ask for confirmation without a terminal." >&2
        return 1
    fi

    printf '%s [y/N] ' "$1" > /dev/tty
    read -r answer < /dev/tty

    [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]
}

# Check whether the current desktop session is GNOME.
desktop="${XDG_CURRENT_DESKTOP:-}:${DESKTOP_SESSION:-}"

if [[ "${desktop,,}" != *gnome* ]]; then
    echo "Error: GNOME desktop session is required." >&2
    exit 1
fi

# Detect the package manager.
if command -v dnf >/dev/null 2>&1; then
    PM="dnf"
    EXTENSIONS_PKG="gnome-extensions-app"
    PYTHON_NAUTILUS_PKG="nautilus-python"

elif command -v apt-get >/dev/null 2>&1; then
    PM="apt"
    EXTENSIONS_PKG="gnome-extensions-app"
    PYTHON_NAUTILUS_PKG="python3-nautilus"

elif command -v pacman >/dev/null 2>&1; then
    PM="pacman"
    EXTENSIONS_PKG="gnome-extensions"
    PYTHON_NAUTILUS_PKG="python-nautilus"

else
    echo "Error: Unsupported package manager." >&2
    exit 1
fi

# Install a system package.
install_package() {
    case "$PM" in
        dnf)
            sudo dnf install -y "$1"
            ;;
        apt)
            sudo apt-get install -y "$1"
            ;;
        pacman)
            sudo pacman -S --needed --noconfirm "$1"
            ;;
    esac
}

# Check whether a package is installed.
is_installed() {
    case "$PM" in
        dnf)
            rpm -q "$1" >/dev/null 2>&1
            ;;
        apt)
            dpkg-query -W -f='${Status}' "$1" 2>/dev/null \
                | grep -q '^install ok installed$'
            ;;
        pacman)
            pacman -Q "$1" >/dev/null 2>&1
            ;;
    esac
}

# Install missing packages only.
ensure_package() {
    local package="$1"

    if is_installed "$package"; then
        echo "[OK] $package is already installed."
    else
        echo "[INSTALL] Installing $package..."
        install_package "$package"
    fi
}

echo "Checking dependencies..."

ensure_package nautilus
ensure_package "$EXTENSIONS_PKG"
ensure_package "$PYTHON_NAUTILUS_PKG"
ensure_package curl
ensure_package git

mkdir -p "$BIN_DIR"
mkdir -p "$ICON_DIR"
mkdir -p "$APPLICATIONS_DIR"

# Download the Platinum binary.
if confirm "Download and install the Platinum binary?"; then
    temp_file="$(mktemp)"

    if curl -fL --progress-bar "$BINARY_URL" -o "$temp_file"; then
        install -m 755 "$temp_file" "$BINARY_PATH"
        echo "[OK] Platinum installed at $BINARY_PATH"
    else
        rm -f "$temp_file"
        echo "[ERROR] Failed to download Platinum." >&2
    fi
else
    echo "[SKIP] Platinum binary installation skipped."
fi

# Download the Platinum icon.
icon_temp_file="$(mktemp "$ICON_DIR/.platinum.XXXXXX")"
if curl -fL --progress-bar "$ICON_URL" -o "$icon_temp_file" \
    && install -m 644 "$icon_temp_file" "$ICON_PATH"; then
    echo "[OK] Platinum icon installed at $ICON_PATH"
else
    echo "[ERROR] Failed to download or install the Platinum icon." >&2
fi
rm -f "$icon_temp_file"

# Download the Platinum desktop entry.
desktop_temp_file="$(mktemp "$APPLICATIONS_DIR/.platinum.desktop.XXXXXX")"
if curl -fL --progress-bar "$DESKTOP_FILE_URL" -o "$desktop_temp_file" \
    && install -m 644 "$desktop_temp_file" "$DESKTOP_FILE_PATH"; then
    echo "[OK] Platinum desktop entry installed at $DESKTOP_FILE_PATH"
else
    echo "[ERROR] Failed to download or install the Platinum desktop entry." >&2
fi
rm -f "$desktop_temp_file"


if confirm "Clone Platinum Extension and run './platinum.sh install'?"; then
    temp_dir="$(mktemp -d)"

    if git clone "$EXTENSION_REPO" "$temp_dir/platinum-extension"; then
        cd "$temp_dir/platinum-extension"

        if [[ -f "./platinum.sh" ]]; then
            chmod +x ./platinum.sh
            ./platinum.sh install
        else
            echo "[ERROR] platinum.sh was not found." >&2
            echo "Repository contents:"
            ls -la
        fi
    else
        echo "[ERROR] Failed to clone Platinum Extension." >&2
    fi

    rm -rf "$temp_dir"
else
    echo "[SKIP] Platinum Extension installation skipped."
fi

echo
echo "Setup finished."
echo "Binary location: $BINARY_PATH"
echo "Icon location: $ICON_PATH"
echo "Desktop entry location: $DESKTOP_FILE_PATH"
echo "Make sure ~/.local/bin is in your PATH."
