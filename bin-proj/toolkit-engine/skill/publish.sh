#!/usr/bin/env bash
# 把 skill 与二进制打包成 install.sh 期望的分发布局，输出到 dist/，然后同步到 S3/CDN。
#
#   ./publish.sh                      # 日常：只打 tke + skill + install.sh + VERSION
#   ./publish.sh --with-drivers       # 连驱动一起打（adb/chromedriver/aapt/go-ios，换版本时才要）
#   ./publish.sh --with-chrome        # 连 Chrome for Testing 一起打（几百 MB，很慢）
#   ./publish.sh --full               # = --with-drivers --with-chrome
#   ./publish.sh --out /tmp/dist      # 指定输出目录
#
# 默认不打驱动/Chrome：它们几乎不变，云上已有的不会因为没重传而消失，
# 每次都传纯属浪费时间和带宽。换驱动版本时再显式带上。
#
# 产出布局（install.sh 按这个约定去取）：
#   dist/
#   ├── install.sh / install.ps1 / uninstall.sh / uninstall.ps1
#   ├── skill/tke-ui-test.tar.gz
#   ├── bin/<platform>/{tke,chromedriver,adb,aapt,go-ios}.gz
#   └── chrome/<chrome-mac-arm64|chrome-linux64|...>.zip
#
# 上传：
#   export TKC_TOKEN=<token>
#   curl -fsSL https://cloud.test-toolkit.app/script/upload.sh | bash -s -- dist/ tookit-engine-resource:tke/

set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_BIN="$(cd "$SCRIPT_DIR/../../.." && pwd)/bin"     # <studio>/bin
OUT="$SCRIPT_DIR/dist"
WITH_CHROME=0
WITH_DRIVERS=0

while [ $# -gt 0 ]; do
    case "$1" in
        --out) OUT="${2:?}"; shift 2 ;;
        --with-chrome) WITH_CHROME=1; shift ;;
        --with-drivers) WITH_DRIVERS=1; shift ;;
        --full) WITH_DRIVERS=1; WITH_CHROME=1; shift ;;
        -h|--help) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "未知参数: $1" >&2; exit 2 ;;
    esac
done

case "$(uname -s)" in
    Darwin) OS_TAG=darwin; CHROME_DIR="$HOME/Library/Application Support/tke" ;;
    Linux)  OS_TAG=linux;  CHROME_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/tke" ;;
    *) echo "❌ 只支持在 macOS / Linux 上打包" >&2; exit 1 ;;
esac
case "$(uname -m)" in
    arm64|aarch64) ARCH_TAG=arm64 ;;
    x86_64|amd64)  ARCH_TAG=amd64 ;;
    *) echo "❌ 不支持的架构" >&2; exit 1 ;;
esac
PLATFORM="$OS_TAG-$ARCH_TAG"
SRC="$REPO_BIN/$PLATFORM"

[ -d "$SRC" ] || { echo "❌ 找不到 $SRC —— 先跑 build-{mac,linux}.sh" >&2; exit 1; }
[ -x "$SRC/tke" ] || { echo "❌ $SRC/tke 不存在或不可执行" >&2; exit 1; }

mkdir -p "$OUT/skill" "$OUT/bin/$PLATFORM" "$OUT/chrome"

echo "== 打包 =="
echo "   平台   $PLATFORM"
echo "   输出   $OUT"

# —— 版本留痕：让人能看出这批是什么 ——
"$SRC/tke" --version > "$OUT/VERSION" 2>/dev/null || echo "unknown" > "$OUT/VERSION"
# 驱动版本要标平台:两个平台的 chromedriver 未必同版本,而 VERSION 是全站一份、
# 谁后传谁覆盖——不标平台的话,mac 用户会看到 linux 那批的驱动版本,反过来也一样。
# (install.sh 只消费第一行的 tke 版本和 build 戳,这行纯给人看)
[ -x "$SRC/chromedriver" ] && echo "[${PLATFORM}] $("$SRC/chromedriver" --version 2>/dev/null)" >> "$OUT/VERSION"
# build 戳 = 下载缓存键。分发走 Cloudflare(max-age 4h,且不认 no-cache 请求头),
# 只有**变化的查询参数**能破缓存——所以每次发布换一个戳,新版本自然绕过旧缓存,
# 同版本则照常命中 CDN。没有它的话:传了新文件,使用者 4 小时内下到的还是旧的。
echo "build: $(date -u +%Y%m%d-%H%M%S)" >> "$OUT/VERSION"

# —— skill 文件 ——（只收 AI 和使用者要的，不含 dist/ 与打包脚本自身）
# **VERSION 要一起打进 skill 包**：装完之后，本地这份 skill 是哪一批就有据可查了。
# 没有它的话 `tke doctor` 只能比 tke 二进制的版本号，而版本号只在 bump 时才变、
# SKILL.md 却天天改 —— 用户抱着两天前的旧文档，体检照样说"一致"（Q-11 就是这么发生的）。
cp "$OUT/VERSION" "$SCRIPT_DIR/tke-ui-test/VERSION"
tar --exclude=".DS_Store" --exclude="__pycache__" -czf "$OUT/skill/tke-ui-test.tar.gz" -C "$SCRIPT_DIR" tke-ui-test
rm -f "$SCRIPT_DIR/tke-ui-test/VERSION"   # 不留在源码树里（它是发布产物，不是源文件）
echo "   ✅ skill/tke-ui-test.tar.gz（含 VERSION）"

# —— 二进制 ——（逐个 gzip，install.sh 按名字取；缺的跳过）
# 默认只打 tke —— 驱动几乎不变，云上已有的不会因为没重传而消失。
# libc++.so 是 Linux 版 aapt 的运行时依赖（aapt 的 RUNPATH 含 ${ORIGIN}，放同目录即可加载）
BINS="tke"
[ "$WITH_DRIVERS" = "1" ] && BINS="tke chromedriver adb aapt libc++.so go-ios"
for name in $BINS; do
    if [ -f "$SRC/$name" ]; then
        gzip -c "$SRC/$name" > "$OUT/bin/$PLATFORM/$name.gz"
        echo "   ✅ bin/$PLATFORM/$name.gz  ($(du -h "$OUT/bin/$PLATFORM/$name.gz" | cut -f1))"
    else
        echo "   -- $name 不在 ${SRC}，跳过"
    fi
done

# —— 安装/卸载脚本（两个平台各一份；Windows 跑不了 bash 那些）——
for f in install.sh install.ps1 uninstall.sh uninstall.ps1; do
    cp "$SCRIPT_DIR/$f" "$OUT/$f"
done
echo "   ✅ install/uninstall × (sh + ps1)"

# —— Chrome for Testing（可选，很大）——
if [ "$WITH_CHROME" = "1" ]; then
    pkg=$(ls "$CHROME_DIR" 2>/dev/null | grep -E '^chrome-(mac|linux|win)' | head -1)
    if [ -n "$pkg" ]; then
        echo "   .. 打包 ${pkg}（600MB+，慢）"
        (cd "$CHROME_DIR" && zip -qr "$OUT/chrome/$pkg.zip" "$pkg" -x "*.DS_Store")
        echo "   ✅ chrome/$pkg.zip ($(du -h "$OUT/chrome/$pkg.zip" | cut -f1))"
    else
        echo "   ⚠️  $CHROME_DIR 下没找到 chrome-* 目录，跳过"
    fi
else
    echo "   -- Chrome 未打包（要打加 --with-chrome）"
fi
[ "$WITH_DRIVERS" = "1" ] || echo "   -- 驱动未打包（要打加 --with-drivers；换驱动版本时才需要）"

echo
echo "版本：$(tr '\n' ' ' < "$OUT/VERSION")"
echo
# mac 上目录里常混进 .DS_Store,传上去是噪音
find "$OUT" -name '.DS_Store' -delete 2>/dev/null

echo "下一步 —— 传到 Toolkit Cloud（VERSION 必须一起传，它是破 CDN 缓存的键）："
echo "  export TKC_TOKEN=<你的token>"
echo "  curl -fsSL https://cloud.test-toolkit.app/script/upload.sh \\"
echo "    | bash -s -- $OUT/ tookit-engine-resource:tke/"
echo
echo "使用者侧：curl -fsSL https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke/install.sh | bash"
