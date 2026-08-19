#!/bin/bash

# 构建 Toolkit Engine (TKE) 并复制到资源目录
set -e  # 遇到错误立即退出

# 获取脚本所在目录（toolkit-engine目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "==============================="


# 读取版本号：package.json → BUILD_VERSION 环境变量
PACKAGE_JSON="$SCRIPT_DIR/../../package.json"
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

# 检测平台和架构（与 bin/<platform>-<arch>/ 目录结构一致）
OS=$(uname)
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) ARCH_NAME="arm64" ;;
    x86_64|amd64)  ARCH_NAME="amd64" ;;
    *)
        echo "Not supported arch: $ARCH"
        exit 1
        ;;
esac

case "$OS" in
    Darwin)
        PLATFORM="darwin-$ARCH_NAME"
        BINARY_NAME="tke"
        ;;
    Linux)
        PLATFORM="linux-$ARCH_NAME"
        BINARY_NAME="tke"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLATFORM="windows-$ARCH_NAME"
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
TARGET_DIR="$SCRIPT_DIR/../../bin/$PLATFORM"
TARGET_BINARY="$TARGET_DIR/$BINARY_NAME"

# 检查源文件是否存在
if [ ! -f "$SOURCE_BINARY" ]; then
    echo "Error：build fault, cannot find: $SOURCE_BINARY"
    exit 1
fi

# 创建目标目录
mkdir -p "$TARGET_DIR"

# 复制二进制文件——先删旧文件(拿新 inode)再拷:macOS(Apple Silicon)内核按 inode 缓存
# 代码签名,原地覆盖会导致签名不匹配、执行直接 Killed: 9
rm -f "$TARGET_BINARY"
cp "$SOURCE_BINARY" "$TARGET_BINARY"

# macOS 保险:ad-hoc 重签(签名缺失/失效时 AMFI 会杀进程)
if [[ "$OS" == darwin* ]]; then
    codesign --force -s - "$TARGET_BINARY" 2>/dev/null || true
fi

# 给二进制文件添加执行权限（Linux/macOS）
if [[ "$OS" != MINGW* && "$OS" != MSYS* && "$OS" != CYGWIN* ]]; then
    chmod +x "$TARGET_BINARY"
fi

echo "Build successfully"
echo "Cp to: $TARGET_BINARY"
echo "Szie: $(du -h "$TARGET_BINARY" | cut -f1)"

# ── 提示：你敲的 `tke` 未必是刚构建的这个 ────────────────────────────────
#
# 构建产物落在仓库的 bin/<platform>/，而日常敲的 `tke` 多半是安装器装到 ~/.tke/bin/
# 的那个——**两个不同的文件**。不说一声就会撞上最坑的组合：编译明明成功了、跑的还是
# 旧版，而且没有任何迹象（实测浪费过一整轮：新加的 `tke device list` 报
# unrecognized subcommand，`-d sim:` 被当成安卓序列号）。
#
# **这里只提示，不覆盖**：那个 tke 是用户日常在用的，构建脚本没有资格替他换掉。
# 要验刚构建的产物，直接用上面那个路径（scripts/verify-*.sh 就是这么做的）。
INSTALLED="$(command -v tke 2>/dev/null || true)"
TARGET_ABS="$(cd "$(dirname "$TARGET_BINARY")" && pwd)/$(basename "$TARGET_BINARY")"
if [ -n "$INSTALLED" ] && [ "$INSTALLED" != "$TARGET_ABS" ]; then
    echo ""
    echo "注意: 你敲 tke 用的是 ${INSTALLED}（不是刚构建的这个）"
    echo "      要用新的: ${TARGET_ABS}"
fi

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
