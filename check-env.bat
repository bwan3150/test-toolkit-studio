@echo off
REM 环境检查脚本 - 验证所有构建工具是否正确安装
setlocal enabledelayedexpansion

echo ========================================
echo   Test Toolkit Studio 环境检查
echo ========================================
echo.

set ERROR_COUNT=0

REM 检查 Node.js
echo [1/6] 检查 Node.js...
node --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ Node.js 未安装或不在 PATH 中
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=*" %%a in ('node --version') do set NODE_VER=%%a
    echo   ✓ Node.js !NODE_VER!
)

REM 检查 npm
npm --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ npm 未安装或不在 PATH 中
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=*" %%a in ('npm --version') do set NPM_VER=%%a
    echo   ✓ npm !NPM_VER!
)
echo.

REM 检查 Rust
echo [2/6] 检查 Rust...
rustc --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ Rust 未安装或不在 PATH 中
    echo   请运行: https://www.rust-lang.org/tools/install
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=1,2" %%a in ('rustc --version') do set RUST_VER=%%b
    echo   ✓ Rust !RUST_VER!
)

REM 检查 Cargo
cargo --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ Cargo 未安装或不在 PATH 中
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=1,2" %%a in ('cargo --version') do set CARGO_VER=%%b
    echo   ✓ Cargo !CARGO_VER!
)
echo.

REM 检查 Python
echo [3/6] 检查 Python...
python --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ Python 未安装或不在 PATH 中
    echo   请访问: https://www.python.org/downloads/
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=1,2" %%a in ('python --version') do set PY_VER=%%b
    echo   ✓ Python !PY_VER!
)

REM 检查 pip
pip --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ pip 未安装或不在 PATH 中
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=1,2" %%a in ('pip --version') do set PIP_VER=%%b
    echo   ✓ pip !PIP_VER!
)
echo.

REM 检查 uv
echo [4/6] 检查 uv (Python 包管理器)...
uv --version >nul 2>&1
if errorlevel 1 (
    echo   ❌ uv 未安装
    echo   请运行: pip install uv
    set /a ERROR_COUNT+=1
) else (
    for /f "tokens=1,2" %%a in ('uv --version') do set UV_VER=%%b
    echo   ✓ uv !UV_VER!
)
echo.

REM 检查 PyInstaller
echo [5/6] 检查 PyInstaller...
pyinstaller --version >nul 2>&1
if errorlevel 1 (
    echo   ⚠ PyInstaller 未安装 (将在构建时自动安装)
) else (
    for /f "tokens=*" %%a in ('pyinstaller --version') do set PYINST_VER=%%a
    echo   ✓ PyInstaller !PYINST_VER!
)
echo.

REM 检查 Visual Studio Build Tools (MSVC)
echo [6/6] 检查 MSVC 工具链...
where cl.exe >nul 2>&1
if errorlevel 1 (
    echo   ⚠ MSVC (cl.exe) 未在 PATH 中
    echo   Rust 需要 MSVC 工具链
    echo   请安装: Visual Studio Build Tools
    echo   https://visualstudio.microsoft.com/downloads/
) else (
    for /f "tokens=*" %%a in ('cl.exe 2^>^&1 ^| findstr /C:"Version"') do set MSVC_VER=%%a
    echo   ✓ MSVC 已安装
)
echo.

echo ========================================
echo   环境检查完成
echo ========================================
echo.

if !ERROR_COUNT! equ 0 (
    echo ✓ 所有必需工具已正确安装
    echo ✓ 可以运行 make-win.bat 开始构建
    exit /b 0
) else (
    echo ❌ 发现 !ERROR_COUNT! 个问题
    echo 请参考 docs/WINDOWS_SETUP.md 配置环境
    exit /b 1
)
