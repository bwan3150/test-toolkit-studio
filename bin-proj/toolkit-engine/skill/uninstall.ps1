<#
.SYNOPSIS
  tke-ui-test 卸载器（Windows）

.DESCRIPTION
  与 uninstall.sh 一一对应。

    irm https://<BASE_URL>/uninstall.ps1 | iex
    # 带参数时先落地再跑：
    iwr https://<BASE_URL>/uninstall.ps1 -OutFile u.ps1; .\u.ps1 -Logs
    .\u.ps1 -All        # 连 Chrome for Testing 也删
    .\u.ps1 -DryRun     # 只看会删什么

  默认**不删**两样（删了很难回来）：检查记录 %USERPROFILE%\.tke\logs（跑过的证据），
  Chrome for Testing（几百 MB，重装要重新下）。要删就显式说：-Logs / -Chrome / -All。
#>
[CmdletBinding()]
param(
    [switch]$Logs,
    [switch]$Chrome,
    [switch]$All,
    [switch]$DryRun,
    [string]$TkeHome = $env:TKE_HOME
)

$ErrorActionPreference = 'Stop'
if ($All) { $Logs = $true; $Chrome = $true }
if (-not $TkeHome) { $TkeHome = Join-Path $env:USERPROFILE '.tke\bin' }

# ── 外观 ──（与 install.ps1 同一套；PowerShell 5.1 不认 $PSStyle，用原始转义序列）
# ⚠️ 两个 PowerShell 的标识符坑，都是实测踩出来的：
#    1) **变量名不区分大小写**：`$T`(颜色) 会被函数参数 `$t` 覆盖，症状是标题打两遍；
#       局部 `$logs` 会覆盖 switch 参数 `$Logs`，赋值时直接报类型转换失败
#    2) **变量名可以包含中文**：`$Ye试运行` 被当成一个变量名，那三个字就没了——
#       必须写 `${Ye}试运行`。**这与 bash 的 P-20 是同一类坑**
$ES = [char]27
$Cy = "$ES[38;5;39m"; $Gn = "$ES[38;5;42m"; $Ye = "$ES[38;5;214m"
$Dm = "$ES[38;5;245m"; $Bd = "$ES[1m"; $Rs = "$ES[0m"
function Section([string]$Text) { Write-Host "`n$Bd$Cy▸ $Text$Rs" }

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
║                    U   N   I   N   S   T   A   L   L      ║
╚═══════════════════════════════════════════════════════════╝
'@ | Write-Host
Write-Host $Rs -NoNewline
if ($DryRun) { Write-Host "  $Bd${Ye}试运行：只列出会删什么，不真删$Rs" }

function RemovePath {
    param([string]$Path, [string]$Label)
    if (-not (Test-Path $Path)) {
        Write-Host "  $Dm·$Rs $Label $Dm(不存在)$Rs"
        return
    }
    $size = ''
    try {
        $bytes = (Get-ChildItem $Path -Recurse -File -ErrorAction SilentlyContinue |
                  Measure-Object -Property Length -Sum).Sum
        if ($bytes) { $size = '{0:N1} MB' -f ($bytes / 1MB) }
    } catch { }
    if ($DryRun) {
        Write-Host "  $Ye!$Rs $Label $Dm$Path  $size$Rs"
        return
    }
    Remove-Item $Path -Recurse -Force
    Write-Host "  $Gn✓$Rs $Label $Dm$Path  $size$Rs"
}

Section 'skill 文件'
foreach ($root in @((Join-Path $env:USERPROFILE '.claude\skills'), (Join-Path (Get-Location) '.claude\skills'))) {
    foreach ($name in @('tke-ui-test', 'ui-check')) {   # 旧名一并带走
        $p = Join-Path $root $name
        if (Test-Path $p) { RemovePath -Path $p -Label $name }
    }
}

Section 'tke 与驱动'
RemovePath -Path $TkeHome -Label 'tke 及同目录驱动'

Section 'PATH'
# 只摘掉我们加的那一段，别动用户 PATH 里的其它内容
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -and ($userPath -split ';' -contains $TkeHome)) {
    if ($DryRun) {
        Write-Host "  $Ye!$Rs 会从用户级 PATH 移除 $TkeHome"
    } else {
        $kept = ($userPath -split ';' | Where-Object { $_ -and $_ -ne $TkeHome }) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $kept, 'User')
        Write-Host "  $Gn✓$Rs 已从用户级 PATH 移除 $TkeHome"
    }
} else {
    Write-Host "  $Dm·$Rs 用户级 PATH 里没有 tke"
}

Section '检查记录'
$logsDir = Join-Path $env:USERPROFILE '.tke\logs'
if ($Logs) {
    RemovePath -Path $logsDir -Label '检查记录（截图/报告）'
} elseif (Test-Path $logsDir) {
    Write-Host "  $Dm·$Rs 保留 $Dm${logsDir}$Rs"
    Write-Host "    ${Dm}那是你跑过的证据；要删加 -Logs$Rs"
} else {
    Write-Host "  $Dm·$Rs 没有检查记录"
}

Section 'Chrome for Testing'
$chromeDir = Join-Path $env:APPDATA 'tke'
$pkgs = @()
if (Test-Path $chromeDir) { $pkgs = Get-ChildItem $chromeDir -Directory -Filter 'chrome-*' -ErrorAction SilentlyContinue }
if ($Chrome) {
    foreach ($p in $pkgs) { RemovePath -Path $p.FullName -Label $p.Name }
    if ((Test-Path $chromeDir) -and -not (Get-ChildItem $chromeDir -Force -ErrorAction SilentlyContinue)) {
        if (-not $DryRun) { Remove-Item $chromeDir -Force; Write-Host "  $Gn✓$Rs 已清理 $chromeDir" }
    }
} elseif ($pkgs) {
    Write-Host "  $Dm·$Rs 保留 $Dm$chromeDir$Rs"
    Write-Host "    ${Dm}几百 MB，重装要重新下；要删加 -Chrome 或 -All$Rs"
} else {
    Write-Host "  $Dm·$Rs 没有安装 Chrome for Testing"
}

Write-Host ''
if ($DryRun) {
    Write-Host "  $Bd${Ye}以上都没真删$Rs —— 去掉 -DryRun 才会动手"
} else {
    Write-Host "  $Bd${Gn}卸载完成$Rs"
    Write-Host "    ${Dm}当前窗口的 PATH 还留着旧值，重开一个即可$Rs"
}
