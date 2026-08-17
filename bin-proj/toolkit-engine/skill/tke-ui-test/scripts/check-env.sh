#!/usr/bin/env bash
# tke-ui-test skill 前置体检：缺什么直接说清楚，别让调用方撞进去猜。
# 退出码：0=至少有一个目标可操作；1=tke 本身不可用。

set -uo pipefail

echo "== tke 本体 =="
if command -v tke >/dev/null 2>&1; then
    TKE="$(command -v tke)"
    echo "  ✅ tke: $TKE ($(tke --version 2>/dev/null || echo '版本未知'))"
else
    echo "  ❌ tke 不在 PATH。构建: bin-proj/toolkit-engine/build-{linux,mac}.sh，再把 bin/<platform>/ 加进 PATH"
    exit 1
fi
TKE_DIR="$(dirname "$(readlink -f "$TKE")")"

targets=0

echo "== 浏览器（-d web）=="
# chromedriver 必须与 tke 同目录：ToolManager 只搜同目录，不回退 PATH（版本配对靠这个约束）
if [ -x "$TKE_DIR/chromedriver" ] || [ -x "$TKE_DIR/chromedriver.exe" ]; then
    echo "  ✅ chromedriver: $("$TKE_DIR/chromedriver" --version 2>/dev/null | head -1)"
    # Chrome for Testing：tke 同目录 或 用户数据目录，按官方 zip 原样结构
    case "$(uname)" in
        Darwin) REL="chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
                DATA="$HOME/Library/Application Support/tke" ;;
        Linux)  REL="chrome-linux64/chrome"
                DATA="${XDG_DATA_HOME:-$HOME/.local/share}/tke" ;;
        *)      REL="chrome-win64/chrome.exe"
                DATA="${APPDATA:-$HOME}/tke" ;;
    esac
    if [ -x "$TKE_DIR/$REL" ] || [ -x "$DATA/$REL" ]; then
        echo "  ✅ Chrome for Testing 就位"
        targets=$((targets+1))
    else
        echo "  ❌ 找不到 Chrome for Testing。补齐：tke doctor --fix --profile web（约 600MB）"
        echo "     或手动解压官方 zip 到 $DATA/（保持原目录名，版本要与 chromedriver 配对）"
    fi
else
    echo "  ❌ chromedriver 不在 tke 同目录（${TKE_DIR}）——tke 只在这里找，不搜 PATH"
    echo "     补齐：tke doctor --fix --profile web"
fi

echo "== 安卓（-d <序列号>）=="
if command -v adb >/dev/null 2>&1 || [ -x "$TKE_DIR/adb" ]; then
    ADB="$(command -v adb || echo "$TKE_DIR/adb")"
    devs=$("$ADB" devices 2>/dev/null | awk 'NR>1 && $2=="device" {print $1}')
    if [ -n "$devs" ]; then
        # 不用 `| while`：管道会开子 shell，里面的 targets 自增传不回来
        for d in $devs; do echo "  ✅ 设备: $d"; done
        targets=$((targets+1))
    else
        echo "  ⚠️  adb 可用但没有已连接设备"
    fi
else
    echo "  ⚠️  没有 adb（不做安卓检查就无所谓；要装：tke doctor --fix --profile android）"
fi

echo "== 版本 =="
# 跟分发源比一下版本,免得 skill 一直用着旧 tke。
# 3 秒超时、失败静默——离线/内网照常能用,不为这个卡住检查。
# 注意:存储平台对不存在的路径回落 200 + HTML,所以必须验内容长得像不像版本号,不能只看状态码。
TKE_BASE_URL="${TKE_BASE_URL:-https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke}"
LOCAL_VER="$(tke --version 2>/dev/null | head -1)"
# 带随机参数：Cloudflare 缓存 4h 且不认 no-cache 请求头，不破缓存就永远看到旧版本号
# 注：这里 `| head -1` 安全，因为 VERSION 只有一百多字节、管道缓冲区装得下，curl 写完才退出。
# **别把这个写法照抄到大文件上**——head 读够就关管道，curl 会拿到 EPIPE 并以退出码 23 失败（P-23）
REMOTE_VER="$(curl -fsSL --max-time 3 "$TKE_BASE_URL/VERSION?t=$$" 2>/dev/null | head -1)"
case "$REMOTE_VER" in
    tke\ *)
        if [ "$REMOTE_VER" = "$LOCAL_VER" ]; then
            echo "  ✅ ${LOCAL_VER}（已是分发源上的版本）"
        else
            echo "  ⬆️  本地 $LOCAL_VER ／ 分发源 $REMOTE_VER"
            echo "     更新：curl -fsSL $TKE_BASE_URL/install.sh | bash"
        fi
        ;;
    *)
        echo "  ℹ️  ${LOCAL_VER}（没连上分发源，跳过版本检查）"
        ;;
esac

echo "== 证据落点 =="
# 默认落用户目录,不往被检查的项目里写(它是过程产物,不该混进人家仓库)
LOG_ROOT="$HOME/.tke/logs"
if [ -d "$LOG_ROOT" ]; then
    echo "  ℹ️  ${LOG_ROOT}（已有 $(find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l | tr -d ' ') 次检查的记录）"
else
    echo "  ℹ️  ${LOG_ROOT}（首次检查时自动创建）"
fi

echo "== 运行模式 =="
if [ "$(uname)" = "Linux" ] && [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
    echo "  ℹ️  无桌面 → 浏览器自动走无头"
else
    echo "  ℹ️  有桌面 → 浏览器默认有头（要无头加 --headless=on，必须带等号）"
fi

echo
if [ "$targets" -eq 0 ]; then
    echo "结论：没有可操作的目标，先把浏览器或安卓设备准备好。"
    exit 1
fi
echo "结论：可以开始检查。"
