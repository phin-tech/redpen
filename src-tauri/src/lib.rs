mod commands;
mod state;

use state::AppState;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri::Emitter;

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
            commands::git::get_git_status,
            commands::export::export_annotations,
        ])
        .setup(|app| {
            // Handle deep links received while app is running
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    let _ = handle.emit("deep-link-open", url.to_string());
                }
            });

            // Handle deep link that launched the app (cold start)
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                let handle = app.handle().clone();
                for url in urls {
                    let _ = handle.emit("deep-link-open", url.to_string());
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
