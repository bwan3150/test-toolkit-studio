#!/bin/bash

echo "🚀 Starting Test Toolkit Studio"
echo "================================"
echo ""

# Kill any existing Electron processes
pkill -f "electron" 2>/dev/null || true
sleep 1

# Check dependencies
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    npm install
fi

# Start the application
echo "✅ Launching application..."
npm start
