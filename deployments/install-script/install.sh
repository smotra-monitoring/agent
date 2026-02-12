#!/bin/sh
# Smotra Agent Quick Install Script
# Usage: curl -fsSL https://install.smotra.net/agent.sh | sh
# Usage with options: curl -fsSL https://install.smotra.net/agent.sh | sh -s -- --server https://api.smotra.net

set -e

# Configuration
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
CONFIG_DIR="${CONFIG_DIR:-/etc/smotra}"
CACHE_DIR="${CACHE_DIR:-/var/cache/smotra}"
BASE_URL="${SMOTRA_INSTALL_URL:-https://install.smotra.net}"
VERSION="${SMOTRA_VERSION:-latest}"
SERVER_URL=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
info() {
    printf "${GREEN}==>${NC} %s\n" "$*"
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$*"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$*" >&2
    exit 1
}

# Parse command line arguments
parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --server)
                SERVER_URL="$2"
                shift 2
                ;;
            --version)
                VERSION="$2"
                shift 2
                ;;
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --help)
                cat <<EOF
Smotra Agent Install Script

Usage: 
  curl -fsSL https://install.smotra.net/agent.sh | sh
  curl -fsSL https://install.smotra.net/agent.sh | sh -s -- [options]

Options:
  --server URL       Server URL (e.g., https://api.smotra.net)
  --version VERSION  Install specific version (default: latest)
  --install-dir DIR  Installation directory (default: /usr/local/bin)
  --help            Show this help message

Environment Variables:
  INSTALL_DIR            Installation directory
  CONFIG_DIR             Configuration directory
  SMOTRA_VERSION         Version to install
  SMOTRA_INSTALL_URL     Base URL for downloads

Examples:
  # Install latest version
  curl -fsSL https://install.smotra.net/agent.sh | sh

  # Install with server URL
  curl -fsSL https://install.smotra.net/agent.sh | sh -s -- --server https://api.smotra.net

  # Install specific version
  curl -fsSL https://install.smotra.net/agent.sh | sh -s -- --version v0.1.0
EOF
                exit 0
                ;;
            *)
                error "Unknown option: $1. Use --help for usage information."
                ;;
        esac
    done
}

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m)"

    case "$OS" in
        linux)
            OS="linux"
            ;;
        darwin)
            OS="darwin"
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            ;;
    esac

    # Detect libc for Linux
    if [ "$OS" = "linux" ]; then
        if ldd --version 2>&1 | grep -q musl; then
            LIBC="musl"
        else
            LIBC="gnu"
        fi
        TARGET="${ARCH}-unknown-${OS}-${LIBC}"
    else
        TARGET="${ARCH}-apple-${OS}"
    fi

    info "Detected platform: $OS/$ARCH${LIBC:+ ($LIBC)}"
}

# Check if running as root (needed for ICMP)
check_privileges() {
    if [ "$(id -u)" -ne 0 ]; then
        warn "Not running as root. ICMP ping monitoring requires elevated privileges."
        warn "You may need to run 'sudo setcap cap_net_raw+ep $INSTALL_DIR/agent' after installation."
    fi
}

# Check dependencies
check_dependencies() {
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        error "Neither curl nor wget found. Please install one of them first."
    fi

    if ! command -v tar >/dev/null 2>&1; then
        error "tar not found. Please install tar first."
    fi

    if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
        warn "sha256sum/shasum not found. Checksum verification will be skipped."
        SKIP_CHECKSUM=1
    fi
}

# Download file using curl or wget
download() {
    url="$1"
    output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$output" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$output" "$url"
    else
        error "Neither curl nor wget available"
    fi
}

# Get latest version
get_version() {
    if [ "$VERSION" = "latest" ]; then
        info "Fetching latest version..."
        VERSION=$(download "${BASE_URL}/releases/latest/version.txt" - | tr -d '\n\r' | sed 's/^v//')
        info "Latest version: v${VERSION}"
    else
        VERSION=$(echo "$VERSION" | sed 's/^v//')
    fi
}

# Download and verify agent
download_agent() {
    FILENAME="agent-v${VERSION}-${TARGET}.tar.gz"
    DOWNLOAD_URL="${BASE_URL}/releases/v${VERSION}/${FILENAME}"
    CHECKSUM_URL="${DOWNLOAD_URL}.sha256"
    
    TMPDIR=$(mktemp -d)
    trap "rm -rf $TMPDIR" EXIT

    info "Downloading agent from $DOWNLOAD_URL..."
    download "$DOWNLOAD_URL" "$TMPDIR/$FILENAME"

    # Verify checksum if available
    if [ -z "$SKIP_CHECKSUM" ]; then
        info "Verifying checksum..."
        download "$CHECKSUM_URL" "$TMPDIR/${FILENAME}.sha256"
        
        cd "$TMPDIR"
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum -c "${FILENAME}.sha256" || error "Checksum verification failed"
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 -c "${FILENAME}.sha256" || error "Checksum verification failed"
        fi
        cd - >/dev/null
        info "Checksum verified successfully"
    fi

    # Extract archive
    info "Extracting archive..."
    tar -xzf "$TMPDIR/$FILENAME" -C "$TMPDIR"
}

# Install agent binaries
install_binaries() {
    info "Installing binaries to $INSTALL_DIR..."
    
    mkdir -p "$INSTALL_DIR"
    
    # Install binaries
    install -m 755 "$TMPDIR/agent" "$INSTALL_DIR/smotra-agent"
    install -m 755 "$TMPDIR/agent-cli" "$INSTALL_DIR/smotra-agent-cli"
    install -m 755 "$TMPDIR/agent-updater" "$INSTALL_DIR/smotra-agent-updater"

    # Set capabilities for ICMP (Linux only)
    if [ "$OS" = "linux" ] && command -v setcap >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
        info "Setting ICMP capabilities..."
        setcap cap_net_raw+ep "$INSTALL_DIR/smotra-agent" || warn "Failed to set capabilities. You may need to run as root."
    fi

    info "Binaries installed successfully"
}

# Generate configuration
generate_config() {
    info "Generating configuration..."
    
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$CACHE_DIR"

    if [ -f "$CONFIG_DIR/config.toml" ]; then
        warn "Configuration file already exists at $CONFIG_DIR/config.toml"
        warn "Skipping configuration generation"
        return
    fi

    smotra-agent --gen-config
    mv config.toml "$CONFIG_DIR/config.toml"
    chmod 640 "$CONFIG_DIR/config.toml"
    info "Configuration file created at $CONFIG_DIR/config.toml"
}

# Install systemd service (Linux)
install_systemd_service() {
    if [ "$OS" != "linux" ] || ! command -v systemctl >/dev/null 2>&1; then
        return
    fi

    if [ "$(id -u)" -ne 0 ]; then
        warn "Not running as root, skipping systemd service installation"
        return
    fi

    info "Installing systemd service..."
    
    cat > /etc/systemd/system/smotra-agent.service <<EOF
[Unit]
Description=Smotra Monitoring Agent
After=network.target
Documentation=https://github.com/smotra/agent

[Service]
Type=simple
User=root
ExecStart=$INSTALL_DIR/smotra-agent -c $CONFIG_DIR/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    info "Systemd service installed"
    info "Enable and start with: systemctl enable --now smotra-agent"
}

# Install launchd service (macOS)
install_launchd_service() {
    if [ "$OS" != "darwin" ]; then
        return
    fi

    info "Installing launchd service..."
    
    mkdir -p ~/Library/LaunchAgents
    
    cat > ~/Library/LaunchAgents/net.smotra.agent.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>net.smotra.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/smotra-agent</string>
        <string>-c</string>
        <string>$CONFIG_DIR/config.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/smotra-agent.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/smotra-agent.error.log</string>
</dict>
</plist>
EOF

    info "Launchd service installed"
    info "Load with: launchctl load ~/Library/LaunchAgents/net.smotra.agent.plist"
}

# Print post-install instructions
print_instructions() {
    cat <<EOF

${GREEN}✓ Smotra Agent installed successfully!${NC}

Binaries installed:
  - $INSTALL_DIR/smotra-agent
  - $INSTALL_DIR/smotra-agent-cli
  - $INSTALL_DIR/smotra-agent-updater

Configuration:
  - Config file: $CONFIG_DIR/config.toml
  - Cache directory: $CACHE_DIR

Next steps:
  1. Edit the configuration file: $CONFIG_DIR/config.toml
  2. Add endpoints to monitor
  3. Start the agent:
EOF

    if [ "$OS" = "linux" ] && command -v systemctl >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
        echo "     sudo systemctl enable --now smotra-agent"
    else
        echo "     sudo $INSTALL_DIR/smotra-agent -c $CONFIG_DIR/config.toml"
    fi

    cat <<EOF

  4. The agent will display a claim token - use it to register at:
     ${SERVER_URL:-https://api.smotra.net}/claim

For more information:
  - Documentation: https://github.com/smotra/agent/docs
  - Run interactive TUI: $INSTALL_DIR/smotra-agent-cli -c $CONFIG_DIR/config.toml tui
  - Get help: $INSTALL_DIR/smotra-agent --help

EOF
}

# Main installation flow
main() {
    info "Smotra Agent Installer"
    info ""

    parse_args "$@"
    detect_platform
    check_privileges
    check_dependencies
    get_version
    download_agent
    install_binaries
    generate_config
    install_systemd_service
    install_launchd_service
    print_instructions
}

main "$@"
