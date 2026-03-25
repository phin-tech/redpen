mod commands;
mod notification;
mod settings;
mod state;
mod workspace_index;

use notification::{NotificationKind, NotificationService};
use state::AppState;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

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
        .plugin(tauri_plugin_notification::init())
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
            commands::annotations::send_notification,
            commands::git::get_git_root,
            commands::git::get_git_status,
            commands::diff::compute_diff,
            commands::diff::list_refs,
            commands::export::export_annotations,
            get_pending_deep_links,
        ])
        .setup(|app| {
            // Build native menu bar
            let settings_item = MenuItemBuilder::with_id("settings", "Settings...")
                .accelerator("Cmd+,")
                .build(app)?;

            let app_submenu = SubmenuBuilder::new(app, "Red Pen")
                .item(&PredefinedMenuItem::about(
                    app,
                    Some("About Red Pen"),
                    None,
                )?)
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

            let view_submenu = SubmenuBuilder::new(app, "View").fullscreen().build()?;

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

            let notification_service = NotificationService::new(app.handle().clone());

            // Handle deep links received while app is running (warm start)
            let handle = app.handle().clone();
            let state_for_links = app.state::<AppState>();
            let settings_for_links = state_for_links.settings.clone();
            app.deep_link().on_open_url(move |event| {
                for raw_url in event.urls() {
                    let url_str = raw_url.to_string();
                    if let Ok(parsed) = Url::parse(&url_str) {
                        match parsed.host_str() {
                            Some("notify") => {
                                let params: std::collections::HashMap<String, String> = parsed
                                    .query_pairs()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect();
                                let kind_str = params.get("kind").map(|s| s.as_str()).unwrap_or("");
                                let file = params.get("file");
                                let line = params.get("line");

                                let kind = match kind_str {
                                    "annotation_reply" => Some(NotificationKind::AnnotationReply),
                                    "review_complete" => Some(NotificationKind::ReviewComplete),
                                    "new_annotation" => Some(NotificationKind::NewAnnotation),
                                    // "deep_link" is not handled here by design — DeepLink
                                    // notifications are fired in the default branch below
                                    _ => None,
                                };

                                if let Some(kind) = kind {
                                    let settings = settings_for_links.lock().unwrap();
                                    let file_name = file
                                        .and_then(|f| f.rsplit('/').next().map(|s| s.to_string()))
                                        .unwrap_or_else(|| "unknown".to_string());
                                    let line_num = line.and_then(|l| l.parse::<u32>().ok());
                                    let (title, body) =
                                        kind.default_title_body(&file_name, line_num);
                                    let _ =
                                        notification_service.send(kind, &title, &body, &settings);
                                }

                                // Emit refresh first so annotations reload, then navigate
                                if let Some(file) = file {
                                    let refresh_url = format!("redpen://refresh?file={}", file);
                                    let _ = handle.emit("deep-link-open", refresh_url);
                                    let mut nav_url = format!("redpen://open?file={}", file);
                                    if let Some(line) = line {
                                        nav_url.push_str(&format!("&line={}", line));
                                    }
                                    let _ = handle.emit("deep-link-open", nav_url);
                                }
                            }
                            _ => {
                                // Fire DeepLink notification
                                if let Some(file) = parsed
                                    .query_pairs()
                                    .find(|(k, _)| k == "file")
                                    .map(|(_, v)| v.to_string())
                                {
                                    let file_name = file.rsplit('/').next().unwrap_or("unknown");
                                    let line = parsed
                                        .query_pairs()
                                        .find(|(k, _)| k == "line")
                                        .map(|(_, v)| v.to_string());
                                    let line_str = line
                                        .as_ref()
                                        .map(|l| format!(":{}", l))
                                        .unwrap_or_default();
                                    let settings = settings_for_links.lock().unwrap();
                                    let _ = notification_service.send(
                                        NotificationKind::DeepLink,
                                        "Opening file",
                                        &format!("{}{}", file_name, line_str),
                                        &settings,
                                    );
                                }
                                let _ = handle.emit("deep-link-open", url_str);
                            }
                        }
                    } else {
                        let _ = handle.emit("deep-link-open", url_str);
                    }
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
