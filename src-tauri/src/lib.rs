mod commands;
mod settings;
mod state;
mod workspace_index;

use state::AppState;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri::{Emitter, Manager};
use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder, PredefinedMenuItem};

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
        .manage(AppState::new().expect("failed to initialize app state"))
        .invoke_handler(tauri::generate_handler![
            commands::files::read_directory,
            commands::files::register_workspace_root,
            commands::files::unregister_workspace_root,
            commands::files::get_workspace_index_status,
            commands::files::query_workspace_files,
            commands::files::read_file,
            commands::annotations::get_annotations,
            commands::annotations::get_all_annotations,
            commands::annotations::create_annotation,
            commands::annotations::update_annotation,
            commands::annotations::delete_annotation,
            commands::annotations::clear_annotations,
            commands::annotations::update_settings,
            commands::annotations::get_settings,
            commands::annotations::signal_review_done,
            commands::git::get_git_root,
            commands::git::get_git_status,
            commands::export::export_annotations,
            get_pending_deep_links,
        ])
        .setup(|app| {
            // Build native menu bar
            let settings_item = MenuItemBuilder::with_id("settings", "Settings...")
                .accelerator("Cmd+,")
                .build(app)?;

            let app_submenu = SubmenuBuilder::new(app, "Red Pen")
                .item(&PredefinedMenuItem::about(app, Some("About Red Pen"), None)?)
                .separator()
                .item(&settings_item)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let view_submenu = SubmenuBuilder::new(app, "View")
                .fullscreen()
                .build()?;

            let window_submenu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .close_window()
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&edit_submenu)
                .item(&view_submenu)
                .item(&window_submenu)
                .build()?;

            app.set_menu(menu)?;

            // Handle menu events
            let handle_menu = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                if event.id().0 == "settings" {
                    let _ = handle_menu.emit("open-settings", ());
                }
            });

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
