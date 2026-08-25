#!/usr/bin/env bash
# 【远程 e2e 冒烟】ADR-0022 P1:证明**接口调用真的把设备操作了**。
#
# 与 tests/serve.rs 的分工:那边测协议(鉴权/路由/租约/沙箱),整条链路里唯独设备是假的;
# 这里补上唯一缺的那一环——起 serve、租一台真设备、纯靠 HTTP 把它开起来、操作、
# 采集、下载证据、释放并确认**复位真的发生了**(INV-17)。
#
#   ./tests/e2e/serve-smoke.sh [web|android|ios] [URL]
#
# 前提:已构建的 tke + 对应平台能用(web 要 chromedriver+Chrome;android 要连着设备)。
# 前提不满足就说清楚再退出——不许假绿。
set -uo pipefail

PLATFORM="${1:-web}"
URL="${2:-https://example.com}"
TKE_BIN="${TKE_BIN:-tke}"
TOKEN="e2e-$RANDOM$RANDOM"

command -v "$TKE_BIN" >/dev/null || { echo "❌ 找不到 tke($TKE_BIN);先 ./build-mac.sh 或设 TKE_BIN"; exit 1; }
command -v python3 >/dev/null || { echo "❌ 需要 python3 解析 JSON"; exit 1; }
command -v curl >/dev/null || { echo "❌ 需要 curl"; exit 1; }

ROOT="$(mktemp -d -t tke-serve-e2e-XXXXXX)"
LOG="$ROOT/serve.log"
SID=""; PORT=""

cleanup() {
  # 会话没释放就释放掉,否则设备会被租约占到 TTL 到期
  [ -n "$SID" ] && curl -s -X DELETE -H "Authorization: Bearer $TOKEN" "$BASE/v1/sessions/$SID" >/dev/null 2>&1
  [ -n "${SERVE_PID:-}" ] && kill "$SERVE_PID" 2>/dev/null
  wait "${SERVE_PID:-}" 2>/dev/null
  echo "（服务端日志: ${LOG}，会话目录: ${ROOT}）"
}
trap cleanup EXIT

jq_() { python3 -c "import sys,json;d=json.load(sys.stdin);print(eval('d'+sys.argv[1]))" "$1" 2>/dev/null; }
api() { # api <METHOD> <PATH> [BODY]
  local m="$1" p="$2" b="${3:-}"
  if [ -n "$b" ]; then
    curl -s -X "$m" -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d "$b" "$BASE$p"
  else
    curl -s -X "$m" -H "Authorization: Bearer $TOKEN" "$BASE$p"
  fi
}

pass=0; fail=0
ok()  { echo "✅ $1"; pass=$((pass+1)); }
bad() { echo "❌ $1"; fail=$((fail+1)); }
step() { echo; echo "── $1"; }

# ── 起服务(--port 0,从监听行拿真实端口)
"$TKE_BIN" serve --port 0 --token "$TOKEN" --root "$ROOT" --web-slots 1 >"$LOG" 2>&1 &
SERVE_PID=$!
for _ in $(seq 1 50); do
  PORT="$(head -1 "$LOG" 2>/dev/null | python3 -c "import sys,json
try: print(json.loads(sys.stdin.readline())['listening'].rsplit(':',1)[1])
except Exception: pass" 2>/dev/null)"
  [ -n "$PORT" ] && break
  sleep 0.2
done
[ -n "$PORT" ] || { echo "❌ serve 没起来,日志:"; cat "$LOG"; exit 1; }
BASE="http://127.0.0.1:$PORT"
ok "serve 起在 $BASE"

# ── 这个节点到底能不能测这个平台:不能就说清楚再退,别跑出一堆看不懂的错
step "设备清单"
DEVS="$(api GET /v1/devices)"
echo "$DEVS" | python3 -m json.tool 2>/dev/null | head -30
HAVE="$(echo "$DEVS" | python3 -c "import sys,json;print(any(d['platform']=='$PLATFORM' and d['available'] for d in json.load(sys.stdin)['devices']))")"
[ "$HAVE" = "True" ] || { echo "❌ 本节点没有可用的 $PLATFORM 设备——先 tke doctor 看缺什么(不是假绿,是真的没环境)"; exit 1; }

# ── 租一台
step "租设备"
S="$(api POST /v1/sessions "{\"capabilities\":{\"platform\":\"$PLATFORM\"},\"ttl_s\":900}")"
SID="$(echo "$S" | jq_ "['session_id']")"
DEV="$(echo "$S" | jq_ "['device']['label']")"
[ -n "$SID" ] || { echo "❌ 租不到设备: $S"; exit 1; }
ok "租到 ${DEV}（会话 ${SID}）"

exec_() { # exec_ <描述> <JSON 数组 argv>
  local what="$1" argv="$2"
  local r; r="$(api POST "/v1/sessions/$SID/exec" "{\"argv\":$argv,\"timeout_s\":120}")"
  local code; code="$(echo "$r" | jq_ "['exit_code']")"
  local ms; ms="$(echo "$r" | jq_ "['timing']['total_ms']")"
  if [ "$code" = "0" ]; then
    ok "${what}（${ms}ms）"
  else
    bad "$what → exit=$code"
    echo "$r" | head -c 900; echo
  fi
}

# ── 真的操作设备
step "经 HTTP 操作设备"
# 证据是 `steps` 产的(--log 就是任务目录,反复调用续写)——
# `control`/`refresh` 只把中间产物写进 cache,不进证据目录。这条分工在远程尤其要紧:
# 调用方只能通过 artifacts 接口看东西,不落进任务目录的等于不存在
case "$PLATFORM" in
  web)
    exec_ "起浏览器"       '["control","boot"]'
    exec_ "打开网页(留证据)" "[\"steps\",\"启动 [\\\"$URL\\\"]\",\"等待 [1s]\"]"
    exec_ "取元素表"       '["fetch","--interactive"]'
    ;;
  android)
    # HOME 键是最无害的"真的动了设备"——不改数据、不动账号状态(INV-12 的分寸)
    exec_ "按 HOME(留证据)" '["steps","按键 [\"KEYCODE_HOME\"]","等待 [1s]"]'
    exec_ "取元素表"        '["fetch","--interactive"]'
    ;;
  ios)
    # iOS 只有 ENTER/BACK,BACK 可能把人带离当前页——这里只等一拍,证据照样落
    exec_ "采集当前页(留证据)" '["steps","等待 [1s]"]'
    exec_ "取元素表"        '["fetch","--interactive"]'
    ;;
esac

# ── 证据必须真的落在会话目录里,而且能下回来
step "证据"
FILES="$(api GET "/v1/sessions/$SID/artifacts/logs?list=true")"
N="$(echo "$FILES" | python3 -c "import sys,json;print(len(json.load(sys.stdin).get('files',[])))" 2>/dev/null || echo 0)"
if [ "${N:-0}" -gt 0 ]; then
  ok "会话目录里有 $N 个证据文件"
  SHOT="$(echo "$FILES" | python3 -c "
import sys,json
fs=[f for f in json.load(sys.stdin)['files'] if f.endswith('.png')]
print(fs[0] if fs else '')")"
  if [ -n "$SHOT" ]; then
    curl -s -H "Authorization: Bearer $TOKEN" "$BASE/v1/sessions/$SID/artifacts/$SHOT" -o "$ROOT/got.png"
    # 截图能下回来、且不是空文件 —— 远程要靠它当证据,空文件等于没有
    if [ -s "$ROOT/got.png" ]; then ok "截图下载回来了（$(wc -c <"$ROOT/got.png") 字节）"; else bad "截图下下来是空的"; fi
  else
    bad "没有截图落盘——远程证据链断在这里"
  fi
else
  bad "会话目录里什么都没有: $FILES"
fi

# ── 释放要真的复位(INV-17):web 那条是关掉浏览器会话
step "释放与复位"
R="$(api DELETE "/v1/sessions/$SID")"; SID=""
echo "$R" | python3 -m json.tool 2>/dev/null | head -20
RESET_OK="$(echo "$R" | python3 -c "
import sys,json
d=json.load(sys.stdin); a=d.get('reset',{}).get('actions',[])
print('none' if not a else all(x.get('ok') for x in a))")"
case "$PLATFORM:$RESET_OK" in
  web:True)  ok "浏览器已复位（下一个租户不会接手上一个人的会话）";;
  web:*)     bad "web 释放时必须关掉浏览器会话，实际: $RESET_OK";;
  *:none)    ok "无需复位动作（这次没启动过 App）";;
  *:True)    ok "复位动作全部成功";;
  *)         bad "复位有失败项: $RESET_OK";;
esac

# ── 设备要回池
AVAIL="$(api GET /v1/devices | python3 -c "import sys,json;print(all(d['available'] for d in json.load(sys.stdin)['devices']))")"
[ "$AVAIL" = "True" ] && ok "设备已回池" || bad "释放后设备没回池"

echo
echo "═══ 远程 e2e: $pass 过 / $fail 挂 ═══"
[ "$fail" -eq 0 ]
