use tauri_plugin_clipboard_manager::ClipboardExt;

#[tauri::command]
pub async fn copy_to_clipboard<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    text: String,
) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tauri::test::{mock_builder, mock_context, noop_assets};

    #[tokio::test]
    #[serial]
    async fn test_copy_to_clipboard() {
        let app = mock_builder()
            .plugin(tauri_plugin_clipboard_manager::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build mock app");

        let test_text = "Hello, clipboard!".to_string();
        let result = copy_to_clipboard(app.handle().clone(), test_text.clone()).await;

        assert!(result.is_ok(), "copy_to_clipboard should succeed");
        let clipboard_content = app.handle().clipboard().read_text();
        assert!(
            clipboard_content.is_ok(),
            "should be able to read clipboard"
        );
        assert_eq!(clipboard_content.unwrap(), test_text);
    }

    #[tokio::test]
    #[serial]
    async fn test_copy_empty_string_to_clipboard() {
        let app = mock_builder()
            .plugin(tauri_plugin_clipboard_manager::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build mock app");

        let result = copy_to_clipboard(app.handle().clone(), String::new()).await;
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
        assert_eq!(clipboard_content.unwrap(), unicode_text);
    }
}
