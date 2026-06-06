.PHONY: help setup setup-mac setup-windows setup-linux install build run test clean logs stop start verify

help:
	@echo "RWID Community API - Development Commands"
	@echo "=========================================="
	@echo ""
	@echo "Setup Commands:"
	@echo "  make setup-mac       - Setup for macOS"
	@echo "  make setup-windows   - Setup for Windows"
	@echo "  make setup-linux     - Setup for Linux"
	@echo ""
	@echo "Development Commands:"
	@echo "  make install         - Install dependencies"
	@echo "  make build           - Build the project"
	@echo "  make run             - Run the application"
	@echo "  make test            - Run tests"
	@echo "  make clean           - Clean build artifacts"
	@echo ""
	@echo "Utility Commands:"
	@echo "  make logs            - View application logs"
	@echo "  make stop            - Stop the application"
	@echo "  make verify          - Verify MongoDB connection"
	@echo ""

setup-mac:
	@echo "Setting up for macOS..."
	@chmod +x scripts/setup-dev.sh
	@./scripts/setup-dev.sh

setup-windows:
	@echo "Setting up for Windows..."
	@echo "Please run: scripts\setup-dev.bat (as Administrator)"

setup-linux:
	@echo "Setting up for Linux..."
	@chmod +x scripts/setup-dev-linux.sh
	@sudo ./scripts/setup-dev-linux.sh

install:
	@echo "Installing dependencies..."
	@mvn clean install -DskipTests

build:
	@echo "Building the project..."
	@mvn clean package -DskipTests

run:
	@echo "Starting the application..."
	@mvn spring-boot:run

test:
	@echo "Running tests..."
	@mvn test

test-coverage:
	@echo "Running tests with coverage..."
	@mvn test jacoco:report
	@echo "Coverage report: target/site/jacoco/index.html"

clean:
	@echo "Cleaning build artifacts..."
	@mvn clean
	@rm -rf logs/

logs:
	@echo "Viewing application logs..."
	@tail -f logs/application.log

stop:
	@echo "Stopping the application..."
	@pkill -f "spring-boot:run" || echo "Application not running"

verify:
	@echo "Verifying MongoDB connection..."
	@mongosh --eval "db.adminCommand('ping')" && echo "✅ MongoDB is running" || echo "❌ MongoDB is not running"

.DEFAULT_GOAL := help
