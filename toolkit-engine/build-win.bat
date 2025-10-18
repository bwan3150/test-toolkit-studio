@echo off
REM Windows 构建脚本 - Toolkit Engine (TKE)
setlocal enabledelayedexpansion

echo ===============================
echo Building Toolkit Engine (Windows)
echo ===============================

REM 获取脚本所在目录（toolkit-engine目录）
cd /d "%~dp0"

REM 读取版本号：package.json → BUILD_VERSION 环境变量
set PACKAGE_JSON=%~dp0..\package.json
if not exist "%PACKAGE_JSON%" (
    echo Error: package.json not found
    exit /b 1
)

REM 使用 PowerShell 提取版本号
for /f "delims=" %%i in ('powershell -Command "(Get-Content '%PACKAGE_JSON%' | ConvertFrom-Json).version"') do set PKG_VERSION=%%i

if "%PKG_VERSION%"=="" (
    echo Error: cannot extract version from package.json
    exit /b 1
)

REM 导出 BUILD_VERSION 环境变量供 build.rs 使用
set BUILD_VERSION=%PKG_VERSION%
echo Build version: %BUILD_VERSION%

echo Building for Windows (win32)...

REM 构建 release 版本
cargo build --release
if errorlevel 1 (
    echo Error: cargo build failed
    exit /b 1
)

REM Windows 平台配置
set PLATFORM=win32
set BINARY_NAME=tke.exe

REM 源文件路径
set SOURCE_BINARY=%~dp0target\release\%BINARY_NAME%

REM 目标目录和文件路径
set TARGET_DIR=%~dp0..\resources\%PLATFORM%\toolkit-engine
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
if errorlevel 1 (
    echo Warning: tke might not be executable
) else (
    echo tke --version successful
)

echo.
echo ===============================
echo TKE Build Finished (Windows)
echo ===============================
echo.
