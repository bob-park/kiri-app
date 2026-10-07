#[path = "src/pubkey_check.rs"]
mod pubkey_check;

fn main() {
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        let conf = std::fs::read_to_string("tauri.conf.json").unwrap_or_default();
        if pubkey_check::pubkey_missing(&conf) {
            panic!("updater pubkey is empty — see docs/development.md");
        }
    }
    tauri_build::build()
}
