@echo off
REM Windows 平台构建脚本
REM 用于在 Windows 上构建 Test Toolkit Studio
setlocal enabledelayedexpansion

echo ==========================================
echo   Test Toolkit Studio - Windows Build
echo ==========================================
echo.

REM 1. 安装 Node.js 依赖
echo ^>^>^> [1/7] Installing Node.js dependencies...
call npm install
if errorlevel 1 (
    echo Error: npm install failed
    exit /b 1
)

REM 2. 修复依赖问题
echo.
echo ^>^>^> [2/7] Fixing npm audit issues...
call npm audit fix

REM 3. 取 TKE 二进制（不再从源码构建：toolkit-engine 已拆成独立仓库，
REM    见 github.com/bwan3150/Test-Toolkit-Engine，它的 CI 发六平台二进制到分发源）
echo.
echo ^>^>^> [3/7] Fetching Toolkit Engine binary...
call scripts\fetch-tke.bat
if errorlevel 1 (
    echo Error: fetch tke failed
    exit /b 1
)

REM 4. 构建 Python 项目：opencv-matcher
echo.
echo ^>^>^> [4/7] Building OpenCV Matcher (Python)...
call opencv-matcher\build-win.bat
if errorlevel 1 (
    echo Error: opencv-matcher build failed
    exit /b 1
)

REM 5. 构建 Rust 项目：tester-ai
echo.
echo ^>^>^> [5/7] Building AI Tester (Rust)...
call tester-ai\build-win.bat
if errorlevel 1 (
    echo Error: tester-ai build failed
    exit /b 1
)

REM 6. 构建 Rust 项目：scrcpy-server
echo.
echo ^>^>^> [6/7] Building Scrcpy Server (Rust)...
call scrcpy-server\build-win.bat
if errorlevel 1 (
    echo Error: scrcpy-server build failed
    exit /b 1
)

REM 7. 检查并下载 FFmpeg（如果不存在）
echo.
echo ^>^>^> [7/7] Checking FFmpeg dependency...
set FFMPEG_PATH=.\resources\win32\toolkit-engine\ffmpeg.exe
if not exist "%FFMPEG_PATH%" (
    echo FFmpeg not found locally, downloading from S3...
    if not exist ".\resources\win32\toolkit-engine" mkdir ".\resources\win32\toolkit-engine"

    powershell -Command "& {Invoke-WebRequest -Uri 'https://toolkit-studio-updates.s3.ap-southeast-2.amazonaws.com/dependency/win32/ffmpeg.exe' -OutFile '%FFMPEG_PATH%'}"

    if errorlevel 1 (
        echo Error: FFmpeg download failed
        exit /b 1
    )

    echo [32m✓ FFmpeg downloaded successfully[0m
) else (
    echo [32m✓ FFmpeg already exists, skipping download[0m
)

REM 8. 构建 Electron 应用（Windows）
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
