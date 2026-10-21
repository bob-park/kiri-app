//! `AppError`, the single error type returned to the webview as `{ code, message }`.

use kiri_core::engine::EngineError;
use serde::ser::{Serialize, SerializeStruct, Serializer};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database schema version {found} is newer than supported version {supported}")]
    SchemaTooNew { found: i64, supported: i64 },
    #[allow(dead_code)] // ponytail: first used by the queue domain (Task 4); drop the allow then
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Failed(String),
    /// Engine failures keep their own code, uppercased (`not_found` -> `NOT_FOUND`).
    #[error("{0}")]
    Engine(#[from] EngineError),
}

impl AppError {
    /// Stable SCREAMING_SNAKE code the frontend looks up in `t.errors`.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::InvalidInput(_) => "INVALID_INPUT",
            AppError::Db(_) => "DB",
            AppError::Io(_) => "IO",
            AppError::SchemaTooNew { .. } => "SCHEMA_TOO_NEW",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Failed(_) => "FAILED",
            AppError::Engine(e) => engine_code(e),
        }
    }
}

/// `EngineError::code()` uppercased. Spelled out so `code()` stays `&'static str`;
/// the exhaustive match breaks the build when the engine gains a variant, and
/// `engine_codes_are_uppercased` checks the spelling.
fn engine_code(e: &EngineError) -> &'static str {
    match e {
        EngineError::NotFound(_) => "NOT_FOUND",
        EngineError::NotStoppable(_) => "NOT_STOPPABLE",
        EngineError::NotRestartable(_) => "NOT_RESTARTABLE",
        EngineError::YtdlpMissing => "YTDLP_MISSING",
        EngineError::InvalidUrl => "INVALID_URL",
        EngineError::Probe(_) => "PROBE_FAILED",
        EngineError::BadQuality(_) => "BAD_QUALITY",
        EngineError::BadPreset(_) => "BAD_PRESET",
        EngineError::SourceNotFound(_) => "SOURCE_NOT_FOUND",
        EngineError::InvalidMedia(_) => "INVALID_MEDIA",
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_code_and_message() {
        let v = serde_json::to_value(AppError::InvalidInput("x".into())).unwrap();
        assert_eq!(v, serde_json::json!({ "code": "INVALID_INPUT", "message": "x" }));
    }

    #[test]
    fn engine_codes_are_uppercased() {
        let all = [
            EngineError::NotFound(1),
            EngineError::NotStoppable(1),
            EngineError::NotRestartable(1),
            EngineError::YtdlpMissing,
            EngineError::InvalidUrl,
            EngineError::Probe("p".into()),
            EngineError::BadQuality("q".into()),
            EngineError::BadPreset("f".into()),
            EngineError::SourceNotFound("s".into()),
            EngineError::InvalidMedia("m".into()),
        ];
        for e in all {
            let (code, message) = (e.code().to_ascii_uppercase(), e.to_string());
            let v = serde_json::to_value(AppError::from(e)).unwrap();
            assert_eq!(v, serde_json::json!({ "code": code, "message": message }));
        }
    }
}
