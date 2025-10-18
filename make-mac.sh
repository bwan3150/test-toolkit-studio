#!/bin/bash
# macOS 平台构建脚本
# 用于在 macOS 上构建 Test Toolkit Studio

set -e  # 遇到错误立即退出

echo "=========================================="
echo "  Test Toolkit Studio - macOS Build"
echo "=========================================="
echo ""

# 1. 安装 Node.js 依赖
echo ">>> [1/5] Installing Node.js dependencies..."
npm install

# 2. 修复依赖问题
echo ""
echo ">>> [2/5] Fixing npm audit issues..."
npm audit fix || true  # 即使失败也继续

# 3. 构建 Rust 项目：toolkit-engine (TKE)
echo ""
echo ">>> [3/5] Building Toolkit Engine (Rust)..."
./toolkit-engine/build-mac.sh

# 4. 构建 Python 项目：opencv-matcher
echo ""
echo ">>> [4/5] Building OpenCV Matcher (Python)..."
./opencv-matcher/build-mac.sh

# 5. 构建 Rust 项目：tester-ai
echo ""
echo ">>> [5/5] Building AI Tester (Rust)..."
./tester-ai/build-mac.sh

# 6. 构建 Electron 应用（macOS）
echo ""
echo ">>> Building Electron app for macOS..."
npm run build-mac

echo ""
echo "=========================================="
echo "  macOS Build Completed Successfully! ✓"
echo "=========================================="
echo ""
echo "Output: ./dist/"
