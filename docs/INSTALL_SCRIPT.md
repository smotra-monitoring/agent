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

Here's the complete [agent.sh](../deployments/install/agent.sh) 


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

Modify the release workflow to deploy to GitHub Pages. Check example in the [release-deployment.yml](../.github/workflows/release-deployment.yml).

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
