use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Filter by exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ExitFilter {
    Success,
    Failure,
}

/// Supported time ranges for Atuin history search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum TimeRange {
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "24h")]
    TwentyFourHours,
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "30d")]
    ThirtyDays,
}

impl TimeRange {
    pub fn atuin_after(self) -> &'static str {
        match self {
            Self::OneHour => "1 hour ago",
            Self::TwentyFourHours => "1 day ago",
            Self::SevenDays => "7 days ago",
            Self::ThirtyDays => "30 days ago",
        }
    }
}

/// IPC-compatible search filters for atuin queries.
///
/// Vocabulary fields deserialize from their IPC string values into Rust enums.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SearchFilters {
    /// Filter by directory path
    pub directory: Option<String>,
    /// Filter by exit code: "success" (0), "failure" (non-0), or None (all)
    pub exit_filter: Option<ExitFilter>,
    /// Time range: "1h", "24h", "7d", "30d", or None (all)
    pub time_range: Option<TimeRange>,
}

pub type ValidatedSearchFilters = SearchFilters;

impl SearchFilters {
    pub(crate) fn validated(self) -> Result<ValidatedSearchFilters, String> {
        let directory = self
            .directory
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        if let Some(ref directory) = directory {
            if directory.contains('\0') {
                return Err("directory filter must not contain NUL bytes".to_string());
            }
        }

        Ok(ValidatedSearchFilters {
            directory,
            exit_filter: self.exit_filter,
            time_range: self.time_range,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_filter_vocabularies() {
        let filters: SearchFilters = serde_json::from_value(serde_json::json!({
            "exit_filter": "success",
            "time_range": "7d"
        }))
        .unwrap();
        let filters = filters.validated().unwrap();

        assert_eq!(filters.exit_filter, Some(ExitFilter::Success));
        assert_eq!(filters.time_range, Some(TimeRange::SevenDays));
    }

    #[test]
    fn rejects_unknown_filter_values() {
        let bad_exit = serde_json::from_value::<SearchFilters>(serde_json::json!({
            "exit_filter": "ok"
        }));
        assert!(bad_exit.is_err());

        let bad_time = serde_json::from_value::<SearchFilters>(serde_json::json!({
            "time_range": "forever"
        }));
        assert!(bad_time.is_err());
    }
}
