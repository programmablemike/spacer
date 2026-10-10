//! Opt-in test harness, excluded from ordinary desktop builds.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Listener, Manager};

static STARTED: AtomicBool = AtomicBool::new(false);

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var("SPACER_DESKTOP_SMOKE_ROOT")?;
    let path = std::path::Path::new(&root);
    if !path.is_absolute() || !path.is_dir() || path.join(".spacer").exists() {
        return Err("native smoke requires an isolated, uninitialized absolute root".into());
    }
    let handle = app.handle().clone();
    app.listen("spacer-native-smoke", move |event| {
        let result: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
        println!("SPACER_NATIVE_SMOKE: {result}");
        handle.exit(if result["success"] == true { 0 } else { 1 });
    });
    Ok(())
}

pub fn page_loaded(webview: &tauri::Webview, payload: &tauri::webview::PageLoadPayload<'_>) {
    if payload.event() != tauri::webview::PageLoadEvent::Finished
        || STARTED.swap(true, Ordering::SeqCst)
    {
        return;
    }
    let root = std::env::var("SPACER_DESKTOP_SMOKE_ROOT").expect("smoke root validated in setup");
    let script = format!(
        "const SMOKE_ROOT = {};\n{}",
        serde_json::to_string(&root).unwrap(),
        include_str!("../../tests/native-smoke.js")
    );
    if let Err(error) = webview.eval(script) {
        eprintln!("could not start native smoke: {error}");
        webview.app_handle().exit(1);
    }
}
