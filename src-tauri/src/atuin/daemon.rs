use super::filters::{ExitFilter, TimeRange};
use super::parser::AtuinResult;
use super::HistorySearchRequest;
use atuin_client::database::{Context, Database, Sqlite};
use atuin_client::history::History;
use atuin_client::settings::{FilterMode, Settings};
use atuin_daemon::client::SearchClient;
use std::time::Duration;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use uuid::Uuid;

static TIME_FMT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour repr:24]:[minute]:[second]");

/// Search Atuin through the experimental daemon API, hydrating result IDs from
/// the local history database. Callers keep the CLI adapter as fallback: the
/// daemon API is not guaranteed to exist or be running for every Atuin user.
#[derive(Debug, Default)]
pub struct AtuinDaemonAdapter;

impl AtuinDaemonAdapter {
    pub fn search(&self, request: HistorySearchRequest) -> Result<Vec<AtuinResult>, String> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("failed to create atuin daemon runtime: {e}"))?;

        runtime.block_on(search_async(request))
    }
}

async fn search_async(request: HistorySearchRequest) -> Result<Vec<AtuinResult>, String> {
    let settings = Settings::new().map_err(|e| format!("failed to load atuin settings: {e}"))?;
    let mut client = connect_search_client(&settings).await?;
    let mut db = Sqlite::new(&settings.db_path, settings.local_timeout)
        .await
        .map_err(|e| format!("failed to open atuin history db {}: {e}", settings.db_path))?;

    let filter_mode = if request.filters.directory.is_some() {
        FilterMode::Directory
    } else {
        FilterMode::Global
    };
    let context = context_for_request(&request).await?;
    let query_id = 1;

    let mut stream = client
        .search(request.query.clone(), query_id, filter_mode, Some(context))
        .await
        .map_err(|e| format!("atuin daemon search failed: {e}"))?;

    let mut ids = Vec::new();
    while let Some(response) = stream
        .message()
        .await
        .map_err(|e| format!("atuin daemon response failed: {e}"))?
    {
        if response.query_id == query_id {
            ids.extend(response.ids.iter().filter_map(|id| {
                let bytes: [u8; 16] = id.as_slice().try_into().ok()?;
                Some(Uuid::from_bytes(bytes).as_simple().to_string())
            }));
        }
    }

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let histories = hydrate_from_db(&mut db, &ids).await?;
    let mut ordered = Vec::with_capacity(histories.len());
    for id in &ids {
        if let Some(history) = histories.iter().find(|h| h.id.0 == *id) {
            if matches_filters(history, &request) {
                ordered.push(history_to_result(history));
            }
        }
        if ordered.len() >= request.limit as usize {
            break;
        }
    }

    Ok(ordered)
}

async fn connect_search_client(settings: &Settings) -> Result<SearchClient, String> {
    #[cfg(unix)]
    {
        if !std::path::Path::new(&settings.daemon.socket_path).exists() {
            return Err(format!(
                "atuin daemon socket does not exist: {}",
                settings.daemon.socket_path
            ));
        }

        SearchClient::new(settings.daemon.socket_path.clone())
            .await
            .map_err(|e| {
                format!(
                    "failed to connect to atuin daemon socket {}: {e}",
                    settings.daemon.socket_path
                )
            })
    }

    #[cfg(not(unix))]
    {
        SearchClient::new(settings.daemon.tcp_port)
            .await
            .map_err(|e| {
                format!(
                    "failed to connect to atuin daemon tcp port {}: {e}",
                    settings.daemon.tcp_port
                )
            })
    }
}

async fn context_for_request(request: &HistorySearchRequest) -> Result<Context, String> {
    let cwd = request.filters.directory.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    Ok(Context {
        session: std::env::var("ATUIN_SESSION").unwrap_or_else(|_| "atuin-bar".to_string()),
        cwd,
        hostname: hostname(),
        host_id: Settings::host_id()
            .await
            .map(|id| id.0.as_simple().to_string())
            .unwrap_or_default(),
        git_root: None,
    })
}

async fn hydrate_from_db(db: &mut dyn Database, ids: &[String]) -> Result<Vec<History>, String> {
    let placeholders: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
    let sql_query = format!(
        "SELECT * FROM history WHERE id IN ({}) ORDER BY timestamp DESC",
        placeholders.join(",")
    );
    db.query_history(&sql_query)
        .await
        .map_err(|e| format!("failed to hydrate atuin daemon result ids: {e}"))
}

fn matches_filters(history: &History, request: &HistorySearchRequest) -> bool {
    if let Some(ref directory) = request.filters.directory {
        if &history.cwd != directory {
            return false;
        }
    }

    match request.filters.exit_filter {
        Some(ExitFilter::Success) if history.exit != 0 => return false,
        Some(ExitFilter::Failure) if history.exit == 0 => return false,
        _ => {}
    }

    if let Some(range) = request.filters.time_range {
        if history.timestamp < cutoff_for(range) {
            return false;
        }
    }

    true
}

fn cutoff_for(range: TimeRange) -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    match range {
        TimeRange::OneHour => now - time::Duration::hours(1),
        TimeRange::TwentyFourHours => now - time::Duration::days(1),
        TimeRange::SevenDays => now - time::Duration::days(7),
        TimeRange::ThirtyDays => now - time::Duration::days(30),
    }
}

fn history_to_result(history: &History) -> AtuinResult {
    AtuinResult {
        command: history.command.trim().to_string(),
        exit: history.exit.to_string(),
        duration: format_duration(Duration::from_nanos(history.duration.max(0) as u64)),
        directory: history.cwd.trim().to_string(),
        time: history
            .timestamp
            .to_offset(time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC))
            .format(TIME_FMT)
            .unwrap_or_else(|_| "invalid".to_string()),
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "atuin-bar".to_string())
}

fn format_duration(dur: Duration) -> String {
    let secs = dur.as_secs();
    let nanos = dur.subsec_nanos();

    let years = secs / 31_557_600;
    let year_days = secs % 31_557_600;
    let months = year_days / 2_630_016;
    let month_days = year_days % 2_630_016;
    let days = month_days / 86_400;
    let day_secs = month_days % 86_400;
    let hours = day_secs / 3_600;
    let minutes = day_secs % 3_600 / 60;
    let seconds = day_secs % 60;
    let millis = nanos / 1_000_000;
    let micros = nanos / 1_000;

    for (unit, value) in [
        ("y", years),
        ("mo", months),
        ("d", days),
        ("h", hours),
        ("m", minutes),
        ("s", seconds),
        ("ms", u64::from(millis)),
        ("us", u64::from(micros)),
        ("ns", u64::from(nanos)),
    ] {
        if value > 0 {
            return format!("{value}{unit}");
        }
    }

    "0s".to_string()
}
