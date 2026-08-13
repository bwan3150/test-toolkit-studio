#!/usr/bin/env bash
# STATE.md 的 Last-Commit 与 HEAD 漂移检查。warn 级别:不阻断,提示上个会话可能没正常收尾。
set -uo pipefail
cd "$(dirname "$0")/../../.."   # → studio 仓库根

S="bin-proj/toolkit-engine/docs/state/STATE.md"
[ -f "$S" ] || { echo "⚠️  $S 不存在"; exit 0; }
declared=$(grep -m1 '^Last-Commit:' "$S" | awk '{print $2}')
head_sha=$(git rev-parse --short=8 HEAD 2>/dev/null || echo "")
prev_sha=$(git rev-parse --short=8 HEAD~1 2>/dev/null || echo "")
[ -z "$head_sha" ] && exit 0

if [ "$declared" = "$head_sha" ] || [ "$declared" = "$prev_sha" ]; then
  echo "✅ STATE.md 与 HEAD 一致($declared)"
else
  echo "⚠️  STATE.md 漂移:Last-Commit=$declared,HEAD=$head_sha"
  echo "    上个会话可能没正常收尾——先读 docs/state/HANDOFF.md 再动手"
fi
exit 0
