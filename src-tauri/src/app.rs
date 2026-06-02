use crate::config::load_config;
use crate::logging::log_debug;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    Manager,
};
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log_debug("=== atuin-bar starting ===");
    log_debug(&format!(
        "HOME={}",
        std::env::var("HOME").unwrap_or_else(|_| "NOT SET".into())
    ));
    log_debug(&format!(
        "USER={}",
        std::env::var("USER").unwrap_or_else(|_| "NOT SET".into())
    ));

    let config = load_config();
    let shortcut = config.shortcut;
    log_debug(&format!("shortcut={}", shortcut));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([shortcut.as_str()])
                .unwrap()
                .with_handler(|app, _shortcut, event| {
                    log_debug(&format!("shortcut event: {:?}", event.state));
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(visible) = window.is_visible() {
                                if visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            crate::commands::greet,
            crate::commands::atuin_search_command,
            crate::clipboard::copy_to_clipboard,
            crate::commands::get_theme,
            crate::commands::get_max_results,
            crate::commands::get_window_width,
            crate::commands::get_config,
            crate::commands::update_config,
            crate::updater::self_update
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(focused) = event {
                    if !focused {
                        let _ = window_clone.hide();
                    }
                }
            });

            let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let menu = MenuBuilder::new(app).item(&settings_item).build()?;

            app.set_menu(menu)?;

            app.on_menu_event(move |app, event| {
                if event.id().as_ref() == "settings" {
                    if let Some(settings_window) = app.get_webview_window("settings") {
                        let _ = settings_window.show();
                        let _ = settings_window.set_focus();
                    } else {
                        use tauri::WebviewUrl;
                        use tauri::WebviewWindowBuilder;

                        let settings_window = WebviewWindowBuilder::new(
                            app,
                            "settings",
                            WebviewUrl::App("settings.html".into()),
                        )
                        .title("Atuin Bar Settings")
                        .inner_size(500.0, 500.0)
                        .resizable(false)
                        .center()
                        .build();

                        if let Ok(win) = settings_window {
                            let _ = win.show();
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
