use tracing::debug;

const VERSION: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../VERSION"));

#[tauri::command]
pub fn app_version() -> String {
    VERSION.trim().to_string()
}

#[tauri::command]
pub fn ping(name: Option<String>) -> String {
    debug!(?name, "ping");
    match name {
        Some(n) => format!("pong, {n}"),
        None => "pong".into(),
    }
}
