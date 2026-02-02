//! Telemetry event types for tastematter CLI.
//!
//! Privacy-first design following Claude Code, Vercel, and HashiCorp patterns:
//! - NEVER: file paths, query content, error messages, user identity
//! - ALWAYS: machine UUID, platform, version, command, duration, success
//! - WITH CARE: result counts, time range buckets, error codes

use serde::Serialize;
use serde_json::{json, Value};

/// Error codes for telemetry (never include actual error messages)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    DbConnection,
    DbQuery,
    ParseFailed,
    GitSync,
    ConfigLoad,
    FileWatch,
    NetworkError,
    Unknown,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::DbConnection => "DB_CONNECTION",
            ErrorCode::DbQuery => "DB_QUERY",
            ErrorCode::ParseFailed => "PARSE_FAILED",
            ErrorCode::GitSync => "GIT_SYNC",
            ErrorCode::ConfigLoad => "CONFIG_LOAD",
            ErrorCode::FileWatch => "FILE_WATCH",
            ErrorCode::NetworkError => "NETWORK_ERROR",
            ErrorCode::Unknown => "UNKNOWN",
        }
    }
}

/// Time range bucket for queries (never exact timestamps)
#[derive(Debug, Clone, Serialize)]
pub enum TimeRangeBucket {
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "30d")]
    ThirtyDays,
    #[serde(rename = "all")]
    All,
}

impl TimeRangeBucket {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeRangeBucket::OneHour => "1h",
            TimeRangeBucket::OneDay => "1d",
            TimeRangeBucket::SevenDays => "7d",
            TimeRangeBucket::ThirtyDays => "30d",
            TimeRangeBucket::All => "all",
        }
    }

    /// Parse a time range string into a bucket
    pub fn from_time_arg(arg: &str) -> Self {
        let arg_lower = arg.to_lowercase();
        if arg_lower.contains("hour") || arg_lower == "1h" {
            TimeRangeBucket::OneHour
        } else if arg_lower.contains("day") || arg_lower == "1d" || arg_lower == "24h" {
            TimeRangeBucket::OneDay
        } else if arg_lower.contains("week") || arg_lower == "7d" {
            TimeRangeBucket::SevenDays
        } else if arg_lower.contains("month") || arg_lower == "30d" {
            TimeRangeBucket::ThirtyDays
        } else {
            TimeRangeBucket::All
        }
    }
}

/// Command executed event - emitted for every CLI command
#[derive(Debug, Clone)]
pub struct CommandExecutedEvent {
    pub command: String,
    pub duration_ms: u64,
    pub success: bool,
    pub result_count: Option<u32>,
    pub time_range_bucket: Option<TimeRangeBucket>,
}

impl CommandExecutedEvent {
    pub fn new(command: &str, duration_ms: u64, success: bool) -> Self {
        Self {
            command: command.to_string(),
            duration_ms,
            success,
            result_count: None,
            time_range_bucket: None,
        }
    }

    pub fn with_result_count(mut self, count: u32) -> Self {
        self.result_count = Some(count);
        self
    }

    pub fn with_time_range(mut self, bucket: TimeRangeBucket) -> Self {
        self.time_range_bucket = Some(bucket);
        self
    }

    pub fn to_properties(&self) -> Value {
        let mut props = json!({
            "command": self.command,
            "duration_ms": self.duration_ms,
            "success": self.success,
        });

        if let Some(count) = self.result_count {
            props["result_count"] = json!(count);
        }

        if let Some(ref bucket) = self.time_range_bucket {
            props["time_range_bucket"] = json!(bucket.as_str());
        }

        props
    }
}

/// Sync completed event - emitted after daemon sync
#[derive(Debug, Clone)]
pub struct SyncCompletedEvent {
    pub sessions_parsed: u32,
    pub chains_built: u32,
    pub duration_ms: u64,
}

impl SyncCompletedEvent {
    pub fn new(sessions_parsed: u32, chains_built: u32, duration_ms: u64) -> Self {
        Self {
            sessions_parsed,
            chains_built,
            duration_ms,
        }
    }

    pub fn to_properties(&self) -> Value {
        json!({
            "sessions_parsed": self.sessions_parsed,
            "chains_built": self.chains_built,
            "duration_ms": self.duration_ms,
        })
    }
}

/// Error occurred event - emitted on failures (error codes only, never messages)
#[derive(Debug, Clone)]
pub struct ErrorOccurredEvent {
    pub error_code: ErrorCode,
    pub command: String,
}

impl ErrorOccurredEvent {
    pub fn new(error_code: ErrorCode, command: &str) -> Self {
        Self {
            error_code,
            command: command.to_string(),
        }
    }

    pub fn to_properties(&self) -> Value {
        json!({
            "error_code": self.error_code.as_str(),
            "command": self.command,
        })
    }
}

/// Feature used event - tracks feature adoption
#[derive(Debug, Clone)]
pub struct FeatureUsedEvent {
    pub feature: String,
    pub first_use: bool,
}

impl FeatureUsedEvent {
    pub fn new(feature: &str, first_use: bool) -> Self {
        Self {
            feature: feature.to_string(),
            first_use,
        }
    }

    pub fn to_properties(&self) -> Value {
        json!({
            "feature": self.feature,
            "first_use": self.first_use,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_executed_basic() {
        let event = CommandExecutedEvent::new("query_flex", 234, true);
        let props = event.to_properties();

        assert_eq!(props["command"], "query_flex");
        assert_eq!(props["duration_ms"], 234);
        assert_eq!(props["success"], true);
        assert!(props.get("result_count").is_none());
        assert!(props.get("time_range_bucket").is_none());
    }

    #[test]
    fn test_command_executed_with_options() {
        let event = CommandExecutedEvent::new("query_flex", 500, true)
            .with_result_count(47)
            .with_time_range(TimeRangeBucket::SevenDays);

        let props = event.to_properties();

        assert_eq!(props["command"], "query_flex");
        assert_eq!(props["duration_ms"], 500);
        assert_eq!(props["success"], true);
        assert_eq!(props["result_count"], 47);
        assert_eq!(props["time_range_bucket"], "7d");
    }

    #[test]
    fn test_time_range_bucket_parsing() {
        assert_eq!(TimeRangeBucket::from_time_arg("1h").as_str(), "1h");
        assert_eq!(TimeRangeBucket::from_time_arg("1d").as_str(), "1d");
        assert_eq!(TimeRangeBucket::from_time_arg("7d").as_str(), "7d");
        assert_eq!(TimeRangeBucket::from_time_arg("30d").as_str(), "30d");
        assert_eq!(TimeRangeBucket::from_time_arg("1 day").as_str(), "1d");
        assert_eq!(TimeRangeBucket::from_time_arg("1 week").as_str(), "7d");
        assert_eq!(TimeRangeBucket::from_time_arg("1 month").as_str(), "30d");
        assert_eq!(TimeRangeBucket::from_time_arg("forever").as_str(), "all");
    }

    #[test]
    fn test_sync_completed() {
        let event = SyncCompletedEvent::new(1079, 2732, 5000);
        let props = event.to_properties();

        assert_eq!(props["sessions_parsed"], 1079);
        assert_eq!(props["chains_built"], 2732);
        assert_eq!(props["duration_ms"], 5000);
    }

    #[test]
    fn test_error_occurred() {
        let event = ErrorOccurredEvent::new(ErrorCode::DbConnection, "daemon_once");
        let props = event.to_properties();

        assert_eq!(props["error_code"], "DB_CONNECTION");
        assert_eq!(props["command"], "daemon_once");
    }

    #[test]
    fn test_feature_used() {
        let event = FeatureUsedEvent::new("daemon_autostart", true);
        let props = event.to_properties();

        assert_eq!(props["feature"], "daemon_autostart");
        assert_eq!(props["first_use"], true);
    }

    #[test]
    fn test_no_file_paths_in_events() {
        // Verify our event types don't have any path-like fields
        // This is a compile-time guarantee via the struct definitions
        let cmd = CommandExecutedEvent::new("test", 100, true);
        let props = cmd.to_properties();

        // Ensure no path-like keys exist
        let keys: Vec<&str> = props.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        assert!(!keys.iter().any(|k| k.contains("path") || k.contains("file") || k.contains("dir")));
    }
}
