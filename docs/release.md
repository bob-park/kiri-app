# 릴리스 절차

빌드·서명·공증·업로드는 모두 Apple Silicon Mac에서 직접 한다. 빌드용 CI는 없고, GitHub Actions는 버전 올림에만 쓴다([버전 규칙](agents/workflow/versioning.md)).

- 지원 플랫폼: macOS Apple Silicon(arm64)만.
- 서명: Developer ID로 서명하고 공증한다. 자동 업데이트 검증용 Tauri 서명(`.sig`)도 만든다.
- 처음 한 번 할 준비(인증서, 공증 API 키, 업데이트 서명 키)는 [development.md의 서명 준비](development.md#서명-준비-한-번)를 따른다. 모두 로그인 키체인과 `~/.config/apple/`에 둔다.
- `gh auth login`이 되어 있어야 한다.

## 절차

예시는 버전 `0.4.0` 기준이다.

1. **버전은 손으로 올리지 않는다.** `master`에 PR이 머지되면 GitHub Actions가 `bump-version-up.sh`로 올린다. `master`를 받아 버전을 확인하고 검사를 돌린다.
   ```sh
   git switch master && git pull
   node -p 'require("./src-tauri/tauri.conf.json").version'
   cargo test --workspace && yarn test && yarn typecheck && yarn lint
   ```
2. **빌드, 서명, 공증, 업로드:**
   ```sh
   scripts/release-macos.sh
   ```
   스크립트가 하는 일:
   - `yarn sidecars`(kiri CLI)
   - `yarn tauri build --bundles app,dmg`: Developer ID 서명, `.app` 공증(API 키), 업데이트 파일과 `.sig` 생성
   - `.dmg`를 `notarytool`로 공증하고 스테이플(Tauri는 `.app`만 공증한다)
   - 초안 릴리스 `v0.4.0`(`--target master`)을 만들고 아래 파일을 올린다.
     - `kiri_0.4.0_aarch64.dmg`
     - `kiri_0.4.0_aarch64.app.tar.gz`와 그 `.sig` (업데이트용)

   업로드 없이 빌드·공증만 해 보려면 `scripts/release-macos.sh --build-only`.
3. **확인:** 결과물은 `target/release/bundle/`에 있다. 두 `spctl` 모두 `accepted`, `source=Notarized Developer ID`가 나와야 한다.
   ```sh
   spctl --assess --type open --context context:primary-signature -vv target/release/bundle/dmg/kiri_0.4.0_aarch64.dmg
   spctl --assess --type execute -vv target/release/bundle/macos/kiri.app
   ```
4. **릴리스 본문**을 GitHub에서 쓴다.
5. **업데이트 매니페스트:**
   ```sh
   node scripts/latest-json.mjs v0.4.0
   ```
   릴리스에 올라간 `.sig`로 `latest.json`을 만들어 같은 릴리스에 올린다.
6. **게시:**
   ```sh
   gh release edit v0.4.0 -R bob-park/kiri-app --draft=false
   ```
   앱은 `https://github.com/bob-park/kiri-app/releases/latest/download/latest.json`을 확인한다(실행 10초 뒤, 그 뒤 하루에 한 번).

## 릴리스 후 확인

- 새로 받은 dmg를 열어 Gatekeeper 경고 없이 설치·실행되는지 확인한다.
- 이전 버전 앱에서 설정 → 업데이트 → 업데이트 확인 → 설치 후 재시작 → 새 버전으로 바뀌는지 확인한다.

## 핫픽스

1. `master`에서 `hotfix/<주제>` 브랜치를 만든다.
2. `master`로 PR을 보내고 머지한다. 머지되면 Actions가 버전을 올린다(제목에 `[minor]`/`[major]`가 없으면 patch).
3. 위 절차를 처음부터 한다.

이미 게시한 버전의 파일은 덮어쓰지 않는다. 덮어쓰면 설치된 앱이 새 빌드를 감지하지 못한다. `release-macos.sh`도 이미 게시된 `v<버전>`이면 실행을 거부한다. 버전을 올린 뒤 다시 릴리스한다.

## 문제 해결

- **`failed to decode secret key: ... Invalid symbol`**
  - 원인: 키체인의 `tauri-updater-kiri`에 개인키 **내용**이 아닌 다른 값(파일 경로, 잘린 값)이 들어 있다.
  - 확인: [업데이트 서명 키](development.md#업데이트-서명-키)대로 다시 저장한다.
  - 이 오류는 공증까지 끝난 뒤 마지막에 나며, 그때는 릴리스에 아무것도 올라가지 않는다.
- **`missing .../AuthKey_<ID>.p8`** 또는 `security: SecKeychainSearchCopyNext`: 키체인 항목(`tauri-apple-api-key`, `tauri-apple-api-issuer`, `tauri-updater-kiri`, `tauri-updater-kiri-pw`)이나 `~/.config/apple/`의 `.env`·`.p8`이 없다. [서명 준비](development.md#서명-준비-한-번)를 다시 본다.
- **`updater pubkey is empty — see docs/development.md`** 또는 **`plugins.updater.pubkey is empty`**: [업데이트 서명 키](development.md#업데이트-서명-키) 절차를 먼저 한다.
