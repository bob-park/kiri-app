//! Text Rust draws itself: the menu bar, native dialogs and window titles.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
}

pub fn resolve(pref: &str) -> Lang {
    resolve_with(pref, sys_locale::get_locale().as_deref())
}

/// `ko` and `en` are explicit choices. Anything else (`system`, the dropped `ja`,
/// unknown values) follows the OS language: `ko*` is Korean, everything else English.
pub fn resolve_with(pref: &str, system: Option<&str>) -> Lang {
    match pref {
        "ko" => Lang::Ko,
        "en" => Lang::En,
        _ if system
            .and_then(|s| s.split(['-', '_']).next())
            .is_some_and(|code| code.eq_ignore_ascii_case("ko")) =>
        {
            Lang::Ko
        }
        _ => Lang::En,
    }
}

pub struct Labels {
    pub open: &'static str,
    pub quit: &'static str,
    pub check_update: &'static str,
    /// `{}` is the version.
    pub install_update: &'static str,
    pub idle: &'static str,
    /// `{n}` is the number of running jobs, `{p}` their average progress.
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
    }
}

pub fn current(app: &tauri::AppHandle) -> Labels {
    use tauri::Manager;
    labels(resolve(
        &app.state::<crate::app::state::AppState>()
            .settings()
            .general
            .ui_language,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_language() {
        assert_eq!(resolve_with("system", Some("ko-KR")), Lang::Ko);
        assert_eq!(resolve_with("system", Some("de")), Lang::En);
        assert_eq!(resolve_with("system", None), Lang::En);
    }

    #[test]
    fn ja_pref_follows_system() {
        assert_eq!(resolve_with("ja", Some("ko-KR")), Lang::Ko);
        assert_eq!(resolve_with("ja", Some("fr")), Lang::En);
    }

    #[test]
    fn labels_are_localized() {
        assert_eq!(labels(Lang::Ko).quit, "종료");
        assert_eq!(labels(Lang::En).open, "Open kiri");
        assert!(labels(Lang::Ko).active.contains("{n}") && labels(Lang::Ko).active.contains("{p}"));
    }
}
