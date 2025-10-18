@echo off
REM Windows 构建脚本 - AI Tester
setlocal enabledelayedexpansion

echo ===============================
echo Building AI Tester (Windows)
echo ===============================

REM 获取脚本所在目录（tester-ai目录）
cd /d "%~dp0"

REM 版本号同步暂时跳过（手动管理 Cargo.toml 版本号）
echo Note: Version sync skipped (manage Cargo.toml version manually)

echo Building for Windows (win32)...

REM 构建 release 版本
cargo build --release
if errorlevel 1 (
    echo Error: cargo build failed
    exit /b 1
)

REM Windows 平台配置
set PLATFORM=win32
set BINARY_NAME=tester-ai.exe

REM 源文件路径
set SOURCE_BINARY=%~dp0target\release\%BINARY_NAME%

REM 目标目录和文件路径
set TARGET_DIR=%~dp0..\resources\%PLATFORM%\tester-ai
set TARGET_BINARY=%TARGET_DIR%\%BINARY_NAME%

REM 检查源文件是否存在
if not exist "%SOURCE_BINARY%" (
    echo Error: build failed, cannot find: %SOURCE_BINARY%
    exit /b 1
)

REM 创建目标目录
if not exist "%TARGET_DIR%" mkdir "%TARGET_DIR%"

REM 复制二进制文件
copy /Y "%SOURCE_BINARY%" "%TARGET_BINARY%"

echo Build successfully
echo Copied to: %TARGET_BINARY%

REM 获取文件大小
for %%A in ("%TARGET_BINARY%") do set SIZE=%%~zA
set /a SIZE_MB=%SIZE% / 1048576
echo Size: %SIZE_MB% MB

REM 验证二进制文件能否运行
"%TARGET_BINARY%" --version >nul 2>&1
if not errorlevel 1 (
    echo tester-ai --version successful
) else (
    "%TARGET_BINARY%" --help >nul 2>&1
    if not errorlevel 1 (
        echo tester-ai --help successful
    ) else (
        echo Warning: tester-ai might not be executable
    )
)

echo.
echo ===============================
echo AI Tester Build Finished (Windows)
echo ===============================
echo.
