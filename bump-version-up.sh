#!/bin/bash
# Bumps the app version (semver X.Y.Z) in every file that carries it.
# Usage: ./bump-version-up.sh [patch|minor|major]   (default: patch)
# Run by .github/workflows/bump-version.yml on every merge into master.
set -euo pipefail
cd "$(dirname "$0")"

# 색상 정의
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

TAURI_CONF="src-tauri/tauri.conf.json"
PACKAGE_JSON="package.json"
CARGO_TOMLS=("src-tauri/Cargo.toml" "crates/kiri-core/Cargo.toml" "crates/kiri-cli/Cargo.toml")
CARGO_LOCK="Cargo.lock"
CRATE_NAMES=("kiri-app" "kiri-core" "kiri-cli")
FILES=("$TAURI_CONF" "$PACKAGE_JSON" "${CARGO_TOMLS[@]}" "$CARGO_LOCK")

for f in "${FILES[@]}"; do
    if [ ! -f "$f" ]; then
        echo -e "${RED}Error: $f 파일을 찾을 수 없습니다.${NC}"
        exit 1
    fi
done

# 현재 버전 추출 (tauri.conf.json 이 기준)
current_version=$(perl -ne 'if (/^\s*"version":\s*"([^"]*)"/) { print $1; exit }' "$TAURI_CONF")

if [ -z "$current_version" ]; then
    echo -e "${RED}Error: 버전 정보를 찾을 수 없습니다.${NC}"
    exit 1
fi

echo -e "${YELLOW}현재 버전: $current_version${NC}"

# 버전 파싱 (예: 0.0.1)
if [[ $current_version =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    major=${BASH_REMATCH[1]}
    minor=${BASH_REMATCH[2]}
    patch=${BASH_REMATCH[3]}
else
    echo -e "${RED}Error: 버전 형식이 올바르지 않습니다. (예: 0.0.1)${NC}"
    exit 1
fi

update_type=${1:-patch}

case $update_type in
    "patch") new_version="${major}.${minor}.$((patch + 1))" ;;
    "minor") new_version="${major}.$((minor + 1)).0" ;;
    "major") new_version="$((major + 1)).0.0" ;;
    *)
        echo -e "${RED}Error: 잘못된 업데이트 타입입니다. [patch|minor|major]${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}새로운 버전: $new_version${NC}"

export NEW_VERSION="$new_version"

# Write all files or none: back up, write, verify, restore on any miss.
backup=$(mktemp -d)
trap 'rm -rf "$backup"' EXIT
for f in "${FILES[@]}"; do
    mkdir -p "$backup/$(dirname "$f")" && cp "$f" "$backup/$f"
done
restore() {
    for f in "${FILES[@]}"; do cp "$backup/$f" "$f"; done
    echo -e "${RED}Error: $1 — 변경을 되돌렸습니다.${NC}"
    exit 1
}
# First top-level "version" only: dependency entries never use that key.
perl -i -pe 'if (!$done && s/^(\s*"version":\s*)"[^"]*"/$1"$ENV{NEW_VERSION}"/) { $done = 1 } $done = 0 if eof' "$TAURI_CONF" "$PACKAGE_JSON"
# [package] comes first in Cargo.toml; dependency tables use inline `version =` after a name.
for f in "${CARGO_TOMLS[@]}"; do
    perl -i -pe 'if (!$done && s/^version = "[^"]*"/version = "$ENV{NEW_VERSION}"/) { $done = 1 }' "$f"
done
for crate in "${CRATE_NAMES[@]}"; do
    CRATE_NAME="$crate" perl -0 -i -pe 's/(name = "\Q$ENV{CRATE_NAME}\E"\nversion = )"[^"]*"/$1"$ENV{NEW_VERSION}"/' "$CARGO_LOCK"
done

grep -q "\"version\": \"$new_version\"" "$TAURI_CONF" || restore "$TAURI_CONF 의 버전을 바꾸지 못했습니다"
grep -q "\"version\": \"$new_version\"" "$PACKAGE_JSON" || restore "$PACKAGE_JSON 의 버전을 바꾸지 못했습니다"
for f in "${CARGO_TOMLS[@]}"; do
    grep -q "^version = \"$new_version\"" "$f" || restore "$f 의 버전을 바꾸지 못했습니다"
done
for crate in "${CRATE_NAMES[@]}"; do
    CRATE_NAME="$crate" perl -0ne 'exit(!/name = "\Q$ENV{CRATE_NAME}\E"\nversion = "\Q$ENV{NEW_VERSION}\E"/)' "$CARGO_LOCK" \
        || restore "$CARGO_LOCK 에서 패키지 \"$crate\" 를 찾지 못했습니다 (CRATE_NAMES 가 Cargo.toml 의 name 과 같은지 확인)"
done

echo -e "${GREEN}✅ 버전이 성공적으로 업데이트되었습니다!${NC}"
echo -e "${YELLOW}$current_version → $new_version${NC}"
