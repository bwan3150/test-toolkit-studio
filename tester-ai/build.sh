#!/bin/bash

# AI Tester 构建脚本

set -e

echo "开始构建 AI Tester..."

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 清理之前的构建
echo "清理之前的构建..."
cargo clean

# 构建 release 版本
echo "构建 release 版本..."
cargo build --release

# 检查构建结果
if [ -f "target/release/tester-ai" ]; then
    echo "✓ 构建成功!"
    echo "可执行文件位置: $SCRIPT_DIR/target/release/tester-ai"

    # 显示文件大小
    du -h target/release/tester-ai

    # 测试可执行文件
    echo ""
    echo "测试可执行文件..."
    ./target/release/tester-ai --help

    echo ""
    echo "构建完成!"
else
    echo "✗ 构建失败"
    exit 1
fi
