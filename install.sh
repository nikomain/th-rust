#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# GitHub repository
REPO="nikomain/th-rust"
GITHUB_URL="https://github.com/${REPO}"

# Installation directory
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="th"

echo -e "${BLUE}🚀 Installing Teleport Helper (th)...${NC}"
echo

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture names
case "$ARCH" in
    x86_64)
        ARCH="x86_64"
        ;;
    arm64|aarch64)
        ARCH="aarch64"
        ;;
    *)
        echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

# Map OS names and create binary name
case "$OS" in
    darwin)
        BINARY_FILE="th-${ARCH}-apple-darwin"
        ;;
    linux)
        BINARY_FILE="th-${ARCH}-unknown-linux-gnu"
        ;;
    mingw*|msys*|cygwin*)
        BINARY_FILE="th-${ARCH}-pc-windows-msvc.exe"
        BINARY_NAME="th.exe"
        ;;
    *)
        echo -e "${RED}❌ Unsupported OS: $OS${NC}"
        echo -e "${RED}   Detected: $OS${NC}"
        echo -e "${RED}   Supported: darwin (macOS), linux, windows${NC}"
        exit 1
        ;;
esac

echo -e "${BLUE}📋 Detected platform: ${OS}-${ARCH}${NC}"
echo -e "${BLUE}📦 Binary: ${BINARY_FILE}${NC}"
echo

# Check if we can write to install directory
if [[ ! -w "$INSTALL_DIR" ]]; then
    echo -e "${YELLOW}⚠️  Need sudo access to install to $INSTALL_DIR${NC}"
    SUDO="sudo"
else
    SUDO=""
fi

# Get latest release info
echo -e "${BLUE}🔍 Fetching latest release...${NC}"
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest")
TAG_NAME=$(echo "$LATEST_RELEASE" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [[ -z "$TAG_NAME" ]]; then
    echo -e "${RED}❌ Failed to get latest release information${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Latest version: ${TAG_NAME}${NC}"

# Download URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG_NAME}/${BINARY_FILE}"

# Create temporary directory
TEMP_DIR=$(mktemp -d)
TEMP_FILE="${TEMP_DIR}/${BINARY_NAME}"

echo -e "${BLUE}⬇️  Downloading ${BINARY_FILE}...${NC}"
if ! curl -L -o "$TEMP_FILE" "$DOWNLOAD_URL"; then
    echo -e "${RED}❌ Failed to download binary${NC}"
    echo -e "${RED}   URL: ${DOWNLOAD_URL}${NC}"
    exit 1
fi

# Make executable
chmod +x "$TEMP_FILE"

# Install binary
echo -e "${BLUE}📦 Installing to ${INSTALL_DIR}/${BINARY_NAME}...${NC}"
if ! $SUDO mv "$TEMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"; then
    echo -e "${RED}❌ Failed to install binary${NC}"
    exit 1
fi

# Cleanup
rm -rf "$TEMP_DIR"

# Create wrapper script
WRAPPER_SCRIPT="${INSTALL_DIR}/th.sh"
echo -e "${BLUE}📝 Creating wrapper script at ${WRAPPER_SCRIPT}...${NC}"

$SUDO tee "$WRAPPER_SCRIPT" > /dev/null << 'EOF'
#!/bin/bash

# Wrapper script for th - sources credentials after execution
function th() {
    # Run the actual th binary
    command th "$@"
    local exit_code=$?
    
    # Source any credential files that were created
    for cred_file in /tmp/yl_* /tmp/admin_*; do
        if [[ -f "$cred_file" ]]; then
            source "$cred_file"
            break
        fi
    done
    
    return $exit_code
}

# If script is being sourced, just define the function
# If script is being executed directly, run th with passed arguments  
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    th "$@"
fi
EOF

$SUDO chmod +x "$WRAPPER_SCRIPT"

echo
echo -e "${GREEN}✅ Installation completed successfully!${NC}"
echo
echo -e "${BLUE}📋 Usage:${NC}"
echo -e "  ${YELLOW}th${NC}                 - Show help"
echo -e "  ${YELLOW}th a${NC}               - AWS login"
echo -e "  ${YELLOW}th k${NC}               - Kubernetes login"  
echo -e "  ${YELLOW}th d${NC}               - Database login"
echo -e "  ${YELLOW}th update${NC}          - Update to latest version"
echo
echo -e "${BLUE}🔧 Setup:${NC}"
echo -e "Add to your shell profile (~/.zshrc or ~/.bash_profile):"
echo -e "${YELLOW}source /usr/local/bin/th.sh${NC}"
echo
echo -e "${BLUE}🚀 Quick start:${NC}"
echo -e "${YELLOW}source /usr/local/bin/th.sh && th${NC}"
echo

# Check if binary works
if command -v th >/dev/null 2>&1; then
    echo -e "${GREEN}🎉 th is now available in your PATH!${NC}"
else
    echo -e "${YELLOW}⚠️  You may need to restart your terminal or run:${NC}"
    echo -e "${YELLOW}   export PATH=\"${INSTALL_DIR}:\$PATH\"${NC}"
fi