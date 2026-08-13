#!/usr/bin/env bash
# 大文件预警(warn,不阻断):src/**/*.rs 超 800 行提示拆分(CLAUDE.md:大文件立刻拆)。
set -uo pipefail
cd "$(dirname "$0")/.."   # → toolkit-engine

over=$(find src -name '*.rs' -exec wc -l {} + | awk '$1>800 && $2!="total" {print $1, $2}' | sort -rn)
if [ -n "$over" ]; then
  echo "⚠️  超 800 行的文件(考虑按职责拆,不要按行数硬切):"
  echo "$over" | sed 's/^/     /'
else
  echo "✅ 无超长文件"
fi
exit 0
