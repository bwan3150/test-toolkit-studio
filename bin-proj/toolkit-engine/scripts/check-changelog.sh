#!/usr/bin/env bash
# 改了 toolkit-engine 的 src/** 就必须同时改它的 CHANGELOG.md(pre-push 跑)
set -uo pipefail
cd "$(dirname "$0")/../../.."   # → studio 仓库根

base="${1:-origin/refactor/toolkit-system}"
git rev-parse --verify "$base" >/dev/null 2>&1 || base="origin/main"
git rev-parse --verify "$base" >/dev/null 2>&1 || { echo "⏭  无远端基线,跳过"; exit 0; }

changed=$(git diff --name-only "$base"...HEAD)
touches_code=$(echo "$changed" | grep -E '^bin-proj/toolkit-engine/src/' || true)
touches_log=$(echo "$changed"  | grep -E '^bin-proj/toolkit-engine/CHANGELOG\.md$' || true)

if [ -n "$touches_code" ] && [ -z "$touches_log" ]; then
  echo "❌ 改了 toolkit-engine/src 但没追加 CHANGELOG.md"
  echo "$touches_code" | head -5 | sed 's/^/     /'
  echo "     → 追加一条(不要重写已有条目)"
  exit 1
fi
echo "✅ CHANGELOG 检查通过"
