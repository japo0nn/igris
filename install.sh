#!/usr/bin/env bash
set -euo pipefail

REPO="japo0nn/igris"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/igris"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
    Linux-x86_64)      TARGET="x86_64-unknown-linux-gnu" ;;
    Linux-aarch64)     TARGET="aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64)     TARGET="x86_64-apple-darwin" ;;
    Darwin-arm64)      TARGET="aarch64-apple-darwin" ;;
    *) error "Unsupported: $OS $ARCH" ;;
esac

info "Detected: $OS $ARCH -> $TARGET"

# Rust
if ! command -v rustc &>/dev/null; then
    info "Installing Rust via rustup..."
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    . "$HOME/.cargo/env"
fi

# Node.js 22 via fnm
if ! command -v node &>/dev/null || [[ "$(node --version)" != v22* ]]; then
    info "Installing Node.js 22 via fnm..."
    if ! command -v fnm &>/dev/null; then
        curl -fsSL https://fnm.vercel.app/install | bash
        export PATH="$HOME/.local/share/fnm:$PATH"
        eval "$(fnm env)"
    fi
    fnm install 22
    fnm use 22
    fnm default 22
fi

# OmniRoute
if command -v npm &>/dev/null; then
    info "Installing omniroute globally..."
    npm install -g omniroute@latest
else
    error "npm not found"
fi

# Download binary
mkdir -p "$BIN_DIR"
info "Downloading latest IGRIS release..."
LATEST="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"'tag_name'"' | cut -d'"' -f4)"
LATEST="${LATEST:-v0.1.0}"
curl -fsSL "https://github.com/$REPO/releases/download/$LATEST/igris-$TARGET" -o "$BIN_DIR/igris"
chmod +x "$BIN_DIR/igris"
info "Binary saved to $BIN_DIR/igris"

# Config from repo
mkdir -p "$CONFIG_DIR"
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    info "Downloading config.toml from repository..."
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/config.toml" -o "$CONFIG_DIR/config.toml"
fi

# Secrets template
if [ ! -f "$CONFIG_DIR/secrets.toml" ]; then
    cat > "$CONFIG_DIR/secrets.toml" << 'EOF'
[llm]
api_key = "sk-your-openrouter-key"

[voice]
groq_api_key = "gsk-your-groq-key"

[telegram]
api_id = 12345
api_hash = "your-telegram-api-hash"
phone_number = "+1234567890"
EOF
    warn "Edit $CONFIG_DIR/secrets.toml with your actual API keys"
fi

# Done
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  IGRIS + OmniRoute installed!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Binary:   $BIN_DIR/igris"
echo "Config:   $CONFIG_DIR/config.toml (from repo)"
echo "Secrets:  $CONFIG_DIR/secrets.toml"
echo "OmniRoute: omniroute (global npm)"
echo ""
echo "Make sure $BIN_DIR is in your PATH"
echo "Run: igris"
echo ""