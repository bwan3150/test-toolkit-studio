#!/bin/bash

# 构建 Toolkit Engine (TKE) —— Linux（开发机 / CI）
#
# 与 build-mac.sh 的关系：那个脚本的 case 分支其实也认 Linux，但它带着 macOS 专属逻辑
# （codesign ad-hoc 重签，治 P-02 的 Killed: 9），且没有依赖预检、没有 CI 需要的
# 跳过离线 OCR 的开关。Linux 单列一个脚本，职责更干净。
#
# 用法：
#   ./build-linux.sh              完整构建（含离线 OCR / tesseract，首次很慢）
#   ./build-linux.sh --no-ocr     跳过 OCR feature（CI 推荐：快得多，不需要 tesseract 系统依赖）
#   ./build-linux.sh --quiet      精简输出（CI 日志友好）
#
# 禁止用 cargo build 直接产二进制（PITFALLS P-02）：bin 落点由本脚本管。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# —— 参数 ——
NO_OCR=0
QUIET=0
for arg in "$@"; do
    case "$arg" in
        --no-ocr) NO_OCR=1 ;;
        --quiet|-q) QUIET=1 ;;
        -h|--help)
            sed -n '3,14p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "未知参数: ${arg}（可用: --no-ocr / --quiet / --help）" >&2
            exit 2
            ;;
    esac
done

say() { [ "$QUIET" -eq 1 ] || echo "$@"; }

say "==============================="
say "Building Toolkit Engine (Linux)"
say "==============================="

# —— 平台校验 ——
OS=$(uname)
if [ "$OS" != "Linux" ]; then
    echo "Error: 本脚本只用于 Linux，当前是 ${OS}（macOS 用 ./build-mac.sh，Windows 用 build-win.bat）" >&2
    exit 1
fi

ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)  ARCH_NAME="amd64" ;;
    aarch64|arm64) ARCH_NAME="arm64" ;;
    *)
        echo "Error: 不支持的架构: $ARCH" >&2
        exit 1
        ;;
esac
PLATFORM="linux-$ARCH_NAME"

# —— 依赖预检 ——（缺了就明确报出来，别让 cargo 在半小时后炸出一堆看不懂的 native 报错）
MISSING=()
command -v cargo >/dev/null 2>&1 || MISSING+=("cargo (rustup)")
command -v cc    >/dev/null 2>&1 || MISSING+=("cc (build-essential)")
if [ "$NO_OCR" -eq 0 ]; then
    # 离线 OCR = tesseract-rs 的 build-tesseract：从源码编译 tesseract + leptonica
    command -v cmake      >/dev/null 2>&1 || MISSING+=("cmake (离线 OCR 需要；或改用 --no-ocr)")
    command -v pkg-config >/dev/null 2>&1 || MISSING+=("pkg-config (离线 OCR 需要；或改用 --no-ocr)")
fi
if [ ${#MISSING[@]} -gt 0 ]; then
    echo "Error: 缺少构建依赖：" >&2
    for m in "${MISSING[@]}"; do echo "  - $m" >&2; done
    echo "  Debian/Ubuntu: sudo apt install build-essential cmake pkg-config" >&2
    exit 1
fi

# —— 版本号：package.json → BUILD_VERSION（build.rs 读它注入 --version）——
PACKAGE_JSON="$SCRIPT_DIR/../../package.json"
if [ ! -f "$PACKAGE_JSON" ]; then
    echo "Error: package.json not found at $PACKAGE_JSON" >&2
    exit 1
fi
PKG_VERSION=$(grep '"version"' "$PACKAGE_JSON" | head -1 | sed -E 's/.*"version": *"([^"]+)".*/\1/')
if [ -z "$PKG_VERSION" ]; then
    echo "Error: cannot extract version from $PACKAGE_JSON" >&2
    exit 1
fi
export BUILD_VERSION="$PKG_VERSION"
say "Build version: $BUILD_VERSION"
say "Platform:      $PLATFORM"

# —— 构建 ——
CARGO_FLAGS=(--release)
if [ "$NO_OCR" -eq 1 ]; then
    CARGO_FLAGS+=(--no-default-features)
    say "OCR:           跳过（--no-ocr：不含在线/离线 OCR）"
else
    say "OCR:           在线 + 离线（首次编译 tesseract 很慢，CI 建议 --no-ocr）"
fi

say "Building..."
if [ "$QUIET" -eq 1 ]; then
    cargo build "${CARGO_FLAGS[@]}" --quiet
else
    cargo build "${CARGO_FLAGS[@]}"
fi

# —— 落点 ——（与 mac/win 一致：bin/<platform>/）
BINARY_NAME="tke"
SOURCE_BINARY="$SCRIPT_DIR/target/release/$BINARY_NAME"
TARGET_DIR="$SCRIPT_DIR/../../bin/$PLATFORM"
TARGET_BINARY="$TARGET_DIR/$BINARY_NAME"

if [ ! -f "$SOURCE_BINARY" ]; then
    echo "Error: build 完成但找不到产物: $SOURCE_BINARY" >&2
    exit 1
fi

mkdir -p "$TARGET_DIR"

# 先删旧文件再拷：Linux 上直接覆盖一个**正在运行**的二进制会 ETXTBSY("Text file busy")；
# 拿新 inode 就没这问题。（macOS 那边先删是为了代码签名，见 P-02——原因不同，做法一样。）
rm -f "$TARGET_BINARY"
cp "$SOURCE_BINARY" "$TARGET_BINARY"
chmod +x "$TARGET_BINARY"

say "Copied to: $TARGET_BINARY"
say "Size:      $(du -h "$TARGET_BINARY" | cut -f1)"

# ── 提示：你敲的 `tke` 未必是刚构建的这个 ────────────────────────────────
# 构建产物落在仓库的 bin/<platform>/，日常敲的 `tke` 多半是 ~/.tke/bin/ 那个。
# **只提示，不覆盖**——那是用户日常在用的，构建脚本没资格替他换掉。
INSTALLED="$(command -v tke 2>/dev/null || true)"
if [ -n "$INSTALLED" ] && [ "$INSTALLED" != "$TARGET_BINARY" ]; then
    say ""
    say "注意: 你敲 tke 用的是 $INSTALLED（不是刚构建的这个）"
    say "      要用新的: $TARGET_BINARY"
fi

# —— 验证 ——（产物跑不起来就是构建失败，别让它悄悄进 bin/）
if ! "$TARGET_BINARY" --version >/dev/null 2>&1; then
    echo "Error: 产物无法执行（$TARGET_BINARY --version 失败）" >&2
    exit 1
fi
say "Verify:    $("$TARGET_BINARY" --version 2>&1 | head -1)"

say ""
say "==============================="
say "TKE Build Finished (Linux)"
say "==============================="
say ""
