#!/bin/bash

# RWID Community API - Local Development Start Script
# This script starts the backend server with MongoDB Atlas

set -e

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

echo "🚀 Starting KTT Community API Backend"
echo "======================================"
echo ""

# Extract port from .env SERVER_PORT variable
# Default to 8080 if not found
PORT=$(grep '^SERVER_PORT=' "$BACKEND_DIR/.env" | cut -d'=' -f2 | tr -d '\r' 2>/dev/null || echo "8080")

echo ""
echo "▶️  Starting Spring Boot application on http://localhost:$PORT..."
echo ""

# Build the project first
echo "Building project..."
cd "$BACKEND_DIR"
mvn clean package -DskipTests -q

# Pass port as environment variable so Spring Boot can read it from application.yml
export SERVER_PORT=$PORT
java -jar target/rwid-community-api-1.0.0.jar

