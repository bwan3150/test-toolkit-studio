#!/bin/bash
# 构建 macOS 版本的 scrcpy-server

set -e

echo "开始编译 scrcpy-server (macOS arm64)..."

# 编译 release 版本
cargo build --release

# 复制到 resources 目录
echo "复制到 resources/darwin/scrcpy-server/..."
mkdir -p ../resources/darwin/scrcpy-server

# 复制二进制文件
cp target/release/tke-scrcpy ../resources/darwin/scrcpy-server/

# 复制 vendor 目录（包含 scrcpy-server.jar）
cp -r vendor ../resources/darwin/scrcpy-server/

echo ""
echo "✅ 编译完成！"
echo "二进制文件: ../resources/darwin/scrcpy-server/tke-scrcpy"
ls -lh ../resources/darwin/scrcpy-server/tke-scrcpy
echo ""
echo "vendor 目录: ../resources/darwin/scrcpy-server/vendor/"
ls -lh ../resources/darwin/scrcpy-server/vendor/Genymobile/scrcpy/
