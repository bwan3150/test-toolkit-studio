#!/usr/bin/env bash
# ui-test skill 前置体检：缺什么直接说清楚，别让调用方撞进去猜。
# 退出码：0=可以跑；1=有硬缺失。

set -uo pipefail

ok=0
fail=0
say_ok()   { echo "  ✅ $*"; ok=$((ok+1)); }
say_bad()  { echo "  ❌ $*"; fail=$((fail+1)); }
say_warn() { echo "  ⚠️  $*"; }

echo "== tke 本体 =="
if command -v tke >/dev/null 2>&1; then
    TKE="$(command -v tke)"
    say_ok "tke: $TKE ($(tke --version 2>/dev/null || echo '版本未知'))"
else
    say_bad "tke 不在 PATH。构建: bin-proj/toolkit-engine/build-{linux,mac}.sh，然后把 bin/<platform>/ 加进 PATH"
    echo; echo "结论：不满足，先装 tke。"; exit 1
fi
TKE_DIR="$(dirname "$(readlink -f "$TKE")")"

echo "== Web 依赖 =="
# chromedriver 必须与 tke 同目录：ToolManager 只搜 tke 所在目录，不回退 PATH（版本配对靠这个约束）
if [ -x "$TKE_DIR/chromedriver" ] || [ -x "$TKE_DIR/chromedriver.exe" ]; then
    say_ok "chromedriver: $("$TKE_DIR/chromedriver" --version 2>/dev/null | head -1)"
else
    say_bad "chromedriver 不在 tke 同目录（$TKE_DIR）——tke 只在这里找，不搜 PATH"
fi

# Chrome for Testing：tke 同目录 或 用户数据目录，按官方 zip 原样结构
case "$(uname)" in
    Darwin) REL="chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
            DATA="$HOME/Library/Application Support/tke" ;;
    Linux)  REL="chrome-linux64/chrome"
            DATA="${XDG_DATA_HOME:-$HOME/.local/share}/tke" ;;
    *)      REL="chrome-win64/chrome.exe"
            DATA="${APPDATA:-$HOME}/tke" ;;
esac
if [ -x "$TKE_DIR/$REL" ]; then
    say_ok "Chrome for Testing: $TKE_DIR/$REL"
elif [ -x "$DATA/$REL" ]; then
    say_ok "Chrome for Testing: $DATA/$REL"
else
    say_bad "找不到 Chrome for Testing。解压官方 zip 到 $DATA/（保持原目录名，如 chrome-linux64/）"
    echo "     版本必须与 chromedriver 配对：https://googlechromelabs.github.io/chrome-for-testing/"
fi

echo "== 运行模式 =="
if [ "$(uname)" = "Linux" ] && [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
    echo "  ℹ️  无桌面 → tke 将自动走无头（--headless=auto 的默认判断）"
    if [ ! -f /.dockerenv ] && [ ! -f /run/.containerenv ] && [ "$(id -u)" != "0" ]; then
        :
    else
        echo "  ℹ️  容器/root → 自动加 --no-sandbox --disable-dev-shm-usage"
    fi
else
    echo "  ℹ️  有桌面 → 默认有头（要无头加 --headless=on，必须带等号）"
fi

echo "== 工作区 =="
[ -d tests/ui ] && say_ok "tests/ui/ 已存在" || say_warn "tests/ui/ 不存在（探索完会创建，用于存放两件套）"

echo
if [ "$fail" -gt 0 ]; then
    echo "结论：$fail 项硬缺失，先补齐再跑。"
    exit 1
fi
echo "结论：环境就绪（$ok 项通过）。"
