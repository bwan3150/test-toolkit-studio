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
#   ③ 给两台开不同 App，各端口报的前台各是各的（内容层面的证据）  ← 最关键
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
    # 这一步**只为把 WDA 拉起来**，fetch 本身成不成不重要：拉起 runner 必然把前台
    # App 挤走，于是采集会报"现在前台是桌面"——那是意料之中的，不是故障。
    # ③ 随后会把两台的前台各自拉成一个真 App。
    # 但真起不来的情况要看得见，所以照样打出来（`>file 2>&1`：**tke 的错误走 stdout**）
    BOOT_LOG="${TMPDIR:-/tmp}/tke-boot.log"
    if ! "${TKE}" -d "sim:${U}" fetch >"${BOOT_LOG}" 2>&1; then
        if grep -q "前台是桌面\|前台是 WebDriverAgent" "${BOOT_LOG}"; then
            note "${U} 的 WDA 起来了（前台被挤成桌面，③ 会把 App 拉回来）"
        else
            bad "把 WDA 拉进 ${U} 可能失败了："
            sed 's/^/      /' "${BOOT_LOG}" | head -6
        fi
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

# ── ③ 给两台开**不同的 App**，再问各自的端口"你前台是谁" ──（最关键）
#
# ⚠️ **不能用 `device info` 验这件事**：模拟器的机型/系统 tke 是问 simctl 拿的，
# 压根不经过 WDA——两台串了台它也照样报得对，等于什么都没验。
#
# ⚠️ **也不能只比分辨率**：用户实测 iPhone 17 Pro 与 16 Pro 都是 1206×2622，
# 两边一样根本判不出是串台还是本来就同尺寸。改成给两台开不同的 App——
# 一台设置、一台 Safari，然后问每个端口的 WDA「你前台是哪个 bundle id」。
# 这是**内容层面**的证据，跟型号、尺寸都无关。
printf '\n%s③ 两台开不同 App，各端口报的前台各是各的%s\n' "${Y}" "${N}"
APP_A=com.apple.Preferences      # 设置
APP_B=com.apple.mobilesafari     # Safari
LAUNCH_LOG="${TMPDIR:-/tmp}/tke-launch.log"
: >"${LAUNCH_LOG}"
xcrun simctl launch "${UDID_A}" "${APP_A}" >>"${LAUNCH_LOG}" 2>&1 || note "A 起 ${APP_A} 没成"
xcrun simctl launch "${UDID_B}" "${APP_B}" >>"${LAUNCH_LOG}" 2>&1 || note "B 起 ${APP_B} 没成"
sleep 2

active_of(){
    curl -s --max-time 15 "http://127.0.0.1:$1/wda/activeAppInfo" | python3 -c "
import json,sys
try: print(json.load(sys.stdin)['value'].get('bundleId',''))
except Exception: print('')"
}
FG_A=$(active_of "${PORT_A}"); FG_B=$(active_of "${PORT_B}")
note "A 端口 ${PORT_A} 前台 → ${FG_A:-（问不到）}   期望 ${APP_A}"
note "B 端口 ${PORT_B} 前台 → ${FG_B:-（问不到）}   期望 ${APP_B}"
# 判据是**两个端口报的不是同一个 App**——串台的话它们必然报同一个。
# 至于前台是不是我们刚 launch 的那个,不重要:某台上原本就开着别的 App、
# launch 被系统挡下、或者 App 自己又切回去了,都会让它跟期望不符,
# 但只要两边不同,"这两个端口连的是两台设备"就已经成立（用户实测撞上:
# A 报的是它原本开着的 com.example.app,而不是刚 launch 的设置）
if [ -z "${FG_A}" ] || [ -z "${FG_B}" ]; then
    fail "有一边问不到前台 —— WDA 没起来？先看 ②"
elif [ "${FG_A}" = "${FG_B}" ]; then
    fail "两个端口报的是同一个 App（${FG_A}）—— **串台了**"
else
    ok "两个端口报的前台不是同一个 —— 没串台"
    if [ "${FG_A}" != "${APP_A}" ] || [ "${FG_B}" != "${APP_B}" ]; then
        note "（前台跟刚 launch 的那个不一样,不影响结论;launch 的输出在 ${LAUNCH_LOG}）"
    fi
fi

# 附带一条：分辨率。同尺寸机型上它判不了串台，但截图能不能要得到本身也是个信息
shot_size_via_wda(){
    curl -s --max-time 20 "http://127.0.0.1:$1/screenshot" | python3 -c "
import base64,json,struct,sys
try:
    b=base64.b64decode(json.load(sys.stdin)['value'])
    w,h=struct.unpack('>II', b[16:24])   # PNG 的宽高就在头 24 字节的 IHDR 里
    print(f'{w}x{h}')
except Exception: print('')"
}
note "截图尺寸 A $(shot_size_via_wda "${PORT_A}")  B $(shot_size_via_wda "${PORT_B}")"

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
    if [ -e "${L}/screenshots" ]; then
        ok "${TAG} 有报告与截图"
    elif [ -e "${L}/report.html" ]; then
        # 报告有、截图没有 = 采集那步失败了。③ 已经把两台的前台都拉成真 App，
        # 这时还没截图，就不是"前台是桌面"那个老原因了
        fail "${TAG} 有报告但**没有截图**，输出："
        sed 's/^/      /' "${O}" | head -6
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
