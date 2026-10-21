//! Guards release builds against a missing updater public key. Shared by build.rs and tests.

pub fn pubkey_missing(conf_json: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(conf_json) else {
        return true;
    };
    match v
        .pointer("/plugins/updater/pubkey")
        .and_then(|k| k.as_str())
    {
        Some(k) => k.trim().is_empty() || k == "<PUBKEY>",
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_pubkey() {
        assert!(pubkey_missing(r#"{"plugins":{"updater":{"pubkey":""}}}"#));
        assert!(pubkey_missing(
            r#"{"plugins":{"updater":{"pubkey":"<PUBKEY>"}}}"#
        ));
        assert!(pubkey_missing(r#"{"plugins":{}}"#));
        assert!(pubkey_missing("not json"));
        assert!(!pubkey_missing(
            r#"{"plugins":{"updater":{"pubkey":"dW50cnVzdGVk"}}}"#
        ));
    }
}
