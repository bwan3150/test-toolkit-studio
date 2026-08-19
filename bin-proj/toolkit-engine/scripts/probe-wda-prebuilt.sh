#!/usr/bin/env bash
# 探针：**预编译的 WDA 能不能脱离源码工程跑起来**——这一条决定 tke 能不能分发它。
#
#   bash scripts/probe-wda-prebuilt.sh
#
# 编译要几分钟。它只往 /tmp 写东西，不碰你的仓库、不碰 ~/.tke。
set -uo pipefail
G=$'\033[0;32m'; R=$'\033[0;31m'; Y=$'\033[1;33m'; D=$'\033[2m'; N=$'\033[0m'
ok(){ printf '%s✓%s %s\n' "$G" "$N" "$1"; }
bad(){ printf '%s✗%s %s\n' "$R" "$N" "$1"; }
note(){ printf '%s  %s%s\n' "$D" "$1" "$N"; }
step(){ printf '\n%s%s%s\n' "$Y" "$1" "$N"; }

[ "$(uname -s)" = Darwin ] || { bad "只在 macOS 上有意义"; exit 1; }
command -v xcodebuild >/dev/null || { bad "没有 xcodebuild"; exit 1; }

SRC=/tmp/wda-src
BUILD=/tmp/wda-build

step "① 取 WebDriverAgent 源码（appium 维护的那份）"
if [ -d "$SRC/.git" ]; then
  ok "已有 $SRC"
else
  git clone --depth 1 https://github.com/appium/WebDriverAgent "$SRC" >/dev/null 2>&1 \
    && ok "clone 到 $SRC" || { bad "clone 失败"; exit 1; }
fi
note "版本 $(cd "$SRC" && git rev-parse --short HEAD)"

step "② 编译模拟器版（免签名，build-for-testing）"
xcodebuild build-for-testing \
  -project "$SRC/WebDriverAgent.xcodeproj" \
  -scheme WebDriverAgentRunner \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$BUILD" \
  CODE_SIGNING_ALLOWED=NO >/tmp/wda-build.log 2>&1 \
  && ok "编译完成" || { bad "编译失败，看 /tmp/wda-build.log 末尾"; tail -20 /tmp/wda-build.log; exit 1; }

step "③ 产物是什么（决定要分发哪些文件）"
APP=$(find "$BUILD/Build/Products" -maxdepth 2 -name "WebDriverAgentRunner-Runner.app" | head -1)
XCTESTRUN=$(find "$BUILD/Build/Products" -maxdepth 1 -name "*.xctestrun" | head -1)
[ -n "$APP" ] && ok ".app  $APP  ($(du -sh "$APP" | cut -f1))" || bad "没找到 .app"
[ -n "$XCTESTRUN" ] && ok ".xctestrun  $XCTESTRUN" || bad "没找到 .xctestrun"

step "④ 【关键】.xctestrun 里有没有写死本机绝对路径"
note "写死了 = 换台机器就不能用 = 分发不了"
if [ -n "$XCTESTRUN" ]; then
  grep -oE "/Users/[^<]*" "$XCTESTRUN" | sort -u | head -10
  CNT=$(grep -cE "/Users/" "$XCTESTRUN" || true)
  [ "$CNT" -eq 0 ] && ok "没有本机绝对路径" || bad "有 $CNT 处本机路径（上面列了前几条）"
fi

step "⑤ 装进模拟器并起起来"
UDID=$(xcrun simctl list devices booted -j | python3 -c "
import json,sys
d=json.load(sys.stdin)['devices']; v=[x['udid'] for l in d.values() for x in l]; print(v[0] if v else '')")
[ -n "$UDID" ] || { bad "没有已启动的模拟器（先 boot 一台）"; exit 1; }
note "模拟器 $UDID"

xcrun simctl install "$UDID" "$APP" && ok "装进去了" || bad "装不进去"

note "先试最省事的：直接 simctl launch（能成的话 tke 连 xcodebuild 都不用调）"
xcrun simctl launch "$UDID" com.facebook.WebDriverAgentRunner.xctrunner 2>&1 | head -3
sleep 6
if curl -s --max-time 3 http://127.0.0.1:8100/status | head -c 200; then
  echo; ok "直接 launch 就能起 —— 这是最理想的情况"
  exit 0
fi
echo
note "直接 launch 起不来（XCTest bundle 通常要 xcodebuild 带环境变量），试标准做法"

xcodebuild test-without-building -xctestrun "$XCTESTRUN" -destination "id=$UDID" \
  >/tmp/wda-run.log 2>&1 &
RUNPID=$!
for i in $(seq 1 20); do
  sleep 2
  if curl -s --max-time 2 http://127.0.0.1:8100/status >/tmp/wda-status.json 2>/dev/null; then
    ok "test-without-building 起来了，8100 通了"
    head -c 300 /tmp/wda-status.json; echo
    kill $RUNPID 2>/dev/null
    exit 0
  fi
done
bad "40 秒内 8100 没通"
note "把 /tmp/wda-run.log 的末尾 30 行发我"
tail -30 /tmp/wda-run.log
kill $RUNPID 2>/dev/null
exit 1
