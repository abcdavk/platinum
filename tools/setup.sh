
#!/usr/bin/env bash
set -Eeuo pipefail

# Check whether the current desktop session is GNOME.
desktop="${XDG_CURRENT_DESKTOP:-}:${DESKTOP_SESSION:-}"

if [[ "${desktop,,}" != *gnome* ]]; then
    echo "Error: GNOME desktop session is required." >&2
    exit 1
fi

# Detect the package manager and define package names.
if command -v dnf >/dev/null 2>&1; then
    PM="dnf"
    NAUTILUS_PKG="nautilus"
    EXTENSIONS_PKG="gnome-extensions-app"
    PYTHON_NAUTILUS_PKG="nautilus-python"

elif command -v apt-get >/dev/null 2>&1; then
    PM="apt"
    NAUTILUS_PKG="nautilus"
    EXTENSIONS_PKG="gnome-extensions-app"
    PYTHON_NAUTILUS_PKG="python3-nautilus"

elif command -v pacman >/dev/null 2>&1; then
    PM="pacman"
    NAUTILUS_PKG="nautilus"
    EXTENSIONS_PKG="gnome-extensions"
    PYTHON_NAUTILUS_PKG="python-nautilus"

else
    echo "Error: Unsupported package manager." >&2
    exit 1
fi

# Run package installation with administrator privileges.
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

# Check whether a package is already installed.
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

# Install a package only when it is missing.
ensure_package() {
    local package="$1"

    if is_installed "$package"; then
        echo "[OK] $package is already installed."
    else
        echo "[INSTALL] Installing $package..."
        install_package "$package"
    fi
}

ensure_package "$NAUTILUS_PKG"
ensure_package "$EXTENSIONS_PKG"
ensure_package "$PYTHON_NAUTILUS_PKG"

echo
echo "Setup completed successfully!"
