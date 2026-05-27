use std::fs;
use std::io::Write;

pub fn log_debug(msg: &str) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_path = format!("{}/atuin-bar-debug.log", home);
    match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
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
