@echo off
REM 取 tke 二进制（Windows）。不再从源码构建 —— toolkit-engine 已拆成独立仓库
REM （github.com/bwan3150/Test-Toolkit-Engine），它的 CI 把六平台二进制发到分发源。
REM 落点是运行时真正会找的地方：bin\win32\tke.exe（handlers 用 process.platform）。
setlocal

if "%TKE_DIST_BASE%"=="" set TKE_DIST_BASE=https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke
set REMOTE=windows-amd64
set OUT=%~dp0..\bin\win32
set TARGET=%OUT%\tke.exe

if exist "%TARGET%" if not "%1"=="--force" (
    echo 已存在: %TARGET%（要换成分发源那一版就加 --force）
    exit /b 0
)

if not exist "%OUT%" mkdir "%OUT%"

echo 下载 %REMOTE% -^> %TARGET%
REM 解压后必须验一下真的拿到了二进制：这个平台对不存在的路径会回 200 + HTML
powershell -NoProfile -Command ^
  "$ErrorActionPreference='Stop';" ^
  "$v = (Invoke-WebRequest -UseBasicParsing \"$env:TKE_DIST_BASE/VERSION?t=$(Get-Random)\").Content;" ^
  "if ($v -notmatch '^tke ') { Write-Error '取不到 VERSION，分发源不可用'; exit 1 };" ^
  "$b = ([regex]::Match($v,'(?m)^build: *(.+)$')).Groups[1].Value.Trim();" ^
  "Write-Host ('分发源版本: ' + ($v -split \"`n\")[0]);" ^
  "$tmp = [IO.Path]::GetTempFileName();" ^
  "Invoke-WebRequest -UseBasicParsing \"$env:TKE_DIST_BASE/bin/%REMOTE%/tke.gz?b=$b\" -OutFile $tmp;" ^
  "$fs = [IO.File]::OpenRead($tmp); $h = New-Object byte[] 2; $null = $fs.Read($h,0,2); $fs.Close();" ^
  "if ($h[0] -ne 0x1f -or $h[1] -ne 0x8b) { Remove-Item $tmp; Write-Error '取回的不是 gzip（多半是 404 兜底页面）'; exit 1 };" ^
  "$in = [IO.File]::OpenRead($tmp); $out = [IO.File]::Create('%TARGET%');" ^
  "$gz = New-Object IO.Compression.GzipStream($in, [IO.Compression.CompressionMode]::Decompress);" ^
  "$gz.CopyTo($out); $gz.Close(); $out.Close(); $in.Close(); Remove-Item $tmp;"
if errorlevel 1 (
    echo Error: 取 tke 失败
    exit /b 1
)

echo 完成: %TARGET%
echo   依赖用 "%TARGET%" doctor --fix 补（adb / chromedriver / aapt / go-ios）
endlocal
