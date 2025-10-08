#!/bin/bash
set -e

if [ -z "$1" ]; then
  echo "Usage: $0 [mac|win]"
  exit 1
fi

# 1. node依赖
echo ">>> Dependancy installing..."
npm install

# 2. 防止出现依赖问题
echo ">>> Audit fixing..."
npm audit fix

# 3. rust toolkit-engine cargo build构建
echo ">>> Rusting tke up..."
./toolkit-engine/build.sh

# 4. pip-installer opencv-matcher构建
echo ">>> Pip installing tke-opencv..."
./opencv-matcher/build.sh

# 5. Electron构建
if [ "$1" = "mac" ]; then
  echo ">>> Building electron for mac..."
  npm run build-mac
elif [ "$1" = "win" ]; then
  echo ">>> Building electron for windows..."
  npm run build-win
else
  echo "Error arg: $1 (use mac or win)"
  exit 1
fi

echo ">>> Make DONE!"
