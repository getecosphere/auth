#!/bin/bash

# RWID Auth Service (Rust) - Local Development Setup Script (Linux)
# This script installs all dependencies needed to run the service locally on Linux (Ubuntu/Debian)

set -e

echo "RWID Auth Service - Local Development Setup (Linux)"
echo "====================================================="
echo ""

if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo "This script is designed for Linux. Please use setup-dev.sh for macOS or setup-dev.bat for Windows."
    exit 1
fi

if [[ $EUID -ne 0 ]]; then
    echo "This script must be run with sudo"
    echo "   Usage: sudo ./scripts/setup-dev-linux.sh"
    exit 1
fi

echo "Updating package manager..."
apt-get update

echo ""
echo "Installing dependencies..."
echo ""

if ! command -v cargo &> /dev/null; then
    echo "Installing Rust toolchain..."
    apt-get install -y curl build-essential
    su - "${SUDO_USER:-$USER}" -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    echo "Rust toolchain installed for user ${SUDO_USER:-$USER}"
else
    echo "Rust is already installed: $(rustc --version)"
fi

if ! command -v mongod &> /dev/null; then
    echo "Installing MongoDB..."
    apt-get install -y gnupg curl
    curl -fsSL https://www.mongodb.org/static/pgp/server-7.0.asc | gpg --dearmor -o /usr/share/keyrings/mongodb-server-7.0.gpg
    echo "deb [ arch=amd64,arm64 signed-by=/usr/share/keyrings/mongodb-server-7.0.gpg ] https://repo.mongodb.org/apt/ubuntu focal/mongodb-org/7.0 multiverse" | tee /etc/apt/sources.list.d/mongodb-org-7.0.list
    apt-get update
    apt-get install -y mongodb-org
    systemctl start mongod
    systemctl enable mongod
    echo "MongoDB installed and started"
else
    echo "MongoDB is already installed"
fi

if ! command -v mongosh &> /dev/null; then
    echo "Installing MongoDB Shell (mongosh)..."
    apt-get install -y mongodb-mongosh
    echo "MongoDB Shell installed"
else
    echo "MongoDB Shell is already installed"
fi

echo ""
echo "Setting up MongoDB service..."
echo ""

systemctl start mongod
systemctl enable mongod
sleep 3

if mongosh --eval "db.adminCommand('ping')" &> /dev/null; then
    echo "MongoDB is running and accessible"
else
    echo "MongoDB may not be running. Try: sudo systemctl start mongod"
fi

echo ""
echo "Creating .env file..."
echo ""

if [ ! -f "backend/.env" ]; then
    cp backend/.env.example backend/.env
    echo ".env file created from .env.example at backend/.env"
    echo "   Update JWT_SECRET and other credentials as needed"
else
    echo ".env file already exists"
fi

echo ""
echo "Creating storage directory..."
mkdir -p backend/storage
chmod -R 755 backend/storage
echo "Storage directory created"

echo ""
echo "Building the project..."
echo ""

cd backend
su - "${SUDO_USER:-$USER}" -c "cd $(pwd) && cargo build"

echo ""
echo "Setup complete!"
echo ""
echo "Next steps:"
echo "   1. Update backend/.env with your configuration"
echo "   2. Run: cd backend && cargo run"
echo "   3. API will be available at http://localhost:8080/api"
