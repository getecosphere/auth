#!/bin/bash

# RWID Community API - Local Development Setup Script (Linux)
# This script installs all dependencies needed to run the backend locally on Linux (Ubuntu/Debian)

set -e

echo "🚀 RWID Community API - Local Development Setup (Linux)"
echo "======================================================"
echo ""

# Check if running on Linux
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo "❌ This script is designed for Linux. Please use setup-dev.sh for macOS or setup-dev.bat for Windows."
    exit 1
fi

# Check if running with sudo
if [[ $EUID -ne 0 ]]; then
    echo "❌ This script must be run with sudo"
    echo "   Usage: sudo ./scripts/setup-dev-linux.sh"
    exit 1
fi

echo "📦 Updating package manager..."
apt-get update

echo ""
echo "📦 Installing dependencies..."
echo ""

# Install Java 17
if ! command -v java &> /dev/null || ! java -version 2>&1 | grep -q "17"; then
    echo "📥 Installing Java 17..."
    apt-get install -y openjdk-17-jdk
    echo "✅ Java 17 installed"
else
    echo "✅ Java 17 is already installed"
fi

# Install Maven
if ! command -v mvn &> /dev/null; then
    echo "📥 Installing Maven..."
    apt-get install -y maven
    echo "✅ Maven installed"
else
    echo "✅ Maven is already installed"
fi

# Install MongoDB
if ! command -v mongod &> /dev/null; then
    echo "📥 Installing MongoDB..."
    
    # Add MongoDB repository
    apt-get install -y gnupg curl
    curl -fsSL https://www.mongodb.org/static/pgp/server-6.0.asc | apt-key add -
    echo "deb [ arch=amd64,arm64 ] https://repo.mongodb.org/apt/ubuntu focal/mongodb-org/6.0 multiverse" | tee /etc/apt/sources.list.d/mongodb-org-6.0.list
    
    apt-get update
    apt-get install -y mongodb-org
    
    # Start MongoDB service
    systemctl start mongod
    systemctl enable mongod
    
    echo "✅ MongoDB installed and started"
else
    echo "✅ MongoDB is already installed"
fi

# Install MongoDB Shell (mongosh)
if ! command -v mongosh &> /dev/null; then
    echo "📥 Installing MongoDB Shell (mongosh)..."
    
    # Add MongoDB repository for mongosh
    curl -fsSL https://www.mongodb.org/static/pgp/server-6.0.asc | apt-key add -
    echo "deb [ arch=amd64,arm64 ] https://repo.mongodb.org/apt/ubuntu focal/mongodb-org/6.0 multiverse" | tee /etc/apt/sources.list.d/mongodb-org-6.0.list
    
    apt-get update
    apt-get install -y mongosh
    
    echo "✅ MongoDB Shell installed"
else
    echo "✅ MongoDB Shell is already installed"
fi

echo ""
echo "🔧 Setting up MongoDB service..."
echo ""

# Start MongoDB service
echo "▶️  Starting MongoDB service..."
systemctl start mongod
systemctl enable mongod
sleep 3

# Verify MongoDB is running
if mongosh --eval "db.adminCommand('ping')" &> /dev/null; then
    echo "✅ MongoDB is running and accessible"
else
    echo "⚠️  MongoDB may not be running. Try: sudo systemctl start mongod"
fi

echo ""
echo "📝 Creating .env file..."
echo ""

# Create .env file if it doesn't exist
if [ ! -f "backend/.env" ]; then
    cat > backend/.env << 'EOF'
# MongoDB Configuration
MONGODB_URI=mongodb://localhost:27017/rwid_community

# JWT Configuration
JWT_SECRET=your-secret-key-change-in-production-$(date +%s)
JWT_EXPIRATION=86400000

# Server Configuration
SERVER_PORT=8080

# Storage Configuration
STORAGE_TYPE=local
STORAGE_LOCAL_PATH=./storage

# Google OAuth (optional - configure if needed)
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
GOOGLE_REDIRECT_URI=http://localhost:8080/api/auth/oauth/google/callback

# Midtrans Payment Gateway (optional - configure if needed)
MIDTRANS_SERVER_KEY=
MIDTRANS_CLIENT_KEY=
MIDTRANS_IS_PRODUCTION=false

# AWS S3 (optional - configure if using S3 storage)
S3_BUCKET=
S3_REGION=us-east-1
S3_ACCESS_KEY=
S3_SECRET_KEY=
EOF
    echo "✅ .env file created at backend/.env"
    echo "   ⚠️  Update JWT_SECRET and other credentials as needed"
else
    echo "✅ .env file already exists"
fi

echo ""
echo "📂 Creating storage directory..."
mkdir -p backend/storage/{avatars,videos,documents}
chmod -R 755 backend/storage
echo "✅ Storage directories created"

echo ""
echo "🏗️  Building the project..."
echo ""

cd backend
mvn clean install -DskipTests

echo ""
echo "✅ Setup complete!"
echo ""
echo "📋 Next steps:"
echo "   1. Update backend/.env with your configuration"
echo "   2. Run: cd backend && mvn spring-boot:run"
echo "   3. API will be available at http://localhost:8080/api"
echo ""
echo "🔍 Useful commands:"
echo "   - Check MongoDB status: sudo systemctl status mongod"
echo "   - Start MongoDB: sudo systemctl start mongod"
echo "   - Stop MongoDB: sudo systemctl stop mongod"
echo "   - Connect to MongoDB: mongosh"
echo ""
