#!/usr/bin/env bash
# ui-check skill 一键安装器
#
#   curl -fsSL <BASE_URL>/install.sh | bash
#   curl -fsSL <BASE_URL>/install.sh | bash -s -- --profile web --user
#
# 干三件事：装 skill 文件 → 装 tke 及同目录驱动 → 装 Chrome for Testing（web profile）。
# 全程幂等：重复跑只会覆盖同名文件，不会装重。
#
# 环境变量：
#   TKE_BASE_URL   分发根地址（默认见下方 DEFAULT_BASE_URL）
#   TKE_HOME       tke 及驱动的落点（默认 ~/.tke/bin）

set -uo pipefail

DEFAULT_BASE_URL="https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke"
BASE_URL="${TKE_BASE_URL:-$DEFAULT_BASE_URL}"
TKE_HOME="${TKE_HOME:-$HOME/.tke/bin}"
PROFILE="all"
SCOPE="project"   # project=<当前目录>/.claude/skills ；user=~/.claude/skills

while [ $# -gt 0 ]; do
    case "$1" in
        --profile) PROFILE="${2:-all}"; shift 2 ;;
        --user)    SCOPE="user"; shift ;;
        --project) SCOPE="project"; shift ;;
        --base-url) BASE_URL="${2:-$BASE_URL}"; shift 2 ;;
        -h|--help)
            sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) echo "未知参数: $1（可用: --profile web|android|ios|all / --user / --project / --base-url）" >&2; exit 2 ;;
    esac
done

case "$PROFILE" in
    web|android|ios|all) ;;
    *) echo "❌ --profile 只能是 web / android / ios / all" >&2; exit 2 ;;
esac

# —— 平台探测 ——（与 bin/<platform>/ 目录命名一致）
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$ARCH" in
    arm64|aarch64) ARCH_NAME="arm64" ;;
    x86_64|amd64)  ARCH_NAME="amd64" ;;
    *) echo "❌ 不支持的架构: $ARCH" >&2; exit 1 ;;
esac
case "$OS" in
    Darwin) PLATFORM="darwin-$ARCH_NAME"
            CHROME_PKG="chrome-mac-$([ "$ARCH_NAME" = arm64 ] && echo arm64 || echo x64)"
            CHROME_DIR="$HOME/Library/Application Support/tke" ;;
    Linux)  PLATFORM="linux-$ARCH_NAME"
            CHROME_PKG="chrome-linux64"
            CHROME_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/tke" ;;
    *) echo "❌ 这个脚本只支持 macOS / Linux；Windows 请用 install.ps1" >&2; exit 1 ;;
esac

need() { command -v "$1" >/dev/null 2>&1 || { echo "❌ 缺少 $1，请先安装" >&2; exit 1; }; }
need curl
need unzip

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# 文件头校验：**存储平台对不存在的路径会回落 200 + 一段 HTML**（SPA 兜底），
# 所以 curl 的 -f 完全拦不住——不验内容就会把网页当成二进制装进去。
magic_ok() {
    local f="$1" kind="$2"
    [ -s "$f" ] || return 1
    case "$kind" in
        gz)   [ "$(head -c2 "$f" | od -An -tx1 | tr -d ' \n')" = "1f8b" ] ;;
        zip)  [ "$(head -c2 "$f")" = "PK" ] ;;
        json) head -c1 "$f" | grep -q '{' ;;
        *)    return 0 ;;
    esac
}

# 下载 + 按类型验内容；不合格一律当失败（并删掉半成品）
fetch() {
    local url="$1" out="$2" kind="${3:-any}"
    curl -fsSL --retry 2 --max-time 900 "$url" -o "$out" || { rm -f "$out"; return 1; }
    if ! magic_ok "$out" "$kind"; then
        rm -f "$out"
        return 1
    fi
    return 0
}

# —— 缓存键 ——
# 分发走 Cloudflare：max-age 4h，且**不认 Cache-Control: no-cache 请求头**，
# 唯一可靠的破缓存手段是变化的查询参数。
# 先用随机数取一次 VERSION（它必须是最新的），再用里面的 build 戳作为后续所有下载的键：
# 发布过新版 → 戳变 → 自然拿到新文件；没发新版 → 戳不变 → 照常命中 CDN 缓存。
CACHE_BUST="?t=$$-$(od -An -N4 -tu4 </dev/urandom 2>/dev/null | tr -d ' ' || echo 0)"
REMOTE_VERSION="$(curl -fsSL --max-time 20 "$BASE_URL/VERSION$CACHE_BUST" 2>/dev/null)"
BUILD_KEY="$(printf '%s' "$REMOTE_VERSION" | sed -n 's/^build: *//p' | head -1)"
if [ -n "$BUILD_KEY" ]; then
    Q="?b=$BUILD_KEY"
else
    # 分发源没有 build 戳（老版本布局）→ 退回随机数，宁可不走缓存也不装到旧文件
    Q="$CACHE_BUST"
fi

echo "== ui-check skill 安装 =="
echo "   来源     $BASE_URL"
[ -n "$REMOTE_VERSION" ] && echo "   版本     $(printf '%s' "$REMOTE_VERSION" | head -1)"
echo "   平台     $PLATFORM"
echo "   profile  $PROFILE"

# —— 1. skill 文件 ——
if [ "$SCOPE" = "user" ]; then
    SKILL_ROOT="$HOME/.claude/skills"
else
    SKILL_ROOT="$PWD/.claude/skills"
fi
mkdir -p "$SKILL_ROOT"
echo
echo "-- skill 文件 → $SKILL_ROOT/ui-check"
if fetch "$BASE_URL/skill/ui-check.tar.gz$Q" "$TMP/skill.tar.gz" gz; then
    rm -rf "$SKILL_ROOT/ui-check"
    tar -xzf "$TMP/skill.tar.gz" -C "$SKILL_ROOT" || { echo "   ❌ skill 包解压失败" >&2; exit 1; }
    echo "   ✅ 已安装"
else
    echo "   ❌ 取不到 skill 包：$BASE_URL/skill/ui-check.tar.gz" >&2
    echo "      （若返回的是网页而非文件，多半是这个路径还没上传）" >&2
    exit 1
fi

# —— 2. tke 及同目录驱动 ——
# 驱动必须与 tke 同目录：tke 只在自己所在目录找外部工具，不搜 PATH
# （这样才能保证 chromedriver 与 Chrome 版本配对）
mkdir -p "$TKE_HOME"
echo
echo "-- tke 及驱动 → $TKE_HOME"

install_bin() {
    local name="$1" required="$2"
    local url="$BASE_URL/bin/$PLATFORM/$name.gz$Q"
    if fetch "$url" "$TMP/$name.gz" gz; then
        gunzip -f "$TMP/$name.gz" || { echo "   ❌ $name 解压失败（文件损坏？）" >&2; return 1; }
        # 先删后拷：覆盖运行中的二进制会 ETXTBSY(Linux) / 签名失配被杀(macOS)
        rm -f "$TKE_HOME/$name"
        mv "$TMP/$name" "$TKE_HOME/$name"
        chmod +x "$TKE_HOME/$name"
        [ "$OS" = "Darwin" ] && xattr -d com.apple.quarantine "$TKE_HOME/$name" 2>/dev/null
        echo "   ✅ $name"
        return 0
    fi
    if [ "$required" = "yes" ]; then
        echo "   ❌ $name 下载失败：$url" >&2
        return 1
    fi
    echo "   ⚠️  $name 跳过（$PROFILE 用不到或源上没有）"
    return 0
}

install_bin tke yes || exit 1
case "$PROFILE" in
    web)     install_bin chromedriver yes || exit 1 ;;
    android) install_bin adb yes || exit 1; install_bin aapt no; install_bin libc++.so no ;;
    ios)     install_bin go-ios yes || exit 1 ;;
    all)     install_bin chromedriver no; install_bin adb no; install_bin aapt no; install_bin libc++.so no; install_bin go-ios no ;;
esac

# —— 3. Chrome for Testing（只有要测网页才需要）——
if [ "$PROFILE" = "web" ] || [ "$PROFILE" = "all" ]; then
    echo
    echo "-- Chrome for Testing → $CHROME_DIR/$CHROME_PKG"
    if [ -d "$CHROME_DIR/$CHROME_PKG" ]; then
        echo "   ✅ 已存在，跳过（要换版本先删掉这个目录）"
    elif fetch "$BASE_URL/chrome/$CHROME_PKG.zip$Q" "$TMP/chrome.zip" zip; then
        mkdir -p "$CHROME_DIR"
        unzip -q -o "$TMP/chrome.zip" -d "$CHROME_DIR"
        # macOS：清隔离属性，否则自动化下会卡在授权弹窗且无任何报错
        [ "$OS" = "Darwin" ] && xattr -cr "$CHROME_DIR/$CHROME_PKG" 2>/dev/null
        echo "   ✅ 已安装"
    else
        echo "   ⚠️  下载失败，网页检查会用不了：$BASE_URL/chrome/$CHROME_PKG.zip"
        echo "      也可手动装，见 skill 里的 README"
    fi
fi

# —— 4. PATH ——
echo
if command -v tke >/dev/null 2>&1 && [ "$(command -v tke)" = "$TKE_HOME/tke" ]; then
    echo "-- PATH 已就绪"
else
    case "${SHELL:-}" in
        */zsh)  RC="$HOME/.zshrc" ;;
        */bash) RC="$HOME/.bashrc" ;;
        *)      RC="" ;;
    esac
    LINE="export PATH=\"$TKE_HOME:\$PATH\""
    if [ -n "$RC" ] && ! grep -qF "$TKE_HOME" "$RC" 2>/dev/null; then
        echo "$LINE" >> "$RC"
        echo "-- 已把 tke 加进 PATH（写入 $RC）"
        echo "   当前终端请先执行：$LINE"
    else
        echo "-- 请把 tke 加进 PATH："
        echo "   $LINE"
    fi
fi

# —— 5. 体检 ——（结论要如实反映，别装完就说"好了"）
echo
export PATH="$TKE_HOME:$PATH"
HEALTH=0
if [ -x "$SKILL_ROOT/ui-check/scripts/check-env.sh" ]; then
    bash "$SKILL_ROOT/ui-check/scripts/check-env.sh" || HEALTH=1
fi

echo
if [ "$HEALTH" = "0" ]; then
    echo "装好了。在 Claude Code 里直接提需求即可，例如："
    echo "  「我刚改完设置页的保存按钮，帮我在浏览器上验一下真的能存」"
else
    echo "⚠️  文件都装好了，但**环境还不完整**（见上面 ❌）——现在还跑不了检查。"
    echo "   补齐缺的那几项后，重跑体检确认："
    echo "   bash $SKILL_ROOT/ui-check/scripts/check-env.sh"
    exit 1
fi
