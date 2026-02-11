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

/// Input for search command - substring search across file paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySearchInput {
    /// Pattern to search for (substring match, case-insensitive)
    pub pattern: String,
    /// Maximum results to return (default: 20)
    pub limit: Option<u32>,
}

/// Input for file command - show sessions that touched a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFileInput {
    /// File path to query (exact, suffix, or substring match)
    pub file_path: String,
    /// Maximum sessions to return (default: 20)
    pub limit: Option<u32>,
}

/// Input for co-access command - find files accessed together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCoAccessInput {
    /// Anchor file to find co-accessed files for
    pub file_path: String,
    /// Maximum results to return (default: 10)
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

    /// Human-readable name: generated_name → first_user_message (truncated) → chain_id[:12]+"..."
    pub display_name: String,

    pub session_count: u32,
    pub file_count: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<ChainTimeRange>,

    /// AI-generated human-readable name for the chain (from Intel service)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_name: Option<String>,

    /// Chain summary from Intel service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Chain query result
/// MUST match: apps/tastematter/src-tauri/src/commands.rs:541-545
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainQueryResult {
    pub chains: Vec<ChainData>,
    pub total_chains: u32,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_search
// =============================================================================

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub file_path: String,
    pub access_count: u32,
}

/// Search query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub pattern: String,
    pub total_matches: usize,
    pub results: Vec<SearchResultItem>,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_file
// =============================================================================

/// Session that touched a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSessionInfo {
    pub session_id: String,
    pub access_types: Vec<String>,
    pub last_access: Option<String>,
    pub chain_id: Option<String>,
}

/// File query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileQueryResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub file_path: String,
    pub found: bool,
    pub matched_path: Option<String>,
    pub sessions: Vec<FileSessionInfo>,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_co_access
// =============================================================================

/// Co-accessed file with PMI score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoAccessItem {
    pub file_path: String,
    pub pmi_score: f64,
}

/// Co-access query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoAccessResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub query_file: String,
    pub results: Vec<CoAccessItem>,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_verify
// =============================================================================

/// Input for verify command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryVerifyInput {
    /// Receipt ID to verify (e.g., "q_abc123")
    pub receipt_id: String,
}

/// Verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    #[serde(rename = "MATCH")]
    Match,
    #[serde(rename = "DRIFT")]
    Drift,
    #[serde(rename = "NOT_FOUND")]
    NotFound,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub receipt_id: String,
    pub status: VerificationStatus,
    pub original_timestamp: Option<String>,
    pub verified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_summary: Option<String>,
}

// =============================================================================
// QUERY OUTPUT TYPES - query_receipts
// =============================================================================

/// Input for receipts command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryReceiptsInput {
    /// Maximum receipts to return (default: 20)
    pub limit: Option<u32>,
}

/// Receipt summary item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptItem {
    pub receipt_id: String,
    pub timestamp: String,
    pub query_type: String,
    pub result_count: usize,
}

/// Receipts list result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptsResult {
    pub receipts: Vec<ReceiptItem>,
    pub total_count: usize,
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

    /// Human-readable chain name (from chain_metadata.generated_name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_name: Option<String>,

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
// WRITE INPUT TYPES (Phase 1: Storage Foundation)
// =============================================================================

/// Input for inserting a git commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInput {
    /// Full commit hash
    pub hash: String,
    /// Short hash (first 7 chars)
    pub short_hash: String,
    /// Commit timestamp (ISO8601)
    pub timestamp: String,
    /// Commit message
    pub message: Option<String>,
    /// Author name
    pub author_name: Option<String>,
    /// Author email
    pub author_email: Option<String>,
    /// Files changed (JSON array)
    pub files_changed: Option<String>,
    /// Number of insertions
    pub insertions: Option<i32>,
    /// Number of deletions
    pub deletions: Option<i32>,
    /// Number of files changed
    pub files_count: Option<i32>,
    /// Whether this is an agent-generated commit
    pub is_agent_commit: bool,
}

/// Input for inserting a Claude session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionInput {
    /// Session ID (UUID from JSONL filename)
    pub session_id: String,
    /// Project path
    pub project_path: Option<String>,
    /// Started timestamp (ISO8601)
    pub started_at: Option<String>,
    /// Ended timestamp (ISO8601)
    pub ended_at: Option<String>,
    /// Duration in seconds
    pub duration_seconds: Option<i32>,
    /// User message count
    pub user_message_count: Option<i32>,
    /// Assistant message count
    pub assistant_message_count: Option<i32>,
    /// Total message count
    pub total_messages: Option<i32>,
    /// Files read (JSON array)
    pub files_read: Option<String>,
    /// Files written (JSON array)
    pub files_written: Option<String>,
    /// Tools used (JSON object)
    pub tools_used: Option<String>,
    /// First user message (captures user intent)
    pub first_user_message: Option<String>,
    /// Full conversation excerpt (all user messages concatenated, truncated)
    pub conversation_excerpt: Option<String>,
    /// JSONL file size in bytes (for incremental sync change detection)
    pub file_size_bytes: Option<i64>,
}

/// Result of a write operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    /// Number of rows affected
    pub rows_affected: u64,
}

// =============================================================================
// TYPE CONVERSIONS (Database Write Path)
// =============================================================================

impl From<crate::capture::jsonl_parser::SessionSummary> for SessionInput {
    fn from(s: crate::capture::jsonl_parser::SessionSummary) -> Self {
        SessionInput {
            session_id: s.session_id,
            project_path: Some(s.project_path),
            started_at: Some(s.started_at.to_rfc3339()),
            ended_at: Some(s.ended_at.to_rfc3339()),
            duration_seconds: Some(s.duration_seconds as i32),
            user_message_count: Some(s.user_message_count),
            assistant_message_count: Some(s.assistant_message_count),
            total_messages: Some(s.total_messages),
            files_read: Some(serde_json::to_string(&s.files_read).unwrap_or_default()),
            files_written: Some(serde_json::to_string(&s.files_written).unwrap_or_default()),
            tools_used: Some(serde_json::to_string(&s.tools_used).unwrap_or_default()),
            first_user_message: s.first_user_message,
            conversation_excerpt: s.conversation_excerpt,
            file_size_bytes: Some(s.file_size_bytes),
        }
    }
}

// =============================================================================
// QUERY INPUT/OUTPUT TYPES - query_heat
// =============================================================================

/// Heat level classification based on composite heat score
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeatLevel {
    #[serde(rename = "HOT")]
    Hot,
    #[serde(rename = "WARM")]
    Warm,
    #[serde(rename = "COOL")]
    Cool,
    #[serde(rename = "COLD")]
    Cold,
}

impl std::fmt::Display for HeatLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeatLevel::Hot => write!(f, "HOT"),
            HeatLevel::Warm => write!(f, "WARM"),
            HeatLevel::Cool => write!(f, "COOL"),
            HeatLevel::Cold => write!(f, "COLD"),
        }
    }
}

/// Sort field for heat query results
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HeatSortBy {
    #[default]
    Heat,
    Rcr,
    Velocity,
    Name,
}

/// Input for query_heat command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryHeatInput {
    /// Long window time range (default: "30d")
    pub time: Option<String>,

    /// File path pattern filter (glob-style)
    pub files: Option<String>,

    /// Maximum results to return (default: 50)
    pub limit: Option<u32>,

    /// Sort by: heat (default), rcr, velocity, name
    pub sort: Option<HeatSortBy>,
}

/// Individual file heat item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatItem {
    pub file_path: String,
    pub count_7d: u32,
    pub count_long: u32,
    pub rcr: f64,
    pub velocity: f64,
    pub heat_score: f64,
    pub heat_level: HeatLevel,
    pub first_access: String,
    pub last_access: String,
}

/// Summary of heat distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatSummary {
    pub total_files: u32,
    pub hot_count: u32,
    pub warm_count: u32,
    pub cool_count: u32,
    pub cold_count: u32,
}

/// Complete heat query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub time_range: String,
    pub results: Vec<HeatItem>,
    pub summary: HeatSummary,
}

// =============================================================================
// QUERY INPUT/OUTPUT TYPES - context restore
// =============================================================================

/// Input for the context restore command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextRestoreInput {
    /// Search query (used as glob pattern *query*)
    pub query: String,
    /// Time window (default: "30d")
    pub time: Option<String>,
    /// Maximum results per sub-query (default: 20)
    pub limit: Option<u32>,
}

/// Complete context restoration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRestoreResult {
    pub receipt_id: String,
    pub query: String,
    pub generated_at: String,
    pub executive_summary: ExecutiveSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<CurrentState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuity: Option<Continuity>,
    pub work_clusters: Vec<WorkCluster>,
    pub suggested_reads: Vec<SuggestedRead>,
    pub timeline: TimelineSection,
    pub insights: Vec<ContextInsight>,
    pub verification: ContextVerification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_start: Option<QuickStart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    /// Phase 2: LLM-generated one-liner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_liner: Option<String>,
    /// healthy | warning | stale | unknown
    pub status: String,
    /// active | cooling | dormant
    pub work_tempo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_meaningful_session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentState {
    /// Phase 2: LLM-generated narrative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative: Option<String>,
    pub key_metrics: serde_json::Value,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub source: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Continuity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_off_at: Option<LeftOffAt>,
    pub pending_items: Vec<PendingItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_context: Option<ChainContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeftOffAt {
    pub file: String,
    pub section: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingItem {
    pub text: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainContext {
    pub chain_id: String,
    pub display_name: String,
    pub session_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCluster {
    /// Phase 2: LLM-generated cluster name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub files: Vec<String>,
    pub pmi_score: f64,
    /// Phase 2: LLM-generated interpretation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<String>,
    /// high_access_high_session | high_access_low_session | low_access_high_session | low_access_low_session
    pub access_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedRead {
    pub path: String,
    /// Phase 2: LLM-generated reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub priority: u32,
    pub surprise: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSection {
    pub recent_focus: Vec<FocusPeriod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_shift: Option<AttentionShift>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusPeriod {
    pub start_date: String,
    pub end_date: String,
    pub top_files: Vec<String>,
    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionShift {
    pub from_period: String,
    pub to_period: String,
    pub jaccard_similarity: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInsight {
    /// stale | abandoned | surprise
    pub insight_type: String,
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
}

/// Verification data for context restore (named to avoid collision with VerifyResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVerification {
    pub receipt_id: String,
    pub files_analyzed: u32,
    pub sessions_analyzed: u32,
    pub co_access_pairs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickStart {
    pub commands: Vec<QuickStartCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickStartCommand {
    pub command: String,
    pub description: String,
}

/// Discovered project context file (from filesystem)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextFile {
    pub path: String,
    pub title: Option<String>,
    pub sections: Vec<String>,
    pub pending_items: Vec<String>,
    pub code_blocks: Vec<String>,
    pub content: String,
    /// Discovery priority tier: high, medium, low
    pub tier: String,
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
// HEAT METRIC FUNCTIONS
// =============================================================================

/// Classify a composite heat score into a HeatLevel
///
/// Thresholds (from heat-metrics-model.md):
///   > 0.7 = HOT | 0.4-0.7 = WARM | 0.2-0.4 = COOL | < 0.2 = COLD
pub fn classify_heat(heat_score: f64) -> HeatLevel {
    if heat_score > 0.7 {
        HeatLevel::Hot
    } else if heat_score >= 0.4 {
        HeatLevel::Warm
    } else if heat_score >= 0.2 {
        HeatLevel::Cool
    } else {
        HeatLevel::Cold
    }
}

/// Compute access velocity: accesses per day over the observation window
///
/// Returns 0.0 if count_long is 0.
/// Floors days_active to minimum 1 to avoid division by zero.
pub fn compute_velocity(count_long: u32, first_access: &str, last_access: &str) -> f64 {
    if count_long == 0 {
        return 0.0;
    }
    let days = compute_days_active(first_access, last_access).max(1) as f64;
    count_long as f64 / days
}

/// Compute the number of days between first and last access
///
/// Returns at least 1 if both timestamps parse successfully.
/// Returns 1 on parse failure (safe fallback).
fn compute_days_active(first_access: &str, last_access: &str) -> i64 {
    let parse = |s: &str| -> Option<chrono::NaiveDateTime> {
        // Try RFC3339 first, then date-only
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.naive_utc())
            .ok()
            .or_else(|| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    .ok()
            })
            .or_else(|| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
    };

    match (parse(first_access), parse(last_access)) {
        (Some(first), Some(last)) => {
            let diff = (last - first).num_days();
            diff.max(1)
        }
        _ => 1,
    }
}

/// Compute recency bonus based on last_access timestamp
///
/// Returns:
///   1.0 if last_access < 24h ago
///   0.5 if last_access < 7d ago
///   0.0 otherwise
fn compute_recency_bonus(last_access: &str) -> f64 {
    let now = chrono::Utc::now();

    let parsed = chrono::DateTime::parse_from_rfc3339(last_access)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .ok()
        .or_else(|| {
            chrono::NaiveDate::parse_from_str(last_access, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                .ok()
        })
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(last_access, "%Y-%m-%d %H:%M:%S")
                .map(|dt| dt.and_utc())
                .ok()
        });

    match parsed {
        Some(ts) => {
            let hours = (now - ts).num_hours();
            if hours < 24 {
                1.0
            } else if hours < 24 * 7 {
                0.5
            } else {
                0.0
            }
        }
        None => 0.0,
    }
}

/// Compute composite heat score
///
/// Formula: (normalized_AV * 0.3) + (RCR * 0.5) + (recency_bonus * 0.2)
/// Where normalized_AV = min(velocity / 5.0, 1.0)
pub fn compute_heat_score(velocity: f64, rcr: f64, last_access: &str) -> f64 {
    let normalized_av = (velocity / 5.0).min(1.0);
    let recency = compute_recency_bonus(last_access);
    (normalized_av * 0.3) + (rcr * 0.5) + (recency * 0.2)
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

    // =========================================================================
    // TDD: Database Unification - ChainData with generated_name
    // =========================================================================

    #[test]
    fn test_chain_data_has_generated_name_field() {
        // ChainData should have an optional generated_name field for Intel enrichment
        let chain = ChainData {
            chain_id: "abc123".to_string(),
            display_name: "Authentication refactor".to_string(),
            session_count: 10,
            file_count: 25,
            time_range: None,
            generated_name: Some("Authentication refactor".to_string()),
            summary: None,
        };

        assert_eq!(
            chain.generated_name,
            Some("Authentication refactor".to_string())
        );
    }

    #[test]
    fn test_chain_data_serializes_generated_name() {
        // generated_name should be included in JSON when present
        let chain = ChainData {
            chain_id: "abc123".to_string(),
            display_name: "Query engine port".to_string(),
            session_count: 10,
            file_count: 25,
            time_range: None,
            generated_name: Some("Query engine port".to_string()),
            summary: None,
        };

        let json = serde_json::to_string(&chain).unwrap();
        assert!(json.contains("generated_name"));
        assert!(json.contains("Query engine port"));
    }

    #[test]
    fn test_chain_data_omits_generated_name_when_none() {
        // generated_name should be omitted from JSON when None (skip_serializing_if)
        let chain = ChainData {
            chain_id: "abc123".to_string(),
            display_name: "abc123...".to_string(),
            session_count: 10,
            file_count: 25,
            time_range: None,
            generated_name: None,
            summary: None,
        };

        let json = serde_json::to_string(&chain).unwrap();
        assert!(!json.contains("generated_name"));
    }

    // =========================================================================
    // TDD: Database Write Path - SessionSummary → SessionInput conversion
    // =========================================================================

    #[test]
    fn test_session_summary_to_input_conversion() {
        use crate::capture::jsonl_parser::SessionSummary;
        use chrono::Utc;
        use std::collections::HashMap;

        let summary = SessionSummary {
            session_id: "test-123".to_string(),
            project_path: "/test/project".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            duration_seconds: 7200,
            user_message_count: 10,
            assistant_message_count: 15,
            total_messages: 25,
            files_read: vec!["file1.rs".to_string()],
            files_written: vec![],
            files_created: vec![],
            tools_used: HashMap::from([("Read".to_string(), 5)]),
            grep_patterns: vec![],
            file_size_bytes: 1000,
            first_user_message: Some("Help me".to_string()),
            conversation_excerpt: Some("[User]: Help me".to_string()),
        };

        let input: SessionInput = summary.into();

        assert_eq!(input.session_id, "test-123");
        assert_eq!(input.project_path, Some("/test/project".to_string()));
        assert_eq!(input.duration_seconds, Some(7200));
        assert_eq!(input.user_message_count, Some(10));
        assert_eq!(input.assistant_message_count, Some(15));
        assert_eq!(input.total_messages, Some(25));
        assert_eq!(input.first_user_message, Some("Help me".to_string()));
        // files_read should be JSON serialized
        assert!(input.files_read.as_ref().unwrap().contains("file1.rs"));
    }

    #[test]
    fn test_session_input_default() {
        let input = SessionInput::default();
        assert!(input.session_id.is_empty());
        assert!(input.project_path.is_none());
        assert!(input.duration_seconds.is_none());
    }

    // =========================================================================
    // Heat metric tests
    // =========================================================================

    #[test]
    fn test_classify_heat_hot() {
        assert_eq!(classify_heat(0.85), HeatLevel::Hot);
    }

    #[test]
    fn test_classify_heat_warm() {
        assert_eq!(classify_heat(0.55), HeatLevel::Warm);
    }

    #[test]
    fn test_classify_heat_cool() {
        assert_eq!(classify_heat(0.30), HeatLevel::Cool);
    }

    #[test]
    fn test_classify_heat_cold() {
        assert_eq!(classify_heat(0.10), HeatLevel::Cold);
    }

    #[test]
    fn test_classify_heat_boundary_hot() {
        // 0.7 is NOT hot (threshold is > 0.7), should be warm
        assert_eq!(classify_heat(0.70), HeatLevel::Warm);
    }

    #[test]
    fn test_classify_heat_boundary_warm() {
        // 0.4 is warm (>= 0.4)
        assert_eq!(classify_heat(0.40), HeatLevel::Warm);
    }

    #[test]
    fn test_classify_heat_boundary_cool() {
        // 0.2 is cool (>= 0.2)
        assert_eq!(classify_heat(0.20), HeatLevel::Cool);
    }

    #[test]
    fn test_compute_velocity_basic() {
        // 30 accesses over 30 days = 1.0 accesses/day
        let v = compute_velocity(30, "2026-01-01T00:00:00Z", "2026-01-31T00:00:00Z");
        assert!((v - 1.0).abs() < 0.01, "Expected ~1.0, got {}", v);
    }

    #[test]
    fn test_compute_velocity_single_day() {
        // Same first/last day -> floor to 1 day
        let v = compute_velocity(5, "2026-01-15T00:00:00Z", "2026-01-15T12:00:00Z");
        assert!((v - 5.0).abs() < 0.01, "Expected ~5.0, got {}", v);
    }

    #[test]
    fn test_compute_velocity_zero_accesses() {
        let v = compute_velocity(0, "2026-01-01T00:00:00Z", "2026-01-31T00:00:00Z");
        assert!((v - 0.0).abs() < f64::EPSILON, "Expected 0.0, got {}", v);
    }

    #[test]
    fn test_recency_bonus_within_24h() {
        let now = chrono::Utc::now().to_rfc3339();
        let bonus = compute_recency_bonus(&now);
        assert!(
            (bonus - 1.0).abs() < f64::EPSILON,
            "Expected 1.0, got {}",
            bonus
        );
    }

    #[test]
    fn test_recency_bonus_within_7d() {
        let three_days_ago = (chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339();
        let bonus = compute_recency_bonus(&three_days_ago);
        assert!(
            (bonus - 0.5).abs() < f64::EPSILON,
            "Expected 0.5, got {}",
            bonus
        );
    }

    #[test]
    fn test_recency_bonus_old() {
        let thirty_days_ago = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        let bonus = compute_recency_bonus(&thirty_days_ago);
        assert!(
            (bonus - 0.0).abs() < f64::EPSILON,
            "Expected 0.0, got {}",
            bonus
        );
    }

    #[test]
    fn test_compute_heat_score_hot_file() {
        // High velocity (5.0), high RCR (0.9), recent access -> should be > 0.7
        let now = chrono::Utc::now().to_rfc3339();
        let score = compute_heat_score(5.0, 0.9, &now);
        assert!(score > 0.7, "Expected > 0.7, got {}", score);
    }

    #[test]
    fn test_compute_heat_score_cold_file() {
        // Low velocity (0.1), low RCR (0.05), old access -> should be < 0.2
        let old = (chrono::Utc::now() - chrono::Duration::days(60)).to_rfc3339();
        let score = compute_heat_score(0.1, 0.05, &old);
        assert!(score < 0.2, "Expected < 0.2, got {}", score);
    }

    #[test]
    fn test_compute_heat_score_velocity_cap() {
        // AV > 5.0 should be normalized to 1.0
        let now = chrono::Utc::now().to_rfc3339();
        let score_at_5 = compute_heat_score(5.0, 0.5, &now);
        let score_at_10 = compute_heat_score(10.0, 0.5, &now);
        assert!(
            (score_at_5 - score_at_10).abs() < f64::EPSILON,
            "Velocity > 5.0 should be capped: {} vs {}",
            score_at_5,
            score_at_10
        );
    }

    #[test]
    fn test_heat_level_serialization() {
        assert_eq!(serde_json::to_string(&HeatLevel::Hot).unwrap(), "\"HOT\"");
        assert_eq!(serde_json::to_string(&HeatLevel::Warm).unwrap(), "\"WARM\"");
        assert_eq!(serde_json::to_string(&HeatLevel::Cool).unwrap(), "\"COOL\"");
        assert_eq!(serde_json::to_string(&HeatLevel::Cold).unwrap(), "\"COLD\"");
    }

    #[test]
    fn test_heat_sort_by_default() {
        let sort: HeatSortBy = Default::default();
        assert!(matches!(sort, HeatSortBy::Heat));
    }

    #[test]
    fn test_rcr_safe_division() {
        // When count_long is 0, RCR should be 0.0 (handled by caller)
        // But velocity should also be safe
        let v = compute_velocity(0, "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z");
        assert!((v - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_heat_level_display() {
        assert_eq!(format!("{}", HeatLevel::Hot), "HOT");
        assert_eq!(format!("{}", HeatLevel::Warm), "WARM");
        assert_eq!(format!("{}", HeatLevel::Cool), "COOL");
        assert_eq!(format!("{}", HeatLevel::Cold), "COLD");
    }

    // =========================================================================
    // Phase 2: parse_time_range edge cases (Stress Tests)
    // =========================================================================

    #[test]
    fn stress_parse_time_range_zero_days() {
        let result = parse_time_range("0d");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn stress_parse_time_range_huge_value() {
        let result = parse_time_range("99999d");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 99999);
    }

    #[test]
    fn stress_parse_time_range_no_suffix() {
        let result = parse_time_range("abc");
        assert!(
            result.is_err(),
            "Non-numeric without 'd' suffix should error"
        );
    }

    #[test]
    fn stress_parse_time_range_empty_string() {
        let result = parse_time_range("");
        assert!(result.is_err(), "Empty string should error");
    }

    #[test]
    fn stress_parse_time_range_negative() {
        // Documents current behavior: "-7d" parses to -7 (no validation)
        let result = parse_time_range("-7d");
        assert_eq!(
            result.unwrap(),
            -7,
            "Current behavior: negative parses without error"
        );
    }

    #[test]
    fn stress_parse_time_range_float() {
        let result = parse_time_range("7.5d");
        assert!(result.is_err(), "Float days should error (i64 parse)");
    }

    #[test]
    fn stress_parse_time_range_overflow() {
        let result = parse_time_range("99999999999999999999d");
        assert!(result.is_err(), "Overflow should error");
    }

    #[test]
    fn stress_parse_time_range_just_d() {
        let result = parse_time_range("d");
        assert!(result.is_err(), "Just 'd' with no number should error");
    }
}
