#!/usr/bin/env bash
# 【真机 e2e 冒烟】需要:已构建的 tke、连着的设备、一个能跑通的两件套(foo.tks+foo.tklib)。
# CI 跑不了(要设备),需要的时候手动跑一遍:
#   ./tests/e2e/smoke.sh <case.tks> [device]
# 验证:①正常回放跑通 ②--copilot false 关闭辅助驾驶也跑通(区分"脚本本身好"vs"AI 救活的")
set -uo pipefail

TKS="${1:?用法: smoke.sh <case.tks> [device]}"
DEVICE="${2:-}"
TKE_BIN="${TKE_BIN:-tke}"

command -v "$TKE_BIN" >/dev/null || { echo "❌ 找不到 tke($TKE_BIN);先 ./build-mac.sh 或设 TKE_BIN"; exit 1; }
[ -f "$TKS" ] || { echo "❌ 脚本不存在: $TKS"; exit 1; }
[ -f "${TKS%.tks}.tklib" ] || { echo "❌ 缺同名 .tklib(两件套)"; exit 1; }

DARGS=(); [ -n "$DEVICE" ] && DARGS=(-d "$DEVICE")
pass=0; fail=0
run_case() {
  local name="$1"; shift
  echo "── e2e: $name"
  if "$TKE_BIN" run "$TKS" "${DARGS[@]}" "$@"; then
    echo "✅ $name"; pass=$((pass+1))
  else
    echo "❌ $name"; fail=$((fail+1))
  fi
  echo
}

run_case "回放(辅助驾驶开,默认)"
run_case "回放(辅助驾驶关)" --copilot false

echo "═══ e2e 结果: $pass 过 / $fail 挂 ═══"
[ "$fail" -eq 0 ]
