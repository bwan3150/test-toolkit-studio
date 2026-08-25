#!/usr/bin/env bash
# 【守卫】远程白名单的宿主路径参数有没有漏登记（INV-16）。
#
# 为什么需要它：`src/serve/allowlist.rs` 里那张表是手写的，而 CLI 会长。
# 有人给某个已在白名单里的命令加一个吃路径的参数，远程就多了一条读写工作区外的路——
# 而且**加参数的人根本不会想到 serve**。所以让机器盯着：扫带 `#[arg(long…)]` 的
# PathBuf 参数，凡是既不在服务端注入清单里、又不在路径参数表里的，报红。
#
# 写这条守卫的当天它就抓到两个真洞（`refresh --out` / `control browser-download --dir`）。
# 改这个脚本前先造一个故意违规的现场，确认它真的会红（P-12）。
set -uo pipefail
cd "$(dirname "$0")/.."

ALLOWLIST="src/serve/allowlist.rs"
[ -f "$ALLOWLIST" ] || { echo "⚠️  找不到 $ALLOWLIST"; exit 0; }

# 不在远程白名单里的命令，它们的路径参数与 serve 无关
# （harness/security = AI 编排走任务层；doctor --fix/update/serve 本身 = 节点运维）
# cli/remote.rs 是**客户端**命令（tke remote pull --into 之类），跑在调用方本地，与节点白名单无关
EXEMPT='src/cli/remote.rs|src/cli/serve.rs|src/cli/fix.rs|src/cli/doctor.rs|src/cli/android_sdk.rs|src/cli/selfmanage.rs|src/cli/workflow/harness.rs|src/cli/security/'

# 只看**带 `#[arg(` 且 long 的结构体字段**：函数参数里的 PathBuf 不是 CLI 参数
# （位置参数没有旗标名，由 allowlist 的 path_first_arg 覆盖，这里本来就扫不到）
found="$(
  grep -rn --include='*.rs' -B4 -E "^[[:space:]]+(pub )?[a-z_]+:[[:space:]]*(Option<)?(std::path::)?PathBuf" src/cli src/main.rs \
  | awk -F: '
      /#\[arg\(/ && /long/ { seen=1; next }
      /PathBuf/ {
        if (seen) { print $1 "\t" $0 }
        seen=0
      }
      /^--$/ { seen=0 }
    '
)"

missing=0
while IFS=$'\t' read -r file rest; do
  [ -n "$file" ] || continue
  echo "$file" | grep -qE "$EXEMPT" && continue
  field="$(echo "$rest" | sed -E 's/.*[[:space:]]([a-z_]+)[[:space:]]*:[[:space:]]*(Option<)?(std::path::)?PathBuf.*/\1/')"
  flag="--$(echo "$field" | tr '_' '-')"
  if ! grep -q -- "\"$flag\"" "$ALLOWLIST"; then
    echo "❌ $file 的 \`$flag\` 取宿主路径，但 $ALLOWLIST 里既没登记成路径参数、也没进禁用清单"
    missing=$((missing+1))
  fi
done <<< "$found"

if [ "$missing" -gt 0 ]; then
  echo
  echo "怎么办：要么把它加进 allowlist.rs 对应命令的 path_flags（会被沙箱进会话工作区），"
  echo "        要么加进 BANNED_FLAGS（由服务端注入、不接受远程指定）。"
  exit 1
fi
echo "✅ 远程宿主路径参数登记齐全"
