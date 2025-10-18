@echo off
REM Windows 构建脚本 - OpenCV Matcher
setlocal enabledelayedexpansion

echo ===============================================
echo Building OpenCV Matcher (Windows)
echo ===============================================

REM 获取脚本所在目录（opencv-matcher目录）
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

REM 导出 BUILD_VERSION 环境变量
set BUILD_VERSION=%PKG_VERSION%
echo Build version: %BUILD_VERSION%

REM 生成 _version.py 文件（打包时嵌入版本号）
echo # 自动生成的版本文件 - 请勿手动修改 > _version.py
echo __version__ = '%BUILD_VERSION%' >> _version.py
echo 已生成 _version.py: %BUILD_VERSION%

REM Windows 平台配置
set PLATFORM=win32
set OUTPUT_DIR=%~dp0..\resources\%PLATFORM%\toolkit-engine
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

echo Syncing dependencies (including dev dependencies)...
python -m uv sync --group dev
if errorlevel 1 (
    echo Error: uv sync failed
    exit /b 1
)

echo Using PyInstaller to package...
.venv\Scripts\pyinstaller --onefile --name tke-opencv --clean --noconfirm opencv_matcher.py
if errorlevel 1 (
    echo Error: PyInstaller failed
    exit /b 1
)

REM 清理生成的 _version.py
if exist "_version.py" del /f /q "_version.py"

REM 检查打包是否成功
if not exist "dist\tke-opencv.exe" (
    echo Error: build failed, cannot find dist\tke-opencv.exe
    exit /b 1
)

REM 复制到目标目录
echo Copying to: %OUTPUT_DIR%\tke-opencv.exe
copy /Y "dist\tke-opencv.exe" "%OUTPUT_DIR%\tke-opencv.exe"

REM 获取文件大小
for %%A in ("%OUTPUT_DIR%\tke-opencv.exe") do set SIZE=%%~zA
set /a SIZE_MB=%SIZE% / 1048576
echo Build successfully
echo Output: %OUTPUT_DIR%\tke-opencv.exe
echo Size: %SIZE_MB% MB

REM 验证二进制文件能否运行
"%OUTPUT_DIR%\tke-opencv.exe" --version >nul 2>&1
if errorlevel 1 (
    echo Warning: tke-opencv might not be executable
) else (
    echo tke-opencv --version successful
)

echo.
echo ===============================================
echo OpenCV Matcher Build Finished (Windows)
echo ===============================================
