use std::process::Command;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn atuin_search(query: &str) -> Result<String, String> {
    let output = Command::new("atuin")
        .arg("search")
        .arg(query)
        .arg("--format")
        .arg("{command}|{exit}|{directory}|{time}")
        .output()
        .map_err(|e| format!("Failed to execute atuin command: {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse atuin output: {}", e))
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        Err(format!("atuin command failed: {}", error_message))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![greet, atuin_search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
