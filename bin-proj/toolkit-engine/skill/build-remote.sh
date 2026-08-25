#!/usr/bin/env bash
# 【生成 remote skill】远程版 = 一份小 delta + **本地版正文原样内联**。
#
# 为什么要生成而不是各写一份：ADR-0022 D4 选二进制客户端就是赌「文档不分叉」，
# 590 行踩坑册各维护一份的话，这个赌注三个月内就会输（两份必然漂）。
# 所以正文只有一处源头（`skill/<名>/SKILL.md`），远程版只维护 `remote-delta/<名>-remote.md`
# 里的差异——覆盖表 + 连接方式。**内联是逐字节复制，结构上不可能漂**。
#
#   ./build-remote.sh          # 生成 skill/<名>-remote/
#   ./build-remote.sh --clean  # 删掉生成物
#
# 生成物不进 git（跟 VERSION 一样是发布产物）；publish.sh 打包前会自己跑一遍。
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DELTA_DIR="$SCRIPT_DIR/remote-delta"

if [ "${1:-}" = "--clean" ]; then
    for d in "$SCRIPT_DIR"/*-remote/; do [ -d "$d" ] && rm -rf "$d" && echo "   🗑  $(basename "$d")"; done
    exit 0
fi

[ -d "$DELTA_DIR" ] || { echo "❌ 找不到 $DELTA_DIR"; exit 1; }

made=0
for delta in "$DELTA_DIR"/*.md; do
    [ -f "$delta" ] || continue
    remote_name="$(basename "$delta" .md)"          # tke-ui-test-remote
    base_name="${remote_name%-remote}"              # tke-ui-test
    base="$SCRIPT_DIR/$base_name/SKILL.md"
    if [ ! -f "$base" ]; then
        echo "❌ $remote_name 找不到本地版正文：$base"; exit 1
    fi

    out="$SCRIPT_DIR/$remote_name"
    rm -rf "$out"; mkdir -p "$out"

    # delta（自带 frontmatter）+ 本地版正文（**去掉它的 frontmatter**，只留正文）
    cat "$delta" > "$out/SKILL.md"
    awk 'BEGIN{fm=0} /^---$/{fm++; next} fm>=2{print}' "$base" >> "$out/SKILL.md"

    # reference/ 原样带过去：踩坑册在远程一字不差地适用
    [ -d "$SCRIPT_DIR/$base_name/reference" ] && cp -R "$SCRIPT_DIR/$base_name/reference" "$out/"

    lines=$(wc -l < "$out/SKILL.md" | tr -d ' ')
    echo "   ✅ ${remote_name}（${lines} 行 = delta + ${base_name} 正文）"
    made=$((made+1))
done

[ "$made" -gt 0 ] || { echo "❌ 一个都没生成"; exit 1; }
