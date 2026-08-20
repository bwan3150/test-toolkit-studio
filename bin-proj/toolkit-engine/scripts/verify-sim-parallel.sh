#!/usr/bin/env bash
# 双模拟器并行验证（只在 macOS 上有意义）——Q-13 那个修复到底成没成。
#
#   bash scripts/verify-sim-parallel.sh
#
# 验的不是"能不能同时跑",是**命令有没有发到该去的那台**。这两件事很容易混：
# 两台都在跑、两条命令都报成功、页面也都动了——但动的可能是同一台。
#
# 四件事，每件单独报成败：
#   ① 两台各自拿到**不同端口**（状态文件里记着）
#   ② 每个端口的监听进程 = 那台模拟器里 WDA 的进程（`lsof` PID vs `launchctl` PID）
#   ③ **并发** refresh，两边的分辨率各是各的（必须走 WDA 那个端口的路）  ← 最关键
#   ④ 两边的证据各落各的目录
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
cd "${SCRIPT_DIR}/.."

G=$'\033[0;32m'; R=$'\033[0;31m'; Y=$'\033[1;33m'; D=$'\033[2m'; N=$'\033[0m'
ok(){ printf '%s✓%s %s\n' "${G}" "${N}" "$1"; }
bad(){ printf '%s✗%s %s\n' "${R}" "${N}" "$1"; }
note(){ printf '%s  %s%s\n' "${D}" "$1" "${N}"; }
FAILED=0
fail(){ bad "$1"; FAILED=1; }

[ "$(uname -s)" = Darwin ] || { bad "这个脚本只在 macOS 上有意义"; exit 1; }

# ⚠️ **一律用刚构建出来的产物，不用 PATH 里那个**（P-42：编译成功、跑的还是旧版）
ARCH=$([ "$(uname -m)" = arm64 ] && echo arm64 || echo amd64)
TKE="${REPO_ROOT}/bin/darwin-${ARCH}/tke"
[ -x "${TKE}" ] || { bad "找不到构建产物: ${TKE}"; note "先跑 ./bin-proj/toolkit-engine/build-mac.sh"; exit 1; }
note "用的是 ${TKE}"

[ -d "${HOME}/.tke/wda/WebDriverAgentRunner-Runner.app" ] || [ -n "${TKE_WDA_APP:-}" ] || {
    bad "没有 WebDriverAgent —— 模拟器操作不了"
    note "装它：tke doctor --fix --profile ios"
    exit 1
}

# —— 准备：挑两台**型号不同**的模拟器 ——
# 型号不同才验得出串台：两台都叫 iPhone 17 Pro 的话，报回来的型号一样，
# 命令跑到哪台上都看不出来（`Controller::describe` 的 label 会撞，单测里也钉着这条）
read -r UDID_A NAME_A UDID_B NAME_B <<<"$(xcrun simctl list devices available -j | python3 -c "
import json,sys
d=json.load(sys.stdin)['devices']
seen={}
for rt,l in d.items():
    if 'iOS' not in rt: continue
    for x in l:
        if 'iPhone' not in x['name']: continue
        seen.setdefault(x['name'], x['udid'])
picks=list(seen.items())[:2]
print(' '.join(f\"{u} {n.replace(' ','_')}\" for n,u in picks) if len(picks)==2 else '')")"

[ -n "${UDID_B:-}" ] || { bad "找不到两台型号不同的 iPhone 模拟器"; note "去 Xcode 里加一台别的型号"; exit 1; }
note "A = ${NAME_A//_/ }  ${UDID_A}"
note "B = ${NAME_B//_/ }  ${UDID_B}"

for U in "${UDID_A}" "${UDID_B}"; do
    xcrun simctl boot "${U}" 2>/dev/null
done
open -a Simulator 2>/dev/null
for U in "${UDID_A}" "${UDID_B}"; do
    xcrun simctl bootstatus "${U}" -b >/dev/null 2>&1
done

# 各自把 WDA 拉起来（第一次会挤掉前台 App，这里不在意——验的是链路不是业务）
for U in "${UDID_A}" "${UDID_B}"; do
    "${TKE}" -d "sim:${U}" fetch >/dev/null 2>&1 || true
done

STATE_DIR="${TMPDIR:-/tmp}/tke/ios"
port_of(){ python3 -c "
import json,sys
try: print(json.load(open(sys.argv[1]))['port'])
except Exception: print('')" "${STATE_DIR}/$1.json"; }

# ── ① 端口不同 ──
printf '\n%s① 两台各自拿到不同端口%s\n' "${Y}" "${N}"
PORT_A=$(port_of "${UDID_A}")
PORT_B=$(port_of "${UDID_B}")
note "A → ${PORT_A:-（状态文件里没有）}   B → ${PORT_B:-（状态文件里没有）}"
if [ -z "${PORT_A}" ] || [ -z "${PORT_B}" ]; then
    fail "状态文件里读不到端口（${STATE_DIR}/<udid>.json）"
elif [ "${PORT_A}" = "${PORT_B}" ]; then
    fail "两台都用 ${PORT_A} —— 端口还是撞的，Q-13 的修复没生效"
else
    ok "端口分开了：${PORT_A} / ${PORT_B}"
fi

# ── ② 端口归属：谁在监听，是不是那台的 WDA ──
printf '\n%s② 每个端口的监听进程属于对应的模拟器%s\n' "${Y}" "${N}"
BUNDLE=com.facebook.WebDriverAgentRunner.xctrunner
wda_pid(){ xcrun simctl spawn "$1" launchctl list "${BUNDLE}" 2>/dev/null \
             | sed -n 's/.*"PID" = \([0-9]*\);.*/\1/p' | head -1; }
lsof_pid(){ lsof -nP -iTCP:"$1" -sTCP:LISTEN -t 2>/dev/null | head -1; }
for pair in "A:${UDID_A}:${PORT_A}" "B:${UDID_B}:${PORT_B}"; do
    TAG="${pair%%:*}"; REST="${pair#*:}"; U="${REST%%:*}"; P="${REST##*:}"
    [ -n "${P}" ] || { fail "${TAG} 没有端口，跳过归属校验"; continue; }
    MINE=$(wda_pid "${U}"); LIS=$(lsof_pid "${P}")
    note "${TAG}: launchctl PID=${MINE:-?}  lsof PID=${LIS:-?}  (端口 ${P})"
    if [ -z "${MINE}" ] || [ -z "${LIS}" ]; then
        bad "${TAG} 有一边问不出来 —— 校验会放行（退回「端口通就算数」），这里也判不了"
    elif [ "${MINE}" = "${LIS}" ]; then
        ok "${TAG} 端口 ${P} 上监听的就是这台的 WDA"
    else
        fail "${TAG} 端口 ${P} 被别人占着（PID ${LIS} ≠ ${MINE}）—— 命令会发到别的设备上"
    fi
done

# ── ③ 并发跑，看两边拿回来的屏幕是不是各自那台 ──（最关键）
#
# ⚠️ **不能用 `device info` 验这件事**：模拟器的机型/系统 tke 是问 simctl 拿的，
# 压根不经过 WDA——两台串了台它也照样报得对，等于什么都没验。
# 要挑一条**必须走 WDA 那个端口**的路：`refresh`（采截图 + 元素树）。
# 型号不同 → 截图分辨率不同 → 串台一眼看得出来。
# 工作区按设备分目录（`$TMPDIR/tke/workarea/<设备id>/`），所以两边各读各的。
printf '\n%s③ 并发 refresh，两边的分辨率各是各的%s\n' "${Y}" "${N}"
WORKAREA="${TMPDIR:-/tmp}/tke/workarea"
# 目录名是设备 id 把非字母数字换成 `_`：sim:ABC-123 → sim_ABC-123
sanitize(){ printf '%s' "$1" | sed 's/[^A-Za-z0-9_-]/_/g'; }
SHOT_A="${WORKAREA}/$(sanitize "sim:${UDID_A}")/current_screenshot.png"
SHOT_B="${WORKAREA}/$(sanitize "sim:${UDID_B}")/current_screenshot.png"
rm -f "${SHOT_A}" "${SHOT_B}"

"${TKE}" -d "sim:${UDID_A}" refresh >/dev/null 2>&1 &
PID_A=$!
"${TKE}" -d "sim:${UDID_B}" refresh >/dev/null 2>&1 &
PID_B=$!
wait "${PID_A}"; wait "${PID_B}"

shot_size(){
    [ -f "$1" ] || { echo ""; return; }
    sips -g pixelWidth -g pixelHeight "$1" 2>/dev/null \
      | sed -n 's/.*pixel\(Width\|Height\): \([0-9]*\)/\2/p' | paste -sd'x' -
}
SIZE_A=$(shot_size "${SHOT_A}"); SIZE_B=$(shot_size "${SHOT_B}")
note "A ${NAME_A//_/ } → ${SIZE_A:-（没截到图）}"
note "B ${NAME_B//_/ } → ${SIZE_B:-（没截到图）}"
if [ -z "${SIZE_A}" ] || [ -z "${SIZE_B}" ]; then
    fail "有一边没截到图 —— 先单跑 verify-ios-sim.sh 看是哪一步断的"
elif [ "${SIZE_A}" = "${SIZE_B}" ]; then
    bad "两边分辨率一样（${SIZE_A}）—— **可能串台，也可能这两台本来就同尺寸**"
    note "换一台屏幕明显不同的再验（比如 iPhone SE 配 iPhone 17 Pro Max）"
    note "或者肉眼比这两张：open ${SHOT_A} ${SHOT_B}"
else
    ok "两边各拿到自己那台的屏幕（${SIZE_A} / ${SIZE_B}）—— 并发下没串台"
fi

# ── ④ 证据各落各的 ──
printf '\n%s④ 并发跑一步，两边的证据各落各的目录%s\n' "${Y}" "${N}"
LOG_A="${HOME}/.tke/logs/par-a"; LOG_B="${HOME}/.tke/logs/par-b"
rm -rf "${LOG_A}" "${LOG_B}"
"${TKE}" -d "sim:${UDID_A}" steps "等待 [1s]" --log "${LOG_A}/" >/dev/null 2>&1 &
PID_A=$!
"${TKE}" -d "sim:${UDID_B}" steps "等待 [1s]" --log "${LOG_B}/" >/dev/null 2>&1 &
PID_B=$!
wait "${PID_A}"; wait "${PID_B}"
for L in "${LOG_A}" "${LOG_B}"; do
    if [ -e "${L}/report.html" ]; then ok "$(basename "${L}") 有报告"; else fail "$(basename "${L}") 没有报告"; fi
done
note "两份报告：open ${LOG_A}/report.html ${LOG_B}/report.html"

printf '\n'
if [ "${FAILED}" = 0 ]; then
    ok "并行没问题"
else
    bad "上面有红的 —— 把那几行连同 ${STATE_DIR}/*.json 发我"
fi
exit "${FAILED}"
