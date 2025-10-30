#!/bin/bash

# Toolkit Engine AutoServer 编译脚本
# 使用 Android Gradle Plugin 编译

set -e

echo "🔧 Building Toolkit Engine AutoServer..."

cd "$(dirname "$0")"

# 使用 Gradle 编译
./gradlew assembleRelease

# 提取 classes.dex 并打包成 jar
echo "📦 Packaging tke-autoserver.jar..."

APK_FILE="build/outputs/apk/release/autoserver-android-release-unsigned.apk"
OUTPUT_JAR="build/tke-autoserver"

if [ ! -f "$APK_FILE" ]; then
    echo "❌ APK not found: $APK_FILE"
    exit 1
fi

# 提取 classes.dex
unzip -q -o "$APK_FILE" classes.dex -d build/

# 打包成 jar
cd build
jar cf tke-autoserver classes.dex
rm -f classes.dex
cd ..

if [ -f "build/tke-autoserver" ]; then
    echo "✅ Build successful!"
    echo "📦 Output: build/tke-autoserver"
    ls -lh build/tke-autoserver
else
    echo "❌ Build failed"
    exit 1
fi
