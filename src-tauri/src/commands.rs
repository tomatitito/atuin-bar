use crate::atuin::{atuin_search, AtuinResult, SearchFilters};
use crate::config::{
    load_config, save_config, validate_max_results, validate_shortcut, validate_window_width,
    Config, Theme,
};
use crate::logging::log_debug;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn get_theme() -> String {
    load_config().theme.to_string()
}

#[tauri::command]
pub fn get_max_results() -> u32 {
    load_config().max_results
}

#[tauri::command]
pub fn get_window_width() -> u32 {
    load_config().window_width
}

#[tauri::command]
pub fn get_config() -> Config {
    load_config()
}

#[tauri::command]
pub fn update_config(
    shortcut: Option<String>,
    theme: Option<String>,
    max_results: Option<u32>,
    window_width: Option<u32>,
) -> Result<Config, String> {
    let mut config = load_config();

    if let Some(s) = shortcut {
        validate_shortcut(&s)?;
        config.shortcut = s;
    }
    if let Some(t) = theme {
        config.theme = Theme::try_from(t.as_str())?;
    }
    if let Some(m) = max_results {
        validate_max_results(m)?;
        config.max_results = m;
    }
    if let Some(w) = window_width {
        validate_window_width(w)?;
        config.window_width = w;
    }

    save_config(&config)?;
    Ok(config)
}

#[tauri::command]
pub fn atuin_search_command(
    query: &str,
    filters: Option<SearchFilters>,
) -> Result<Vec<AtuinResult>, String> {
    log_debug(&format!(
        "atuin_search_command invoked with query: {:?}",
        query
    ));
    let result = atuin_search(query, filters);
    log_debug(&format!(
        "atuin_search_command result: {:?}",
        result.as_ref().map(|v| v.len())
    ));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_config_default_window_width() {
        let config = Config::default();
        assert_eq!(config.window_width, 700);
    }

    #[test]
    fn test_get_window_width_command() {
        let width = get_window_width();
        assert!(width > 0, "Window width should be positive, got: {}", width);
    }

    #[test]
    fn validates_theme_updates() {
        assert!(Theme::try_from("dark").is_ok());
        assert!(Theme::try_from("sepia").is_err());
    }
}
