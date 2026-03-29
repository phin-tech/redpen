mod bridge;
mod commands;
mod event_bus;
mod notification;
mod settings;
mod state;
mod storage;
mod workspace_index;

use notification::{NotificationKind, NotificationService};
use state::AppState;
use tauri::menu::{
    IconMenuItemBuilder, MenuBuilder, MenuItemBuilder, NativeIcon, PredefinedMenuItem,
    SubmenuBuilder,
};
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

#[tauri::command]
fn get_pending_deep_links(state: tauri::State<AppState>) -> Vec<String> {
    let Ok(mut pending) = state.pending_deep_links.lock() else {
        return Vec::new();
    };
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
            commands::annotations::read_file_lines,
            commands::annotations::delete_annotation,
            commands::annotations::clear_annotations,
            commands::annotations::update_settings,
            commands::annotations::get_settings,
            commands::annotations::signal_review_done,
            commands::annotations::send_notification,
            commands::git::get_git_root,
            commands::git::get_git_status,
            commands::github_review::list_github_review_queue,
            commands::github_review::open_github_pr_review,
            commands::github_review::resync_github_pr_review,
            commands::github_review::submit_github_pr_review,
            commands::github_review::discard_pending_github_review_changes,
            commands::review_history::get_review_history,
            commands::review_history::resume_review_session,
            commands::review_history::cleanup_stale_review_sessions,
            commands::diff::compute_diff,
            commands::diff::list_refs,
            commands::export::export_annotations,
            get_pending_deep_links,
        ])
        .setup(|app| {
            let event_bus = crate::event_bus::TauriEventBus::new(app.handle().clone());
            let annotation_service = redpen_runtime::annotations::AnnotationService::new(event_bus);
            app.manage(annotation_service);

            // ── Red Pen app menu ──────────────────────────────────────────────
            let settings_item = IconMenuItemBuilder::with_id("settings", "Settings…")
                .accelerator("Cmd+,")
                .native_icon(NativeIcon::PreferencesGeneral)
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

            // ── File menu ─────────────────────────────────────────────────────
            let open_folder = IconMenuItemBuilder::with_id("file.open_folder", "Open Folder…")
                .accelerator("Cmd+O")
                .native_icon(NativeIcon::Folder)
                .build(app)?;

            let go_to_file = IconMenuItemBuilder::with_id("file.go_to_file", "Go to File…")
                .accelerator("Cmd+P")
                .native_icon(NativeIcon::QuickLook)
                .build(app)?;

            let export_annotations =
                IconMenuItemBuilder::with_id("file.export_annotations", "Export Annotations…")
                    .native_icon(NativeIcon::Share)
                    .build(app)?;

            let file_submenu = SubmenuBuilder::new(app, "File")
                .item(&open_folder)
                .item(&go_to_file)
                .separator()
                .item(&export_annotations)
                .build()?;

            // ── Edit menu ─────────────────────────────────────────────────────
            let find = MenuItemBuilder::with_id("edit.find", "Find…")
                .accelerator("Cmd+F")
                .build(app)?;

            let find_next = MenuItemBuilder::with_id("edit.find_next", "Find Next")
                .accelerator("Cmd+G")
                .build(app)?;

            let find_previous = MenuItemBuilder::with_id("edit.find_previous", "Find Previous")
                .accelerator("Cmd+Shift+G")
                .build(app)?;

            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .separator()
                .item(&find)
                .item(&find_next)
                .item(&find_previous)
                .build()?;

            // ── Annotations menu ──────────────────────────────────────────────
            let add_annotation =
                IconMenuItemBuilder::with_id("annotations.add", "Add Annotation")
                    .accelerator("Cmd+Return")
                    .native_icon(NativeIcon::Add)
                    .build(app)?;

            let reload_annotations =
                IconMenuItemBuilder::with_id("annotations.reload", "Reload Annotations")
                    .native_icon(NativeIcon::Refresh)
                    .build(app)?;

            let clear_annotations =
                IconMenuItemBuilder::with_id("annotations.clear", "Clear All Annotations…")
                    .native_icon(NativeIcon::TrashEmpty)
                    .build(app)?;

            let annotations_submenu = SubmenuBuilder::new(app, "Annotations")
                .item(&add_annotation)
                .item(&reload_annotations)
                .separator()
                .item(&clear_annotations)
                .build()?;

            // ── View → Diff submenu ───────────────────────────────────────────
            let diff_split = MenuItemBuilder::with_id("diff.split", "Split View").build(app)?;
            let diff_unified =
                MenuItemBuilder::with_id("diff.unified", "Unified View").build(app)?;
            let diff_highlights =
                MenuItemBuilder::with_id("diff.highlights", "Highlights Only").build(app)?;
            let diff_exit = MenuItemBuilder::with_id("diff.exit", "Exit Diff").build(app)?;

            let diff_submenu = SubmenuBuilder::new(app, "Diff")
                .item(&diff_split)
                .item(&diff_unified)
                .item(&diff_highlights)
                .separator()
                .item(&diff_exit)
                .build()?;

            // ── View menu ─────────────────────────────────────────────────────
            let toggle_markdown =
                MenuItemBuilder::with_id("view.toggle_markdown", "Toggle Markdown Preview")
                    .accelerator("Cmd+Shift+M")
                    .build(app)?;

            let command_palette =
                MenuItemBuilder::with_id("view.command_palette", "Command Palette")
                    .accelerator("Cmd+K")
                    .build(app)?;

            let view_submenu = SubmenuBuilder::new(app, "View")
                .item(&command_palette)
                .item(&toggle_markdown)
                .separator()
                .item(&diff_submenu)
                .separator()
                .fullscreen()
                .build()?;

            // ── Review menu ───────────────────────────────────────────────────
            let review_changes = MenuItemBuilder::with_id("review.changes", "Review Changes")
                .accelerator("Cmd+Shift+R")
                .build(app)?;

            let agent_feedback =
                MenuItemBuilder::with_id("review.feedback", "Agent Feedback").build(app)?;

            let approve_review = IconMenuItemBuilder::with_id("review.approve", "Approve")
                .native_icon(NativeIcon::StatusAvailable)
                .build(app)?;

            let request_changes =
                IconMenuItemBuilder::with_id("review.request_changes", "Request Changes")
                    .native_icon(NativeIcon::StatusUnavailable)
                    .build(app)?;

            let review_submenu = SubmenuBuilder::new(app, "Review")
                .item(&review_changes)
                .item(&agent_feedback)
                .separator()
                .item(&approve_review)
                .item(&request_changes)
                .build()?;

            // ── Window menu ───────────────────────────────────────────────────
            let window_submenu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .close_window()
                .build()?;

            // ── Assemble ──────────────────────────────────────────────────────
            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&file_submenu)
                .item(&edit_submenu)
                .item(&annotations_submenu)
                .item(&view_submenu)
                .item(&review_submenu)
                .item(&window_submenu)
                .build()?;

            app.set_menu(menu)?;

            // ── Handle menu events ────────────────────────────────────────────
            let handle_menu = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                let name = match event.id().0.as_str() {
                    "settings" => "open-settings",
                    "file.open_folder" => "menu-open-folder",
                    "file.go_to_file" => "menu-go-to-file",
                    "file.export_annotations" => "menu-export-annotations",
                    "annotations.add" => "menu-add-annotation",
                    "annotations.reload" => "menu-reload-annotations",
                    "annotations.clear" => "menu-clear-annotations",
                    "view.toggle_markdown" => "menu-toggle-markdown-preview",
                    "view.command_palette" => "menu-command-palette",
                    "diff.split" => "menu-diff-split",
                    "diff.unified" => "menu-diff-unified",
                    "diff.highlights" => "menu-diff-highlights",
                    "diff.exit" => "menu-diff-exit",
                    "review.changes" => "menu-review-changes",
                    "review.feedback" => "menu-agent-feedback",
                    "review.approve" => "menu-approve-review",
                    "review.request_changes" => "menu-request-changes",
                    "edit.find" => "menu-find",
                    "edit.find_next" => "menu-find-next",
                    "edit.find_previous" => "menu-find-previous",
                    _ => return,
                };
                let _ = handle_menu.emit(name, ());
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
                if let Ok(mut pending) = state.pending_deep_links.lock() {
                    for url in urls {
                        pending.push(url.to_string());
                    }
                }; // semicolon ensures MutexGuard drops before state
            }

            // Start optional local HTTP server for CLI/agent communication
            let bridge = bridge::TauriBridge::new(app.handle().clone());
            let review_sessions = app.state::<AppState>().review_sessions.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = redpen_server::start_server(bridge, review_sessions).await {
                    eprintln!("Red Pen server failed to start: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
