#!/bin/sh
# kiri CLI 를 릴리스로 빌드해 sidecar 위치에 둔다. 앱 번들 안에서는 kiri-cli 라는 이름이 된다.
set -eu
cd "$(dirname "$0")/.."
cargo build -p kiri-cli --release
mkdir -p src-tauri/binaries
install -m 755 target/release/kiri src-tauri/binaries/kiri-cli-aarch64-apple-darwin
