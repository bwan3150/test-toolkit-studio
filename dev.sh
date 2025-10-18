#!/bin/bash

echo "================================"
echo ""

# Kill existing Electron
pkill -f "electron" 2>/dev/null || true
sleep 1

if [ ! -d "node_modules" ]; then
    echo "Dependency installing..."
    npm install
fi

echo "Running Toolkit Studio(Electron) in Dev..."
export ELECTRON_DEV_MODE=true
export ELECTRON_PROJECT_ROOT="$(pwd)"
export ELECTRON_SIMULATE_UPDATE=false  # 测试更新弹窗
npm start
