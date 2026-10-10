#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod composition;
#[cfg(feature = "native-smoke")]
mod native_smoke;

use bridge::{BridgeError, Operation, Result, Session, Snapshot, Target};
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri_plugin_opener::OpenerExt;

type SharedSession = Arc<Mutex<Session>>;

async fn blocking<T: Send + 'static>(
    state: &SharedSession,
    work: impl FnOnce(&mut Session) -> Result<T> + Send + 'static,
) -> Result<T> {
    let state = Arc::clone(state);
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = state
            .lock()
            .map_err(|_| BridgeError::internal("workspace session interrupted; restart Spacer"))?;
        work(&mut session)
    })
    .await
    .map_err(|error| BridgeError::internal(error.to_string()))?
}

#[tauri::command]
async fn open_workspace(
    state: State<'_, SharedSession>,
    root: String,
    initialize: bool,
) -> Result<Snapshot> {
    blocking(&state, move |session| session.open(&root, initialize)).await
}
#[tauri::command]
async fn read_workspace(state: State<'_, SharedSession>, root: String) -> Result<Snapshot> {
    blocking(&state, move |session| session.read(&root)).await
}
#[tauri::command]
async fn mutate_workspace(
    state: State<'_, SharedSession>,
    root: String,
    operation: Operation,
) -> Result<Snapshot> {
    blocking(&state, move |session| session.mutate(&root, operation)).await
}
#[tauri::command]
async fn resolve_target(
    state: State<'_, SharedSession>,
    root: String,
    target: Target,
) -> Result<std::path::PathBuf> {
    blocking(&state, move |session| session.resolve(&root, target)).await
}
#[tauri::command]
async fn reveal_target(
    app: tauri::AppHandle,
    state: State<'_, SharedSession>,
    root: String,
    target: Target,
) -> Result<()> {
    blocking(&state, move |session| {
        let path = session
            .resolve(&root, target)?
            .into_os_string()
            .into_string()
            .map_err(|_| BridgeError::internal("the desktop app requires UTF-8 folder paths"))?;
        app.opener()
            .open_path(path, None::<&str>)
            .map_err(|error| BridgeError::internal(error.to_string()))
    })
    .await
}

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(SharedSession::default())
        .invoke_handler(tauri::generate_handler![
            open_workspace,
            read_workspace,
            mutate_workspace,
            resolve_target,
            reveal_target
        ]);
    #[cfg(feature = "native-smoke")]
    let builder = builder
        .setup(native_smoke::setup)
        .on_page_load(native_smoke::page_loaded);
    let context = tauri::generate_context!();
    #[cfg(feature = "native-smoke")]
    let context = {
        let mut context = context;
        for window in &mut context.config_mut().app.windows {
            window.incognito = true;
        }
        context
    };
    builder
        .run(context)
        .expect("could not start Spacer desktop");
}
