use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

fn log_debug(msg: &str) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_path = format!("{}/atuin-bar-debug.log", home);
    match fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(mut file) => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
            let _ = file.flush();
        }
        Err(e) => {
            eprintln!("log_debug failed to open {}: {}", log_path, e);
        }
    }
}

/// Resolve the full path to the `atuin` binary.
/// macOS GUI apps don't inherit the user's shell PATH, so we check common locations.
fn find_atuin_binary() -> String {
    let candidates = [
        // Homebrew (Apple Silicon)
        "/opt/homebrew/bin/atuin",
        // Homebrew (Intel)
        "/usr/local/bin/atuin",
        // Cargo install
        &format!("{}/.cargo/bin/atuin", std::env::var("HOME").unwrap_or_default()),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    // Fallback: hope it's on PATH
    "atuin".to_string()
}

static ATUIN_BIN: LazyLock<String> = LazyLock::new(find_atuin_binary);
use tauri::{menu::{MenuBuilder, MenuItemBuilder}, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::ShortcutState;

/// Application configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    /// Global shortcut to toggle the window (e.g., "CommandOrControl+Shift+Space")
    pub shortcut: String,
    /// Theme: "dark" or "light" (default: "dark")
    pub theme: String,
    /// Maximum number of results to display (default: 20)
    pub max_results: u32,
    /// Window width in pixels (default: 700)
    pub window_width: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shortcut: if cfg!(target_os = "macos") {
                "CommandOrControl+Shift+Space".to_string()
            } else {
                "Control+Shift+Space".to_string()
            },
            theme: "dark".to_string(),
            max_results: 20,
            window_width: 700,
        }
    }
}

/// Get the config file path (~/.config/atuin-bar/config.toml)
pub fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("atuin-bar").join("config.toml"))
}

/// Load configuration from file, falling back to defaults
pub fn load_config() -> Config {
    let Some(config_path) = get_config_path() else {
        return Config::default();
    };

    if !config_path.exists() {
        // Create default config file for user reference
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let default_config = r#"# Atuin Bar Configuration

# Global shortcut to toggle the window
# Examples: "CommandOrControl+Shift+Space", "Alt+Space", "Super+H"
shortcut = "CommandOrControl+Shift+Space"

# Theme: "dark" or "light" (default: "dark")
theme = "dark"

# Maximum number of results to display (default: 20)
max_results = 20

# Window width in pixels (default: 700)
window_width = 700
"#;
        let _ = fs::write(&config_path, default_config);
        return Config::default();
    }

    match fs::read_to_string(&config_path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Failed to parse config file: {}", e);
            Config::default()
        }),
        Err(e) => {
            eprintln!("Failed to read config file: {}", e);
            Config::default()
        }
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_theme() -> String {
    let config = load_config();
    config.theme
}

#[tauri::command]
fn get_max_results() -> u32 {
    let config = load_config();
    config.max_results
}

#[tauri::command]
fn get_window_width() -> u32 {
    let config = load_config();
    config.window_width
}

#[tauri::command]
fn get_config() -> Config {
    load_config()
}

#[tauri::command]
fn update_config(
    shortcut: Option<String>,
    theme: Option<String>,
    max_results: Option<u32>,
    window_width: Option<u32>,
) -> Result<Config, String> {
    let Some(config_path) = get_config_path() else {
        return Err("Could not determine config path".to_string());
    };

    // Load current config
    let mut config = load_config();

    // Update fields if provided
    if let Some(s) = shortcut {
        config.shortcut = s;
    }
    if let Some(t) = theme {
        config.theme = t;
    }
    if let Some(m) = max_results {
        config.max_results = m;
    }
    if let Some(w) = window_width {
        config.window_width = w;
    }

    // Serialize to TOML
    let toml_str = format!(
        r#"# Atuin Bar Configuration

# Global shortcut to toggle the window
# Examples: "CommandOrControl+Shift+Space", "Alt+Space", "Super+H"
shortcut = "{}"

# Theme: "dark" or "light" (default: "dark")
theme = "{}"

# Maximum number of results to display (default: 20)
max_results = {}

# Window width in pixels (default: 700)
window_width = {}
"#,
        config.shortcut, config.theme, config.max_results, config.window_width
    );

    // Write to file
    if let Some(parent) = config_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&config_path, toml_str).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(config)
}

/// A single parsed result from atuin search output
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AtuinResult {
    pub command: String,
    pub exit: String,
    pub duration: String,
    pub directory: String,
    pub time: String,
}

/// Parse a single line of atuin output in the format:
/// `{command}|{exit}|{duration}|{directory}|{time}`
///
/// The command field may contain `|` characters, so we split from the right.
pub fn parse_atuin_line(line: &str) -> Option<AtuinResult> {
    let parts: Vec<&str> = line.rsplitn(5, '|').collect();
    if parts.len() == 5 {
        Some(AtuinResult {
            command:   parts[4].to_string(),
            exit:      parts[3].to_string(),
            duration:  parts[2].to_string(),
            directory: parts[1].to_string(),
            time:      parts[0].to_string(),
        })
    } else {
        None
    }
}

/// Parse the full raw output from atuin search into a vec of results.
pub fn parse_atuin_output(raw: &str) -> Vec<AtuinResult> {
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(parse_atuin_line)
        .collect()
}

/// Search filters for atuin queries
#[derive(Debug, Default, serde::Deserialize)]
pub struct SearchFilters {
    /// Filter by directory path
    pub directory: Option<String>,
    /// Filter by exit code: "success" (0), "failure" (non-0), or None (all)
    pub exit_filter: Option<String>,
    /// Time range: "1h", "24h", "7d", "30d", or None (all)
    pub time_range: Option<String>,
}

// Public function that can be called from integration tests
pub fn atuin_search(query: &str, filters: Option<SearchFilters>) -> Result<Vec<AtuinResult>, String> {
    let home = std::env::var("HOME")
        .unwrap_or_else(|_| format!("/Users/{}", std::env::var("USER").unwrap_or_default()));

    log_debug(&format!("--- atuin_search called with query: {:?}", query));
    log_debug(&format!("HOME={}", home));
    log_debug(&format!("ATUIN_BIN={}", ATUIN_BIN.as_str()));
    log_debug(&format!("PATH={}", std::env::var("PATH").unwrap_or_default()));
    log_debug(&format!("atuin binary exists: {}", std::path::Path::new(ATUIN_BIN.as_str()).exists()));

    let mut cmd = Command::new(ATUIN_BIN.as_str());
    cmd.env("HOME", &home)
        // atuin requires ATUIN_SESSION even for non-interactive search;
        // GUI apps don't have a shell session, so provide a dummy value.
        .env("ATUIN_SESSION", "atuin-bar")
        .arg("search")
        .arg("--search-mode")
        .arg("prefix")
        // No real shell session, so search all history regardless of session.
        .arg("--filter-mode")
        .arg("global")
        .arg("--limit")
        .arg("50")
        .arg("--format")
        .arg("{command}|{exit}|{duration}|{directory}|{time}");

    let filters = filters.unwrap_or_default();

    // Apply directory filter
    if let Some(ref dir) = filters.directory {
        if !dir.is_empty() {
            cmd.arg("--cwd").arg(dir);
        }
    }

    // Apply exit code filter
    if let Some(ref exit_filter) = filters.exit_filter {
        match exit_filter.as_str() {
            "success" => {
                cmd.arg("--exit").arg("0");
            }
            "failure" => {
                cmd.arg("--exclude-exit").arg("0");
            }
            _ => {}
        }
    }

    // Apply time range filter
    if let Some(ref time_range) = filters.time_range {
        let after = match time_range.as_str() {
            "1h" => Some("1 hour ago"),
            "24h" => Some("1 day ago"),
            "7d" => Some("7 days ago"),
            "30d" => Some("30 days ago"),
            _ => None,
        };
        if let Some(after_str) = after {
            cmd.arg("--after").arg(after_str);
        }
    }

    cmd.arg(query);

    let output = cmd
        .output()
        .map_err(|e| {
            log_debug(&format!("Failed to execute atuin: {}", e));
            format!("Failed to execute atuin command: {}", e)
        })?;

    log_debug(&format!("exit status: {}", output.status));
    log_debug(&format!("stdout len: {}", output.stdout.len()));
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    if !stderr_str.is_empty() {
        log_debug(&format!("stderr: {}", stderr_str));
    }

    if output.status.success() {
        let raw = String::from_utf8(output.stdout).map_err(|e| format!("Failed to parse atuin output: {}", e))?;
        log_debug(&format!("success, output lines: {}", raw.lines().count()));
        Ok(parse_atuin_output(&raw))
    } else {
        // atuin exits with code 1 when there are no matching results —
        // treat this as an empty result set, not an error.
        let stderr_msg = String::from_utf8_lossy(&output.stderr);
        if stderr_msg.trim().is_empty() {
            log_debug("atuin exited non-zero with empty stderr, treating as no results");
            Ok(vec![])
        } else {
            Err(format!("atuin command failed: {}", stderr_msg))
        }
    }
}

// Tauri command wrapper (private)
#[tauri::command]
fn atuin_search_command(query: &str, filters: Option<SearchFilters>) -> Result<Vec<AtuinResult>, String> {
    log_debug(&format!("atuin_search_command invoked with query: {:?}", query));
    let result = atuin_search(query, filters);
    log_debug(&format!("atuin_search_command result: {:?}", result.as_ref().map(|v| v.len())));
    result
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
    log_debug("=== atuin-bar starting ===");
    log_debug(&format!("HOME={}", std::env::var("HOME").unwrap_or_else(|_| "NOT SET".into())));
    log_debug(&format!("USER={}", std::env::var("USER").unwrap_or_else(|_| "NOT SET".into())));
    log_debug(&format!("ATUIN_BIN={}", ATUIN_BIN.as_str()));

    // Load configuration
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            atuin_search_command,
            copy_to_clipboard,
            get_theme,
            get_max_results,
            get_window_width,
            get_config,
            update_config
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

            // Create menu
            let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&settings_item)
                .build()?;

            app.set_menu(menu)?;

            // Handle menu events
            app.on_menu_event(move |app, event| {
                if event.id().as_ref() == "settings" {
                    // Check if settings window already exists
                    if let Some(settings_window) = app.get_webview_window("settings") {
                        let _ = settings_window.show();
                        let _ = settings_window.set_focus();
                    } else {
                        // Create settings window
                        use tauri::WebviewWindowBuilder;
                        use tauri::WebviewUrl;

                        let settings_window = WebviewWindowBuilder::new(
                            app,
                            "settings",
                            WebviewUrl::App("settings.html".into())
                        )
                        .title("Atuin Bar Settings")
                        .inner_size(500.0, 400.0)
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

    #[test]
    fn test_config_default_window_width() {
        let config = Config::default();
        assert_eq!(
            config.window_width, 700,
            "Default window width should be 700"
        );
    }

    #[test]
    fn test_get_window_width_command() {
        // Test the get_window_width command returns the configured value
        let width = get_window_width();
        assert!(
            width > 0,
            "Window width should be a positive number, got: {}",
            width
        );
    }
}
