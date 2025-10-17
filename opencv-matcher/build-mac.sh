#!/bin/bash
# OpenCV Matcher 打包脚本（独立模块）

set -e

echo "==============================================="
echo "开始打包 OpenCV Matcher..."
echo "==============================================="

# 获取脚本所在目录（opencv-matcher目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 获取当前操作系统
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

# 确定输出目录（项目根目录的 resources）
OUTPUT_DIR="../resources/${OS}/toolkit-engine"
mkdir -p "$OUTPUT_DIR"

# 激活 uv 环境并打包
echo "同步依赖（包括 dev 依赖）..."
uv sync --group dev

echo "使用 PyInstaller 打包..."
.venv/bin/pyinstaller \
    --onefile \
    --name tke-opencv \
    --clean \
    --noconfirm \
    opencv_matcher.py

# 检查打包是否成功
if [ ! -f "dist/tke-opencv" ]; then
    echo "❌ 打包失败：未找到 dist/tke-opencv"
    exit 1
fi

# 复制到目标目录
echo "复制到: $OUTPUT_DIR/tke-opencv"
cp dist/tke-opencv "$OUTPUT_DIR/tke-opencv"

# 添加可执行权限
chmod +x "$OUTPUT_DIR/tke-opencv"

# 获取文件大小
SIZE=$(du -h "$OUTPUT_DIR/tke-opencv" | cut -f1)
echo "✓ 打包成功"
echo "  输出: $OUTPUT_DIR/tke-opencv"
echo "  大小: $SIZE"

# 测试可执行文件
echo ""
echo "测试可执行文件..."
"$OUTPUT_DIR/tke-opencv" 2>&1 | head -1 || true

echo ""
echo "==============================================="
echo "OpenCV Matcher 打包完成！"
echo "==============================================="
