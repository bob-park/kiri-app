# kiri 개발 · 릴리즈

빌드는 로컬에서만 한다(CI 없음). Apple Silicon 전용.

## 준비 (한 번)

```sh
yarn install
yarn sidecars          # ffmpeg/ffprobe(scripts/ffmpeg.lock 고정) + kiri CLI 를 src-tauri/binaries 에
```

- 새로 clone 했다면 `cargo build`, `cargo test -p kiri-app`(또는 `--workspace`), `yarn tauri dev` 보다 **먼저** `yarn sidecars` 를 돌린다. tauri-build 가 `externalBin` 파일이 있는지 검사하므로 없으면 빌드가 실패한다.
- CLI sidecar 이름은 `kiri-cli` 다(`scripts/build-cli.sh` 가 `target/release/kiri` 를 `src-tauri/binaries/kiri-cli-aarch64-apple-darwin` 로 복사). 설치된 앱에서는 설정 > CLI 에서 `/usr/local/bin/kiri` 로 링크한다.

ffmpeg 버전을 올릴 때: `sh scripts/fetch-ffmpeg.sh --update` → `scripts/ffmpeg.lock` 커밋.

### 업데이트 서명 키 (한 번)

```sh
yarn tauri signer generate -w ~/.tauri/kiri.key   # 암호를 정해 입력한다
```

출력된 공개키(`~/.tauri/kiri.key.pub` 내용)를 `src-tauri/tauri.conf.json` 의 `plugins.updater.pubkey` 에 붙여 넣고 커밋한다. 지금은 비어 있으며, 비어 있거나 `<PUBKEY>` 이면 `scripts/latest-json.mjs` 가 릴리즈를 거부한다(그 빌드는 업데이트를 검증하지 못하므로). 개인키와 암호는 잃어버리면 기존 설치본에 더 이상 업데이트를 보낼 수 없으니 따로 보관한다.

## 개발

```sh
yarn tauri dev
cargo test --workspace && yarn test
```

- yt-dlp·Deno 는 앱이 `~/Library/Application Support/org.bobpark.kiri/bin/` 에 받는다.
- 큐: `~/Library/Application Support/org.bobpark.kiri/queue.json`, 작업 로그: `~/Library/Logs/org.bobpark.kiri/jobs/<id>.log`
- CLI 를 dev 앱에 붙이기: `./target/release/kiri list`

## 서명 정보

`~/.config/kiri/sign.env` (커밋 금지):

```sh
APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
APPLE_ID="<apple id 이메일>"
APPLE_PASSWORD="<앱 전용 암호>"
APPLE_TEAM_ID="<TEAM_ID>"
TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/kiri.key"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<키 암호>"
```

- Developer ID 인증서는 키체인에 있어야 한다(`security find-identity -v -p codesigning`).
- 업데이트 서명 키: `yarn tauri signer generate -w ~/.tauri/kiri.key`. 공개키는 `src-tauri/tauri.conf.json` 의 `plugins.updater.pubkey`(위 "업데이트 서명 키" 참고).

## 릴리즈

1. 버전 올리기: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` 의 `version` 을 같은 값으로 (예: 0.2.0) → 커밋.
2. sidecar 갱신: `yarn sidecars`
3. 빌드·서명·`.app` 공증:
   ```sh
   set -a; source ~/.config/kiri/sign.env; set +a
   yarn tauri build
   ```
4. `.dmg` 공증:
   ```sh
   DMG=target/release/bundle/dmg/kiri_0.2.0_aarch64.dmg
   xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
   xcrun stapler staple "$DMG"
   spctl --assess --type open --context context:primary-signature -v "$DMG"
   ```
5. GitHub Release:
   ```sh
   B=target/release/bundle/macos
   gh release create v0.2.0 "$DMG" "$B/kiri.app.tar.gz" "$B/kiri.app.tar.gz.sig" --title v0.2.0 --notes "…"
   ```
6. `node scripts/latest-json.mjs v0.2.0`

## 확인

- 새로 받은 dmg 를 열어 Gatekeeper 경고 없이 실행되는지.
- 이전 버전 앱에서 설정 > 업데이트 > 지금 확인 → 배너 → 설치 후 재시작 → 새 버전.
