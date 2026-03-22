mod commands;
mod state;

use state::AppState;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri::{Emitter, Manager};

#[tauri::command]
fn get_pending_deep_links(state: tauri::State<AppState>) -> Vec<String> {
    let mut pending = state.pending_deep_links.lock().unwrap();
    pending.drain(..).collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::files::read_directory,
            commands::files::read_file,
            commands::annotations::get_annotations,
            commands::annotations::create_annotation,
            commands::annotations::update_annotation,
            commands::annotations::delete_annotation,
            commands::annotations::update_settings,
            commands::annotations::get_settings,
            commands::annotations::signal_review_done,
            commands::git::get_git_root,
            commands::git::get_git_status,
            commands::export::export_annotations,
            get_pending_deep_links,
        ])
        .setup(|app| {
            // Handle deep links received while app is running (warm start)
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    let _ = handle.emit("deep-link-open", url.to_string());
                }
            });

            // Handle deep link that launched the app (cold start)
            // Store in state so the frontend can fetch on mount
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                let state = app.state::<AppState>();
                let mut pending = state.pending_deep_links.lock().unwrap();
                for url in urls {
                    pending.push(url.to_string());
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
