use std::process::Command;
use tauri::Manager;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::ShortcutState;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Public function that can be called from integration tests
pub fn atuin_search(query: &str) -> Result<String, String> {
    let output = Command::new("atuin")
        .arg("search")
        .arg("--search-mode")
        .arg("prefix")
        .arg("--limit")
        .arg("50")
        .arg("--format")
        .arg("{command}|{exit}|{directory}|{time}")
        .arg(query)
        .output()
        .map_err(|e| format!("Failed to execute atuin command: {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("Failed to parse atuin output: {}", e))
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        Err(format!("atuin command failed: {}", error_message))
    }
}

// Tauri command wrapper (private)
#[tauri::command]
fn atuin_search_command(query: &str) -> Result<String, String> {
    atuin_search(query)
}

#[tauri::command]
async fn copy_to_clipboard<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    text: String,
) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Define the global shortcut: Cmd+Shift+Space (macOS) or Ctrl+Shift+Space (other platforms)
    let shortcut = if cfg!(target_os = "macos") {
        "CommandOrControl+Shift+Space"
    } else {
        "Control+Shift+Space"
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([shortcut])
                .unwrap()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        // Toggle window visibility
                        if let Some(window) = app.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(visible) => {
                                    if visible {
                                        let _ = window.hide();
                                    } else {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            atuin_search_command,
            copy_to_clipboard
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tauri::test::{mock_builder, mock_context, noop_assets};

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_empty_string() {
        let result = greet("");
        assert_eq!(result, "Hello, ! You've been greeted from Rust!");
    }

    #[tokio::test]
    #[serial]
    async fn test_copy_to_clipboard() {
        // Create a mock Tauri app with clipboard plugin
        let app = mock_builder()
            .plugin(tauri_plugin_clipboard_manager::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build mock app");

        let test_text = "Hello, clipboard!".to_string();

        // Test the copy_to_clipboard command
        let result = copy_to_clipboard(app.handle().clone(), test_text.clone()).await;

        // Verify the command executed successfully
        assert!(result.is_ok(), "copy_to_clipboard should succeed");

        // Verify the text was written to clipboard
        let clipboard_content = app.handle().clipboard().read_text();
        assert!(
            clipboard_content.is_ok(),
            "should be able to read clipboard"
        );
        assert_eq!(
            clipboard_content.unwrap(),
            test_text,
            "clipboard should contain the copied text"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_copy_empty_string_to_clipboard() {
        let app = mock_builder()
            .plugin(tauri_plugin_clipboard_manager::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build mock app");

        let empty_text = "".to_string();
        let result = copy_to_clipboard(app.handle().clone(), empty_text).await;

        assert!(
            result.is_ok(),
            "copy_to_clipboard should handle empty strings"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_copy_unicode_to_clipboard() {
        let app = mock_builder()
            .plugin(tauri_plugin_clipboard_manager::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build mock app");

        let unicode_text = "Hello 世界 🌍".to_string();
        let result = copy_to_clipboard(app.handle().clone(), unicode_text.clone()).await;

        assert!(
            result.is_ok(),
            "copy_to_clipboard should handle unicode text"
        );

        let clipboard_content = app.handle().clipboard().read_text();
        assert!(clipboard_content.is_ok());
        assert_eq!(
            clipboard_content.unwrap(),
            unicode_text,
            "clipboard should preserve unicode characters"
        );
    }
}
