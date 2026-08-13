#!/usr/bin/env bash
# 提示词登记一致性:builtin/**/*.md 必须在 defaults.rs 里 include_str 登记。
# 漏登记 = 运行时静默发空串给 LLM(PITFALLS P-05)。pre-commit 跑,阻断。
set -uo pipefail
cd "$(dirname "$0")/.."   # → toolkit-engine

P="src/workflow/agent/prompt"
[ -d "$P/builtin" ] || { echo "⏭  builtin 目录不存在,跳过"; exit 0; }

missing=0
while IFS= read -r f; do
  rel="${f#"$P/"}"   # builtin/messages/verify/heal_pick.md
  if ! grep -rqF "include_str!(\"$rel\")" "$P"/*.rs; then
    echo "❌ 未在 defaults.rs 登记: $rel"
    missing=1
  fi
done < <(find "$P/builtin" -name '*.md' | sort)

[ "$missing" -eq 1 ] && { echo "   → 补 include_str 登记,否则运行时静默空提示词(P-05)"; exit 1; }
echo "✅ 提示词登记一致($(find "$P/builtin" -name '*.md' | wc -l | tr -d ' ') 个)"
