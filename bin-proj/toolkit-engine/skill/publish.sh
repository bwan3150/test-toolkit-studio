#!/usr/bin/env bash
# 把 skill 与二进制打包成 install.sh 期望的分发布局，输出到 dist/，然后同步到 S3/CDN。
#
#   ./publish.sh                      # 打包当前平台的二进制 + skill
#   ./publish.sh --out /tmp/dist      # 指定输出目录
#   ./publish.sh --with-chrome        # 连 Chrome for Testing 一起打（600MB+，慢）
#
# 产出布局（install.sh 按这个约定去取）：
#   dist/
#   ├── install.sh
#   ├── skill/ui-check.tar.gz
#   ├── bin/<platform>/{tke,chromedriver,adb,aapt,go-ios}.gz
#   └── chrome/<chrome-mac-arm64|chrome-linux64|...>.zip
#
# 上传（示例）：
#   aws s3 sync dist/ s3://<bucket>/tke/ --acl public-read
#   然后 install.sh 里的 DEFAULT_BASE_URL 指向 https://<cdn>/tke

set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_BIN="$(cd "$SCRIPT_DIR/../../.." && pwd)/bin"     # <studio>/bin
OUT="$SCRIPT_DIR/dist"
WITH_CHROME=0

while [ $# -gt 0 ]; do
    case "$1" in
        --out) OUT="${2:?}"; shift 2 ;;
        --with-chrome) WITH_CHROME=1; shift ;;
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

# —— skill 文件 ——（只收 AI 和使用者要的，不含 dist/ 与打包脚本自身）
tar --exclude=".DS_Store" --exclude="__pycache__" -czf "$OUT/skill/ui-check.tar.gz" -C "$SCRIPT_DIR" ui-check
echo "   ✅ skill/ui-check.tar.gz"

# —— 二进制 ——（逐个 gzip，install.sh 按名字取；缺的跳过）
# libc++.so 是 Linux 版 aapt 的运行时依赖（aapt 的 RUNPATH 含 $ORIGIN，放同目录即可加载）
for name in tke chromedriver adb aapt libc++.so go-ios; do
    if [ -f "$SRC/$name" ]; then
        gzip -c "$SRC/$name" > "$OUT/bin/$PLATFORM/$name.gz"
        echo "   ✅ bin/$PLATFORM/$name.gz  ($(du -h "$OUT/bin/$PLATFORM/$name.gz" | cut -f1))"
    else
        echo "   -- $name 不在 $SRC，跳过"
    fi
done

# —— install.sh 自身 ——
cp "$SCRIPT_DIR/install.sh" "$OUT/install.sh"
echo "   ✅ install.sh"

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

# —— 版本留痕：让人能看出这批是什么 ——
"$SRC/tke" --version > "$OUT/VERSION" 2>/dev/null || echo "unknown" > "$OUT/VERSION"
[ -x "$SRC/chromedriver" ] && "$SRC/chromedriver" --version >> "$OUT/VERSION" 2>/dev/null
# build 戳 = 下载缓存键。分发走 Cloudflare(max-age 4h,且不认 no-cache 请求头),
# 只有**变化的查询参数**能破缓存——所以每次发布换一个戳,新版本自然绕过旧缓存,
# 同版本则照常命中 CDN。没有它的话:传了新文件,使用者 4 小时内下到的还是旧的。
echo "build: $(date -u +%Y%m%d-%H%M%S)" >> "$OUT/VERSION"
echo
echo "版本：$(tr '\n' ' ' < "$OUT/VERSION")"
echo
# mac 上目录里常混进 .DS_Store,传上去是噪音
find "$OUT" -name '.DS_Store' -delete 2>/dev/null

echo "下一步："
echo "  aws s3 sync $OUT/ s3://<bucket>/tke/ --acl public-read"
echo "  用户侧：curl -fsSL https://<cdn>/tke/install.sh | bash"
