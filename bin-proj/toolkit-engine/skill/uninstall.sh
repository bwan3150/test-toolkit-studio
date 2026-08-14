#!/usr/bin/env bash
# tke-ui-test 卸载器（macOS / Linux）
#
#   curl -fsSL <BASE_URL>/uninstall.sh | bash              # 卸载 tke、驱动、skill
#   curl -fsSL <BASE_URL>/uninstall.sh | bash -s -- --logs # 连检查记录一起删
#   curl -fsSL <BASE_URL>/uninstall.sh | bash -s -- --all  # 连 Chrome for Testing 也删
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
DRY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --logs)    DEL_LOGS=1; shift ;;
        --chrome)  DEL_CHROME=1; shift ;;
        --all)     DEL_LOGS=1; DEL_CHROME=1; shift ;;
        --dry-run) DRY=1; shift ;;
        -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "未知参数: $1（可用: --logs / --chrome / --all / --dry-run）" >&2; exit 2 ;;
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
║                    U   N   I   N   S   T   A   L   L      ║
╚═══════════════════════════════════════════════════════════╝
LOGO
printf '%s' "$C_R"
[ "$DRY" = 1 ] && printf '  %s%s试运行：只列出会删什么，不真删%s\n' "$C_B" "$C_WARN" "$C_R"

# 删一个路径并报告（试运行只报告）
rm_path() {
    local path="$1" label="$2"
    if [ ! -e "$path" ]; then
        printf '  %s %s %s(不存在)%s\n' "$S_DOT" "$label" "$C_DIM" "$C_R"
        return
    fi
    local size
    size="$(du -sh "$path" 2>/dev/null | cut -f1)"
    if [ "$DRY" = 1 ]; then
        printf '  %s %s %s%s  %s%s\n' "$S_WARN" "$label" "$C_DIM" "$path" "${size:-?}" "$C_R"
        return
    fi
    rm -rf "$path"
    printf '  %s %s %s%s  %s%s\n' "$S_OK" "$label" "$C_DIM" "$path" "${size:-?}" "$C_R"
}

section "skill 文件"
# 用户级 + 当前项目级都清；旧名 ui-check 一并带走
for root in "$HOME/.claude/skills" "$PWD/.claude/skills"; do
    for name in tke-ui-test ui-check; do
        [ -e "$root/$name" ] && rm_path "$root/$name" "$name"
    done
done

section "tke 与驱动"
rm_path "$TKE_HOME" "tke 及同目录驱动"

section "PATH"
# 只删我们加的那一行，别动用户 rc 文件里的其它内容
CLEANED=0
for RC in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    [ -f "$RC" ] || continue
    grep -qF "$TKE_HOME" "$RC" 2>/dev/null || continue
    if [ "$DRY" = 1 ]; then
        printf '  %s 会从 %s 删掉含 %s 的那行\n' "$S_WARN" "$RC" "$TKE_HOME"
        CLEANED=1
        continue
    fi
    # 先备份再改——动别人的 rc 文件要留退路
    cp "$RC" "${RC}.tke-backup"
    grep -vF "$TKE_HOME" "${RC}.tke-backup" > "$RC"
    printf '  %s 已从 %s 移除 %s(备份 %s.tke-backup)%s\n' "$S_OK" "$RC" "$C_DIM" "$RC" "$C_R"
    CLEANED=1
done
[ "$CLEANED" = 0 ] && printf '  %s 没有找到 tke 的 PATH 行\n' "$S_DOT"

section "检查记录"
if [ "$DEL_LOGS" = 1 ]; then
    rm_path "$HOME/.tke/logs" "检查记录（截图/报告）"
else
    if [ -d "$HOME/.tke/logs" ]; then
        printf '  %s 保留 %s%s%s\n' "$S_DOT" "$C_DIM" "$HOME/.tke/logs" "$C_R"
        printf '    %s那是你跑过的证据；要删加 --logs%s\n' "$C_DIM" "$C_R"
    else
        printf '  %s 没有检查记录\n' "$S_DOT"
    fi
fi

section "Chrome for Testing"
if [ "$DEL_CHROME" = 1 ]; then
    for d in "$CHROME_DIR"/chrome-*; do
        [ -e "$d" ] && rm_path "$d" "$(basename "$d")"
    done
    # 目录空了就一并收掉
    rmdir "$CHROME_DIR" 2>/dev/null && printf '  %s 已清理 %s\n' "$S_OK" "$CHROME_DIR"
else
    if ls "$CHROME_DIR"/chrome-* >/dev/null 2>&1; then
        printf '  %s 保留 %s%s%s\n' "$S_DOT" "$C_DIM" "$CHROME_DIR" "$C_R"
        printf '    %s几百 MB，重装要重新下；要删加 --chrome 或 --all%s\n' "$C_DIM" "$C_R"
    else
        printf '  %s 没有安装 Chrome for Testing\n' "$S_DOT"
    fi
fi

printf '\n'
if [ "$DRY" = 1 ]; then
    printf '  %s%s 以上都没真删%s —— 去掉 --dry-run 才会动手\n' "$C_B" "$C_WARN" "$C_R"
else
    printf '  %s%s 卸载完成%s\n' "$C_B" "$C_OK" "$C_R"
    printf '    %s当前终端的 PATH 还留着旧值，重开一个即可%s\n' "$C_DIM" "$C_R"
fi
