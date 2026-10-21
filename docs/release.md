# 릴리스 절차

빌드·서명·업로드는 모두 개인 장비에서 한다. 빌드용 CI는 없다. GitHub Actions는 버전 올림에만 쓴다([버전 규칙](agents/workflow/versioning.md)).

- 지원 플랫폼: macOS는 Apple Silicon만, Windows는 x64만 지원한다.
- 서명:
  - macOS는 Developer ID로 서명하고 공증한다.
  - Windows는 코드 서명을 하지 않는다. 자동 업데이트 검증용 Tauri 서명(`.sig`)만 만든다.

## 처음 한 번

### 업데이터 서명 키 (새 앱마다 한 번)

```bash
yarn tauri signer generate -w ~/.config/template-rust-app/updater.key
```

1. 출력된 **공개키**를 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 넣고 커밋한다.
   - 공개키가 비어 있으면 `latest-json.mjs`가 실행을 거부한다.
   - 이 값이 비어 있는 상태로 빌드된 앱은 업데이트를 검증하지 못한다.
2. 개인키 파일과 비밀번호는 아래 `sign.env`에 둔다. **저장소에는 절대 넣지 않는다.**
3. 개인키를 잃어버리면 이미 설치된 앱은 더 이상 업데이트를 받지 못한다. 안전한 곳에 백업한다.

### macOS 장비

- `gh auth login`
- 로그인 키체인에 `Developer ID Application` 인증서를 설치한다.
- `~/.config/template-rust-app/sign.env`:

  ```bash
  APPLE_SIGNING_IDENTITY="Developer ID Application: ..."
  APPLE_ID="..."
  APPLE_PASSWORD="..."          # 앱 암호
  APPLE_TEAM_ID="..."
  TAURI_SIGNING_PRIVATE_KEY="/Users/<me>/.config/template-rust-app/updater.key"   # 절대 경로 또는 키 내용
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="..."
  ```

### Windows PC

- 필요한 도구: Rust, Node 26 / Yarn 4.18.1, `gh auth login`, PowerShell 7
- `%USERPROFILE%\.config\template-rust-app\sign.env`에 Tauri 키 두 줄을 넣는다. 경로는 Windows 경로로 쓴다.
- 개인키 파일도 같은 위치에 복사한다.

### Yarn 버전 주의

yarn은 반드시 4.18.1(`.mise.toml`)로 실행한다. mise가 활성화되지 않아 전역 yarn 1.x가 잡히면 `yarn.lock`이 Yarn 1 형식으로 통째로 바뀐다.

- `yarn --version`으로 버전을 확인한다.
- 4.x가 아니면 `corepack yarn@4.18.1 <명령>`으로 실행한다.

## 절차

1. **버전은 손으로 올리지 않는다.** `develop`에 PR이 머지되면 GitHub Actions가 `bump-version-up.sh`로 올린다. `main`에 `hotfix/*` PR이 머지될 때도 올린다. 버전이 들어가는 곳은 `tauri.conf.json`, `package.json`, `Cargo.toml`, `Cargo.lock`이다.
2. `develop`을 `main`으로 머지하는 릴리스 PR을 만들고 머지한다. 이후 단계는 모두 `main`에서 한다. 릴리스 태그는 `--target main`으로 만들어져 항상 `main`을 가리킨다.
3. macOS에서 `scripts/release.sh`를 실행한다. 빌드·서명·공증을 한 뒤 초안 릴리스 `v<버전>`에 다음 파일을 올린다.
   - `App_<버전>_aarch64.dmg`
   - `App_<버전>_aarch64.app.tar.gz`와 그 `.sig`
4. Windows에서 `pwsh scripts/release.ps1`을 실행한다. 같은 릴리스에 `App_<버전>_x64-setup.exe`와 그 `.sig`를 올린다.
5. 3과 4는 순서가 상관없다. 둘 다 끝나면 릴리스 본문을 완성한다. 본문은 `latest.json`의 업데이트 노트로 들어간다.
6. 아무 기계에서 `node scripts/latest-json.mjs v<버전>`을 실행한다. 릴리스의 `.sig`를 모아 `latest.json`을 만들어 올린다. 올라온 플랫폼만 들어간다.
7. 출력된 플랫폼 목록에 `darwin-aarch64`와 `windows-x86_64`가 모두 있는지 확인한다.
8. `gh release edit v<버전> -R bob-park/template-rust-app --draft=false`로 게시한다.
   - 설치된 앱은 6시간 안에 새 버전을 감지한다.
   - 설정 → 업데이트 → "업데이트 확인"이나 트레이 메뉴로 바로 확인할 수도 있다.

## macOS만 먼저 게시하기

Windows 빌드를 바로 할 수 없을 때는 macOS만 먼저 내보내고, Windows는 나중에 같은 릴리스에 더한다.

1. 위 절차 1~3을 한다.
2. `node scripts/latest-json.mjs v<버전>`을 실행하고, 출력이 `darwin-aarch64`뿐인지 확인한다.
3. `gh release edit v<버전> -R bob-park/template-rust-app --draft=false`로 게시한다.
4. `curl -sL https://github.com/bob-park/template-rust-app/releases/latest/download/latest.json`의 결과에 새 버전과 `darwin-aarch64`가 보이는지 확인한다.

이 상태에서는 이렇게 된다.

- macOS 사용자는 바로 업데이트를 받는다.
- Windows 사용자는 `latest.json`에 `windows-x86_64`가 없어서 업데이트를 받지 않고 이전 버전에 머문다. 오류는 나지 않는다.
- 이 릴리스가 "Latest"가 되므로, Releases 페이지에서 새로 내려받으려는 Windows 사용자는 설치 파일을 찾을 수 없다. 가능한 빨리 Windows를 더한다.

나중에 Windows를 더할 때:

1. Windows PC에서 `pwsh scripts/release.ps1`을 실행한다. 이미 게시된 같은 릴리스에 올라가고, 새 초안은 만들지 않는다.
2. 아무 기계에서 `node scripts/latest-json.mjs v<버전>`을 다시 실행해 `latest.json`을 덮어쓴다. 출력에 두 플랫폼이 모두 있는지 확인한다. 이미 게시된 상태이므로 `--draft=false`는 다시 할 필요 없다.

## 핫픽스

1. `main`에서 `hotfix/<주제>` 브랜치를 만든다.
2. `main`으로 PR을 보내고 머지한다. 머지되면 Actions가 patch 버전을 올린다.
3. 위 절차의 3번부터 진행한다.
4. 릴리스 후 `main`을 `develop`에 머지한다. 이 머지는 버전을 다시 올리지 않는다.

릴리스 스크립트는 **이미 게시된** `v<버전>`에 같은 플랫폼 빌드가 있으면 실행을 거부한다. 게시된 버전을 덮어쓰면 설치된 앱이 업데이트를 감지하지 못하기 때문이다. 버전을 올린 뒤 다시 릴리스한다.

## 문제 해결

- **`failed to decode secret key: ... Invalid symbol 46`**
  - 원인: `TAURI_SIGNING_PRIVATE_KEY`가 경로인데 그 파일이 없다. 그래서 경로 문자열을 키 내용으로 읽으려다 실패한 것이다.
  - 확인: `sign.env`의 경로에 오타(예: `.cofing`)가 없는지 본다.
  - 이 오류는 공증까지 끝난 뒤 마지막에 나며, 그때는 릴리스에 아무것도 올라가지 않는다.
- **`src-tauri/tauri.conf.json: plugins.updater.pubkey is empty`**: 위 "업데이터 서명 키" 절차를 먼저 한다.
