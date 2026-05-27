use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for Theme {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            other => Err(format!(
                "Invalid theme '{other}'. Expected 'dark' or 'light'"
            )),
        }
    }
}

/// Application configuration
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct Config {
    /// Global shortcut to toggle the window (e.g., "CommandOrControl+Shift+Space")
    pub shortcut: String,
    /// Theme: "dark" or "light" (default: "dark")
    pub theme: Theme,
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
            theme: Theme::default(),
            max_results: 20,
            window_width: 700,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        validate_shortcut(&self.shortcut)?;
        validate_max_results(self.max_results)?;
        validate_window_width(self.window_width)?;
        Ok(())
    }
}

pub fn validate_shortcut(shortcut: &str) -> Result<(), String> {
    if shortcut.trim().is_empty() {
        return Err("shortcut must not be empty".to_string());
    }
    Ok(())
}

pub fn validate_max_results(max_results: u32) -> Result<(), String> {
    if max_results == 0 || max_results > 500 {
        return Err("max_results must be between 1 and 500".to_string());
    }
    Ok(())
}

pub fn validate_window_width(window_width: u32) -> Result<(), String> {
    if !(300..=3000).contains(&window_width) {
        return Err("window_width must be between 300 and 3000".to_string());
    }
    Ok(())
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
        let _ = write_default_config(&config_path, &Config::default());
        return Config::default();
    }

    match fs::read_to_string(&config_path) {
        Ok(contents) => match toml::from_str::<Config>(&contents) {
            Ok(config) => match config.validate() {
                Ok(()) => config,
                Err(e) => {
                    eprintln!("Invalid config file: {}", e);
                    Config::default()
                }
            },
            Err(e) => {
                eprintln!("Failed to parse config file: {}", e);
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Failed to read config file: {}", e);
            Config::default()
        }
    }
}

pub fn save_config(config: &Config) -> Result<(), String> {
    config.validate()?;
    let Some(config_path) = get_config_path() else {
        return Err("Could not determine config path".to_string());
    };
    write_default_config(&config_path, config)
}

fn write_default_config(config_path: &PathBuf, config: &Config) -> Result<(), String> {
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
        config.shortcut,
        config.theme.as_str(),
        config.max_results,
        config.window_width
    );

    if let Some(parent) = config_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(config_path, toml_str).map_err(|e| format!("Failed to write config: {}", e))
}
