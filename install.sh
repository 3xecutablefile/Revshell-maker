#!/bin/bash

# Revshell Maker Installation Script
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }
print_info() { echo -e "${BLUE}ℹ${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}      Revshell Maker Installation       ${NC}"
echo -e "${BLUE}      Rust Edition - Better Than Py     ${NC}"
echo -e "${BLUE}========================================${NC}"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -f "src/main.rs" ]; then
    print_error "Please run from Revshell Maker repository root"
    exit 1
fi

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
else
    print_error "Unsupported OS: $OSTYPE"
    exit 1
fi

print_info "Detected OS: $OS"

# Check if command exists
cmd_exists() { command -v "$1" >/dev/null 2>&1; }

# Install package
install_pkg() {
    local pkg=$1 cmd=$2
    if cmd_exists "$cmd"; then
        print_success "$pkg already installed"
        return
    fi

    print_info "Installing $pkg..."

    if [ "$OS" = "linux" ]; then
        if cmd_exists apt-get; then
            sudo apt-get update && sudo apt-get install -y "$pkg"
        elif cmd_exists yum; then
            sudo yum install -y "$pkg"
        elif cmd_exists dnf; then
            sudo dnf install -y "$pkg"
        else
            print_error "No supported package manager found"
            exit 1
        fi
    elif [ "$OS" = "macos" ]; then
        if cmd_exists brew; then
            brew install "$pkg"
        else
            print_error "Homebrew required. Install from https://brew.sh"
            exit 1
        fi
    fi
}

# Ensure sudo access is available
if [ "$EUID" -ne 0 ]; then
    print_info "This script requires sudo access for installing dependencies."
    print_info "You will be prompted for your password."
    sudo -v || { print_error "Sudo access is required for installation"; exit 1; }

    # Keep sudo timestamp updated throughout the installation
    (while true; do sudo -n true; sleep 60; kill -0 "$$" || exit; done) & SUDO_REFRESH_PID=$!
fi

# Install Rust
if ! cmd_exists cargo; then
    print_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    print_success "Rust installed"
else
    print_success "Rust already installed"
fi

# Install system dependencies for Rust compilation
print_info "Installing system dependencies..."
if [ "$OS" = "linux" ]; then
    if cmd_exists apt-get; then
        print_info "Installing build-essential, pkg-config, and libssl-dev..."
        sudo apt-get update
        sudo apt-get install -y build-essential pkg-config libssl-dev
    elif cmd_exists yum; then
        sudo yum install -y gcc pkgconfig openssl-devel
    elif cmd_exists dnf; then
        sudo dnf install -y gcc pkgconfig openssl-devel
    else
        print_error "No supported package manager found for system dependencies"
        exit 1
    fi
fi

# Install netcat for testing listener functionality
print_info "Installing essential tools..."
install_pkg "netcat" "nc"
install_pkg "curl" "curl"

# Build Revshell Maker
print_info "Building Revshell Maker Rust Edition..."
cargo build --release
print_success "Build complete"

# Install binary
print_info "Installing to system..."
INSTALL_DIR="/usr/local/bin"
if [ -w "$INSTALL_DIR" ] || sudo -n true 2>/dev/null; then
    sudo cp target/release/revsh "$INSTALL_DIR/revsh"
    sudo chmod +x "$INSTALL_DIR/revsh"
    print_success "Installed to $INSTALL_DIR as 'revsh'"
    print_info "Run with: revsh"
else
    print_warning "Cannot install to system PATH"
    print_info "Binary available at: $(pwd)/target/release/revsh"
    print_info "Run with: ./target/release/revsh (or copy to desired location)"
fi

# Final verification
echo -e "\n${GREEN}Installation Summary:${NC}"
cmd_exists nc && print_success "netcat (for testing)" || print_warning "netcat"
cmd_exists curl && print_success "curl (for public IP detection)" || print_warning "curl"

if cmd_exists revsh; then
    print_success "revsh (system)"
    echo -e "\n${GREEN}Ready to generate shells!${NC}"
    echo "  revsh"
    echo ""
    echo "${BLUE}New in Rust Edition:${NC}"
    echo "  ✨ Built-in listener - No more external tools needed"
    echo "  🚀 Better performance with Rust native speed"
    echo "  🛡️ Enhanced security and error handling"
    echo "  📦 More payload templates & obfuscation options"
    echo "  💾 Save/load configuration capability"
elif [ -f "target/release/revsh" ]; then
    print_success "revsh (local - use 'revsh' after system installation)"
    echo -e "\n${GREEN}Ready to generate shells!${NC}"
    echo "  ./target/release/revsh"
fi

echo -e "\n${GREEN}Happy hacking! 🚀${NC}"