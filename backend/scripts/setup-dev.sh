#!/bin/bash

# RWID Auth Service (Rust) - Local Development Setup Script
# This script installs all dependencies needed to run the service locally on macOS

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR/.."

echo "RWID Auth Service - Local Development Setup"
echo "============================================"
echo ""

if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "This script is designed for macOS. Please adjust for your OS."
    exit 1
fi

if ! command -v brew &> /dev/null; then
    echo "Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
else
    echo "Homebrew is already installed"
fi

echo ""
echo "Installing dependencies..."
echo ""

if ! command -v cargo &> /dev/null; then
    echo "Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "Rust toolchain installed"
else
    echo "Rust is already installed: $(rustc --version)"
fi

if ! command -v mongod &> /dev/null; then
    echo "Installing MongoDB..."
    brew tap mongodb/brew
    brew install mongodb-community
    echo "MongoDB installed"
else
    echo "MongoDB is already installed"
fi

if ! command -v mongosh &> /dev/null; then
    echo "Installing MongoDB Shell (mongosh)..."
    brew install mongosh
    echo "MongoDB Shell installed"
else
    echo "MongoDB Shell is already installed"
fi

echo ""
echo "Setting up MongoDB service..."
echo ""

if brew services list | grep -q "mongodb-community"; then
    brew services stop mongodb-community || true
    sleep 2
fi

brew services start mongodb-community
sleep 3

if mongosh --eval "db.adminCommand('ping')" &> /dev/null; then
    echo "MongoDB is running and accessible"
else
    echo "MongoDB may not be running. Try: brew services start mongodb-community"
fi

echo ""
echo "Creating .env file..."
echo ""

if [ ! -f ".env" ]; then
    cp .env.example .env
    echo ".env file created from .env.example"
    echo "   Update JWT_SECRET and other credentials as needed"
else
    echo ".env file already exists"
fi

echo ""
echo "Creating storage directory..."
mkdir -p storage
echo "Storage directory created"

echo ""
echo "Building the project..."
echo ""

cargo build

echo ""
echo "Setup complete!"
echo ""
echo "Next steps:"
echo "   1. Update .env with your configuration"
echo "   2. Run: cargo run"
echo "   3. API will be available at http://localhost:8080/api"
