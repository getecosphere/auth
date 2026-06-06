#!/bin/bash

# RWID Auth Service - Development Mode
# Uses spring-boot:run for hot reload support

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")/backend"

echo "RWID Auth Service - Development Mode"
echo "====================================="
echo ""

# Check prerequisites
if ! command -v java &> /dev/null; then
  echo "Java not found. Install it first."
  exit 1
fi

if ! command -v mvn &> /dev/null; then
  echo "Maven not found. Install it first."
  exit 1
fi

# Start MongoDB if not running
if command -v mongosh &> /dev/null; then
  if ! mongosh --quiet --eval "db.adminCommand('ping')" &> /dev/null; then
    echo "Starting MongoDB..."
    brew services start mongodb-community 2>/dev/null || true
    sleep 2
  fi
fi

# Load .env and run with dev profile
export $(grep -v '^\s*#' "$BACKEND_DIR/.env" | grep -v '^\s*$' | xargs)

cd "$BACKEND_DIR"
echo "Running on http://localhost:${SERVER_PORT:-8080}/api"
echo ""
mvn spring-boot:run -Dspring-boot.run.profiles=dev
