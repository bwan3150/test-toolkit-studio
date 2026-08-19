#!/usr/bin/env bash
# 打包模拟器版 WebDriverAgent，供 `tke doctor --fix --profile ios` 下载。
#
#   bash scripts/package-wda-sim.sh            # 编译 + 打包 + 自检
#   WDA_REF=<commit|tag> bash …                # 换个版本
#
# **版本锁在这个脚本里**（见 WDA_REF 默认值）——这正是自己分发的意义：
# 上游哪天变了不会突然把用户的环境搞坏。要升就改这里、重跑、重传，且要重验一遍。
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/.."

# 锁定版本：2026-08-19 实测通过的那个 commit（appium/WebDriverAgent）
WDA_REF="${WDA_REF:-8976450}"

G=$'\033[0;32m'; R=$'\033[0;31m'; Y=$'\033[1;33m'; D=$'\033[2m'; N=$'\033[0m'
ok(){ printf '%s✓%s %s\n' "$G" "$N" "$1"; }
bad(){ printf '%s✗%s %s\n' "$R" "$N" "$1"; exit 1; }
step(){ printf '\n%s%s%s\n' "$Y" "$1" "$N"; }

[ "$(uname -s)" = Darwin ] || bad "只能在 macOS 上打包（要 xcodebuild）"
command -v xcodebuild >/dev/null || bad "没有 xcodebuild"

SRC=/tmp/wda-src
BUILD=/tmp/wda-build
OUT="$(pwd)/dist-wda"
ZIP="$OUT/WebDriverAgentRunner-Runner-sim.zip"

step "① 取源码（锁定 ${WDA_REF}）"
if [ ! -d "$SRC/.git" ]; then
    git clone https://github.com/appium/WebDriverAgent "$SRC" >/dev/null 2>&1 || bad "clone 失败"
fi
( cd "$SRC" && git fetch --all -q && git checkout -q "$WDA_REF" ) || bad "切不到 $WDA_REF"
ok "$(cd "$SRC" && git log -1 --format='%h %s' | head -c 70)"

step "② 编译（模拟器 SDK，免签名）"
rm -rf "$BUILD"
xcodebuild build-for-testing \
    -project "$SRC/WebDriverAgent.xcodeproj" \
    -scheme WebDriverAgentRunner \
    -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath "$BUILD" \
    CODE_SIGNING_ALLOWED=NO >/tmp/wda-package.log 2>&1 || {
        tail -20 /tmp/wda-package.log; bad "编译失败（完整日志 /tmp/wda-package.log）"; }
ok "编译完成"

APP=$(find "$BUILD/Build/Products" -maxdepth 2 -name "WebDriverAgentRunner-Runner.app" | head -1)
[ -n "$APP" ] || bad "没找到 .app"
ok "$APP ($(du -sh "$APP" | cut -f1))"

step "③ 打包"
rm -rf "$OUT"; mkdir -p "$OUT"
# **在 .app 的父目录里打包**，这样 zip 里第一层就是 WebDriverAgentRunner-Runner.app/
# ——tke 解压后直接就是 ~/.tke/wda/WebDriverAgentRunner-Runner.app，路径对得上
( cd "$(dirname "$APP")" && zip -qry "$ZIP" "$(basename "$APP")" ) || bad "打包失败"
ok "$ZIP ($(du -h "$ZIP" | cut -f1))"

step "④ 自检：解出来的结构对不对"
T=$(mktemp -d)
unzip -q "$ZIP" -d "$T" || bad "解压失败"
[ -d "$T/WebDriverAgentRunner-Runner.app" ] \
    || bad "zip 里第一层不是 WebDriverAgentRunner-Runner.app（tke 会找不到）"
[ -f "$T/WebDriverAgentRunner-Runner.app/WebDriverAgentRunner-Runner" ] \
    || bad "包里没有主程序"
ok "结构正确"
rm -rf "$T"

# 版本留痕：分发源上放一份，出问题时能对得上是哪个 WDA
printf 'wda-ref: %s\nwda-commit: %s\nbuilt: %s\n' \
    "$WDA_REF" "$(cd "$SRC" && git rev-parse HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    > "$OUT/WDA-VERSION"
ok "$OUT/WDA-VERSION"

step "⑤ 传上去"
cat <<EOF
  export TKC_TOKEN=tkc_xxx
  curl -fsSL https://cloud.test-toolkit.app/script/upload.sh | bash -s -- \\
    $OUT/ tookit-engine-resource:tke/wda/

传完验一次（它会从分发源下载到 ~/.tke/wda/）：
  rm -rf ~/.tke/wda
  <仓库>/bin/darwin-arm64/tke doctor --fix -y --profile ios
EOF
