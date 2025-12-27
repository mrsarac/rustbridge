#!/bin/bash
# =============================================================================
# RustBridge Installation Script
# Installs RustBridge as a systemd service
# =============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}"
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                  RustBridge Installation                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./install.sh)${NC}"
    exit 1
fi

# Configuration
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/rustbridge"
SERVICE_FILE="/etc/systemd/system/rustbridge.service"
BINARY_NAME="rustbridge"
USER_NAME="rustbridge"

# Check for binary
if [ ! -f "../target/release/$BINARY_NAME" ]; then
    echo -e "${YELLOW}Building release binary...${NC}"
    cd ..
    cargo build --release
    cd deploy
fi

# Create user if not exists
if ! id "$USER_NAME" &>/dev/null; then
    echo -e "${GREEN}Creating user: $USER_NAME${NC}"
    useradd -r -s /bin/false $USER_NAME
    usermod -aG dialout $USER_NAME  # For serial port access
fi

# Create directories
echo -e "${GREEN}Creating directories...${NC}"
mkdir -p $CONFIG_DIR

# Copy binary
echo -e "${GREEN}Installing binary to $INSTALL_DIR...${NC}"
cp ../target/release/$BINARY_NAME $INSTALL_DIR/
chmod +x $INSTALL_DIR/$BINARY_NAME

# Copy configuration
echo -e "${GREEN}Installing configuration to $CONFIG_DIR...${NC}"
if [ ! -f "$CONFIG_DIR/config.yaml" ]; then
    cp ../config.yaml $CONFIG_DIR/
else
    echo -e "${YELLOW}Config file exists, skipping...${NC}"
fi

# Set permissions
chown -R $USER_NAME:$USER_NAME $CONFIG_DIR

# Install systemd service
echo -e "${GREEN}Installing systemd service...${NC}"
cp systemd/rustbridge.service $SERVICE_FILE
systemctl daemon-reload

# Enable and start service
echo -e "${GREEN}Enabling service...${NC}"
systemctl enable rustbridge

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Commands:"
echo "  Start:   sudo systemctl start rustbridge"
echo "  Stop:    sudo systemctl stop rustbridge"
echo "  Status:  sudo systemctl status rustbridge"
echo "  Logs:    sudo journalctl -u rustbridge -f"
echo ""
echo "Configuration: $CONFIG_DIR/config.yaml"
echo ""
