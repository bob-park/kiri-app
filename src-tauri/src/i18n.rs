//! Rust 쪽에서 직접 그리는 문구(메뉴 막대, 네이티브 대화상자, 창 제목).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
    Ja,
}

pub fn resolve(pref: &str) -> Lang {
    resolve_with(pref, sys_locale::get_locale().as_deref())
}

pub fn resolve_with(pref: &str, system: Option<&str>) -> Lang {
    let code = if pref == "system" {
        system.unwrap_or("en")
    } else {
        pref
    };
    match code
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "ko" => Lang::Ko,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

// check_update·install_update·settings_title 은 업데이트·설정 창 작업에서 쓴다.
#[allow(dead_code)]
pub struct Labels {
    pub open: &'static str,
    pub quit: &'static str,
    pub check_update: &'static str,
    /// `{}` 자리에 버전
    pub install_update: &'static str,
    pub idle: &'static str,
    /// `{n}` 실행 중 개수, `{p}` 평균 진행률
    pub active: &'static str,
    pub settings_title: &'static str,
    pub quit_title: &'static str,
    pub quit_message: &'static str,
    pub quit_ok: &'static str,
    pub quit_cancel: &'static str,
}

pub fn labels(lang: Lang) -> Labels {
    match lang {
        Lang::Ko => Labels {
            open: "kiri 열기",
            quit: "종료",
            check_update: "업데이트 확인",
            install_update: "v{} 설치",
            idle: "대기 중인 작업 없음",
            active: "진행 중 {n}개 · {p}%",
            settings_title: "kiri 설정",
            quit_title: "kiri를 종료할까요?",
            quit_message: "진행 중인 다운로드가 있습니다. 종료하면 다음 실행 때 이어서 받습니다.",
            quit_ok: "종료",
            quit_cancel: "취소",
        },
        Lang::En => Labels {
            open: "Open kiri",
            quit: "Quit",
            check_update: "Check for Updates",
            install_update: "Install v{}",
            idle: "No active jobs",
            active: "{n} running · {p}%",
            settings_title: "kiri Settings",
            quit_title: "Quit kiri?",
            quit_message: "Downloads are in progress. They will resume the next time kiri starts.",
            quit_ok: "Quit",
            quit_cancel: "Cancel",
        },
        Lang::Ja => Labels {
            open: "kiri を開く",
            quit: "終了",
            check_update: "アップデートを確認",
            install_update: "v{} をインストール",
            idle: "待機中のジョブはありません",
            active: "実行中 {n} 件 · {p}%",
            settings_title: "kiri 設定",
            quit_title: "kiri を終了しますか？",
            quit_message: "ダウンロード中です。次回起動時に再開します。",
            quit_ok: "終了",
            quit_cancel: "キャンセル",
        },
    }
}

pub fn current(app: &tauri::AppHandle) -> Labels {
    use tauri::Manager;
    labels(resolve(
        &app.state::<crate::settings::SettingsState>()
            .get()
            .general
            .ui_language,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_language() {
        assert_eq!(resolve_with("ja", Some("ko-KR")), Lang::Ja);
        assert_eq!(resolve_with("system", Some("ko-KR")), Lang::Ko);
        assert_eq!(resolve_with("system", Some("de")), Lang::En);
        assert_eq!(resolve_with("system", None), Lang::En);
    }

    #[test]
    fn labels_are_localized() {
        assert_eq!(labels(Lang::Ko).quit, "종료");
        assert_eq!(labels(Lang::En).open, "Open kiri");
        assert_eq!(labels(Lang::Ja).idle, "待機中のジョブはありません");
        assert!(labels(Lang::Ko).active.contains("{n}") && labels(Lang::Ko).active.contains("{p}"));
    }
}
