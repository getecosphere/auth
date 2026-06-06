#!/bin/bash

# RWID Community API - Local Development Setup Script
# This script installs all dependencies needed to run the backend locally on macOS

set -e

# Get the directory where this script is located and change to it
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR/.."

echo "🚀 RWID Community API - Local Development Setup"
echo "================================================"
echo ""

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ This script is designed for macOS. Please adjust for your OS."
    exit 1
fi

# Check if Homebrew is installed
if ! command -v brew &> /dev/null; then
    echo "📦 Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
else
    echo "✅ Homebrew is already installed"
fi

echo ""
echo "📦 Installing dependencies..."
echo ""

# Install Java 17 if not already installed
if ! command -v java &> /dev/null; then
    echo "📥 Installing Java 17..."
    brew install openjdk@17
    sudo ln -sfn /usr/local/opt/openjdk@17/libexec/openjdk.jdk /Library/Java/JavaVirtualMachines/openjdk-17.jdk
    echo "✅ Java 17 installed"
else
    echo "✅ Java is already installed: $(java -version 2>&1 | head -n 1)"
fi

# Install Maven
if ! command -v mvn &> /dev/null; then
    echo "📥 Installing Maven..."
    brew install maven
    echo "✅ Maven installed"
else
    echo "✅ Maven is already installed"
fi

# Install MongoDB
if ! command -v mongod &> /dev/null; then
    echo "📥 Installing MongoDB..."
    brew tap mongodb/brew
    brew install mongodb-community
    echo "✅ MongoDB installed"
else
    echo "✅ MongoDB is already installed"
fi

# Install MongoDB Shell (mongosh)
if ! command -v mongosh &> /dev/null; then
    echo "📥 Installing MongoDB Shell (mongosh)..."
    brew install mongosh
    echo "✅ MongoDB Shell installed"
else
    echo "✅ MongoDB Shell is already installed"
fi

echo ""
echo "🔧 Setting up MongoDB service..."
echo ""

# Start MongoDB service
if brew services list | grep -q "mongodb-community"; then
    echo "⏸️  Stopping existing MongoDB service..."
    brew services stop mongodb-community || true
    sleep 2
fi

echo "▶️  Starting MongoDB service..."
brew services start mongodb-community
sleep 3

# Verify MongoDB is running
if mongosh --eval "db.adminCommand('ping')" &> /dev/null; then
    echo "✅ MongoDB is running and accessible"
else
    echo "⚠️  MongoDB may not be running. Try: brew services start mongodb-community"
fi

echo ""
echo "📝 Creating .env file..."
echo ""

# Create .env file if it doesn't exist
if [ ! -f ".env" ]; then
    cat > .env << 'EOF'
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
    echo "✅ .env file created at .env"
    echo "   ⚠️  Update JWT_SECRET and other credentials as needed"
else
    echo "✅ .env file already exists"
fi

echo ""
echo "📂 Creating storage directory..."
mkdir -p storage/{avatars,videos,documents}
echo "✅ Storage directories created"

echo ""
echo "🏗️  Building the project..."
echo ""

mvn clean install -DskipTests

echo ""
echo "✅ Setup complete!"
echo ""
echo "📋 Next steps:"
echo "   1. Update .env with your configuration"
echo "   2. Run: cd backend && mvn spring-boot:run"
echo "   3. API will be available at http://localhost:8080/api"
echo ""
echo "🔍 Useful commands:"
echo "   - Check MongoDB status: brew services list | grep mongodb"
echo "   - Start MongoDB: brew services start mongodb-community"
echo "   - Stop MongoDB: brew services stop mongodb-community"
echo "   - Connect to MongoDB: mongosh"
echo ""
