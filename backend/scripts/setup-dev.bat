@echo off
REM RWID Auth Service (Rust) - Local Development Setup Script (Windows)
REM This script installs all dependencies needed to run the service locally on Windows

setlocal enabledelayedexpansion

echo.
echo RWID Auth Service - Local Development Setup (Windows)
echo =======================================================
echo.

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo This script must be run as Administrator
    echo    Right-click Command Prompt and select "Run as administrator"
    pause
    exit /b 1
)

where choco >nul 2>&1
if %errorLevel% neq 0 (
    echo Installing Chocolatey...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "iex ((New-Object System.Net.ServicePointManager).SecurityProtocol = 3072; iex(New-Object Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))"
    if %errorLevel% neq 0 (
        echo Failed to install Chocolatey
        pause
        exit /b 1
    )
) else (
    echo Chocolatey is already installed
)

echo.
echo Installing dependencies...
echo.

where cargo >nul 2>&1
if %errorLevel% neq 0 (
    echo Installing Rust toolchain...
    choco install rust -y
    if %errorLevel% neq 0 (
        echo Failed to install Rust
        pause
        exit /b 1
    )
    echo Rust toolchain installed
) else (
    echo Rust is already installed
)

where mongod >nul 2>&1
if %errorLevel% neq 0 (
    echo Installing MongoDB...
    choco install mongodb -y
    if %errorLevel% neq 0 (
        echo Failed to install MongoDB
        pause
        exit /b 1
    )
    echo MongoDB installed
) else (
    echo MongoDB is already installed
)

where mongosh >nul 2>&1
if %errorLevel% neq 0 (
    echo Installing MongoDB Shell (mongosh)...
    choco install mongosh -y
    if %errorLevel% neq 0 (
        echo Failed to install mongosh (optional)
    ) else (
        echo MongoDB Shell installed
    )
) else (
    echo MongoDB Shell is already installed
)

echo.
echo Setting up MongoDB service...
echo.

net start MongoDB >nul 2>&1
if %errorLevel% equ 0 (
    echo MongoDB service started
) else (
    echo MongoDB service may already be running or failed to start
    echo    Try: net start MongoDB (in Administrator Command Prompt)
)

timeout /t 3 /nobreak

mongosh --eval "db.adminCommand('ping')" >nul 2>&1
if %errorLevel% equ 0 (
    echo MongoDB is running and accessible
) else (
    echo MongoDB may not be running. Try: net start MongoDB
)

echo.
echo Creating .env file...
echo.

if not exist "backend\.env" (
    copy backend\.env.example backend\.env >nul
    echo .env file created from .env.example at backend\.env
    echo    Update JWT_SECRET and other credentials as needed
) else (
    echo .env file already exists
)

echo.
echo Creating storage directory...
if not exist "backend\storage" mkdir backend\storage
echo Storage directory created

echo.
echo Building the project...
echo.

cd backend
call cargo build

if %errorLevel% neq 0 (
    echo Build failed
    pause
    exit /b 1
)

echo.
echo Setup complete!
echo.
echo Next steps:
echo    1. Update backend\.env with your configuration
echo    2. Run: cd backend ^&^& cargo run
echo    3. API will be available at http://localhost:8080/api
echo.
echo Useful commands:
echo    - Start MongoDB: net start MongoDB
echo    - Stop MongoDB: net stop MongoDB
echo    - Connect to MongoDB: mongosh
echo.
pause
