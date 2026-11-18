#!/usr/bin/env bash
# Rasterise rendered snapshot HTML to PNG with headless Chrome (local files only).
# Usage: scripts/screenshot.sh <snapshot-dir> <out-dir> <WxH> scene...
set -euo pipefail
src=$1 out=$2 size=$3
shift 3
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
w=${size%x*} h=${size#*x}
px_w=$(( w * 843 / 100 + 40 ))
px_h=$(( h * 1652 / 100 + 36 ))
mkdir -p "$out"
for scene in "$@"; do
  "$chrome" --headless=new --disable-gpu --hide-scrollbars \
    --window-size="$px_w,$px_h" \
    --screenshot="$out/$scene-$size.png" \
    "file://$(cd "$src" && pwd)/$size/$scene.html" >/dev/null 2>&1
  echo "$out/$scene-$size.png"
done
