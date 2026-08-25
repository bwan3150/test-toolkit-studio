#!/usr/bin/env bash
# tke security 侦察底座 · macOS 一键冒烟
# ---------------------------------------------------------------------------
# 构建 tke → 依次跑 http + 七个 recon verb → 证据落进一个任务目录 → 打印结果。
#
# 用法：
#   ./tests/security-smoke.sh [目标URL] [--no-build] [--graphql <url>] [--bundle <url>]
#   默认目标 https://example.com（只读被动检查，安全）。
#
# ⚠️ 只对你**有授权**的目标跑。这些检查都是只读 GET（safe 档），不写入不破坏；
#    但仍会向目标发若干请求（endpoints 会探 .env/.git 等路径）——别拿去打不属于你的站。
#
# 跑的是**刚构建的** bin/<platform>/tke，不是你 PATH 里日常那个（两者常是不同文件）。
set -uo pipefail

# ── 解析参数 ────────────────────────────────────────────────────────────
TARGET="https://example.com"
DO_BUILD=1
GRAPHQL_URL=""
BUNDLE_URL=""
while [ $# -gt 0 ]; do
  case "$1" in
    --no-build) DO_BUILD=0; shift ;;
    --graphql)  GRAPHQL_URL="${2:-}"; shift 2 ;;
    --bundle)   BUNDLE_URL="${2:-}"; shift 2 ;;
    http*)      TARGET="$1"; shift ;;
    *) echo "未知参数: $1"; exit 2 ;;
  esac
done
: "${GRAPHQL_URL:=$TARGET}"
: "${BUNDLE_URL:=$TARGET}"

# ── 路径 ────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"          # toolkit-engine
STUDIO_DIR="$(cd "$TE_DIR/../.." && pwd)"        # studio 仓库根

ARCH="$(uname -m)"; case "$ARCH" in arm64|aarch64) A=arm64 ;; x86_64|amd64) A=amd64 ;; *) echo "不支持的架构 $ARCH"; exit 1 ;; esac
OS="$(uname)"; case "$OS" in Darwin) P="darwin-$A" ;; Linux) P="linux-$A" ;; *) echo "此脚本给 macOS/Linux 用"; exit 1 ;; esac
TKE="$STUDIO_DIR/bin/$P/tke"

# ── 构建 ────────────────────────────────────────────────────────────────
if [ "$DO_BUILD" -eq 1 ]; then
  echo "▶ 构建 tke（build-mac.sh，产物 → bin/$P/tke）"
  ( cd "$TE_DIR" && ./build-mac.sh ) || { echo "✗ 构建失败"; exit 1; }
fi
[ -x "$TKE" ] || { echo "✗ 找不到构建产物 $TKE —— 去掉 --no-build 先构建"; exit 1; }
echo "✔ 使用二进制: $TKE"
echo "  版本: $("$TKE" --version 2>/dev/null || echo '?')"

# ── 任务目录（证据落这里）───────────────────────────────────────────────
TASK="$(mktemp -d "${TMPDIR:-/tmp}/tke-sec-smoke.XXXXXX")"
echo "✔ 任务目录: $TASK"
echo "✔ 目标: $TARGET"
echo

# JSON 美化：有 jq 用 jq，否则 python3，再否则原样
pretty() { if command -v jq >/dev/null; then jq .; elif command -v python3 >/dev/null; then python3 -m json.tool; else cat; fi; }

run() {   # run <标题> <参数...>
  local title="$1"; shift
  echo "════════════════════════════════════════════════"
  echo "▶ $title"
  echo "  \$ tke --log <task> $*"
  echo "────────────────────────────────────────────────"
  "$TKE" --log "$TASK" "$@" 2>/dev/null | pretty
  echo
}

# ── 逐条跑 ──────────────────────────────────────────────────────────────
run "http GET（原始探测）"          http GET "$TARGET"
run "recon headers（安全响应头）"    recon headers "$TARGET"
run "recon fingerprint（技术指纹）"  recon fingerprint "$TARGET"
run "recon cors（跨域配置）"         recon cors "$TARGET"
run "recon tls（传输层·轻量）"       recon tls "$TARGET"
run "recon endpoints（敏感路径）"    recon endpoints "$TARGET"
run "recon graphql（introspection）" recon graphql "$GRAPHQL_URL"
run "recon bundle（密钥扫描）"       recon bundle "$BUNDLE_URL"

# ── 证据一览 ────────────────────────────────────────────────────────────
echo "════════════════════════════════════════════════"
echo "▶ 证据落盘（$TASK/evidence）"
echo "────────────────────────────────────────────────"
if [ -d "$TASK/evidence" ]; then
  ls -1 "$TASK/evidence" | sed 's/^/  /'
  echo
  echo "  共 $(ls -1 "$TASK/evidence" | wc -l | tr -d ' ') 个文件；看某条：cat $TASK/evidence/step_001_resp.txt"
else
  echo "  （无证据——检查是否都失败了）"
fi
echo
echo "✔ 完成。graphql/bundle 建议单独指真实端点/JS："
echo "    ./tests/security-smoke.sh $TARGET --no-build --graphql https://<host>/graphql --bundle https://<host>/app.js"
