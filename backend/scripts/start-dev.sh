#!/bin/bash

# RWID Auth Service (Rust) - Local Development Start Script

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"
cd "$BACKEND_DIR"

PORT=$(grep '^SERVER_PORT=' "$BACKEND_DIR/.env" | cut -d'=' -f2 | tr -d '\r' 2>/dev/null || echo "8080")

echo "Starting RWID Auth Service on http://localhost:$PORT..."
cargo run --release
