#!/usr/bin/env bash
# 守卫：`"$VAR中文"` —— macOS 自带的 bash 3.2 会把多字节字符的头几个字节
# 吃进变量名，于是变量展开成空、后面的字符也烂掉。
#
#   注意: 你敲 tke 用的是 ��不是刚构建的这个）      ← 路径没了
#   line 28: WDA_REF�: unbound variable            ← set -u 下直接崩
#
# **Linux 的 bash 5 两种写法都对**，所以这类问题在开发机上永远测不出来，
# 只会在用户的 mac 上现形（P-42 记过一次，然后又犯了一次——所以才有这个守卫）。
#
# 修法：写 `${VAR}`，把边界标出来。
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/.."

# 只看**会被展开**的地方：注释行里的 $ORIGIN 之类不算
HITS=$(grep -rnP '\$[A-Za-z_][A-Za-z0-9_]*[^\x00-\x7F]' --include="*.sh" . 2>/dev/null \
       | grep -v '^\./target' \
       | awk -F: '{ line=$0; sub(/^[^:]*:[^:]*:/, "", line);
                    sub(/^[ \t]*/, "", line);
                    if (substr(line,1,1) != "#") print }')

if [ -z "$HITS" ]; then
    echo "✅ shell 变量边界检查通过"
    exit 0
fi
echo "⚠️  \$VAR 后面紧跟中文（macOS bash 3.2 会吃字节，写成 \${VAR}）："
printf '%s\n' "$HITS"
exit 1
