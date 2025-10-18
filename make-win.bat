@echo off
REM Windows 平台构建脚本
REM 用于在 Windows 上构建 Test Toolkit Studio
setlocal enabledelayedexpansion

echo ==========================================
echo   Test Toolkit Studio - Windows Build
echo ==========================================
echo.

REM 1. 安装 Node.js 依赖
echo ^>^>^> [1/5] Installing Node.js dependencies...
call npm install
if errorlevel 1 (
    echo Error: npm install failed
    exit /b 1
)

REM 2. 修复依赖问题
echo.
echo ^>^>^> [2/5] Fixing npm audit issues...
call npm audit fix

REM 3. 构建 Rust 项目：toolkit-engine (TKE)
echo.
echo ^>^>^> [3/5] Building Toolkit Engine (Rust)...
call toolkit-engine\build-win.bat
if errorlevel 1 (
    echo Error: toolkit-engine build failed
    exit /b 1
)

REM 4. 构建 Python 项目：opencv-matcher
echo.
echo ^>^>^> [4/5] Building OpenCV Matcher (Python)...
call opencv-matcher\build-win.bat
if errorlevel 1 (
    echo Error: opencv-matcher build failed
    exit /b 1
)

REM 5. 构建 Rust 项目：tester-ai
echo.
echo ^>^>^> [5/5] Building AI Tester (Rust)...
call tester-ai\build-win.bat
if errorlevel 1 (
    echo Error: tester-ai build failed
    exit /b 1
)

REM 6. 构建 Electron 应用（Windows）
echo.
echo ^>^>^> Building Electron app for Windows...
call npm run build-win
if errorlevel 1 (
    echo Error: Electron build failed
    exit /b 1
)

echo.
echo ==========================================
echo   Windows Build Completed Successfully!
echo ==========================================
echo.
echo Output: .\dist\
