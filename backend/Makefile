.PHONY: help setup install build run test clean logs stop verify

help:
	@echo "RWID Auth Service (Rust) - Development Commands"
	@echo "================================================="
	@echo ""
	@echo "Setup Commands:"
	@echo "  make setup           - Check for the Rust toolchain"
	@echo ""
	@echo "Development Commands:"
	@echo "  make install         - Fetch dependencies"
	@echo "  make build           - Build the project (release)"
	@echo "  make run             - Run the application"
	@echo "  make test            - Run tests"
	@echo "  make clean           - Clean build artifacts"
	@echo ""
	@echo "Utility Commands:"
	@echo "  make logs            - View application logs"
	@echo "  make stop            - Stop the application"
	@echo "  make verify          - Verify MongoDB connection"
	@echo ""

setup:
	@echo "Checking for the Rust toolchain..."
	@command -v cargo >/dev/null 2>&1 || { echo "Install Rust first: https://rustup.rs"; exit 1; }
	@rustc --version
	@cargo --version

install:
	@echo "Fetching dependencies..."
	@cargo fetch

build:
	@echo "Building the project..."
	@cargo build --release

run:
	@echo "Starting the application..."
	@cargo run

test:
	@echo "Running tests..."
	@cargo test

clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf logs/

logs:
	@echo "Viewing application logs..."
	@tail -f logs/application.log

stop:
	@echo "Stopping the application..."
	@pkill -f "target/(debug|release)/rwid-auth-service" || echo "Application not running"

verify:
	@echo "Verifying MongoDB connection..."
	@mongosh --eval "db.adminCommand('ping')" && echo "MongoDB is running" || echo "MongoDB is not running"

.DEFAULT_GOAL := help
