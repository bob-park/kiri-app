# 릴리스 절차

빌드·서명·공증·업로드는 모두 Apple Silicon Mac에서 직접 한다. 빌드용 CI는 없고, GitHub Actions는 버전 올림에만 쓴다([버전 규칙](agents/workflow/versioning.md)).

- 지원 플랫폼: macOS Apple Silicon(arm64)만.
- 서명: Developer ID로 서명하고 공증한다. 자동 업데이트 검증용 Tauri 서명(`.sig`)도 만든다.
- 처음 한 번 할 준비(인증서, 업데이트 서명 키, `~/.config/kiri/sign.env`)는 [development.md의 서명 준비](development.md#서명-준비-한-번)를 따른다.

## 절차

예시는 버전 `0.4.0` 기준이다. 모든 명령은 같은 셸에서 이어서 실행한다.

1. **버전은 손으로 올리지 않는다.** `master`에 PR이 머지되면 GitHub Actions가 `bump-version-up.sh`로 올린다. `master`를 받아 버전을 확인한다.
   ```sh
   git switch master && git pull
   node -p 'require("./src-tauri/tauri.conf.json").version'
   ```
2. **검사와 sidecar:**
   ```sh
   cargo test --workspace && yarn test && yarn typecheck && yarn lint
   yarn sidecars
   ```
3. **빌드, 서명, `.app` 공증, 업데이트 파일 생성:** 업데이트 키는 파일 경로가 아니라 **파일 내용**으로 넘긴다.
   ```sh
   set -a; . ~/.config/kiri/sign.env; set +a
   TAURI_SIGNING_PRIVATE_KEY="$(cat "$TAURI_SIGNING_PRIVATE_KEY")" yarn tauri build --bundles app,dmg
   ```
   `target/release/bundle/`에 결과물이 생긴다.
   - `macos/kiri.app`
   - `macos/kiri.app.tar.gz`, `macos/kiri.app.tar.gz.sig` (업데이트용)
   - `dmg/kiri_0.4.0_aarch64.dmg`
4. **`.dmg` 공증과 확인:**
   ```sh
   DMG=target/release/bundle/dmg/kiri_0.4.0_aarch64.dmg
   xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
   xcrun stapler staple "$DMG"
   spctl --assess --type open --context context:primary-signature -vv "$DMG"
   spctl --assess --type execute -vv target/release/bundle/macos/kiri.app
   ```
   두 `spctl` 모두 `accepted`, `source=Notarized Developer ID`가 나와야 한다.
5. **GitHub Release:**
   ```sh
   B=target/release/bundle/macos
   gh release create v0.4.0 --target master --title "kiri v0.4.0" "$DMG" "$B/kiri.app.tar.gz" "$B/kiri.app.tar.gz.sig" --notes "…"
   ```
6. **업데이트 매니페스트:**
   ```sh
   node scripts/latest-json.mjs v0.4.0
   ```
   릴리스에 올라간 `.sig`로 `latest.json`을 만들어 같은 릴리스에 올린다. 앱은 `https://github.com/bob-park/kiri-app/releases/latest/download/latest.json`을 확인한다(실행 10초 뒤, 그 뒤 하루에 한 번).

## 릴리스 후 확인

- 새로 받은 dmg를 열어 Gatekeeper 경고 없이 설치·실행되는지 확인한다.
- 이전 버전 앱에서 설정 → 업데이트 → 업데이트 확인 → 설치 후 재시작 → 새 버전으로 바뀌는지 확인한다.

## 핫픽스

1. `master`에서 `hotfix/<주제>` 브랜치를 만든다.
2. `master`로 PR을 보내고 머지한다. 머지되면 Actions가 버전을 올린다(제목에 `[minor]`/`[major]`가 없으면 patch).
3. 위 절차를 처음부터 한다.

이미 게시한 버전의 파일은 덮어쓰지 않는다. 덮어쓰면 설치된 앱이 새 빌드를 감지하지 못한다. 버전을 올린 뒤 다시 릴리스한다.

## 문제 해결

- **`failed to decode secret key: ... Invalid symbol 46`**
  - 원인: `TAURI_SIGNING_PRIVATE_KEY`에 키 내용 대신 경로가 들어갔거나, 그 경로의 파일이 없다.
  - 확인: 3번처럼 `"$(cat "$TAURI_SIGNING_PRIVATE_KEY")"`로 넘겼는지, `sign.env`의 경로에 오타가 없는지 본다.
- **`updater pubkey is empty — see docs/development.md`** 또는 **`plugins.updater.pubkey is empty`**: [업데이트 서명 키](development.md#업데이트-서명-키) 절차를 먼저 한다.
