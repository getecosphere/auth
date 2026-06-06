@echo off
REM RWID Community API - Local Development Setup Script (Windows)
REM This script installs all dependencies needed to run the backend locally on Windows

setlocal enabledelayedexpansion

echo.
echo 🚀 RWID Community API - Local Development Setup (Windows)
echo ========================================================
echo.

REM Check if running as administrator
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo ❌ This script must be run as Administrator
    echo    Right-click Command Prompt and select "Run as administrator"
    pause
    exit /b 1
)

REM Check if Chocolatey is installed
where choco >nul 2>&1
if %errorLevel% neq 0 (
    echo 📦 Installing Chocolatey...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "iex ((New-Object System.Net.ServicePointManager).SecurityProtocol = 3072; iex(New-Object Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))"
    if %errorLevel% neq 0 (
        echo ❌ Failed to install Chocolatey
        pause
        exit /b 1
    )
) else (
    echo ✅ Chocolatey is already installed
)

echo.
echo 📦 Installing dependencies...
echo.

REM Install Java 17
java -version >nul 2>&1
if %errorLevel% neq 0 (
    echo 📥 Installing Java 17...
    choco install openjdk17 -y
    if %errorLevel% neq 0 (
        echo ❌ Failed to install Java 17
        pause
        exit /b 1
    )
    echo ✅ Java 17 installed
) else (
    echo ✅ Java 17 is already installed
)

REM Install Maven
mvn -version >nul 2>&1
if %errorLevel% neq 0 (
    echo 📥 Installing Maven...
    choco install maven -y
    if %errorLevel% neq 0 (
        echo ❌ Failed to install Maven
        pause
        exit /b 1
    )
    echo ✅ Maven installed
) else (
    echo ✅ Maven is already installed
)

REM Install MongoDB
where mongod >nul 2>&1
if %errorLevel% neq 0 (
    echo 📥 Installing MongoDB...
    choco install mongodb -y
    if %errorLevel% neq 0 (
        echo ❌ Failed to install MongoDB
        pause
        exit /b 1
    )
    echo ✅ MongoDB installed
) else (
    echo ✅ MongoDB is already installed
)

REM Install MongoDB Shell (mongosh)
where mongosh >nul 2>&1
if %errorLevel% neq 0 (
    echo 📥 Installing MongoDB Shell (mongosh)...
    choco install mongosh -y
    if %errorLevel% neq 0 (
        echo ⚠️  Failed to install mongosh (optional)
    ) else (
        echo ✅ MongoDB Shell installed
    )
) else (
    echo ✅ MongoDB Shell is already installed
)

echo.
echo 🔧 Setting up MongoDB service...
echo.

REM Start MongoDB service
echo ▶️  Starting MongoDB service...
net start MongoDB >nul 2>&1
if %errorLevel% equ 0 (
    echo ✅ MongoDB service started
) else (
    echo ⚠️  MongoDB service may already be running or failed to start
    echo    Try: net start MongoDB (in Administrator Command Prompt)
)

timeout /t 3 /nobreak

REM Verify MongoDB is running
mongosh --eval "db.adminCommand('ping')" >nul 2>&1
if %errorLevel% equ 0 (
    echo ✅ MongoDB is running and accessible
) else (
    echo ⚠️  MongoDB may not be running. Try: net start MongoDB
)

echo.
echo 📝 Creating .env file...
echo.

REM Create .env file if it doesn't exist
if not exist "backend\.env" (
    (
        echo # MongoDB Configuration
        echo MONGODB_URI=mongodb://localhost:27017/rwid_community
        echo.
        echo # JWT Configuration
        echo JWT_SECRET=your-secret-key-change-in-production
        echo JWT_EXPIRATION=86400000
        echo.
        echo # Server Configuration
        echo SERVER_PORT=8080
        echo.
        echo # Storage Configuration
        echo STORAGE_TYPE=local
        echo STORAGE_LOCAL_PATH=./storage
        echo.
        echo # Google OAuth (optional - configure if needed^)
        echo GOOGLE_CLIENT_ID=
        echo GOOGLE_CLIENT_SECRET=
        echo GOOGLE_REDIRECT_URI=http://localhost:8080/api/auth/oauth/google/callback
        echo.
        echo # Midtrans Payment Gateway (optional - configure if needed^)
        echo MIDTRANS_SERVER_KEY=
        echo MIDTRANS_CLIENT_KEY=
        echo MIDTRANS_IS_PRODUCTION=false
        echo.
        echo # AWS S3 (optional - configure if using S3 storage^)
        echo S3_BUCKET=
        echo S3_REGION=us-east-1
        echo S3_ACCESS_KEY=
        echo S3_SECRET_KEY=
    ) > backend\.env
    echo ✅ .env file created at backend\.env
    echo    ⚠️  Update JWT_SECRET and other credentials as needed
) else (
    echo ✅ .env file already exists
)

echo.
echo 📂 Creating storage directory...
if not exist "backend\storage\avatars" mkdir backend\storage\avatars
if not exist "backend\storage\videos" mkdir backend\storage\videos
if not exist "backend\storage\documents" mkdir backend\storage\documents
echo ✅ Storage directories created

echo.
echo 🏗️  Building the project...
echo.

cd backend
call mvn clean install -DskipTests

if %errorLevel% neq 0 (
    echo ❌ Build failed
    pause
    exit /b 1
)

echo.
echo ✅ Setup complete!
echo.
echo 📋 Next steps:
echo    1. Update backend\.env with your configuration
echo    2. Run: cd backend ^&^& mvn spring-boot:run
echo    3. API will be available at http://localhost:8080/api
echo.
echo 🔍 Useful commands:
echo    - Start MongoDB: net start MongoDB
echo    - Stop MongoDB: net stop MongoDB
echo    - Connect to MongoDB: mongosh
echo.
pause
