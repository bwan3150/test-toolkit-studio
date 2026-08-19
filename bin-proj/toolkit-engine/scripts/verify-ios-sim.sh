#!/usr/bin/env bash
# iOS 模拟器链路验证（只在 macOS 上有意义）。
#
#   bash scripts/verify-ios-sim.sh "要点的按钮文字" [bundle-id]
#
# 验四件事,每件都单独报成败——**别只看最后一行**：
#   ① 模拟器列不列得出来（device list + idb 装没装）
#   ② 元素采不采得到（describe-all → 归一化 → 元素表）
#   ③ 坐标换算对不对（按文字点一下，看页面**真的变了没有**）  ← 最关键
#   ④ 证据落没落盘（截图/元素表/报告）
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

WANT="${1:-}"
BUNDLE="${2:-}"
G=$'\033[0;32m'; R=$'\033[0;31m'; Y=$'\033[1;33m'; D=$'\033[2m'; N=$'\033[0m'
ok(){ printf '%s✓%s %s\n' "$G" "$N" "$1"; }
bad(){ printf '%s✗%s %s\n' "$R" "$N" "$1"; }
note(){ printf '%s  %s%s\n' "$D" "$1" "$N"; }

[ "$(uname -s)" = Darwin ] || { bad "这个脚本只在 macOS 上有意义"; exit 1; }

# ⚠️ **一律用刚构建出来的产物，不用 PATH 里那个**。
# `tke` 命令解析到的多半是安装器装到 ~/.tke/bin/ 的发布版——拿它验，验的就不是你
# 刚改的代码（实测踩过：编译成功、跑的还是旧版，新子命令报 unrecognized subcommand）。
# 这里也**不去覆盖**那个 tke：它是日常在用的，验证脚本没资格替人换掉。
ARCH=$([ "$(uname -m)" = arm64 ] && echo arm64 || echo amd64)
TKE="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)/bin/darwin-$ARCH/tke"
[ -x "$TKE" ] || { bad "找不到构建产物: $TKE"; note "先跑 ./bin-proj/toolkit-engine/build-mac.sh"; exit 1; }
note "用的是 $TKE"
note "版本 $("$TKE" --version 2>&1 | head -1)"
[ -n "$WANT" ] || { bad "用法: bash scripts/verify-ios-sim.sh \"屏幕上某个按钮的文字\" [bundle-id]"; exit 1; }

# —— 准备：确保有一台跑着的模拟器 ——
UDID=$(xcrun simctl list devices available -j | python3 -c "
import json,sys
d=json.load(sys.stdin)['devices']; cand=None
for rt,l in d.items():
    if 'iOS' not in rt: continue
    for x in l:
        if x.get('state')=='Booted': print(x['udid']); raise SystemExit
        if 'iPhone' in x['name'] and cand is None: cand=x['udid']
print(cand or '')")
[ -n "$UDID" ] || { bad "一台 iPhone 模拟器都没有"; exit 1; }
xcrun simctl boot "$UDID" 2>/dev/null
open -a Simulator 2>/dev/null
xcrun simctl bootstatus "$UDID" -b >/dev/null 2>&1
[ -n "$BUNDLE" ] && { xcrun simctl launch "$UDID" "$BUNDLE" >/dev/null 2>&1; sleep 3; }
note "模拟器 $UDID"

LOG="$HOME/.tke/logs/sim-verify"
rm -rf "$LOG"

# ── ① 设备发现 ──
printf '\n%s① 模拟器列不列得出来%s\n' "$Y" "$N"
if "$TKE" device list | grep -q "sim:$UDID"; then
  ok "device list 列出了 sim:$UDID"
else
  bad "device list 里没有这台"
  note "把 \"$TKE device list\" 的完整输出发我"
fi
"$TKE" device list | grep -i "idb" && note "（上面这行是 idb 的状态）"

# ── ② 元素采集 ──
printf '\n%s② 元素采不采得到%s\n' "$Y" "$N"
OUT=$("$TKE" -d "sim:$UDID" fetch --interactive 2>&1)
# 必须是**非空数组**。早先只判"非空"，于是 {"success":false,...} 这种错误对象
# 也被当成"采到了"，然后在下一行 [:6] 上炸出 KeyError——报错还报在无关的地方
if printf '%s' "$OUT" | python3 -c "
import json,sys
d=json.load(sys.stdin)
sys.exit(0 if isinstance(d,list) and d else 1)" 2>/dev/null; then
  CNT=$(printf '%s' "$OUT" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")
  ok "采到 $CNT 个可交互元素"
  printf '%s' "$OUT" | python3 -c "
import json,sys
for e in json.load(sys.stdin)[:6]:
    b=e['bounds']
    print(f\"    {e.get('class_name',''):<14} {repr(e.get('text'))[:26]:<28} [{b['x1']},{b['y1']}][{b['x2']},{b['y2']}]\")"
  note "上面的坐标应当在**截图像素**量级（iPhone 竖屏约 1179×2556，不是 393×852）"
else
  bad "采集失败（下面是原样输出）"
  printf '%s\n' "$OUT" | head -5
  note "把这段发我；另外看看 idb ui describe-all 单独跑通不通"
  exit 1
fi

# ── ③ 坐标换算：点一下，看页面真的变了没有 ──
printf '\n%s③ 按文字点「%s」——页面变没变（最关键）%s\n' "$Y" "$WANT" "$N"
BEFORE=$("$TKE" -d "sim:$UDID" fetch 2>/dev/null | python3 -c "
import json,sys
print('|'.join(sorted(filter(None,(e.get('text') for e in json.load(sys.stdin))))))" 2>/dev/null)

"$TKE" -d "sim:$UDID" steps "点击 [\"$WANT\"] # 验证坐标换算：点中了页面就该变" --log "$LOG/" 2>&1 | tail -3
sleep 2

AFTER=$("$TKE" -d "sim:$UDID" fetch 2>/dev/null | python3 -c "
import json,sys
print('|'.join(sorted(filter(None,(e.get('text') for e in json.load(sys.stdin))))))" 2>/dev/null)

if [ "$BEFORE" = "$AFTER" ]; then
  bad "点完页面**一个字都没变** —— 多半是坐标换算偏了，或者根本没点中"
  note "把 $LOG/raw_pages/step_001.json（原始 AX 树）和 screenshots/step_001.png 发我"
  note "那张标注截图上画着点击点，一眼能看出打到哪儿去了"
else
  ok "页面变了 —— 说明真的点中了（坐标换算、语义定位都对）"
fi

# ── ④ 证据 ──
printf '\n%s④ 证据落没落盘%s\n' "$Y" "$N"
for f in report.html screenshots pages raw_pages log.json; do
  [ -e "$LOG/$f" ] && ok "$f" || bad "$f 没有"
done
printf '\n%s报告：%s%s\n' "$D" "$LOG/report.html" "$N"
printf '%s打开：open %s/report.html%s\n' "$D" "$LOG" "$N"
