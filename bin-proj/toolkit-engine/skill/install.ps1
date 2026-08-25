<#
.SYNOPSIS
  tke skill 一键安装器（Windows；多 skill：-Skill tke-ui-test|tke-security-test）

.DESCRIPTION
  与 install.sh 一一对应，干三件事：装 skill 文件 → 装 tke 及同目录驱动 → 装 Chrome for Testing。
  全程幂等：重复跑只会覆盖同名文件，不会装重。

  用法（PowerShell）：
    irm https://<BASE_URL>/install.ps1 | iex
    # 带参数时要先落地再跑：
    iwr https://<BASE_URL>/install.ps1 -OutFile install.ps1; .\install.ps1 -Profile web

  skill 默认装用户级（%USERPROFILE%\.claude\skills，所有项目通用）；
  -Project 装到当前项目的 .claude\skills（跟着仓库走，团队 clone 即得）。

.NOTES
  两个分发源的坑（P-19，install.sh 踩过，这里同样防）：
    - 存储平台对不存在的路径回落 200 + 一段 HTML（SPA 兜底），所以每个文件都要**验文件头**，
      否则会把网页当二进制装进去
    - Cloudflare 缓存 4h 且不认 no-cache，只有变化的查询参数能破缓存，
      所以先取 VERSION 拿 build 戳，再用它当所有下载的键
#>
[CmdletBinding()]
param(
    # 装哪个 skill：tke-ui-test / tke-security-test
    [string]$Skill = 'tke-ui-test',

    # 只装这一类：web / android / ios / all / none（none = 只装 tke，安全测试用）
    [ValidateSet('web', 'android', 'ios', 'all', 'none', '')]
    [string]$Profile = '',

    # 装到当前项目的 .claude\skills（默认装用户级）
    [switch]$Project,

    # 分发源地址
    [string]$BaseUrl = $env:TKE_BASE_URL,

    # tke 及驱动的落点
    [string]$TkeHome = $env:TKE_HOME
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # 不显进度条：管道执行时它会刷屏且拖慢下载

# skill 决定默认 profile：安全测试只用 tke 的 http/recon，不需要设备驱动 → none
if (-not $Profile) { $Profile = if ($Skill -eq 'tke-security-test') { 'none' } else { 'all' } }

# ── 外观 ──（与 install.sh 同一套；PowerShell 5.1 不认 $PSStyle，用原始转义序列）
# ⚠️ 两个 PowerShell 标识符坑（实测踩过，见 uninstall.ps1 头注释）：
#    变量名**不区分大小写**（别用与参数同名的）、变量名**可以含中文**（`${Ye}文字` 必须加花括号）
$ES = [char]27
$Cy = "$ES[38;5;39m"; $Gn = "$ES[38;5;42m"; $Ye = "$ES[38;5;214m"
$Dm = "$ES[38;5;245m"; $Bd = "$ES[1m"; $Rs = "$ES[0m"
$SOK = "$Gn✓$Rs"; $SWARN = "$Ye!$Rs"; $SDOT = "$Dm·$Rs"
function Section([string]$Text) { Write-Host "`n$Bd$Cy▸ $Text$Rs" }
function KV([string]$K, [string]$V) { Write-Host ("  $Dm{0,-6}$Rs {1}" -f $K, $V) }

if (-not $BaseUrl) { $BaseUrl = 'https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke' }
if (-not $TkeHome) { $TkeHome = Join-Path $env:USERPROFILE '.tke\bin' }

# ── 平台探测（与分发源 bin/<platform>/ 的命名一致）──
$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'amd64' }
    'x86'   { '386' }
    # Windows on ARM 自带 x64 模拟，直接用 amd64 那套（Chrome for Testing 也没有 arm64 Windows 版）
    'ARM64' { 'amd64' }
    default { 'amd64' }
}
$platform = "windows-$arch"

# ── 下载 + 验文件头 ──
# 分发平台对不存在的路径回落 200 + HTML，只看状态码会把网页当二进制装进去
function Test-Magic {
    param([string]$Path, [string]$Kind)
    if (-not (Test-Path $Path)) { return $false }
    $fs = [System.IO.File]::OpenRead($Path)
    try {
        $b = New-Object byte[] 2
        if ($fs.Read($b, 0, 2) -lt 2) { return $false }
    } finally { $fs.Dispose() }
    switch ($Kind) {
        'gz'  { return ($b[0] -eq 0x1f -and $b[1] -eq 0x8b) }
        'zip' { return ($b[0] -eq 0x50 -and $b[1] -eq 0x4b) }   # "PK"
        default { return $true }
    }
}

function Get-File {
    param([string]$Url, [string]$Out, [string]$Kind = 'any')
    try {
        Invoke-WebRequest -Uri $Url -OutFile $Out -UseBasicParsing -TimeoutSec 900
    } catch {
        if (Test-Path $Out) { Remove-Item $Out -Force }
        return $false
    }
    if (-not (Test-Magic -Path $Out -Kind $Kind)) {
        Remove-Item $Out -Force -ErrorAction SilentlyContinue
        return $false
    }
    return $true
}

# gzip 解压（.NET 自带，不依赖外部命令）
function Expand-Gzip {
    param([string]$In, [string]$Out)
    $src = [System.IO.File]::OpenRead($In)
    $dst = [System.IO.File]::Create($Out)
    try {
        $gz = New-Object System.IO.Compression.GzipStream($src, [System.IO.Compression.CompressionMode]::Decompress)
        try { $gz.CopyTo($dst) } finally { $gz.Dispose() }
    } finally { $dst.Dispose(); $src.Dispose() }
}

# ── 缓存键：先破缓存取 VERSION，再用里面的 build 戳当后续下载的键 ──
$nonce = [System.Guid]::NewGuid().ToString('N').Substring(0, 8)
$remoteVersion = ''
try {
    # .Content 可能是 byte[]（取决于响应头与 PowerShell 版本）——直接当字符串用会得到
    # 一串 ASCII 码（实测显示成 "116"，那是 't'），build 戳也就解析不出来、缓存键失效
    $raw = (Invoke-WebRequest -Uri "$BaseUrl/VERSION?t=$nonce" -UseBasicParsing -TimeoutSec 30).Content
    $remoteVersion = if ($raw -is [byte[]]) { [System.Text.Encoding]::UTF8.GetString($raw) } else { [string]$raw }
} catch { }
$buildKey = ($remoteVersion -split "`n" | Where-Object { $_ -match '^build:' } | Select-Object -First 1) -replace '^build:\s*', ''
$q = if ($buildKey) { "?b=$($buildKey.Trim())" } else { "?t=$nonce" }

Write-Host $Cy -NoNewline
@'
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║  ████████╗ ██████╗  ██████╗ ██╗     ██╗  ██╗██╗████████╗  ║
║  ╚══██╔══╝██╔═══██╗██╔═══██╗██║     ██║ ██╔╝██║╚══██╔══╝  ║
║     ██║   ██║   ██║██║   ██║██║     █████╔╝ ██║   ██║     ║
║     ██║   ██║   ██║██║   ██║██║     ██╔═██╗ ██║   ██║     ║
║     ██║   ╚██████╔╝╚██████╔╝███████╗██║  ██╗██║   ██║     ║
║     ╚═╝    ╚═════╝  ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝   ╚═╝     ║
║                                                           ║
║                    E   N   G   I   N   E                  ║
╚═══════════════════════════════════════════════════════════╝
'@ | Write-Host
Write-Host $Rs -NoNewline
$verLine = if ($remoteVersion) { ($remoteVersion -split "`n")[0].Trim() } else { 'tke' }
Write-Host "  $Bd$verLine$Rs $Dm·$Rs $platform $Dm·$Rs $Profile"

$tmp = Join-Path $env:TEMP "tke-install-$nonce"
New-Item -ItemType Directory -Path $tmp -Force | Out-Null

try {
    # ── 1. skill 文件 ──
    $skillRoot = if ($Project) { Join-Path (Get-Location) '.claude\skills' } else { Join-Path $env:USERPROFILE '.claude\skills' }
    New-Item -ItemType Directory -Path $skillRoot -Force | Out-Null
    Section 'SKILL'

    $skillTgz = Join-Path $tmp 'skill.tar.gz'
    if (Get-File -Url "$BaseUrl/skill/$Skill.tar.gz$q" -Out $skillTgz -Kind 'gz') {
        Remove-Item (Join-Path $skillRoot $Skill) -Recurse -Force -ErrorAction SilentlyContinue
        # Windows 10 1803+ 自带 bsdtar，能直接解 .tar.gz
        tar -xzf $skillTgz -C $skillRoot
        if ($LASTEXITCODE -ne 0) { throw "skill 包解压失败（需要 Windows 10 1803+ 自带的 tar）" }
        Write-Host "  $SOK $Dm$skillRoot\$Skill$Rs"
        # 旧名残留：两个 skill 同时在册、description 几乎一样，AI 会乱挑
        $old = Join-Path $skillRoot 'ui-check'
        if (Test-Path $old) {
            Remove-Item $old -Recurse -Force
            Write-Host "  $SDOT 已清除旧版 ui-check（本 skill 已更名）"
        }
    } else {
        Write-Error "取不到 skill 包：$BaseUrl/skill/$Skill.tar.gz`n（若返回的是网页而非文件，多半是这个路径还没上传）"
        exit 1
    }

    # ── 2. tke 及同目录驱动 ──
    # 驱动必须与 tke 同目录：tke 只在自己所在目录找外部工具，不搜 PATH
    # （这样才能保证 chromedriver 与 Chrome 版本配对）
    New-Item -ItemType Directory -Path $TkeHome -Force | Out-Null
    Section 'DEPENDENCY'

    function Install-Bin {
        param([string]$Name, [bool]$Required)
        $gz = Join-Path $tmp "$Name.gz"
        if (Get-File -Url "$BaseUrl/bin/$platform/$Name.gz$q" -Out $gz -Kind 'gz') {
            # 分发源上统一不带 .exe，落地时补回来——否则 Windows 上执行不了
            $leaf = if ($Name -like '*.*') { $Name } else { "$Name.exe" }
            $dest = Join-Path $TkeHome $leaf
            # Windows 锁住**正在运行**的 exe：删不掉，但**可以改名**。
            # `tke update` 是 tke 自己拉起这个脚本的，那时 tke.exe 正在跑——
            # 不改名就会卡在这一步报"另一个程序正在使用此文件"。
            # 改开后原位就空出来了，旧文件下次安装时清掉（此刻还被占用，删不动）。
            Get-ChildItem -Path $TkeHome -Filter '*.old-*' -ErrorAction SilentlyContinue |
                Remove-Item -Force -ErrorAction SilentlyContinue
            if (Test-Path $dest) {
                try { Remove-Item $dest -Force -ErrorAction Stop }
                catch { Move-Item $dest "$dest.old-$(Get-Random)" -Force }
            }
            Expand-Gzip -In $gz -Out $dest
            Write-Host "  $SOK $leaf"
            return $true
        }
        if ($Required) {
            Write-Error "$Name 下载失败：$BaseUrl/bin/$platform/$Name.gz"
            exit 1
        }
        Write-Host "  $SDOT $Name $Dm(这个平台用不到)$Rs"
        return $false
    }

    Install-Bin -Name 'tke' -Required $true | Out-Null
    switch ($Profile) {
        'web'     { Install-Bin -Name 'chromedriver' -Required $true | Out-Null }
        'android' {
            Install-Bin -Name 'adb' -Required $true | Out-Null
            Install-Bin -Name 'aapt' -Required $false | Out-Null
            # adb.exe 直接依赖 AdbWinApi.dll，USB 还要 AdbWinUsbApi.dll（运行时加载）
            Install-Bin -Name 'AdbWinApi.dll' -Required $false | Out-Null
            Install-Bin -Name 'AdbWinUsbApi.dll' -Required $false | Out-Null
        }
        'ios'     { Install-Bin -Name 'go-ios' -Required $true | Out-Null }
        'all'     {
            Install-Bin -Name 'chromedriver' -Required $false | Out-Null
            Install-Bin -Name 'adb' -Required $false | Out-Null
            Install-Bin -Name 'aapt' -Required $false | Out-Null
            Install-Bin -Name 'AdbWinApi.dll' -Required $false | Out-Null
            Install-Bin -Name 'AdbWinUsbApi.dll' -Required $false | Out-Null
            # 32 位 Windows 没有 go-ios：上游只发布 64 位包
            if ($arch -ne '386') { Install-Bin -Name 'go-ios' -Required $false | Out-Null }
        }
    }

    # ── 3. Chrome for Testing（只有要测网页才需要）──
    if ($Profile -eq 'web' -or $Profile -eq 'all') {
        $chromePkg = if ($arch -eq '386') { 'chrome-win32' } else { 'chrome-win64' }
        $chromeDir = Join-Path $env:APPDATA 'tke'
        
        if (Test-Path (Join-Path $chromeDir $chromePkg)) {
            Write-Host "  $SOK $chromePkg ${Dm}已在 $chromeDir（换版本先删这个目录）$Rs"
        } else {
            $zip = Join-Path $tmp 'chrome.zip'
            # 不写"下载中（几百 MB）"——进度自己会说话，完成后这一行变成对钩。
            # PowerShell 的进度是顶部横幅（Write-Progress 的固有形式，Windows 上是惯例），
            # 做不成 bash 那种"接在名字后面"；为此手写整个下载循环不值得。
            $ProgressPreference = 'Continue'
            $chromeOk = Get-File -Url "$BaseUrl/chrome/$chromePkg.zip$q" -Out $zip -Kind 'zip'
            $ProgressPreference = 'SilentlyContinue'
            if ($chromeOk) {
                New-Item -ItemType Directory -Path $chromeDir -Force | Out-Null
                Expand-Archive -Path $zip -DestinationPath $chromeDir -Force
                Write-Host "  $SOK $chromePkg"
            } else {
                Write-Host "  $SWARN $chromePkg 下载失败，网页检查会用不了"
            }
        }
    }

    # ── 4. PATH（用户级环境变量，新开的终端生效）──
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -split ';' -contains $TkeHome) {
        Write-Host "  $SOK PATH 已就绪"
    } else {
        [Environment]::SetEnvironmentVariable('Path', "$userPath;$TkeHome", 'User')
        Write-Host "  $SOK PATH 已写入用户级$Dm（新窗口生效）$Rs"
    }
    $env:Path += ";$TkeHome"

    # ── 5. 体检 ──（结论要如实反映，别装完就说"好了"）
    $tkeExe = Join-Path $TkeHome 'tke.exe'
    # none 档不装驱动（安全测试只用 http/recon）——只验 tke 能跑，别拿设备 profile 体检
    if ($Profile -eq 'none') { & $tkeExe --version *> $null } else { & $tkeExe fix --check --profile $Profile }
    $health = $LASTEXITCODE

    Write-Host ""
    # 结论上面的体检已经说过了，这里只补一句它不会讲的：怎么用
    if ($health -eq 0) {
        Write-Host "  在 Claude Code 中输入 $Bd/$Skill$Rs 以调用"
    }
    Write-Host "  $Dim升级 tke update  ·  卸载 tke uninstall$Rs"
    if ($health -ne 0) { exit 1 }
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
