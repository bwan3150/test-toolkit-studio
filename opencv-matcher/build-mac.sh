#!/bin/bash
# OpenCV Matcher 打包脚本（独立模块）

set -e

echo "==============================================="
echo "开始打包 OpenCV Matcher..."
echo "==============================================="

# 获取脚本所在目录（opencv-matcher目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 读取版本号：package.json → BUILD_VERSION 环境变量
PACKAGE_JSON="$SCRIPT_DIR/../package.json"
if [ ! -f "$PACKAGE_JSON" ]; then
    echo "Error: package.json not found at $PACKAGE_JSON"
    exit 1
fi

PKG_VERSION=$(grep '"version"' "$PACKAGE_JSON" | head -1 | sed -E 's/.*"version": *"([^"]+)".*/\1/')
if [ -z "$PKG_VERSION" ]; then
    echo "Error: cannot extract version from $PACKAGE_JSON"
    exit 1
fi

# 导出 BUILD_VERSION 环境变量
export BUILD_VERSION="$PKG_VERSION"
echo "Build version: $BUILD_VERSION"

# 生成 _version.py 文件（打包时嵌入版本号）
echo "# 自动生成的版本文件 - 请勿手动修改" > _version.py
echo "__version__ = '$BUILD_VERSION'" >> _version.py
echo "✓ 已生成 _version.py: $BUILD_VERSION"

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

# 清理生成的 _version.py
rm -f _version.py

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

# 验证二进制文件能否运行
if "$OUTPUT_DIR/tke-opencv" --version > /dev/null 2>&1; then
    echo "✓ tke-opencv --version successful"
else
    echo "⚠ Warning: tke-opencv might not be executable"
fi

echo ""
echo "==============================================="
echo "OpenCV Matcher 打包完成！"
echo "==============================================="
