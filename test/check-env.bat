@echo off
setlocal

echo ========================================
echo   Test Toolkit Studio - Env Check
echo ========================================
echo.

set ERROR_COUNT=0

REM ============ Node.js ============
echo [1/6] Checking Node.js...
where node >nul 2>&1
if errorlevel 1 (
    echo   [X] Node.js missing from PATH
    set /a ERROR_COUNT+=1
) else (
    echo   [OK] Node.js found in PATH
    where node

    REM Test if node can actually run
    echo   Testing node execution...
    node -e "console.log('OK')" >nul 2>&1
    if errorlevel 1 (
        echo   [ERROR] Node.js found but cannot execute!
        echo   [ERROR] Your Node.js installation is broken
        set /a ERROR_COUNT+=1
    ) else (
        echo   [OK] Node.js can execute
    )
)

where npm >nul 2>&1
if errorlevel 1 (
    echo   [X] npm missing from PATH
    set /a ERROR_COUNT+=1
) else (
    echo   [OK] npm found in PATH
)
echo.

REM ============ Rust ============
echo [2/6] Checking Rust...
where rustc >nul 2>&1
if errorlevel 1 (
    echo   [X] Rust missing from PATH
    echo       Download: https://www.rust-lang.org/tools/install
    set /a ERROR_COUNT+=1
) else (
    echo   [OK] Rust found in PATH
    where rustc
)

where cargo >nul 2>&1
if errorlevel 1 (
    echo   [X] Cargo missing from PATH
    set /a ERROR_COUNT+=1
) else (
    echo   [OK] Cargo found in PATH
)
echo.

REM ============ Python ============
echo [3/6] Checking Python...
where python >nul 2>&1
if errorlevel 1 (
    echo   [X] Python missing from PATH
    echo       Download: https://www.python.org/downloads/
    set /a ERROR_COUNT+=1
) else (
    echo   [OK] Python found in PATH
    where python

    REM Test if python can actually run
    echo   Testing python execution...
    python -c "print('OK')" >nul 2>&1
    if errorlevel 1 (
        echo   [ERROR] Python found but cannot execute!
        set /a ERROR_COUNT+=1
    ) else (
        echo   [OK] Python can execute
    )
)

where pip >nul 2>&1
if errorlevel 1 (
    echo   [X] pip missing from PATH
    set /a ERROR_COUNT+=1
) else (
    echo   [OK] pip found in PATH
)
echo.

REM ============ uv ============
echo [4/6] Checking uv...
where uv >nul 2>&1
if errorlevel 1 (
    REM Check if uv can be run via python -m
    python -m uv --version >nul 2>&1
    if errorlevel 1 (
        echo   [X] uv not installed
        echo       Run: pip install uv
        set /a ERROR_COUNT+=1
    ) else (
        echo   [OK] uv installed ^(via python -m uv^)
    )
) else (
    echo   [OK] uv found in PATH
    where uv
)
echo.

REM ============ PyInstaller ============
echo [5/6] Checking PyInstaller...
where pyinstaller >nul 2>&1
if errorlevel 1 (
    echo   [WARN] PyInstaller missing ^(will auto-install during build^)
) else (
    echo   [OK] PyInstaller found in PATH
)
echo.

REM ============ MSVC ============
echo [6/6] Checking MSVC toolchain...
where cl.exe >nul 2>&1
if errorlevel 1 (
    echo   [WARN] MSVC ^(cl.exe^) missing from PATH
    echo          Rust requires MSVC to compile on Windows
    echo          You need: Visual Studio Build Tools
    echo          Download: https://visualstudio.microsoft.com/downloads/
    echo          Choose: "Desktop development with C++"
) else (
    echo   [OK] MSVC found in PATH
    where cl.exe
)
echo.

REM ============ PATH Diagnosis ============
echo ========================================
echo   PATH Diagnosis
echo ========================================
echo.
echo Your PATH contains:
echo %PATH:;=&echo %
echo.

echo ========================================
echo   Summary
echo ========================================
echo.

if %ERROR_COUNT% EQU 0 (
    echo [OK] All required tools are properly installed
    echo [OK] You can run: make-win.bat
    exit /b 0
) else (
    echo [ERROR] Found %ERROR_COUNT% issue(s)
    echo.
    echo Common fixes:
    echo   1. Restart your terminal/CMD after installing tools
    echo   2. Check if tools are in PATH (see PATH Diagnosis above)
    echo   3. Reinstall broken tools (Node.js, Python, etc.)
    echo.
    echo See: docs\WINDOWS_SETUP.md for installation guide
    exit /b 1
)
