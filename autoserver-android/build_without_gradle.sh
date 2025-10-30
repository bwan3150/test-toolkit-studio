#!/usr/bin/env bash
#
# 构建 Toolkit Engine AutoServer 并复制到资源目录
#
# Adapt Android platform and build tools versions (via ANDROID_PLATFORM and
# ANDROID_BUILD_TOOLS environment variables).
#
# Then execute:
#
#     ./build_without_gradle.sh

set -e

# 获取脚本所在目录（autoserver-android目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "==============================="

# 读取版本号：package.json → VERSION
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

echo "Build version: $PKG_VERSION"

SCRCPY_DEBUG=false
SCRCPY_VERSION_NAME="$PKG_VERSION"

PLATFORM=${ANDROID_PLATFORM:-35}
BUILD_TOOLS=${ANDROID_BUILD_TOOLS:-35.0.0}
PLATFORM_TOOLS="$ANDROID_HOME/platforms/android-$PLATFORM"
BUILD_TOOLS_DIR="$ANDROID_HOME/build-tools/$BUILD_TOOLS"

BUILD_DIR="$SCRIPT_DIR/build"
mkdir -p "$BUILD_DIR"
CLASSES_DIR="$BUILD_DIR/classes"
GEN_DIR="$BUILD_DIR/gen"
SERVER_DIR="$SCRIPT_DIR"
SERVER_BINARY=tke-autoserver
ANDROID_JAR="$PLATFORM_TOOLS/android.jar"
ANDROID_AIDL="$PLATFORM_TOOLS/framework.aidl"
LAMBDA_JAR="$BUILD_TOOLS_DIR/core-lambda-stubs.jar"

echo "Platform: android-$PLATFORM"
echo "Build-tools: $BUILD_TOOLS"
echo "Build dir: $BUILD_DIR"

rm -rf "$CLASSES_DIR" "$GEN_DIR" "$BUILD_DIR/$SERVER_BINARY" classes.dex
mkdir -p "$CLASSES_DIR"
mkdir -p "$GEN_DIR/app/TestToolkit/TKE/AutoServer"

<< EOF cat > "$GEN_DIR/app/TestToolkit/TKE/AutoServer/BuildConfig.java"
package app.TestToolkit.TKE.AutoServer;

public final class BuildConfig {
  public static final boolean DEBUG = $SCRCPY_DEBUG;
  public static final String VERSION_NAME = "$SCRCPY_VERSION_NAME";
}
EOF

echo "Generating java from aidl..."
cd "$SERVER_DIR/src/main/aidl"
"$BUILD_TOOLS_DIR/aidl" -o"$GEN_DIR" -I. \
    android/content/IOnPrimaryClipChangedListener.aidl
"$BUILD_TOOLS_DIR/aidl" -o"$GEN_DIR" -I. -p "$ANDROID_AIDL" \
    android/view/IDisplayWindowListener.aidl

# Fake sources to expose hidden Android types to the project
FAKE_SRC=( \
    android/content/*java \
)

SRC=( \
    app/TestToolkit/TKE/AutoServer/*.java \
    app/TestToolkit/TKE/AutoServer/audio/*.java \
    app/TestToolkit/TKE/AutoServer/control/*.java \
    app/TestToolkit/TKE/AutoServer/device/*.java \
    app/TestToolkit/TKE/AutoServer/opengl/*.java \
    app/TestToolkit/TKE/AutoServer/util/*.java \
    app/TestToolkit/TKE/AutoServer/video/*.java \
    app/TestToolkit/TKE/AutoServer/wrappers/*.java \
)

CLASSES=()
for src in "${SRC[@]}"
do
    CLASSES+=("${src%.java}.class")
done

echo "Compiling java sources..."
cd ../java
javac -encoding UTF-8 -bootclasspath "$ANDROID_JAR" \
    -cp "$LAMBDA_JAR:$GEN_DIR" \
    -d "$CLASSES_DIR" \
    -source 1.8 -target 1.8 \
    ${FAKE_SRC[@]} \
    ${SRC[@]}

echo "Dexing..."
cd "$CLASSES_DIR"

if [[ $PLATFORM -lt 31 ]]
then
    # use dx
    "$BUILD_TOOLS_DIR/dx" --dex --output "$BUILD_DIR/classes.dex" \
        android/view/*.class \
        android/content/*.class \
        ${CLASSES[@]}

    echo "Archiving..."
    cd "$BUILD_DIR"
    jar cvf "$SERVER_BINARY" classes.dex
    rm -rf classes.dex
else
    # use d8
    "$BUILD_TOOLS_DIR/d8" --classpath "$ANDROID_JAR" \
        --output "$BUILD_DIR/classes.zip" \
        android/view/*.class \
        android/content/*.class \
        ${CLASSES[@]}

    cd "$BUILD_DIR"
    mv classes.zip "$SERVER_BINARY"
fi

rm -rf "$GEN_DIR" "$CLASSES_DIR"

echo "Server generated in $BUILD_DIR/$SERVER_BINARY"

# 检测平台
OS=$(uname)
case "$OS" in
    Darwin)
        PLATFORM_NAME="darwin"
        ;;
    Linux)
        PLATFORM_NAME="linux"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLATFORM_NAME="win32"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "OS: $PLATFORM_NAME"

# 源文件路径
SOURCE_BINARY="$BUILD_DIR/$SERVER_BINARY"

# 目标目录和文件路径
TARGET_DIR="$SCRIPT_DIR/../resources/$PLATFORM_NAME/toolkit-engine"
TARGET_BINARY="$TARGET_DIR/$SERVER_BINARY"

# 检查源文件是否存在
if [ ! -f "$SOURCE_BINARY" ]; then
    echo "Error: build fault, cannot find: $SOURCE_BINARY"
    exit 1
fi

# 创建目标目录
mkdir -p "$TARGET_DIR"

# 复制二进制文件
cp "$SOURCE_BINARY" "$TARGET_BINARY"

echo "Build successfully"
echo "Cp to: $TARGET_BINARY"
echo "Size: $(du -h "$TARGET_BINARY" | cut -f1)"

echo ""
echo "==============================="
echo "TKE AutoServer Build Finished"
echo "==============================="
echo ""
