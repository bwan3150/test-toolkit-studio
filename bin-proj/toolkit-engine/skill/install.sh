#!/usr/bin/env bash
# tke-ui-test skill 一键安装器
#
#   curl -fsSL <BASE_URL>/install.sh | bash
#   curl -fsSL <BASE_URL>/install.sh | bash -s -- --profile web --project
#
# 干三件事：装 skill 文件 → 装 tke 及同目录驱动 → 装 Chrome for Testing（web profile）。
# 全程幂等：重复跑只会覆盖同名文件，不会装重。
#
# 环境变量：
#   TKE_BASE_URL   分发根地址（默认见下方 DEFAULT_BASE_URL）
#   TKE_HOME       tke 及驱动的落点（默认 ~/.tke/bin）
#
# skill 默认装用户级（~/.claude/skills，所有项目通用）；--project 装到当前项目
# 的 .claude/skills（跟着仓库走，团队 clone 即得）。

set -uo pipefail

# ── 外观 ──（管道执行时 stdout 仍是终端，所以 -t 1 判得准；重定向到文件时自动关掉颜色）
if [ -t 1 ]; then
    C_TITLE=$'\033[38;5;39m'; C_OK=$'\033[38;5;42m'; C_WARN=$'\033[38;5;214m'
    C_ERR=$'\033[38;5;203m'; C_DIM=$'\033[38;5;245m'; C_B=$'\033[1m'; C_R=$'\033[0m'
else
    C_TITLE=; C_OK=; C_WARN=; C_ERR=; C_DIM=; C_B=; C_R=
fi
# 不用 emoji：等宽终端里对不齐，SSH/CI 日志里还常变成方块
S_OK="${C_OK}✓${C_R}"; S_WARN="${C_WARN}!${C_R}"; S_ERR="${C_ERR}✗${C_R}"; S_DOT="${C_DIM}·${C_R}"

banner() {
    printf '%s' "$C_TITLE"
    cat <<'LOGO'
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
LOGO
    printf '%s' "$C_R"
}
# 分节标题用前缀标记而不是补横线到固定宽——中文占两格，shell 里算显示宽度
# 既麻烦又不可靠（awk/wc 都要额外处理 UTF-8），前缀式永远对齐
section() { printf '\n%s%s▸ %s%s\n' "$C_B" "$C_TITLE" "$1" "$C_R"; }
kv()      { printf '  %s%-9s%s %s\n' "$C_DIM" "$1" "$C_R" "$2"; }

DEFAULT_BASE_URL="https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke"
BASE_URL="${TKE_BASE_URL:-$DEFAULT_BASE_URL}"
TKE_HOME="${TKE_HOME:-$HOME/.tke/bin}"
PROFILE="all"
SCOPE="user"      # user=~/.claude/skills(默认,所有项目通用) ；project=<当前目录>/.claude/skills

while [ $# -gt 0 ]; do
    case "$1" in
        --profile) PROFILE="${2:-all}"; shift 2 ;;
        --user)    SCOPE="user"; shift ;;
        --project) SCOPE="project"; shift ;;
        --base-url) BASE_URL="${2:-$BASE_URL}"; shift 2 ;;
        -h|--help)
            sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
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
    *)
        # Windows 还没有对应的安装脚本。可行路径：手工把 tke.exe 放进某个目录并加进
        # PATH，然后 `tke doctor --fix` 补齐驱动（它是 Rust 写的，三平台都能跑）。
        echo "❌ 这个脚本只支持 macOS / Linux。" >&2
        echo "   Windows：先手工放好 tke.exe 并加进 PATH，再跑 \`tke doctor --fix\` 补齐依赖。" >&2
        exit 1 ;;
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
# 第 4 个参数给 "bar" 时显示进度条——Chrome 有几百 MB，静默下载会让人以为卡死了
# fetch 失败时的具体原因（curl 那句），由调用方决定要不要展示
FETCH_ERR=""

fetch() {
    local url="$1" out="$2" kind="${3:-any}" show="${4:-}" label="${5:-}"
    FETCH_ERR=""
    if [ "$show" = "bar" ] && [ -t 1 ]; then
        # 进度条**接在名字后面**，而不是自己单占一行。
        # curl 的 `-#` 走 stderr、用 \r 原地刷新；把 \r 拆成行、逐帧重画整行，
        # 就能把「· 名字」和进度条拼在同一行上（\033[K 清掉上一帧的残尾）。
        # PIPESTATUS 取的是 curl 的退出码——管道最后一个是 while，它总是成功。
        # 先把名字打出来：建连接/握手期间 curl 一个字节都不输出，
        # 不先占位的话人要盯着**空白**等好几秒，才是真正的"慢"
        printf '  %s %s ' "$S_DOT" "$label"
        # tee 留一份：curl 的**报错也走 stderr**，和进度帧混在一起。
        # 失败时进度条会被擦掉，不留一份就等于把失败原因也擦了（INV-9）
        #
        # ⚠️ 这里**不能用 `tr '\r' '\n'` 转一道**：tr 输出到管道时是**块缓冲**，
        # 要攒够 4KB 进度帧才吐给下游，于是进度条迟迟不出现、出现后又一次跳好几帧。
        # 改用 bash 内建的 `read -d`（按 \r 切帧），管道里就没有会缓冲的外部命令了。
        curl -fL# --retry 2 --max-time 1800 "$url" -o "$out" 2>&1 >/dev/null \
            | tee "$out.log" \
            | while IFS= read -r -d $'\r' frame; do
                  [ -n "$frame" ] && printf '\r  %s %s %s\033[K' "$S_DOT" "$label" "$frame"
              done
        local rc="${PIPESTATUS[0]}"
        printf '\r\033[K'   # 擦掉进度条：成功让调用方原地打对钩，失败下面把原因摆出来
        if [ "$rc" != "0" ]; then
            # 不锚行首：curl 把报错**追加在进度条同一行的右侧**，前面是进度条的空白。
            # 只**存**不打——打印交给调用方，否则和它那句"下载失败"重复成两行
            FETCH_ERR="$(tr '\r' '\n' < "$out.log" 2>/dev/null | grep -m1 -o 'curl: .*')"
            rm -f "$out" "$out.log"
            return 1
        fi
        rm -f "$out.log"
    else
        curl -fsSL --retry 2 --max-time 900 "$url" -o "$out" || { rm -f "$out"; return 1; }
    fi
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

banner
printf '  %s%s%s %s·%s %s %s·%s %s\n' \
    "$C_B" "$([ -n "$REMOTE_VERSION" ] && printf '%s' "$REMOTE_VERSION" | head -1 || echo 'tke')" "$C_R" \
    "$C_DIM" "$C_R" "$PLATFORM" "$C_DIM" "$C_R" "$PROFILE"

# —— 1. skill 文件 ——
if [ "$SCOPE" = "user" ]; then
    SKILL_ROOT="$HOME/.claude/skills"
else
    SKILL_ROOT="$PWD/.claude/skills"
fi
mkdir -p "$SKILL_ROOT"
section "SKILL"
if fetch "$BASE_URL/skill/tke-ui-test.tar.gz$Q" "$TMP/skill.tar.gz" gz; then
    rm -rf "$SKILL_ROOT/tke-ui-test"
    tar -xzf "$TMP/skill.tar.gz" -C "$SKILL_ROOT" || { printf '  %s skill 包解压失败\n' "$S_ERR" >&2; exit 1; }
    printf '  %s %s%s%s\n' "$S_OK" "$C_DIM" "$SKILL_ROOT/tke-ui-test" "$C_R"
    # 旧名 ui-check 的残留：不清掉的话两个 skill 同时在册,description 几乎一样,
    # AI 会在两者间乱挑,用户也看不出该用哪个。改名是 2026-08-13。
    if [ -d "${SKILL_ROOT}/ui-check" ]; then
        rm -rf "${SKILL_ROOT}/ui-check"
        printf '  %s 已清除旧版 ui-check（本 skill 已更名）\n' "$S_DOT"
    fi
else
    printf '  %s 取不到 skill 包：%s\n' "$S_ERR" "$BASE_URL/skill/tke-ui-test.tar.gz" >&2
    printf '    %s（若返回的是网页而非文件，多半是这个路径还没上传）%s\n' "$C_DIM" "$C_R" >&2
    exit 1
fi

# —— 2. tke 及同目录驱动 ——
# 驱动必须与 tke 同目录：tke 只在自己所在目录找外部工具，不搜 PATH
# （这样才能保证 chromedriver 与 Chrome 版本配对）
mkdir -p "$TKE_HOME"
section "DEPENDENCY"

install_bin() {
    local name="$1" required="$2"
    local url="$BASE_URL/bin/$PLATFORM/$name.gz$Q"
    if fetch "$url" "$TMP/$name.gz" gz; then
        gunzip -f "$TMP/$name.gz" || { printf '  %s %s 解压失败（文件损坏？）\n' "$S_ERR" "$name" >&2; return 1; }
        # 先删后拷：覆盖运行中的二进制会 ETXTBSY(Linux) / 签名失配被杀(macOS)
        rm -f "$TKE_HOME/$name"
        mv "$TMP/$name" "$TKE_HOME/$name"
        chmod +x "$TKE_HOME/$name"
        [ "$OS" = "Darwin" ] && xattr -d com.apple.quarantine "$TKE_HOME/$name" 2>/dev/null
        printf '  %s %s\n' "$S_OK" "$name"
        return 0
    fi
    if [ "$required" = "yes" ]; then
        printf '  %s %s 下载失败：%s\n' "$S_ERR" "$name" "$url" >&2
        return 1
    fi
    printf '  %s %s %s(这个平台用不到)%s\n' "$S_DOT" "$name" "$C_DIM" "$C_R"
    return 0
}

install_bin tke yes || exit 1
case "$PROFILE" in
    web)     install_bin chromedriver yes || exit 1 ;;
    android) install_bin adb yes || exit 1; install_bin aapt no
             # libc++.so 只有 **Linux 版 aapt** 需要（其 RUNPATH 含 $ORIGIN）。
             # 无条件装的话，mac/Windows 上会去请求一个根本不存在的文件、拿到 404
             [ "$OS" = "Linux" ] && install_bin libc++.so no ;;
    ios)     install_bin go-ios yes || exit 1 ;;
    all)     install_bin chromedriver no; install_bin adb no; install_bin aapt no
             [ "$OS" = "Linux" ] && install_bin libc++.so no
             install_bin go-ios no ;;
esac

# —— 3. Chrome for Testing（只有要测网页才需要）——
if [ "$PROFILE" = "web" ] || [ "$PROFILE" = "all" ]; then
    if [ -d "$CHROME_DIR/$CHROME_PKG" ]; then
        printf '  %s %s\n' "$S_OK" "$CHROME_PKG"
    elif fetch "$BASE_URL/chrome/$CHROME_PKG.zip$Q" "$TMP/chrome.zip" zip bar "$CHROME_PKG"; then
        mkdir -p "$CHROME_DIR"
        unzip -q -o "$TMP/chrome.zip" -d "$CHROME_DIR"
        # macOS：清隔离属性，否则自动化下会卡在授权弹窗且无任何报错
        [ "$OS" = "Darwin" ] && xattr -cr "$CHROME_DIR/$CHROME_PKG" 2>/dev/null
        printf '  %s %s\n' "$S_OK" "$CHROME_PKG"
    else
        printf '  %s %s 下载失败，网页检查会用不了\n' "$S_WARN" "$CHROME_PKG"
        [ -n "$FETCH_ERR" ] && printf '      %s%s%s\n' "$C_DIM" "$FETCH_ERR" "$C_R"
    fi
fi

# —— 4. PATH ——
if command -v tke >/dev/null 2>&1 && [ "$(command -v tke)" = "$TKE_HOME/tke" ]; then
    printf '  %s PATH 已就绪\n' "$S_OK"
else
    case "${SHELL:-}" in
        */zsh)  RC="$HOME/.zshrc" ;;
        */bash) RC="$HOME/.bashrc" ;;
        *)      RC="" ;;
    esac
    LINE="export PATH=\"$TKE_HOME:\$PATH\""
    if [ -n "$RC" ] && ! grep -qF "$TKE_HOME" "$RC" 2>/dev/null; then
        echo "$LINE" >> "$RC"
        printf '  %s PATH 已写入 %s%s%s（新终端生效）\n' "$S_OK" "$C_DIM" "$RC" "$C_R"
    else
        printf '  %s 请自行加进 PATH：%s%s%s\n' "$S_WARN" "$C_B" "$LINE" "$C_R"
    fi
fi

# —— 5. 体检 ——（结论要如实反映，别装完就说"好了"）
export PATH="$TKE_HOME:$PATH"
HEALTH=0
# 用别名 `fix --check` 而不是 `doctor`：这一步跑的是**刚下下来那个** tke，
# 万一分发源上还是旧版（没有 doctor 子命令），用新名字会直接报"命令不存在"。
# 别名永久保留，新旧二进制都认（ADR-0014）。
"$TKE_HOME/tke" fix --check --profile "$PROFILE" || HEALTH=1

printf '\n'
if [ "$HEALTH" = "0" ]; then
    printf '  %s%s全局已就绪%s，在 Claude Code 中输入 %s/tke-ui-test%s 以调用\n' \
        "$C_B" "$C_OK" "$C_R" "$C_B" "$C_R"
else
    # 如实反映：文件装好了不等于能用（INV-9）
    printf '  %s%s环境还不完整%s  补齐：%stke doctor --fix%s\n' "$C_B" "$C_WARN" "$C_R" "$C_B" "$C_R"
fi
printf '  %s升级 tke update  ·  卸载 tke uninstall%s\n' "$C_DIM" "$C_R"
[ "$HEALTH" = "0" ] || exit 1
