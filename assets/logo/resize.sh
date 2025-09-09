#!/bin/bash

# ./resize.sh input.icns 80
# 目前仅用于icns

INPUT="$1"
SCALE="${2:-70}"
OUTPUT="${INPUT%.*}_${SCALE}percent.icns"

sips -s format png "$INPUT" --out icon_temp.png

SIZE=$(magick identify -format "%wx%h" icon_temp.png)

magick icon_temp.png -resize ${SCALE}% -gravity center -background transparent -extent $SIZE icon_small.png

sips -s format icns icon_small.png --out "$OUTPUT"

rm icon_temp.png icon_small.png

echo "Resize后的文件: $OUTPUT"
