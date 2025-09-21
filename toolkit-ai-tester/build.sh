#!/bin/bash

# 构建 AI自动化测试员 (ai-tester)
set -e  # 遇到错误立即退出

# 获取脚本所在目录（toolkit-ai-tester目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "==============================="
echo "构建 AI自动化测试员 (ai-tester)"
echo "==============================="

# 同步版本号：package.json → Cargo.toml
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
echo "Sync version: $PKG_VERSION"

CARGO_TOML="$SCRIPT_DIR/Cargo.toml"
if [ ! -f "$CARGO_TOML" ]; then
    echo "Error: Cargo.toml not found at $CARGO_TOML"
    exit 1
fi

# 跨平台 sed 替换（Linux 用 -i，macOS 用 -i ""）
if sed --version >/dev/null 2>&1; then
    # GNU sed (Linux)
    sed -i -E "s/^version *= *\"[^\"]+\"/version = \"$PKG_VERSION\"/" "$CARGO_TOML"
else
    # BSD sed (macOS)
    sed -i "" -E "s/^version *= *\"[^\"]+\"/version = \"$PKG_VERSION\"/" "$CARGO_TOML"
fi

echo "Building AI自动化测试员..."

# 构建 release 版本
cargo build --release

# 检测平台
OS=$(uname)
case "$OS" in
    Darwin)
        PLATFORM="darwin"
        BINARY_NAME="ai-tester"
        ;;
    Linux)
        PLATFORM="linux"
        BINARY_NAME="ai-tester"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLATFORM="win32"
        BINARY_NAME="ai-tester.exe"
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
TARGET_DIR="$SCRIPT_DIR/../resources/$PLATFORM/ai-tester"
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
echo "Size: $(du -h "$TARGET_BINARY" | cut -f1)"

# 验证二进制文件能否运行
if "$TARGET_BINARY" --version > /dev/null 2>&1; then
    echo "ai-tester --version successful"
else
    echo "Warning: ai-tester might not be executable"
fi

echo ""
echo "🚀 使用方法:"
echo "  $TARGET_BINARY test --name '登录测试' --description '测试用户登录功能' \\"
echo "    --project /path/to/project --device DEVICE_ID --api-key YOUR_OPENAI_API_KEY"
echo ""
echo "  $TARGET_BINARY status --project /path/to/project --device DEVICE_ID --api-key YOUR_OPENAI_API_KEY"