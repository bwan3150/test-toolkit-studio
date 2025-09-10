#!/bin/bash

echo "================================"
echo ""

# Kill existing Electron
pkill -f "electron" 2>/dev/null || true
sleep 1

if [ ! -d "node_modules" ]; then
    echo "Dependacy installing..."
    npm install
fi

echo "Running Toolkit Studio(Electron) in Dev..."
npm start
