use super::daemon::AtuinDaemonAdapter;
use super::filters::{SearchFilters, ValidatedSearchFilters};
use super::parser::{parse_atuin_output, AtuinResult};
use crate::logging::log_debug;
use std::process::Command;
use std::sync::LazyLock;

/// Resolve the full path to the `atuin` binary.
/// macOS GUI apps don't inherit the user's shell PATH, so we check common locations.
fn find_atuin_binary() -> String {
    let candidates = [
        "/opt/homebrew/bin/atuin",
        "/usr/local/bin/atuin",
        &format!(
            "{}/.cargo/bin/atuin",
            std::env::var("HOME").unwrap_or_default()
        ),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "atuin".to_string()
}

static ATUIN_BIN: LazyLock<String> = LazyLock::new(find_atuin_binary);

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HistorySearchRequest {
    pub query: String,
    pub filters: ValidatedSearchFilters,
    pub limit: u32,
}

pub trait HistorySearch {
    fn search(&self, request: HistorySearchRequest) -> Result<Vec<AtuinResult>, String>;
}

#[derive(Debug, Default)]
pub struct AtuinCliAdapter;

impl AtuinCliAdapter {
    fn build_command(&self, request: &HistorySearchRequest) -> Command {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| format!("/Users/{}", std::env::var("USER").unwrap_or_default()));

        let mut cmd = Command::new(ATUIN_BIN.as_str());
        cmd.env("HOME", &home)
            .env("ATUIN_SESSION", "atuin-bar")
            .arg("search")
            .arg("--search-mode")
            .arg("fuzzy")
            .arg("--filter-mode")
            .arg("global")
            .arg("--limit")
            .arg(request.limit.to_string())
            .arg("--format")
            .arg("{command}|{exit}|{duration}|{directory}|{time}");

        if let Some(ref dir) = request.filters.directory {
            cmd.arg("--cwd").arg(dir);
        }

        if let Some(exit_filter) = request.filters.exit_filter {
            match exit_filter {
                super::filters::ExitFilter::Success => {
                    cmd.arg("--exit").arg("0");
                }
                super::filters::ExitFilter::Failure => {
                    cmd.arg("--exclude-exit").arg("0");
                }
            }
        }

        if let Some(time_range) = request.filters.time_range {
            cmd.arg("--after").arg(time_range.atuin_after());
        }

        cmd.arg(&request.query);
        cmd
    }
}

impl HistorySearch for AtuinCliAdapter {
    fn search(&self, request: HistorySearchRequest) -> Result<Vec<AtuinResult>, String> {
        log_debug(&format!(
            "--- atuin cli search called with query: {:?}",
            request.query
        ));
        log_debug(&format!(
            "HOME={}",
            std::env::var("HOME").unwrap_or_default()
        ));
        log_debug(&format!("ATUIN_BIN={}", ATUIN_BIN.as_str()));
        log_debug(&format!(
            "PATH={}",
            std::env::var("PATH").unwrap_or_default()
        ));
        log_debug(&format!(
            "atuin binary exists: {}",
            std::path::Path::new(ATUIN_BIN.as_str()).exists()
        ));

        let output = self.build_command(&request).output().map_err(|e| {
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
            let raw = String::from_utf8(output.stdout)
                .map_err(|e| format!("Failed to parse atuin output: {}", e))?;
            log_debug(&format!("success, output lines: {}", raw.lines().count()));
            Ok(parse_atuin_output(&raw))
        } else if stderr_str.trim().is_empty() {
            log_debug("atuin exited non-zero with empty stderr, treating as no results");
            Ok(vec![])
        } else {
            Err(format!("atuin command failed: {}", stderr_str))
        }
    }
}

/// Public compatibility function used by integration tests and the Tauri command.
pub fn atuin_search(
    query: &str,
    filters: Option<SearchFilters>,
) -> Result<Vec<AtuinResult>, String> {
    let filters = filters.unwrap_or_default().validated()?;
    let request = HistorySearchRequest {
        query: query.to_string(),
        filters,
        limit: 50,
    };
    match AtuinDaemonAdapter.search(request.clone()) {
        Ok(results) => Ok(results),
        Err(err) => {
            log_debug(&format!(
                "atuin daemon search unavailable, falling back to CLI: {err}"
            ));
            AtuinCliAdapter.search(request)
        }
    }
}
