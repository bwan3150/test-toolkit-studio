#!/usr/bin/env bash
# tke-ui-test 卸载器（macOS / Linux）
#
#   curl -fsSL <BASE_URL>/uninstall.sh | bash              # 卸载 tke、驱动、skill
#   curl -fsSL <BASE_URL>/uninstall.sh | bash -s -- --logs # 连检查记录一起删
#   curl -fsSL <BASE_URL>/uninstall.sh | bash -s -- --all  # 连 Chrome 与安卓模拟器也删
#   ... -s -- --dry-run                                    # 只看会删什么，不动手
#
# 默认**不删**两样东西，因为删了很难回来：
#   - 检查记录 ~/.tke/logs   —— 那是你跑过的证据（截图/报告），删了就没了
#   - Chrome for Testing     —— 几百 MB，重装要重新下
# 要删就显式说：--logs / --chrome / --all。

set -uo pipefail

# ── 外观 ──（与 install.sh 同一套；重定向到文件时自动关颜色）
if [ -t 1 ]; then
    C_TITLE=$'\033[38;5;39m'; C_OK=$'\033[38;5;42m'; C_WARN=$'\033[38;5;214m'
    C_ERR=$'\033[38;5;203m'; C_DIM=$'\033[38;5;245m'; C_B=$'\033[1m'; C_R=$'\033[0m'
else
    C_TITLE=; C_OK=; C_WARN=; C_ERR=; C_DIM=; C_B=; C_R=
fi
S_OK="${C_OK}✓${C_R}"; S_WARN="${C_WARN}!${C_R}"; S_DOT="${C_DIM}·${C_R}"
section() { printf '\n%s%s▸ %s%s\n' "$C_B" "$C_TITLE" "$1" "$C_R"; }

DEL_LOGS=0
DEL_CHROME=0
DEL_ANDROID=0
DRY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --logs)    DEL_LOGS=1; shift ;;
        --chrome)  DEL_CHROME=1; shift ;;
        # 安卓模拟器是选装的大件(约 1GB),跟 Chrome 一样默认留着
        --android) DEL_ANDROID=1; shift ;;
        --all)     DEL_LOGS=1; DEL_CHROME=1; DEL_ANDROID=1; shift ;;
        --dry-run) DRY=1; shift ;;
        -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "未知参数: $1（可用: --logs / --chrome / --android / --all / --dry-run）" >&2; exit 2 ;;
    esac
done

TKE_HOME="${TKE_HOME:-$HOME/.tke/bin}"
case "$(uname -s)" in
    Darwin) CHROME_DIR="$HOME/Library/Application Support/tke" ;;
    Linux)  CHROME_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/tke" ;;
    *)      CHROME_DIR="$HOME/.local/share/tke" ;;
esac

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


# 删一个路径并报告（试运行只报告）
rm_path() {
    local path="$1" label="$2"
    # 不存在就默默跳过：列一堆"没有这个""没有那个"是噪音，不是信息
    [ -e "$path" ] || return
    local size
    size="$(du -sh "$path" 2>/dev/null | cut -f1)"
    if [ "$DRY" = 1 ]; then
        printf '  %s %s %s%s  %s%s\n' "$S_WARN" "$label" "$C_DIM" "$path" "${size:-?}" "$C_R"
        return
    fi
    rm -rf "$path"
    printf '  %s %s %s%s  %s%s\n' "$S_OK" "$label" "$C_DIM" "$path" "${size:-?}" "$C_R"
}

section "$([ "$DRY" = 1 ] && echo 'DRY RUN' || echo 'REMOVED')"
# 用户级 + 当前项目级都清；旧名 ui-check 一并带走
for root in "$HOME/.claude/skills" "$PWD/.claude/skills"; do
    for name in tke-ui-test tke-security-test ui-check; do
        rm_path "$root/$name" "skill     "
    done
done
rm_path "$TKE_HOME" "dependency"

# 只删我们加的那一行，别动用户 rc 文件里的其它内容
for RC in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    [ -f "$RC" ] || continue
    grep -qF "$TKE_HOME" "$RC" 2>/dev/null || continue
    if [ "$DRY" = 1 ]; then
        printf '  %s path      %s%s%s\n' "$S_WARN" "$C_DIM" "$RC" "$C_R"
        continue
    fi
    # 先备份再改——动别人的 rc 文件要留退路
    cp "$RC" "${RC}.tke-backup"
    grep -vF "$TKE_HOME" "${RC}.tke-backup" > "$RC"
    printf '  %s path      %s%s%s\n' "$S_OK" "$C_DIM" "$RC" "$C_R"
done

[ "$DEL_LOGS" = 1 ] && rm_path "$HOME/.tke/logs" "logs      "
# iOS 模拟器用的 WebDriverAgent（21MB）。跟 tke 一起装的,就跟 tke 一起删——
# 留着它既占地方又会让下次 doctor 误判成"已装"
rm_path "$HOME/.tke/wda" "wda       "
# 安卓模拟器(选装,约 1GB)。**只删我们自己装的那份**——用户已有的 ~/Android/Sdk 不碰
[ "$DEL_ANDROID" = 1 ] && rm_path "$HOME/.tke/android-sdk" "android   "
if [ "$DEL_CHROME" = 1 ]; then
    for d in "$CHROME_DIR"/chrome-*; do
        rm_path "$d" "chrome    "
    done
    rmdir "$CHROME_DIR" 2>/dev/null
fi

# 保留了什么只用一句话带过——没删的东西不值得各占一节
KEPT=""; KEPT_FLAGS=""
[ "$DEL_LOGS" = 0 ] && [ -d "$HOME/.tke/logs" ] && { KEPT="日志 $HOME/.tke/logs"; KEPT_FLAGS="--logs"; }
if [ "$DEL_CHROME" = 0 ] && ls "$CHROME_DIR"/chrome-* >/dev/null 2>&1; then
    [ -n "$KEPT" ] && { KEPT="$KEPT · Chrome"; KEPT_FLAGS="$KEPT_FLAGS / --chrome"; } \
                   || { KEPT="Chrome"; KEPT_FLAGS="--chrome"; }
fi
if [ "$DEL_ANDROID" = 0 ] && [ -d "$HOME/.tke/android-sdk" ]; then
    [ -n "$KEPT" ] && { KEPT="$KEPT · 安卓模拟器"; KEPT_FLAGS="$KEPT_FLAGS / --android"; } \
                   || { KEPT="安卓模拟器"; KEPT_FLAGS="--android"; }
fi

printf '\n'
if [ "$DRY" = 1 ]; then
    printf '  %s%s试运行%s  以上都没真删，去掉 --dry-run 才动手\n' "$C_B" "$C_WARN" "$C_R"
else
    printf '  %s%s卸载完成%s  新终端生效\n' "$C_B" "$C_OK" "$C_R"
fi
# 说清"留了什么"**和"怎么删"**——原来只写 `检查记录(--logs)`，
# 那个括号想表达"加这个参数才会删",但没人看得出来
if [ -n "$KEPT" ]; then
    printf '  %s已保留  %s%s\n' "$C_DIM" "$KEPT" "$C_R"
    printf '  %s        重跑并加 %s 可一并删除（--all 全删）%s\n' "$C_DIM" "$KEPT_FLAGS" "$C_R"
fi
