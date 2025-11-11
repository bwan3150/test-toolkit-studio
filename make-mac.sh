#!/bin/bash
# macOS 平台构建脚本
# 用于在 macOS 上构建 Test Toolkit Studio

set -e  # 遇到错误立即退出

echo "=========================================="
echo "  Test Toolkit Studio - macOS Build"
echo "=========================================="
echo ""

# 1. 安装 Node.js 依赖
echo ">>> [1/6] Installing Node.js dependencies..."
npm install

# 2. 修复依赖问题
echo ""
echo ">>> [2/6] Fixing npm audit issues..."
npm audit fix || true  # 即使失败也继续

# 3. 构建 Rust 项目：toolkit-engine (TKE)
echo ""
echo ">>> [3/6] Building Toolkit Engine (Rust)..."
./toolkit-engine/build-mac.sh

# 4. 构建 Python 项目：opencv-matcher
echo ""
echo ">>> [4/6] Building OpenCV Matcher (Python)..."
./opencv-matcher/build-mac.sh

# 5. 构建 Rust 项目：tester-ai
echo ""
echo ">>> [5/6] Building AI Tester (Rust)..."
./tester-ai/build-mac.sh

# 6. 构建 Rust 项目：scrcpy-server
echo ""
echo ">>> [6/6] Building Scrcpy Server (Rust)..."
./scrcpy-server/build-mac.sh

# 7. 检查并下载 FFmpeg（如果不存在）
echo ""
echo ">>> [7/7] Checking FFmpeg dependency..."
FFMPEG_PATH="./resources/darwin/toolkit-engine/ffmpeg"
if [ ! -f "$FFMPEG_PATH" ]; then
    echo "FFmpeg not found locally, downloading from S3..."
    mkdir -p "./resources/darwin/toolkit-engine"
    curl -L "https://toolkit-studio-updates.s3.ap-southeast-2.amazonaws.com/dependency/darwin/ffmpeg" \
         -o "$FFMPEG_PATH"

    # 添加执行权限
    chmod +x "$FFMPEG_PATH"

    echo "✓ FFmpeg downloaded successfully"
else
    echo "✓ FFmpeg already exists, skipping download"
fi

# 8. 构建 Electron 应用（macOS）
echo ""
echo ">>> Building Electron app for macOS..."
# 使用 unsigned 版本，本地开发无需签名
# 如需签名版本，请使用: npm run build-mac
npm run build-mac-unsigned

echo ""
echo "=========================================="
echo "  macOS Build Completed Successfully! ✓"
echo "=========================================="
echo ""
echo "Output: ./dist/"
