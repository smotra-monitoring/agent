# Quick Install Script Implementation

This document describes how to implement the `curl | sh` quick install script for the Smotra agent.

## Overview

The quick install script allows users to install the agent with a single command:

```bash
curl -fsSL https://install.smotra.net/agent.sh | sh
```

Or with options:

```bash
curl -fsSL https://install.smotra.net/agent.sh | sh -s -- --server https://api.smotra.net
```

## Infrastructure Requirements

### 1. File Hosting

You need to host the following files publicly:

```
https://install.smotra.net/
├── agent.sh                           # The install script
└── releases/
    ├── latest/
    │   ├── version.txt                # Contains: v0.1.0
    │   └── manifest.json              # Release metadata
    └── v0.1.0/
        ├── smotra-agent-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
        ├── smotra-agent-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
        ├── smotra-agent-v0.1.0-x86_64-unknown-linux-musl.tar.gz
        ├── smotra-agent-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
        ├── smotra-agent-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
        ├── smotra-agent-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256
        ├── smotra-agent-v0.1.0-x86_64-apple-darwin.tar.gz
        ├── smotra-agent-v0.1.0-x86_64-apple-darwin.tar.gz.sha256
        ├── smotra-agent-v0.1.0-aarch64-apple-darwin.tar.gz
        └── smotra-agent-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
```

### 2. Release Manifest Format

`manifest.json` example:

```json
{
  "version": "0.1.0",
  "released_at": "2026-02-08T12:00:00Z",
  "checksums": {
    "smotra-agent-v0.1.0-x86_64-unknown-linux-gnu.tar.gz": "abc123...",
    "smotra-agent-v0.1.0-aarch64-unknown-linux-gnu.tar.gz": "def456..."
  },
  "targets": [
    {
      "os": "linux",
      "arch": "x86_64",
      "libc": "gnu",
      "file": "smotra-agent-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    },
    {
      "os": "linux",
      "arch": "x86_64",
      "libc": "musl",
      "file": "smotra-agent-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
    },
    {
      "os": "linux",
      "arch": "aarch64",
      "libc": "gnu",
      "file": "smotra-agent-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
    },
    {
      "os": "darwin",
      "arch": "x86_64",
      "file": "smotra-agent-v0.1.0-x86_64-apple-darwin.tar.gz"
    },
    {
      "os": "darwin",
      "arch": "aarch64",
      "file": "smotra-agent-v0.1.0-aarch64-apple-darwin.tar.gz"
    }
  ]
}
```

## Install Script Implementation

Here's the complete `agent.sh` script. Save this as `scripts/install.sh` in your repository:

```bash
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
    FILENAME="smotra-agent-v${VERSION}-${TARGET}.tar.gz"
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

    cat > "$CONFIG_DIR/config.toml" <<EOF
version = 1
agent_id = "00000000-0000-0000-0000-000000000000"
agent_name = "Agent $(hostname)"
tags = ["production"]

[monitoring]
interval_secs = 60
timeout_secs = 5
ping_count = 3
max_concurrent = 10

[server]
url = "${SERVER_URL:-https://api.smotra.net}"
report_interval_secs = 300
heartbeat_interval_secs = 300
verify_tls = true
timeout_secs = 30
retry_attempts = 3

[server.claiming]
poll_interval_secs = 30
max_registration_retries = 5

[storage]
cache_dir = "$CACHE_DIR"
max_cached_results = 10000
max_cache_age_secs = 86400

# Add your endpoints here
[[endpoints]]
address = "8.8.8.8"
tags = ["dns", "google"]
enabled = true
EOF

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
```

## Build Pipeline Requirements

### 1. Release Build Script

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin

    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
        
      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf smotra-agent-${{ github.ref_name }}-${{ matrix.target }}.tar.gz \
            agent agent-cli agent-updater
          sha256sum smotra-agent-${{ github.ref_name }}-${{ matrix.target }}.tar.gz \
            > smotra-agent-${{ github.ref_name }}-${{ matrix.target }}.tar.gz.sha256
            
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: release-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/smotra-agent-*.tar.gz
            target/${{ matrix.target }}/release/smotra-agent-*.tar.gz.sha256

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v3
        with:
          path: artifacts
        
      - name: Organize release files
        run: |
          VERSION="${{ github.ref_name }}"
          mkdir -p "releases/${VERSION}"
          
          # Move all artifacts to release directory
          find artifacts -name "*.tar.gz*" -exec mv {} "releases/${VERSION}/" \;
          
          # Create manifest.json
          cat > "releases/${VERSION}/manifest.json" <<EOF
          {
            "version": "${VERSION#v}",
            "released_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
            "targets": []
          }
          EOF
          
          # Create latest directory
          mkdir -p "releases/latest"
          echo "${VERSION}" > "releases/latest/version.txt"
          cp "releases/${VERSION}/manifest.json" "releases/latest/manifest.json"
          
          # Copy install script to root for hosting
          cp scripts/install.sh agent.sh
        
      - name: Create GitHub release
        uses: softprops/action-gh-release@v1
        with:
          files: releases/${{ github.ref_name }}/*
```

**Note**: This workflow creates a folder structure that matches the hosting structure shown at the beginning of this document:
- `releases/v0.1.0/*.tar.gz` - Binary archives
- `releases/v0.1.0/*.tar.gz.sha256` - Checksums
- `releases/latest/version.txt` - Latest version pointer
- `agent.sh` - Install script (copied from `scripts/install.sh`)

### 2. Upload to CDN/Hosting

After the GitHub Actions workflow completes, it creates a `releases/` directory structure.
Upload these files to your hosting:

```bash
#!/bin/bash
# upload-release.sh
# Run this after downloading the release artifacts from GitHub Actions

VERSION="$1"
AWS_BUCKET="s3://install.smotra.net"

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.1.0"
    exit 1
fi

# Upload release files (assumes releases/ directory exists from GitHub Actions)
aws s3 cp "releases/${VERSION}/" "${AWS_BUCKET}/releases/${VERSION}/" --recursive

# Update latest version pointer
aws s3 cp "releases/latest/version.txt" "${AWS_BUCKET}/releases/latest/version.txt"
aws s3 cp "releases/latest/manifest.json" "${AWS_BUCKET}/releases/latest/manifest.json"

# Upload install script
aws s3 cp "agent.sh" "${AWS_BUCKET}/agent.sh" --content-type "text/x-shellscript"

# Invalidate CloudFront cache if using CDN
if [ -n "$CLOUDFRONT_DISTRIBUTION_ID" ]; then
    aws cloudfront create-invalidation \
        --distribution-id "$CLOUDFRONT_DISTRIBUTION_ID" \
        --paths "/releases/latest/*" "/agent.sh"
fi

echo "✓ Release ${VERSION} uploaded successfully"
```

## Testing the Install Script

### Local Testing with GitHub Pages

After deploying to GitHub Pages, test your install script:

```bash
# Test the install script from GitHub Pages
curl -fsSL https://your-username.github.io/smotra-agent/agent.sh | sh -s -- --help

# Or with custom domain
curl -fsSL https://install.smotra.net/agent.sh | sh -s -- --help
```

### Local Testing with Local Server

Before deploying, test with a local HTTP server:

```bash
# Test with local files
BASE_URL="http://localhost:8000" sh agent.sh --help

# Test full installation
sudo BASE_URL="http://localhost:8000" sh agent.sh --server https://api.smotra.net
```

### Shellcheck

Validate the script:

```bash
shellcheck agent.sh
```

## Security Considerations

1. **HTTPS Only**: Always use HTTPS for downloads
2. **Checksum Verification**: Always verify sha256 checksums
3. **Code Review**: The piping to sh pattern is convenient but risky - users should review the script first
4. **Provide Alternative**: Offer manual download option:

```bash
# Safer alternative
curl -fsSL -o agent.sh https://install.smotra.net/agent.sh
# Review the script
less agent.sh
# Run it
sh agent.sh
```

## Documentation for Users

Add to your README:

```markdown
### Quick Install (Linux/macOS)

```bash
curl -fsSL https://install.smotra.net/agent.sh | sh
```

With custom server:

```bash
curl -fsSL https://install.smotra.net/agent.sh | sh -s -- --server https://api.smotra.net
```

For a safer installation, download and review the script first:

```bash
curl -fsSL -o agent.sh https://install.smotra.net/agent.sh
less agent.sh  # Review the script
sh agent.sh
```

## Hosting Options

### Option 1: GitHub Pages (Recommended for Getting Started)

GitHub Pages is perfect for hosting install scripts and release files - it's **free, reliable, and easy to set up**.

#### Setup Steps

1. **Enable GitHub Pages** in your repository settings:
   - Go to Settings → Pages
   - Source: Deploy from a branch
   - Branch: `gh-pages` (or create a separate branch)
   - Folder: `/ (root)`

2. **Your files will be accessible at**:
   ```
   https://your-username.github.io/smotra-agent/agent.sh
   https://your-username.github.io/smotra-agent/releases/v0.1.0/...
   ```

3. **Optional: Use a custom domain** (e.g., `install.smotra.net`):
   - Add CNAME record: `install.smotra.net` → `your-username.github.io`
   - Add `CNAME` file to gh-pages branch with content: `install.smotra.net`

#### Updated GitHub Actions Workflow

Modify the release workflow to deploy to GitHub Pages:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'
  release:
    types: ['published']

jobs:
  release-pre-checks:
    name:  Test, lint and attempt to build
    runs-on: ubuntu-latest

    steps:
    - name: checkout code
      uses: actions/checkout@latest

    # - name: Run clippy
    #   run: cargo clippy --verbose -- -D warnings

    - name: Run tests
      run: cargo test --verbose

    - name: Run fmt
      run: cargo fmt -- --check

    - name: Install dependencies
      run: cargo build

  create-release:
    name: create-release
    needs: ['release-pre-checks']
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@latest
      
      - name: Get the release version from the tag
        if: env.VERSION == ''
        run: echo "VERSION=${{ github.ref_name }}" >> $GITHUB_ENV
        
      - name: Show the version
        run: |
          echo "version is: $VERSION"
          
      - name: Create GitHub release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: gh release create $VERSION --draft --verify-tag --title $VERSION

    outputs:
      version: ${{ env.VERSION }}


  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: windows-latest
            target: x86_64-pc-windows-gnu

    steps:
      - uses: actions/checkout@latest
      
      - name: Install Rust
        uses: actions-rs/toolchain@latest
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          
      - name: Update version in Cargo.toml to match release tag
        run: |
          TAG_VERSION="${{ github.ref_name#v }}"
          sed -i.bak "s/^version = \".*\"/version = \"${TAG_VERSION}\"/" Cargo.toml

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
        
      - name: Package
        if: matrix.os != 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf smotra-agent-${{ github.ref_name }}-${{ matrix.target }}.tar.gz \
            agent agent-cli agent-updater
          sha256sum smotra-agent-${{ github.ref_name }}-${{ matrix.target }}.tar.gz \
            > smotra-agent-${{ github.ref_name }}-${{ matrix.target }}.tar.gz.sha256
            
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: release-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/smotra-agent-*.tar.gz
            target/${{ matrix.target }}/release/smotra-agent-*.tar.gz.sha256

  deploy-release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v3
        with:
          name: release

  deploy-gh-pages:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write  # Required for GitHub Pages deployment
    steps:
      - name: Checkout gh-pages branch
        uses: actions/checkout@v3
        with:
          ref: gh-pages
          path: gh-pages
        # If gh-pages doesn't exist, create it manually first
        
      - name: Download artifacts
        uses: actions/download-artifact@v3
        with:
          path: artifacts
        
      - name: Checkout main branch for install script
        uses: actions/checkout@v3
        with:
          path: main
        
      - name: Organize release files
        run: |
          VERSION="${{ github.ref_name }}"
          cd gh-pages
          
          # Create release directory
          mkdir -p "releases/${VERSION}"
          
          # Move all artifacts to release directory
          find ../artifacts -name "*.tar.gz*" -exec mv {} "releases/${VERSION}/" \;
          
          # Create manifest.json
          cat > "releases/${VERSION}/manifest.json" <<EOF
          {
            "version": "${VERSION#v}",
            "released_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
            "checksums": {},
            "targets": []
          }
          EOF
          
          # Update latest directory
          mkdir -p "releases/latest"
          echo "${VERSION}" > "releases/latest/version.txt"
          cp "releases/${VERSION}/manifest.json" "releases/latest/manifest.json"
          
          # Copy install script from main branch
          cp ../main/scripts/install.sh agent.sh
          chmod +x agent.sh
          
          # Create index.html for documentation
          cat > index.html <<EOF
          <!DOCTYPE html>
          <html>
          <head>
              <title>Smotra Agent Install</title>
              <meta charset="utf-8">
              <style>
                  body { font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
                  pre { background: #f4f4f4; padding: 15px; border-radius: 5px; overflow-x: auto; }
                  code { background: #f4f4f4; padding: 2px 5px; border-radius: 3px; }
              </style>
          </head>
          <body>
              <h1>Smotra Agent Installation</h1>
              <p>Latest Version: <strong>${VERSION}</strong></p>
              
              <h2>Quick Install</h2>
              <pre><code>curl -fsSL https://\$(YOUR_DOMAIN)/agent.sh | sh</code></pre>
              
              <h2>Available Releases</h2>
              <ul>
                  <li><a href="releases/${VERSION}/">Version ${VERSION}</a></li>
                  <li><a href="releases/latest/">Latest</a></li>
              </ul>
              
              <h2>Documentation</h2>
              <p>Visit <a href="https://github.com/your-org/smotra-agent">GitHub Repository</a> for full documentation.</p>
          </body>
          </html>
          EOF
        
      - name: Commit and push to gh-pages
        run: |
          cd gh-pages
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add .
          git commit -m "Release ${{ github.ref_name }}" || echo "No changes to commit"
          git push origin gh-pages
        
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: gh-pages/releases/${{ github.ref_name }}/*
```

#### Initial Setup of gh-pages Branch

Before the first release, create the `gh-pages` branch:

```bash
# Create an orphan branch (no history)
git checkout --orphan gh-pages

# Remove all files from staging
git rm -rf .

# Create a basic structure
mkdir -p releases/latest
echo "# Smotra Agent Installation Files" > README.md

# Commit and push
git add .
git commit -m "Initial gh-pages setup"
git push origin gh-pages
```

#### Using with Custom Domain

If you want to use `install.smotra.net` instead of `username.github.io`:

1. **Add CNAME file** to gh-pages branch:
   ```bash
   echo "install.smotra.net" > CNAME
   git add CNAME
   git commit -m "Add custom domain"
   git push
   ```

2. **Configure DNS** at your domain registrar:
   ```
   Type: CNAME
   Name: install
   Value: your-username.github.io
   ```

3. **Update GitHub Pages settings** to use custom domain

Then your install command becomes:
```bash
curl -fsSL https://install.smotra.net/agent.sh | sh
```

#### Advantages of GitHub Pages
- ✅ Free hosting with generous bandwidth
- ✅ HTTPS enabled automatically
- ✅ Custom domain support
- ✅ Global CDN (via GitHub)
- ✅ Version control for releases
- ✅ No AWS credentials needed
- ✅ Simple deployment via GitHub Actions

#### Limitations
- ⚠️ 1GB repository size limit (should be fine for binaries)
- ⚠️ 100GB monthly bandwidth soft limit (usually sufficient)
- ⚠️ 10 builds per hour limit (rarely an issue)

### Option 2: AWS S3 + CloudFront

For higher traffic or more control:

```bash
#!/bin/bash
# upload-release.sh

VERSION="$1"
AWS_BUCKET="s3://install.smotra.net"

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.1.0"
    exit 1
fi

# Download release from GitHub
gh release download "$VERSION" --dir "releases/${VERSION}"

# Upload to S3
aws s3 cp "releases/${VERSION}/" "${AWS_BUCKET}/releases/${VERSION}/" --recursive

# Update latest version pointer
mkdir -p releases/latest
echo "$VERSION" > releases/latest/version.txt
aws s3 cp releases/latest/version.txt "${AWS_BUCKET}/releases/latest/version.txt"

# Upload install script
aws s3 cp scripts/install.sh "${AWS_BUCKET}/agent.sh" --content-type "text/x-shellscript"

# Invalidate CloudFront cache
aws cloudfront create-invalidation \
    --distribution-id "$CLOUDFRONT_DISTRIBUTION_ID" \
    --paths "/releases/latest/*" "/agent.sh"
```

**Advantages:**
- ✅ No size limits
- ✅ Better for very high traffic
- ✅ More control over caching

**Disadvantages:**
- ❌ Costs money (though minimal for this use case)
- ❌ Requires AWS account and configuration
- ❌ More complex setup

### Option 3: Self-Hosted
- Host on your own infrastructure
- Full control
- Includes update server

## Next Steps

### Recommended Quick Start (GitHub Pages)

1. **Create the install script**: Save `scripts/install.sh` with the script content from above
2. **Create gh-pages branch**:
   ```bash
   git checkout --orphan gh-pages
   git rm -rf .
   echo "# Smotra Agent Releases" > README.md
   git add README.md
   git commit -m "Initial gh-pages"
   git push origin gh-pages
   git checkout main
   ```
3. **Enable GitHub Pages** in repository Settings → Pages → Source: gh-pages branch
4. **Create the release workflow**: Copy the GitHub Pages workflow to `.github/workflows/release.yml`
5. **Create first release**:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
6. **Access your install script** at `https://your-username.github.io/your-repo/agent.sh`

### For Production with Custom Domain

1. Add `CNAME` file to gh-pages branch: `echo "install.smotra.net" > CNAME`
2. Configure DNS CNAME record at your domain registrar
3. Update GitHub Pages settings with custom domain
4. Your install command: `curl -fsSL https://install.smotra.net/agent.sh | sh`

### Alternative Approaches

- **High traffic sites**: Use AWS S3 + CloudFront (Option 2)
- **Self-hosted infrastructure**: Deploy to your own web server (Option 3)
- **Simple releases without install script**: Use GitHub Releases directly and document manual installation

---

**Summary**: GitHub Pages provides a free, reliable, and simple solution for hosting install scripts and release binaries. It's perfect for open-source projects and handles the vast majority of use cases without any costs.

### Repository Structure

Your repository should include:

**Main branch:**
```
smotra-agent/ (main branch)
├── .github/
│   └── workflows/
│       └── release.yml          # Build and release workflow
├── scripts/
│   ├── install.sh               # The install script shown above
│   ├── uninstall.sh             # Uninstall script (to be created)
│   └── upload-release.sh        # Optional: upload helper for S3
├── src/
│   └── ...                      # Your Rust source code
└── Cargo.toml
```

**gh-pages branch** (automatically managed by GitHub Actions):
```
smotra-agent/ (gh-pages branch)
├── agent.sh                     # Install script (copied from main)
├── index.html                   # Landing page
├── CNAME                        # Optional: for custom domain
└── releases/
    ├── latest/
    │   ├── version.txt
    │   └── manifest.json
    └── v0.1.0/
        ├── smotra-agent-v0.1.0-*.tar.gz
        └── smotra-agent-v0.1.0-*.tar.gz.sha256
```
