#!/bin/bash

echo "================================"
echo ""

# Kill existing Electron
pkill -f "electron" 2>/dev/null || true
sleep 1

if [ ! -d "node_modules" ]; then
    echo "安装依赖中..."
    npm install
fi

echo "正在启动App..."
npm start
