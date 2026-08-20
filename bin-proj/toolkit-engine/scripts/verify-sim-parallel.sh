#!/usr/bin/env bash
# 双模拟器并行验证（只在 macOS 上有意义）——Q-13 那个修复到底成没成。
#
#   bash scripts/verify-sim-parallel.sh <UDID-A> <UDID-B>
#
# 不给参数就列出这台机器上的模拟器让你挑。**两台要型号不同**——同型号分辨率一样、
# `describe` 的 label 也会撞，串没串台根本看不出来（第③步就是靠这个差异判的）。
#
# 验的不是"能不能同时跑",是**命令有没有发到该去的那台**。这两件事很容易混：
# 两台都在跑、两条命令都报成功、页面也都动了——但动的可能是同一台。
#
# 四件事，每件单独报成败：
#   ① 两台各自拿到**不同端口**（状态文件里记着）
#   ② 每个端口的监听进程 = 那台模拟器里 WDA 的进程（`lsof` PID vs `launchctl` PID）
#   ③ 两个端口各要一张截图，分辨率各是各的（直接问 WDA，绕开会话）  ← 最关键
#   ④ tke 层并发跑一步，证据各落各的目录
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
# **产物比代码旧就直接拦下**。P-42 记的是"别用 PATH 里的 tke"，但用了构建产物也可能是
# 上一轮编的——那时验的还是旧行为，红绿都不作数（这一条就是这么白跑了一轮才加的）
BIN_TS=$(stat -f %m "${TKE}" 2>/dev/null || echo 0)
SRC_TS=$(git -C "${SCRIPT_DIR}/.." log -1 --format=%ct -- src 2>/dev/null || echo 0)
if [ "${BIN_TS}" -lt "${SRC_TS}" ]; then
    bad "构建产物比 src/ 的最后一次提交还旧 —— 验的会是上一版的行为"
    note "产物 $(date -r "${BIN_TS}" '+%m-%d %H:%M')   代码 $(date -r "${SRC_TS}" '+%m-%d %H:%M')"
    note "先跑 ./bin-proj/toolkit-engine/build-mac.sh 再来"
    exit 1
fi

[ -d "${HOME}/.tke/wda/WebDriverAgentRunner-Runner.app" ] || [ -n "${TKE_WDA_APP:-}" ] || {
    bad "没有 WebDriverAgent —— 模拟器操作不了"
    note "装它：tke doctor --fix --profile ios"
    exit 1
}

# —— 准备：两台由你指定 ——
# 不自己挑：哪两台该拿来验、哪台上装着你要看的 App，脚本不知道；
# 猜错了还得从头再跑一遍。列出来让你选，比替你决定快
list_sims(){
    xcrun simctl list devices available -j | python3 -c "
import json,sys
for rt,l in json.load(sys.stdin)['devices'].items():
    if 'iOS' not in rt: continue
    ver=rt.rsplit('.',1)[-1].replace('iOS-','iOS ').replace('-','.')
    for x in l:
        mark='●' if x.get('state')=='Booted' else '○'
        print(f\"  {mark} {x['udid']}  {x['name']:<22} {ver}\")"
}
name_of(){
    xcrun simctl list devices available -j | python3 -c "
import json,sys
u=sys.argv[1]
for l in json.load(sys.stdin)['devices'].values():
    for x in l:
        if x['udid']==u: print(x['name']); raise SystemExit
print('')" "$1"
}

UDID_A="${1:-}"; UDID_B="${2:-}"
if [ -z "${UDID_A}" ] || [ -z "${UDID_B}" ]; then
    bad "用法: bash scripts/verify-sim-parallel.sh <UDID-A> <UDID-B>"
    note "● = 已启动    ○ = 关着（脚本会 boot）"
    list_sims
    note "挑**型号不同**的两台——同型号分辨率一样，第③步验不出串台"
    exit 1
fi
NAME_A=$(name_of "${UDID_A}"); NAME_B=$(name_of "${UDID_B}")
[ -n "${NAME_A}" ] || { bad "找不到这台: ${UDID_A}"; list_sims; exit 1; }
[ -n "${NAME_B}" ] || { bad "找不到这台: ${UDID_B}"; list_sims; exit 1; }
[ "${UDID_A}" != "${UDID_B}" ] || { bad "给的是同一台"; exit 1; }
note "A = ${NAME_A}  ${UDID_A}"
note "B = ${NAME_B}  ${UDID_B}"
if [ "${NAME_A}" = "${NAME_B}" ]; then
    bad "两台是同一个型号（${NAME_A}）—— 第③步分辨率一样，验不出串台"
    note "换一台别的型号再来；①②照样能验，但最关键的③会是歧义结果"
fi

for U in "${UDID_A}" "${UDID_B}"; do
    xcrun simctl boot "${U}" 2>/dev/null
done
open -a Simulator 2>/dev/null
for U in "${UDID_A}" "${UDID_B}"; do
    xcrun simctl bootstatus "${U}" -b >/dev/null 2>&1
done

# 各自把 WDA 拉起来（第一次会挤掉前台 App，这里不在意——验的是链路不是业务）。
# **失败要看得见**（INV-9）：早先这里 `>/dev/null 2>&1`，于是后面三步全红，
# 而真正的原因（WDA 没起来）被吞在这一行里
for U in "${UDID_A}" "${UDID_B}"; do
    # `>file 2>&1` 而不是只收 stderr：**tke 的错误走的是 stdout**
    #（`{"success":false,"error":…}`）。上一版只留 stderr，于是报错行后面空空如也
    if ! "${TKE}" -d "sim:${U}" fetch >"${TMPDIR:-/tmp}/tke-boot.log" 2>&1; then
        bad "把 WDA 拉进 ${U} 失败："
        sed 's/^/      /' "${TMPDIR:-/tmp}/tke-boot.log" | head -6
    fi
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
# ⚠️ 不能 `launchctl list <bundle>`：iOS 里 App 的 label 是
# `UIKitApplication:com.facebook.…[0x…][rb-legacy]`，精确查永远查不到
# （第一版就栽在这儿：两边 PID 都报 ?，归属校验形同虚设）。列全表按子串找
wda_pid(){ xcrun simctl spawn "$1" launchctl list 2>/dev/null \
             | grep -F "${BUNDLE}" | awk '{print $1}' | grep -E '^[0-9]+$' | head -1; }
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

# ── ③ 直接问各自的端口要一张截图，看是不是各自那台 ──（最关键）
#
# ⚠️ **不能用 `device info` 验这件事**：模拟器的机型/系统 tke 是问 simctl 拿的，
# 压根不经过 WDA——两台串了台它也照样报得对，等于什么都没验。
#
# 这里直接 curl WDA 的 `/screenshot`，**绕开 tke 的会话逻辑**：要验的是
# 「这个端口背后是哪台设备」，不该被"前台有没有 App""会话建没建起来"干扰。
# 型号不同 → 分辨率不同 → 串台一眼看得出来。
printf '\n%s③ 两个端口各给一张截图，分辨率各是各的%s\n' "${Y}" "${N}"
shot_size_via_wda(){
    curl -s --max-time 20 "http://127.0.0.1:$1/screenshot" | python3 -c "
import base64,json,struct,sys
try:
    b=base64.b64decode(json.load(sys.stdin)['value'])
    # PNG 的 IHDR 就在头 24 字节里：宽高各一个大端 u32
    w,h=struct.unpack('>II', b[16:24])
    print(f'{w}x{h}')
except Exception: print('')"
}
SIZE_A=$(shot_size_via_wda "${PORT_A}")
SIZE_B=$(shot_size_via_wda "${PORT_B}")
note "A ${NAME_A} (端口 ${PORT_A}) → ${SIZE_A:-（要不到截图）}"
note "B ${NAME_B} (端口 ${PORT_B}) → ${SIZE_B:-（要不到截图）}"
if [ -z "${SIZE_A}" ] || [ -z "${SIZE_B}" ]; then
    fail "有一边要不到截图 —— WDA 没起来？先看 ② 的 PID 有没有"
elif [ "${SIZE_A}" = "${SIZE_B}" ]; then
    bad "两边分辨率一样（${SIZE_A}）—— **可能串台，也可能这两台本来就同尺寸**"
    note "换一台屏幕明显不同的再验（比如 iPhone SE 配 iPhone 17 Pro Max）"
else
    ok "两个端口各连着自己那台（${SIZE_A} / ${SIZE_B}）—— 没串台"
fi

# ── ④ tke 层并发：两条命令同时跑，各自出各自的产物 ──
printf '\n%s④ tke 并发跑一步，证据各落各的目录%s\n' "${Y}" "${N}"
LOG_A="${HOME}/.tke/logs/par-a"; LOG_B="${HOME}/.tke/logs/par-b"
OUT_A="${TMPDIR:-/tmp}/tke-par-a.log"; OUT_B="${TMPDIR:-/tmp}/tke-par-b.log"
rm -rf "${LOG_A}" "${LOG_B}"
"${TKE}" -d "sim:${UDID_A}" steps "等待 [1s]" --log "${LOG_A}/" >"${OUT_A}" 2>&1 &
PID_A=$!
"${TKE}" -d "sim:${UDID_B}" steps "等待 [1s]" --log "${LOG_B}/" >"${OUT_B}" 2>&1 &
PID_B=$!
wait "${PID_A}"; wait "${PID_B}"
for pair in "par-a:${LOG_A}:${OUT_A}" "par-b:${LOG_B}:${OUT_B}"; do
    TAG="${pair%%:*}"; REST="${pair#*:}"; L="${REST%%:*}"; O="${REST##*:}"
    if [ -e "${L}/report.html" ]; then
        ok "${TAG} 有报告"
    else
        fail "${TAG} 没有报告，输出："
        sed 's/^/      /' "${O}" | head -6
    fi
done
note "两份报告：open ${LOG_A}/report.html ${LOG_B}/report.html"

printf '\n'
if [ "${FAILED}" = 0 ]; then
    ok "并行没问题"
else
    bad "上面有红的 —— 把那几行连同 ${STATE_DIR}/*.json 发我"
fi
exit "${FAILED}"
