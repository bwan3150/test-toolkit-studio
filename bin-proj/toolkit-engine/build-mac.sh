#!/bin/bash

# 构建 Toolkit Engine (TKE) 并复制到资源目录
set -e  # 遇到错误立即退出

# 获取脚本所在目录（toolkit-engine目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "==============================="


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

# 导出 BUILD_VERSION 环境变量供 build.rs 使用
export BUILD_VERSION="$PKG_VERSION"
echo "Build version: $BUILD_VERSION"

echo "Building Toolkit Engine..."

# 构建 release 版本
cargo build --release

# 检测平台
OS=$(uname)
case "$OS" in
    Darwin)
        PLATFORM="darwin"
        BINARY_NAME="tke"
        ;;
    Linux)
        PLATFORM="linux"
        BINARY_NAME="tke"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLATFORM="win32"
        BINARY_NAME="tke.exe"
        ;;
    *)
        echo "Not supported for: $OS"
        exit 1
        ;;
esac

echo "OS: $PLATFORM"

# 源文件路径
SOURCE_BINARY="$SCRIPT_DIR/target/release/$BINARY_NAME"

# 目标目录和文件路径
TARGET_DIR="$SCRIPT_DIR/../resources/$PLATFORM/toolkit-engine"
TARGET_BINARY="$TARGET_DIR/$BINARY_NAME"

# 检查源文件是否存在
if [ ! -f "$SOURCE_BINARY" ]; then
    echo "Error：build fault, cannot find: $SOURCE_BINARY"
    exit 1
fi

# 创建目标目录
mkdir -p "$TARGET_DIR"

# 复制二进制文件
cp "$SOURCE_BINARY" "$TARGET_BINARY"

# 给二进制文件添加执行权限（Linux/macOS）
if [[ "$OS" != MINGW* && "$OS" != MSYS* && "$OS" != CYGWIN* ]]; then
    chmod +x "$TARGET_BINARY"
fi

echo "Build successfully"
echo "Cp to: $TARGET_BINARY"
echo "Szie: $(du -h "$TARGET_BINARY" | cut -f1)"

# 验证二进制文件能否运行
if "$TARGET_BINARY" --version > /dev/null 2>&1; then
    echo "tke --version successful"
else
    echo "Warning: tke might not be executable"
fi

echo ""
echo "==============================="
echo "TKE Build Finished"
echo "==============================="
echo ""
