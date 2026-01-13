//! Type definitions for context-os-core
//!
//! These types define the API contract between context-os-core and consumers.
//! CRITICAL: These types MUST serialize to the EXACT same JSON as the current
//! Tauri commands in apps/tastematter/src-tauri/src/commands.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CoreError;

// =============================================================================
// QUERY INPUT TYPES
// =============================================================================

/// Input for query_flex - the main query command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFlexInput {
    /// File path pattern filter (glob-style)
    pub files: Option<String>,

    /// Time range filter: "7d", "14d", "30d", or custom
    pub time: Option<String>,

    /// Filter by chain ID
    pub chain: Option<String>,

    /// Filter by session ID
    pub session: Option<String>,

    /// Aggregations to compute: "count", "recency", "sessions"
    #[serde(default)]
    pub agg: Vec<String>,

    /// Maximum results to return (default: 20)
    pub limit: Option<u32>,

    /// Sort order: "count" (default) or "recency"
    pub sort: Option<String>,
}

/// Input for query_timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTimelineInput {
    /// Time range: "7d", "14d", "30d"
    pub time: String,

    /// File path pattern filter
    pub files: Option<String>,

    /// Filter by chain ID
    pub chain: Option<String>,

    /// Maximum files to include (default: 30)
    pub limit: Option<u32>,
}

/// Input for query_sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySessionsInput {
    /// Time range: "7d", "14d", "30d"
    pub time: String,

    /// Filter by chain ID
    pub chain: Option<String>,

    /// Maximum sessions to return (default: 50)
    pub limit: Option<u32>,
}

/// Input for query_chains
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryChainsInput {
    /// Maximum chains to return (default: 20)
    pub limit: Option<u32>,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_flex
// =============================================================================

/// Main query result - returned by query_flex
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:20-27
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub result_count: usize,
    pub results: Vec<FileResult>,
    pub aggregations: Aggregations,
}

/// Individual file result
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:29-37
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub file_path: String,
    pub access_count: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_access: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chains: Option<Vec<String>>,
}

/// Aggregation results
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:39-43
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Aggregations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<CountAgg>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recency: Option<RecencyAgg>,
}

/// Count aggregation
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:45-49
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountAgg {
    pub total_files: u32,
    pub total_accesses: u32,
}

/// Recency aggregation
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:51-55
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecencyAgg {
    pub newest: String,
    pub oldest: String,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_timeline
// =============================================================================

/// Timeline bucket for a single day
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:339-348
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBucket {
    pub date: String,
    pub day_of_week: String,
    pub access_count: u32,
    pub files_touched: u32,

    #[serde(default)]
    pub sessions: Vec<String>,
}

/// Per-file timeline data
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:350-357
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTimeline {
    pub file_path: String,
    pub total_accesses: u32,
    pub buckets: HashMap<String, u32>,
    pub first_access: String,
    pub last_access: String,
}

/// Timeline summary statistics
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:359-365
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummary {
    pub total_accesses: u32,
    pub total_files: u32,
    pub peak_day: String,
    pub peak_count: u32,
}

/// Complete timeline data response
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:367-375
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineData {
    pub time_range: String,
    pub start_date: String,
    pub end_date: String,
    pub buckets: Vec<TimeBucket>,
    pub files: Vec<FileTimeline>,
    pub summary: TimelineSummary,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_chains
// =============================================================================

/// Chain time range
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:518-522
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTimeRange {
    pub start: String,
    pub end: String,
}

/// Individual chain data
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:524-530
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainData {
    pub chain_id: String,
    pub session_count: u32,
    pub file_count: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<ChainTimeRange>,
}

/// Chain query result
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:541-545
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainQueryResult {
    pub chains: Vec<ChainData>,
    pub total_chains: u32,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_sessions
// =============================================================================

/// File access within a session
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:549-555
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub file_path: String,
    pub access_count: u32,
    pub access_types: Vec<String>,
    pub last_access: String,
}

/// Individual session data
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:557-568
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,

    pub started_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,

    pub file_count: u32,
    pub total_accesses: u32,
    pub files: Vec<SessionFile>,
    pub top_files: Vec<SessionFile>,
}

/// Chain summary for session view
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:570-576
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummary {
    pub chain_id: String,
    pub session_count: u32,
    pub file_count: u32,
    pub last_active: String,
}

/// Session summary statistics
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:578-584
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub total_sessions: u32,
    pub total_files: u32,
    pub total_accesses: u32,
    pub active_chains: u32,
}

/// Complete session query result
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:586-592
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionQueryResult {
    pub time_range: String,
    pub sessions: Vec<SessionData>,
    pub chains: Vec<ChainSummary>,
    pub summary: SessionSummary,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Parse time range string to days
/// "7d" -> 7, "14d" -> 14, "30d" -> 30
pub fn parse_time_range(time: &str) -> Result<i64, CoreError> {
    match time {
        "7d" => Ok(7),
        "14d" => Ok(14),
        "30d" => Ok(30),
        other => {
            // Try to parse "Nd" format
            if let Some(n) = other.strip_suffix('d') {
                n.parse().map_err(|_| CoreError::Query {
                    message: format!("Invalid time range: {}", other),
                })
            } else {
                Err(CoreError::Query {
                    message: format!("Invalid time range format: {}", other),
                })
            }
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_result_serialization() {
        let result = QueryResult {
            receipt_id: "test-123".to_string(),
            timestamp: "2026-01-08T12:00:00Z".to_string(),
            result_count: 1,
            results: vec![FileResult {
                file_path: "src/main.rs".to_string(),
                access_count: 10,
                last_access: Some("2026-01-08".to_string()),
                session_count: Some(2),
                sessions: None,
                chains: None,
            }],
            aggregations: Aggregations {
                count: Some(CountAgg {
                    total_files: 1,
                    total_accesses: 10,
                }),
                recency: None,
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("receipt_id"));
        assert!(json.contains("result_count"));
        assert!(!json.contains("sessions")); // None should be skipped
    }

    #[test]
    fn test_parse_time_range() {
        assert_eq!(parse_time_range("7d").unwrap(), 7);
        assert_eq!(parse_time_range("14d").unwrap(), 14);
        assert_eq!(parse_time_range("30d").unwrap(), 30);
        assert!(parse_time_range("invalid").is_err());
    }

    #[test]
    fn test_file_result_optional_fields() {
        // Test that None fields are not serialized
        let file = FileResult {
            file_path: "test.rs".to_string(),
            access_count: 5,
            last_access: None,
            session_count: None,
            sessions: None,
            chains: None,
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(!json.contains("last_access"));
        assert!(!json.contains("session_count"));
        assert!(!json.contains("sessions"));
        assert!(!json.contains("chains"));
    }
}
