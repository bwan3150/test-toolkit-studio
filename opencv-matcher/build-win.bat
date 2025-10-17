@echo off
REM Windows 构建脚本 - OpenCV Matcher
setlocal enabledelayedexpansion

echo ===============================================
echo Building OpenCV Matcher (Windows)
echo ===============================================

REM 获取脚本所在目录（opencv-matcher目录）
cd /d "%~dp0"

REM Windows 平台配置
set PLATFORM=win32
set OUTPUT_DIR=%~dp0..\resources\%PLATFORM%\toolkit-engine
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

echo Syncing dependencies (including dev dependencies)...
uv sync --group dev
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

REM 测试可执行文件
echo.
echo Testing executable...
"%OUTPUT_DIR%\tke-opencv.exe" 2>nul | findstr /C:"" >nul
if errorlevel 1 (
    echo Warning: tke-opencv might not be executable
) else (
    echo tke-opencv executable test passed
)

echo.
echo ===============================================
echo OpenCV Matcher Build Finished (Windows)
echo ===============================================
