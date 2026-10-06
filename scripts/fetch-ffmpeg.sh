#!/bin/sh
# ffmpeg/ffprobe arm64 정적 빌드(ffmpeg.martin-riedl.de, 서명·공증됨)를 sidecar 위치에 받는다.
#   처음 또는 버전을 올릴 때:  sh scripts/fetch-ffmpeg.sh --update   (scripts/ffmpeg.lock 갱신)
#   평소:                     sh scripts/fetch-ffmpeg.sh            (lock 의 URL·SHA256 그대로)
set -eu
cd "$(dirname "$0")/.."
LOCK=scripts/ffmpeg.lock
OUT=src-tauri/binaries
TRIPLE=aarch64-apple-darwin
BASE=https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release

if [ "${1:-}" = "--update" ]; then
  : > "$LOCK.tmp"
  for tool in ffmpeg ffprobe; do
    # redirect 는 HEAD 에 404 를 준다. GET 의 Location 만 읽는다(본문은 받지 않음).
    url=$(curl -fsS -o /dev/null -w '%{redirect_url}' "$BASE/$tool.zip")
    [ -n "$url" ] || { echo "$tool: no redirect from $BASE/$tool.zip" >&2; exit 1; }
    sha=$(curl -fsSL "$url" | shasum -a 256 | cut -d' ' -f1)
    echo "$tool $url $sha" >> "$LOCK.tmp"
  done
  mv "$LOCK.tmp" "$LOCK"
fi

[ -f "$LOCK" ] || { echo "missing $LOCK — run: sh scripts/fetch-ffmpeg.sh --update" >&2; exit 1; }
mkdir -p "$OUT"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
while read -r tool url sha; do
  curl -fsSL "$url" -o "$tmp/$tool.zip"
  got=$(shasum -a 256 "$tmp/$tool.zip" | cut -d' ' -f1)
  [ "$got" = "$sha" ] || { echo "$tool: checksum mismatch ($got != $sha)" >&2; exit 1; }
  mkdir -p "$tmp/$tool"
  ditto -x -k "$tmp/$tool.zip" "$tmp/$tool"
  install -m 755 "$tmp/$tool/$tool" "$OUT/$tool-$TRIPLE"
done < "$LOCK"

# 프리셋이 쓰는 인코더가 모두 들어 있는지 확인한다.
encoders=$("$OUT/ffmpeg-$TRIPLE" -hide_banner -encoders)
for enc in libx264 libx265 libvpx-vp9 libmp3lame libopus h264_videotoolbox hevc_videotoolbox prores_videotoolbox prores_ks; do
  echo "$encoders" | grep -q " $enc " || { echo "ffmpeg build lacks encoder: $enc" >&2; exit 1; }
done
echo "ffmpeg sidecars ready in $OUT"
