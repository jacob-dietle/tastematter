#!/bin/bash
# tastematter CLI installer
# Usage: curl -fsSL https://install.tastematter.dev/install.sh | bash
set -euo pipefail

BASE_URL="https://install.tastematter.dev"
BINARY_NAME="tastematter"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${VERSION:-latest}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}[tastematter]${NC} Installing..."

# Detect platform
detect_platform() {
    local os arch
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        darwin) os="darwin" ;;
        linux) os="linux" ;;
        *)
            echo -e "${RED}[tastematter] Error: Unsupported OS: $os${NC}"
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)
            echo -e "${RED}[tastematter] Error: Unsupported architecture: $arch${NC}"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

PLATFORM=$(detect_platform)

# Get version
if [ "$VERSION" = "latest" ]; then
    VERSION=$(curl -fsSL "$BASE_URL/latest.txt" 2>/dev/null || echo "")
    if [ -z "$VERSION" ]; then
        echo -e "${RED}[tastematter] Error: Could not fetch latest version${NC}"
        echo -e "${YELLOW}  Check your internet connection or set VERSION env var${NC}"
        exit 1
    fi
fi
echo -e "${CYAN}[tastematter]${NC} Version: $VERSION, Platform: $PLATFORM"

# Download URL
DOWNLOAD_URL="$BASE_URL/releases/$VERSION/$BINARY_NAME-$PLATFORM"

# Create install directory
mkdir -p "$INSTALL_DIR"

# Stop any running tastematter processes (ensures clean update)
if pgrep -x "$BINARY_NAME" > /dev/null 2>&1; then
    echo -e "${YELLOW}[tastematter] Stopping running processes for update...${NC}"
    pkill -x "$BINARY_NAME" 2>/dev/null || true
    sleep 1  # Brief pause to ensure file handles are released
    echo -e "${CYAN}[tastematter]${NC} Stopped existing processes"
fi

# Download binary
echo -e "${CYAN}[tastematter]${NC} Downloading from $DOWNLOAD_URL"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/$BINARY_NAME"; then
    echo -e "${RED}[tastematter] Error: Download failed${NC}"
    echo -e "${YELLOW}  URL: $DOWNLOAD_URL${NC}"
    exit 1
fi

# Make executable
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Verify binary exists and is not truncated (should be at least 10 MB)
FILE_SIZE=$(stat -f%z "$INSTALL_DIR/$BINARY_NAME" 2>/dev/null || stat -c%s "$INSTALL_DIR/$BINARY_NAME" 2>/dev/null || echo "0")
FILE_SIZE_MB=$((FILE_SIZE / 1048576))
MIN_SIZE=$((10 * 1048576))

if [ "$FILE_SIZE" -lt "$MIN_SIZE" ]; then
    echo -e "${RED}[tastematter] Error: Download appears truncated (${FILE_SIZE_MB} MB < 10 MB minimum)${NC}"
    echo -e "${YELLOW}  This usually means the download was interrupted.${NC}"
    echo -e "${YELLOW}  Please try running the install script again.${NC}"
    rm -f "$INSTALL_DIR/$BINARY_NAME"
    exit 1
fi

echo -e "${CYAN}[tastematter]${NC} Downloaded ${FILE_SIZE_MB} MB"

# Verify executable
if [ ! -x "$INSTALL_DIR/$BINARY_NAME" ]; then
    echo -e "${RED}[tastematter] Error: Binary not executable${NC}"
    exit 1
fi

# Check PATH
PATH_UPDATED=false
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}[tastematter] Add to PATH:${NC}"
    echo ""
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    echo ""
    echo -e "${YELLOW}  Add this to ~/.bashrc or ~/.zshrc for persistence${NC}"
    PATH_UPDATED=true
    # Temporarily add to PATH for daemon install
    export PATH="$PATH:$INSTALL_DIR"
fi

# Register daemon to run on login (best-effort, warn on failure)
echo -e "${CYAN}[tastematter]${NC} Setting up background sync..."
if "$INSTALL_DIR/$BINARY_NAME" daemon install --interval 30 2>/dev/null; then
    echo -e "${GREEN}[tastematter] Background sync registered (runs on login)${NC}"
else
    echo -e "${YELLOW}[tastematter] Warning: Could not register background sync${NC}"
    echo -e "${YELLOW}  Run 'tastematter daemon install' manually to enable${NC}"
fi

echo ""
echo -e "${GREEN}[tastematter] Installation complete!${NC}"
echo "  Run '$BINARY_NAME --help' to get started"
echo "  Background sync will start on next login"
echo "  Check status with: tastematter daemon status"
if [ "$PATH_UPDATED" = true ]; then
    echo "  (Add $INSTALL_DIR to PATH first)"
fi
