#!/bin/bash
# =============================================================================
# RustBridge Uninstallation Script
# =============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${RED}"
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                 RustBridge Uninstallation                     ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./uninstall.sh)${NC}"
    exit 1
fi

# Configuration
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/rustbridge"
SERVICE_FILE="/etc/systemd/system/rustbridge.service"
BINARY_NAME="rustbridge"
USER_NAME="rustbridge"

# Stop and disable service
if systemctl is-active --quiet rustbridge; then
    echo -e "${YELLOW}Stopping service...${NC}"
    systemctl stop rustbridge
fi

if systemctl is-enabled --quiet rustbridge 2>/dev/null; then
    echo -e "${YELLOW}Disabling service...${NC}"
    systemctl disable rustbridge
fi

# Remove service file
if [ -f "$SERVICE_FILE" ]; then
    echo -e "${YELLOW}Removing service file...${NC}"
    rm -f $SERVICE_FILE
    systemctl daemon-reload
fi

# Remove binary
if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
    echo -e "${YELLOW}Removing binary...${NC}"
    rm -f $INSTALL_DIR/$BINARY_NAME
fi

# Ask about config removal
read -p "Remove configuration directory $CONFIG_DIR? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf $CONFIG_DIR
    echo -e "${YELLOW}Configuration removed.${NC}"
fi

# Ask about user removal
read -p "Remove user $USER_NAME? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    userdel $USER_NAME 2>/dev/null || true
    echo -e "${YELLOW}User removed.${NC}"
fi

echo ""
echo -e "${GREEN}Uninstallation complete!${NC}"
echo ""
