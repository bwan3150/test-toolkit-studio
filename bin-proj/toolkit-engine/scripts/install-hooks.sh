#!/usr/bin/env bash
# 挂 pre-commit / pre-push hook 到 studio 仓库(只需跑一次)。
# hook 只在改动触及 bin-proj/toolkit-engine 时才执行 tke 守卫,不影响 app 侧提交。
set -euo pipefail
cd "$(dirname "$0")/../../.."   # → studio 仓库根
HOOKS=".git/hooks"
SCRIPTS="bin-proj/toolkit-engine/scripts"

cat > "$HOOKS/pre-commit" << 'HOOK'
#!/usr/bin/env bash
# tke 守卫:仅当本次提交触及 toolkit-engine 时执行
set -uo pipefail
staged=$(git diff --cached --name-only)
echo "$staged" | grep -q '^bin-proj/toolkit-engine/' || exit 0

S="bin-proj/toolkit-engine/scripts"
bash "$S/check-prompts.sh" || exit 1
bash "$S/check-linecount.sh"
bash "$S/check-state.sh"
# 编译检查(快,秒级增量)
( cd bin-proj/toolkit-engine && cargo check --no-default-features --quiet ) || {
  echo "❌ cargo check 未通过"; exit 1; }
HOOK

cat > "$HOOKS/pre-push" << 'HOOK'
#!/usr/bin/env bash
# tke 守卫(push 前):CHANGELOG + 全量测试。仅当待推提交触及 toolkit-engine 时执行。
set -uo pipefail
range_changed=$(git diff --name-only @{push}...HEAD 2>/dev/null || git diff --name-only origin/main...HEAD 2>/dev/null)
echo "$range_changed" | grep -q '^bin-proj/toolkit-engine/' || exit 0

S="bin-proj/toolkit-engine/scripts"
bash "$S/check-changelog.sh" || exit 1
( cd bin-proj/toolkit-engine && cargo test --no-default-features --lib --quiet ) || {
  echo "❌ 测试未通过,拒绝 push"; exit 1; }
( cd bin-proj/toolkit-engine && cargo test --no-default-features --test cli --quiet ) || {
  echo "❌ CLI 契约测试未通过,拒绝 push"; exit 1; }
HOOK

chmod +x "$HOOKS/pre-commit" "$HOOKS/pre-push"
echo "✅ hooks 已挂: $HOOKS/pre-commit, $HOOKS/pre-push"
