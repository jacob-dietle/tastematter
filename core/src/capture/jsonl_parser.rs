//! JSONL Parser for Claude Code session files.
//!
//! Implements 3-source extraction algorithm:
//! - Source 1: Assistant tool_use blocks (~190K)
//! - Source 2: User toolUseResult (Gap 1 fix, ~4K)
//! - Source 3: file-history-snapshot (Gap 2 fix, ~2K)
//!
//! Target: 196K tool uses (matches Python baseline)

use chrono::{DateTime, Utc};
use glob::glob;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// =============================================================================
// Tool Classification Constants
// =============================================================================

/// Tools that read files/content
pub const READ_TOOLS: &[&str] = &["Read", "Grep", "Glob", "WebFetch", "WebSearch", "Skill"];

/// Tools that write/modify files
pub const WRITE_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];

// =============================================================================
// Core Types
// =============================================================================

/// A single tool use extracted from a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Tool invocation ID (e.g., "toolu_abc123")
    pub id: String,
    /// Tool name (e.g., "Read", "Edit", "Grep")
    pub name: String,
    /// Raw tool input preserved as JSON
    pub input: Value,
    /// Timestamp of the containing message
    pub timestamp: DateTime<Utc>,
    /// Primary file path or GREP:/GLOB: pseudo-path
    pub file_path: Option<String>,
    /// True if tool is a read operation
    pub is_read: bool,
    /// True if tool is a write operation
    pub is_write: bool,
}

/// A parsed message from a JSONL line.
#[derive(Debug, Clone)]
pub struct ParsedMessage {
    /// Message type: user, assistant, tool_result, file-history-snapshot
    pub msg_type: String,
    /// Role: user or assistant (if applicable)
    pub role: Option<String>,
    /// Message content (string or array of content blocks)
    pub content: Value,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Tool uses extracted from this message
    pub tool_uses: Vec<ToolUse>,
}

/// Aggregated summary of a complete session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session ID (UUID from filename)
    pub session_id: String,
    /// Decoded filesystem path to project
    pub project_path: String,

    // Timing
    /// Session start time (min of timestamps)
    pub started_at: DateTime<Utc>,
    /// Session end time (max of timestamps)
    pub ended_at: DateTime<Utc>,
    /// Duration in seconds
    pub duration_seconds: i64,

    // Message counts
    pub user_message_count: i32,
    pub assistant_message_count: i32,
    pub total_messages: i32,

    // Files (DEDUPLICATED)
    /// Unique files read (Read, Grep, Glob)
    pub files_read: Vec<String>,
    /// Unique files written (Edit, Write)
    pub files_written: Vec<String>,
    /// Files created (heuristic: Write operations)
    pub files_created: Vec<String>,

    // Tool usage (NOT deduplicated - counts all invocations)
    /// Tool name → invocation count
    pub tools_used: HashMap<String, i32>,

    // Patterns
    /// Grep patterns from GREP: pseudo-paths
    pub grep_patterns: Vec<String>,

    // Incremental sync
    /// File size for change detection
    pub file_size_bytes: i64,

    // Intent extraction
    /// First user message content (captures user's stated intent)
    /// Extracted from first type="user" record with string message.content
    pub first_user_message: Option<String>,

    /// Session conversation excerpt for chain naming
    /// Contains user messages concatenated (truncated at ~8K chars)
    /// Provides full context for Haiku classification
    pub conversation_excerpt: Option<String>,
}

/// Options for parsing sessions.
#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    /// Skip unchanged sessions (compare file size)
    pub incremental: bool,
    /// Filter to specific project path
    pub project_filter: Option<String>,
}

/// Result of a sync operation.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ParseResult {
    /// Number of sessions parsed
    pub sessions_parsed: i32,
    /// Number of sessions skipped (unchanged)
    pub sessions_skipped: i32,
    /// Total tool uses across all sessions
    pub total_tool_uses: i64,
    /// Errors encountered during parsing
    pub errors: Vec<String>,
}

// =============================================================================
// Path Encoding/Decoding
// =============================================================================

/// Encode a project path for use in Claude's directory structure.
///
/// Windows: `C:\Users\foo` → `C--Users-foo`
/// Unix: `/home/user` → `-home-user`
pub fn encode_project_path(path: &Path) -> String {
    let path_str = path.to_string_lossy().to_string();

    if path_str.contains(':') {
        // Windows: C:\Users\foo → C--Users-foo
        path_str.replace(":\\", "--").replace(['\\', ' ', '_'], "-")
    } else {
        // Unix: /home/user → -home-user
        path_str.replace(['/', ' ', '_'], "-")
    }
}

/// Decode an encoded project path back to filesystem format.
///
/// Note: This is lossy - cannot distinguish original `-` from spaces/underscores.
pub fn decode_project_path(encoded: &str) -> String {
    // Detection: Windows if matches ^[A-Za-z]--
    let is_windows = encoded.len() >= 3
        && encoded
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
        && encoded.get(1..3) == Some("--");

    if is_windows {
        // C--Users-foo → C:\Users\foo
        let drive = &encoded[0..1];
        let rest = &encoded[3..]; // Skip "X--"
        format!("{}:\\{}", drive, rest.replace('-', "\\"))
    } else {
        // -home-user → /home/user
        encoded.replace('-', "/")
    }
}

// =============================================================================
// Path Normalization
// =============================================================================

/// Normalize a file path to project-relative form.
///
/// Rules:
/// 1. If path starts with project_path (case-insensitive on Windows), strip it
/// 2. Convert backslashes to forward slashes for consistency
/// 3. Strip leading separator after prefix removal
/// 4. Leave pseudo-paths (GREP:, GLOB:) unchanged
/// 5. Leave already-relative paths unchanged
/// 6. Leave paths outside the project unchanged (cannot normalize)
pub fn normalize_file_path(raw_path: &str, project_path: &str) -> String {
    // Rule 4: Skip pseudo-paths
    if raw_path.starts_with("GREP:") || raw_path.starts_with("GLOB:") {
        return raw_path.to_string();
    }

    // Skip empty paths
    if raw_path.is_empty() {
        return raw_path.to_string();
    }

    // Normalize separators for comparison
    let normalized_raw = raw_path.replace('\\', "/");
    let normalized_project = project_path.replace('\\', "/");

    // Rule 1: Strip project path prefix (case-insensitive for Windows)
    let relative = if normalized_raw
        .to_lowercase()
        .starts_with(&normalized_project.to_lowercase())
    {
        let remainder = &normalized_raw[normalized_project.len()..];
        // Rule 3: Strip leading separator
        remainder.trim_start_matches('/')
    } else {
        // Rule 5/6: Already relative or outside project
        &normalized_raw
    };

    // Rule 2: Result already has forward slashes from normalization
    relative.to_string()
}

// =============================================================================
// Tool Classification
// =============================================================================

/// Check if a tool name is a read operation.
pub fn is_read_tool(name: &str) -> bool {
    READ_TOOLS.contains(&name)
}

/// Check if a tool name is a write operation.
pub fn is_write_tool(name: &str) -> bool {
    WRITE_TOOLS.contains(&name)
}

// =============================================================================
// File Path Extraction
// =============================================================================

/// Extract the primary file path from tool input.
///
/// Returns pseudo-paths for pattern-based tools:
/// - Grep: `GREP:pattern`
/// - Glob: `GLOB:pattern`
pub fn extract_file_path(tool_name: &str, input: &Value) -> Option<String> {
    // Special handling for pattern-based tools
    if tool_name == "Grep" {
        if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
            return Some(format!("GREP:{}", pattern));
        }
    }
    if tool_name == "Glob" {
        if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
            return Some(format!("GLOB:{}", pattern));
        }
    }
    if tool_name == "Skill" {
        if let Some(skill_name) = input.get("skill").and_then(|v| v.as_str()) {
            return Some(format!(".claude/skills/{}/SKILL.md", skill_name));
        }
        return None;
    }

    // Direct field extraction (in priority order)
    if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
        return Some(path.to_string());
    }
    if let Some(path) = input.get("notebook_path").and_then(|v| v.as_str()) {
        return Some(path.to_string());
    }
    if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
        return Some(path.to_string());
    }

    None
}

// =============================================================================
// Source 1: Assistant Tool Use Extraction
// =============================================================================

/// Extract tool uses from assistant message content blocks.
///
/// Source 1 of 3-source extraction (~190K tool uses).
/// Iterates content array, finds type=="tool_use" blocks.
pub fn extract_from_assistant(data: &Value, timestamp: DateTime<Utc>) -> Vec<ToolUse> {
    let mut tool_uses = Vec::new();

    // Get message.content array
    let content = match data
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        Some(arr) => arr,
        None => return tool_uses,
    };

    for block in content {
        // Skip non-tool_use blocks
        if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
            continue;
        }

        let id = block
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let name = block
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let input = block.get("input").cloned().unwrap_or(Value::Null);

        let file_path = extract_file_path(&name, &input);
        let is_read = is_read_tool(&name);
        let is_write = is_write_tool(&name);

        tool_uses.push(ToolUse {
            id,
            name,
            input,
            timestamp,
            file_path,
            is_read,
            is_write,
        });
    }

    tool_uses
}

// =============================================================================
// Source 2: User toolUseResult Extraction (Gap 1 Fix)
// =============================================================================

/// Extract tool uses from user message toolUseResult field.
///
/// Source 2 of 3-source extraction (~4K tool uses).
/// This was GAP 1 - user messages with toolUseResult containing file paths.
pub fn extract_from_tool_use_result(data: &Value, timestamp: DateTime<Utc>) -> Vec<ToolUse> {
    let mut tool_uses = Vec::new();

    let tool_use_result = match data.get("toolUseResult") {
        Some(result) if result.is_object() => result,
        _ => return tool_uses,
    };

    // Try direct filePath first
    let mut file_path = tool_use_result
        .get("filePath")
        .and_then(|v| v.as_str())
        .map(String::from);

    // If not found, try nested file.filePath
    if file_path.is_none() {
        file_path = tool_use_result
            .get("file")
            .and_then(|f| f.get("filePath"))
            .and_then(|v| v.as_str())
            .map(String::from);
    }

    // Only create tool use if we found a file path
    if let Some(path) = file_path {
        // Map result type to read/write classification
        let result_type = tool_use_result
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let is_read = result_type == "text";
        let is_write = result_type == "update" || result_type == "create";

        tool_uses.push(ToolUse {
            id: "toolUseResult".to_string(),
            name: "toolUseResult".to_string(),
            input: tool_use_result.clone(),
            timestamp,
            file_path: Some(path),
            is_read,
            is_write,
        });
    }

    tool_uses
}

// =============================================================================
// Source 3: file-history-snapshot Extraction (Gap 2 Fix)
// =============================================================================

/// Extract tool uses from file-history-snapshot records.
///
/// Source 3 of 3-source extraction (~2K tool uses).
/// This was GAP 2 - file-history-snapshot with trackedFileBackups.
pub fn extract_from_snapshot(data: &Value, timestamp: DateTime<Utc>) -> Vec<ToolUse> {
    let mut tool_uses = Vec::new();

    let tracked_backups = match data
        .get("snapshot")
        .and_then(|s| s.get("trackedFileBackups"))
        .and_then(|t| t.as_object())
    {
        Some(obj) => obj,
        None => return tool_uses,
    };

    // Each key in trackedFileBackups is a file path
    for file_path in tracked_backups.keys() {
        tool_uses.push(ToolUse {
            id: "file-history-snapshot".to_string(),
            name: "file-history-snapshot".to_string(),
            input: serde_json::json!({"file_path": file_path}),
            timestamp,
            file_path: Some(file_path.clone()),
            is_read: true, // Tracking is reading
            is_write: false,
        });
    }

    tool_uses
}

// =============================================================================
// Timestamp Parsing
// =============================================================================

/// Parse timestamp from a JSONL record.
/// Handles ISO8601 with Z suffix (replaces with +00:00).
///
/// Timestamp locations by record type:
/// - user, assistant, system, tool_result: `.timestamp` (root)
/// - file-history-snapshot: `.snapshot.timestamp` (nested)
/// - summary: no timestamp (returns Utc::now() as fallback)
fn parse_timestamp(data: &Value) -> DateTime<Utc> {
    // Try timestamp field locations in priority order:
    // 1. Root level (most record types)
    // 2. message.timestamp (legacy/nested)
    // 3. snapshot.timestamp (file-history-snapshot records)
    let ts_str = data
        .get("timestamp")
        .or_else(|| data.get("message").and_then(|m| m.get("timestamp")))
        .or_else(|| data.get("snapshot").and_then(|s| s.get("timestamp")))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Replace Z suffix with +00:00 for chrono compatibility
    let normalized = if let Some(stripped) = ts_str.strip_suffix('Z') {
        format!("{}+00:00", stripped)
    } else {
        ts_str.to_string()
    };

    normalized
        .parse::<DateTime<Utc>>()
        .unwrap_or_else(|_| Utc::now())
}

// =============================================================================
// Message Parsing (3-Source Dispatch)
// =============================================================================

/// Parse a single JSONL line into a ParsedMessage.
///
/// Dispatches to appropriate extraction function based on message type:
/// - assistant → Source 1 (tool_use blocks)
/// - user → Source 2 (toolUseResult, Gap 1)
/// - file-history-snapshot → Source 3 (trackedFileBackups, Gap 2)
/// - tool_result → no extraction (skipped)
/// - unknown → None
pub fn parse_jsonl_line(line: &str) -> Option<ParsedMessage> {
    // Skip empty lines
    if line.trim().is_empty() {
        return None;
    }

    // Parse JSON
    let data: Value = serde_json::from_str(line).ok()?;

    // Get message type (required)
    let msg_type = data.get("type")?.as_str()?.to_string();

    // Parse timestamp
    let timestamp = parse_timestamp(&data);

    // Get role (optional - from message.role or inferred)
    let role = data
        .get("message")
        .and_then(|m| m.get("role"))
        .and_then(|r| r.as_str())
        .map(String::from)
        .or_else(|| {
            // Infer role from type if not explicit
            match msg_type.as_str() {
                "user" => Some("user".to_string()),
                "assistant" => Some("assistant".to_string()),
                _ => None,
            }
        });

    // Get content
    let content = data
        .get("message")
        .and_then(|m| m.get("content"))
        .cloned()
        .unwrap_or(Value::Null);

    // 3-Source Extraction Dispatch
    let tool_uses = match msg_type.as_str() {
        // SOURCE 1: Assistant tool_use blocks (~190K)
        "assistant" => extract_from_assistant(&data, timestamp),

        // SOURCE 2: User toolUseResult (Gap 1 fix, ~4K)
        "user" => extract_from_tool_use_result(&data, timestamp),

        // SOURCE 3: file-history-snapshot (Gap 2 fix, ~2K)
        "file-history-snapshot" => extract_from_snapshot(&data, timestamp),

        // tool_result has no tool uses to extract
        "tool_result" => vec![],

        // Unknown types - skip entirely
        _ => return None,
    };

    Some(ParsedMessage {
        msg_type,
        role,
        content,
        timestamp,
        tool_uses,
    })
}

// =============================================================================
// Session Aggregation
// =============================================================================

/// Aggregate a list of parsed messages into a session summary.
///
/// Key behaviors:
/// - Files are DEDUPLICATED (same file read twice → listed once)
/// - Tool counts are NOT deduplicated (each invocation counted)
/// - Grep patterns extracted from GREP: pseudo-paths
/// - Duration calculated from min/max timestamps
pub fn aggregate_session(
    session_id: &str,
    project_path: &str,
    messages: &[ParsedMessage],
    file_size_bytes: i64,
) -> SessionSummary {
    let mut files_read_set: HashSet<String> = HashSet::new(); // non-snapshot reads
    let mut snapshot_paths: HashSet<String> = HashSet::new(); // snapshot-only tracking
    let mut files_written_set = HashSet::new();
    let mut files_created_set = HashSet::new();
    let mut tools_used: HashMap<String, i32> = HashMap::new();
    let mut grep_patterns = Vec::new();

    let mut user_count = 0i32;
    let mut assistant_count = 0i32;

    let mut min_ts: Option<DateTime<Utc>> = None;
    let mut max_ts: Option<DateTime<Utc>> = None;

    // Track first user message and full conversation excerpt for intent extraction
    let mut first_user_message: Option<String> = None;
    let mut user_messages: Vec<String> = Vec::new();
    const MAX_EXCERPT_CHARS: usize = 8000; // ~2K tokens for Haiku

    for msg in messages {
        // Count messages by role
        match msg.role.as_deref() {
            Some("user") => {
                user_count += 1;
                // Extract user message content if it's a string
                // message.content can be string or array - we want the string form
                if let Some(text) = msg.content.as_str() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        // Capture first user message
                        if first_user_message.is_none() {
                            first_user_message = Some(trimmed.to_string());
                        }
                        // Collect all user messages for excerpt
                        user_messages.push(trimmed.to_string());
                    }
                }
            }
            Some("assistant") => assistant_count += 1,
            _ => {}
        }

        // Track timestamps
        if min_ts.is_none() || msg.timestamp < min_ts.unwrap() {
            min_ts = Some(msg.timestamp);
        }
        if max_ts.is_none() || msg.timestamp > max_ts.unwrap() {
            max_ts = Some(msg.timestamp);
        }

        // Process tool uses
        for tool_use in &msg.tool_uses {
            // Count tools (NOT deduplicated)
            *tools_used.entry(tool_use.name.clone()).or_insert(0) += 1;

            // Extract grep patterns
            if let Some(ref path) = tool_use.file_path {
                if let Some(pattern) = path.strip_prefix("GREP:") {
                    grep_patterns.push(pattern.to_string());
                }
            }

            // Collect files (DEDUPLICATED via HashSet)
            if let Some(ref path) = tool_use.file_path {
                // Skip pseudo-paths for file tracking
                if path.starts_with("GREP:") || path.starts_with("GLOB:") {
                    continue;
                }

                let normalized = normalize_file_path(path, project_path);
                if tool_use.name == "file-history-snapshot" {
                    // Track separately — don't pollute files_read with snapshot noise
                    snapshot_paths.insert(normalized.clone());
                } else {
                    if tool_use.is_read {
                        files_read_set.insert(normalized.clone());
                    }
                    if tool_use.is_write {
                        files_written_set.insert(normalized.clone());
                        // Heuristic: Write tool = created file
                        if tool_use.name == "Write" {
                            files_created_set.insert(normalized);
                        }
                    }
                }
            }
        }
    }

    let started_at = min_ts.unwrap_or_else(Utc::now);
    let ended_at = max_ts.unwrap_or_else(Utc::now);
    let duration_seconds = (ended_at - started_at).num_seconds();

    // Build conversation excerpt from user messages (truncated if too long)
    let conversation_excerpt = if user_messages.is_empty() {
        None
    } else {
        let mut excerpt = String::new();
        for (i, msg) in user_messages.iter().enumerate() {
            if !excerpt.is_empty() {
                excerpt.push_str("\n---\n");
            }
            excerpt.push_str(&format!("[User {}]: {}", i + 1, msg));
            // Truncate if exceeding max length
            if excerpt.len() > MAX_EXCERPT_CHARS {
                excerpt.truncate(MAX_EXCERPT_CHARS);
                excerpt.push_str("...[truncated]");
                break;
            }
        }
        Some(excerpt)
    };

    SessionSummary {
        session_id: session_id.to_string(),
        project_path: project_path.to_string(),
        started_at,
        ended_at,
        duration_seconds,
        user_message_count: user_count,
        assistant_message_count: assistant_count,
        total_messages: messages.len() as i32,
        files_read: files_read_set.into_iter().collect(),
        files_written: files_written_set.into_iter().collect(),
        files_created: files_created_set.into_iter().collect(),
        tools_used,
        grep_patterns,
        file_size_bytes,
        first_user_message,
        conversation_excerpt,
    }
}

// =============================================================================
// Incremental Sync Detection
// =============================================================================

/// Check if a session needs to be re-parsed based on file size.
///
/// Returns true if:
/// - Session not found in existing_sessions map (new session)
/// - Current file size differs from stored size (file changed)
///
/// This is a simple heuristic - file size changes indicate content changes.
pub fn session_needs_update(
    session_id: &str,
    current_file_size: i64,
    existing_sessions: &HashMap<String, i64>, // session_id → file_size_bytes
) -> bool {
    match existing_sessions.get(session_id) {
        Some(&stored_size) => current_file_size != stored_size,
        None => true, // New session
    }
}

// =============================================================================
// File Discovery & Session ID Extraction
// =============================================================================

/// Find all JSONL session files under the Claude projects directory.
///
/// Uses recursive glob `**/*.jsonl` to find files in nested directories.
/// This is critical - using `*.jsonl` misses 223+ agent sessions in subdirs.
///
/// If `project_path` is provided, only looks in that specific encoded directory
/// (exact match, like Python). Otherwise scans all projects.
pub fn find_session_files(
    claude_dir: &Path,
    project_path: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let pattern = match project_path {
        Some(path) => {
            // Exact directory lookup: encode path and look only there
            let encoded = encode_project_path(path);
            claude_dir
                .join("projects")
                .join(&encoded)
                .join("**/*.jsonl")
        }
        None => {
            // Scan all projects
            claude_dir.join("projects/**/*.jsonl")
        }
    };

    let pattern_str = pattern
        .to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())?;

    let mut files = Vec::new();
    for entry in glob(pattern_str).map_err(|e| format!("Glob pattern error: {}", e))? {
        match entry {
            Ok(path) => files.push(path),
            Err(e) => eprintln!("Warning: Error reading path: {}", e),
        }
    }

    Ok(files)
}

/// Extract session ID from a JSONL filename.
///
/// Format: `uuid.jsonl` or `agent-uuid.jsonl`
/// Returns the UUID portion without extension.
pub fn extract_session_id(path: &Path) -> Option<String> {
    path.file_stem()?.to_str().map(String::from)
}

/// Extract project path from a JSONL file path.
///
/// Session files are stored at: ~/.claude/projects/{encoded-path}/{session}.jsonl
/// Returns the decoded project path.
pub fn extract_project_path_from_file(path: &Path) -> Option<String> {
    // Walk up to find the 'projects' directory
    let mut current = path.parent()?;

    // Skip any session subdirectory (e.g., subagents)
    while current.file_name()?.to_str()? != "projects" {
        if let Some(parent) = current.parent() {
            // Check if parent is 'projects'
            if parent.file_name().and_then(|n| n.to_str()) == Some("projects") {
                // current is the encoded project path
                let encoded = current.file_name()?.to_str()?;
                return Some(decode_project_path(encoded));
            }
            current = parent;
        } else {
            return None;
        }
    }

    None
}

/// Parse a single JSONL session file into messages.
///
/// Returns (messages, file_size, cwd). The `cwd` is extracted from the first
/// record's `cwd` field, giving the real project path (vs lossy filename decoding).
pub fn parse_session_file(
    path: &Path,
) -> Result<(Vec<ParsedMessage>, i64, Option<String>), String> {
    let file =
        fs::File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;

    let metadata = file
        .metadata()
        .map_err(|e| format!("Failed to get metadata: {}", e))?;
    let file_size = metadata.len() as i64;

    let reader = BufReader::new(file);
    let mut messages = Vec::new();
    let mut cwd: Option<String> = None;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!(
                    "Warning: Error reading line {} in {}: {}",
                    line_num,
                    path.display(),
                    e
                );
                continue;
            }
        };

        // Extract cwd from the first record that has it
        if cwd.is_none() {
            if let Ok(data) = serde_json::from_str::<Value>(&line) {
                if let Some(c) = data.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(c.to_string());
                }
            }
        }

        if let Some(msg) = parse_jsonl_line(&line) {
            messages.push(msg);
        }
    }

    Ok((messages, file_size, cwd))
}

/// Main orchestration: Parse all sessions and return summaries.
///
/// This is the entry point for the JSONL parser module.
pub fn sync_sessions(
    claude_dir: &Path,
    options: &ParseOptions,
    existing_sessions: &HashMap<String, i64>,
) -> Result<(Vec<SessionSummary>, ParseResult), String> {
    // If project_filter is provided, use exact directory lookup (parity with Python)
    let project_path = options.project_filter.as_ref().map(Path::new);
    let files = find_session_files(claude_dir, project_path)?;

    let mut summaries = Vec::new();
    let mut result = ParseResult::default();
    let mut total_tool_uses: i64 = 0;

    for path in files {
        // Extract session ID
        let session_id = match extract_session_id(&path) {
            Some(id) => id,
            None => {
                result.errors.push(format!(
                    "Could not extract session ID from: {}",
                    path.display()
                ));
                continue;
            }
        };

        // Get file size for incremental check
        let file_size = match fs::metadata(&path) {
            Ok(m) => m.len() as i64,
            Err(e) => {
                result.errors.push(format!(
                    "Could not read metadata for {}: {}",
                    path.display(),
                    e
                ));
                continue;
            }
        };

        // Incremental check
        if options.incremental && !session_needs_update(&session_id, file_size, existing_sessions) {
            result.sessions_skipped += 1;
            continue;
        }

        // Parse the session file (also extracts cwd for accurate project path)
        let (messages, _, cwd) = match parse_session_file(&path) {
            Ok(result) => result,
            Err(e) => {
                result.errors.push(e);
                continue;
            }
        };

        // Use cwd from JSONL (accurate) over lossy filename decoding
        let project_path = cwd.unwrap_or_else(|| {
            extract_project_path_from_file(&path).unwrap_or_else(|| "unknown".to_string())
        });

        // Skip sessions with no parseable messages (summary-only JSONL files)
        if messages.is_empty() {
            result.sessions_skipped += 1;
            continue;
        }

        // Count tool uses
        let session_tool_uses: i64 = messages.iter().map(|m| m.tool_uses.len() as i64).sum();
        total_tool_uses += session_tool_uses;

        // Aggregate into summary
        let summary = aggregate_session(&session_id, &project_path, &messages, file_size);
        summaries.push(summary);
        result.sessions_parsed += 1;
    }

    result.total_tool_uses = total_tool_uses;
    Ok((summaries, result))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Cycle 1: Path Encoding (6 tests)
    // =========================================================================

    #[test]
    fn test_encode_windows_basic() {
        let path = Path::new("C:\\Users\\foo");
        assert_eq!(encode_project_path(path), "C--Users-foo");
    }

    #[test]
    fn test_encode_windows_spaces() {
        let path = Path::new("C:\\My Project");
        assert_eq!(encode_project_path(path), "C--My-Project");
    }

    #[test]
    fn test_encode_windows_underscores() {
        let path = Path::new("C:\\my_project");
        assert_eq!(encode_project_path(path), "C--my-project");
    }

    #[test]
    fn test_encode_unix_basic() {
        let path = Path::new("/home/user");
        assert_eq!(encode_project_path(path), "-home-user");
    }

    #[test]
    fn test_decode_windows() {
        assert_eq!(decode_project_path("C--Users-foo"), "C:\\Users\\foo");
    }

    #[test]
    fn test_decode_unix() {
        assert_eq!(decode_project_path("-home-user"), "/home/user");
    }

    // =========================================================================
    // Cycle 2: File Path Extraction (6 tests)
    // =========================================================================

    #[test]
    fn test_extract_file_path_direct() {
        let input = serde_json::json!({"file_path": "/path/to/file.rs"});
        assert_eq!(
            extract_file_path("Read", &input),
            Some("/path/to/file.rs".to_string())
        );
    }

    #[test]
    fn test_extract_file_path_notebook() {
        let input = serde_json::json!({"notebook_path": "/path/to/notebook.ipynb"});
        assert_eq!(
            extract_file_path("NotebookEdit", &input),
            Some("/path/to/notebook.ipynb".to_string())
        );
    }

    #[test]
    fn test_extract_file_path_path() {
        let input = serde_json::json!({"path": "/some/path"});
        assert_eq!(
            extract_file_path("SomeTool", &input),
            Some("/some/path".to_string())
        );
    }

    #[test]
    fn test_extract_grep_pattern() {
        let input = serde_json::json!({"pattern": "fn main"});
        assert_eq!(
            extract_file_path("Grep", &input),
            Some("GREP:fn main".to_string())
        );
    }

    #[test]
    fn test_extract_glob_pattern() {
        let input = serde_json::json!({"pattern": "**/*.rs"});
        assert_eq!(
            extract_file_path("Glob", &input),
            Some("GLOB:**/*.rs".to_string())
        );
    }

    #[test]
    fn test_extract_no_path() {
        let input = serde_json::json!({"other_field": "value"});
        assert_eq!(extract_file_path("SomeTool", &input), None);
    }

    // =========================================================================
    // Cycle 3: Tool Classification (4 tests)
    // =========================================================================

    #[test]
    fn test_is_read_tool() {
        assert!(is_read_tool("Read"));
        assert!(is_read_tool("Grep"));
        assert!(is_read_tool("Glob"));
        assert!(is_read_tool("WebFetch"));
        assert!(is_read_tool("WebSearch"));
    }

    #[test]
    fn test_is_write_tool() {
        assert!(is_write_tool("Edit"));
        assert!(is_write_tool("Write"));
        assert!(is_write_tool("NotebookEdit"));
    }

    #[test]
    fn test_mixed_tools() {
        // Tools that are neither read nor write
        assert!(!is_read_tool("Bash"));
        assert!(!is_write_tool("Bash"));
        assert!(!is_read_tool("Task"));
        assert!(!is_write_tool("Task"));
    }

    #[test]
    fn test_case_sensitivity() {
        // Tool names are case-sensitive
        assert!(!is_read_tool("read"));
        assert!(!is_read_tool("READ"));
        assert!(!is_write_tool("edit"));
        assert!(!is_write_tool("EDIT"));
    }

    // =========================================================================
    // Cycle 4: Source 1 - Assistant Extraction (6 tests)
    // =========================================================================

    fn test_timestamp() -> DateTime<Utc> {
        "2026-01-17T12:00:00Z".parse().unwrap()
    }

    #[test]
    fn test_extract_single_tool_use() {
        let data = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "tool_use",
                        "id": "toolu_123",
                        "name": "Read",
                        "input": {"file_path": "/path/to/file.rs"}
                    }
                ]
            }
        });
        let tool_uses = extract_from_assistant(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].name, "Read");
        assert_eq!(tool_uses[0].file_path, Some("/path/to/file.rs".to_string()));
        assert!(tool_uses[0].is_read);
    }

    #[test]
    fn test_extract_multiple_tool_uses() {
        let data = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "/a.rs"}},
                    {"type": "tool_use", "id": "t2", "name": "Edit", "input": {"file_path": "/b.rs"}}
                ]
            }
        });
        let tool_uses = extract_from_assistant(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 2);
        assert_eq!(tool_uses[0].name, "Read");
        assert_eq!(tool_uses[1].name, "Edit");
    }

    #[test]
    fn test_extract_empty_content() {
        let data = serde_json::json!({
            "type": "assistant",
            "message": {"content": []}
        });
        let tool_uses = extract_from_assistant(&data, test_timestamp());
        assert!(tool_uses.is_empty());
    }

    #[test]
    fn test_extract_text_only() {
        let data = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Hello world"}
                ]
            }
        });
        let tool_uses = extract_from_assistant(&data, test_timestamp());
        assert!(tool_uses.is_empty());
    }

    #[test]
    fn test_extract_mixed_content() {
        let data = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Let me read that file."},
                    {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "/a.rs"}},
                    {"type": "text", "text": "Done."}
                ]
            }
        });
        let tool_uses = extract_from_assistant(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].name, "Read");
    }

    #[test]
    fn test_extract_preserves_input() {
        let data = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "tool_use",
                        "id": "t1",
                        "name": "Bash",
                        "input": {"command": "ls -la", "timeout": 5000}
                    }
                ]
            }
        });
        let tool_uses = extract_from_assistant(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].input["command"], "ls -la");
        assert_eq!(tool_uses[0].input["timeout"], 5000);
    }

    // =========================================================================
    // Cycle 5: Source 2 - toolUseResult (Gap 1) (6 tests)
    // =========================================================================

    #[test]
    fn test_tool_use_result_direct_path() {
        let data = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "filePath": "/path/to/file.rs",
                "type": "text"
            }
        });
        let tool_uses = extract_from_tool_use_result(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].file_path, Some("/path/to/file.rs".to_string()));
    }

    #[test]
    fn test_tool_use_result_nested_path() {
        let data = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "file": {"filePath": "/nested/path.rs"},
                "type": "text"
            }
        });
        let tool_uses = extract_from_tool_use_result(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].file_path, Some("/nested/path.rs".to_string()));
    }

    #[test]
    fn test_tool_use_result_type_text() {
        let data = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "filePath": "/file.rs",
                "type": "text"
            }
        });
        let tool_uses = extract_from_tool_use_result(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert!(tool_uses[0].is_read);
        assert!(!tool_uses[0].is_write);
    }

    #[test]
    fn test_tool_use_result_type_update() {
        let data = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "filePath": "/file.rs",
                "type": "update"
            }
        });
        let tool_uses = extract_from_tool_use_result(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert!(!tool_uses[0].is_read);
        assert!(tool_uses[0].is_write);
    }

    #[test]
    fn test_tool_use_result_type_create() {
        let data = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "filePath": "/new_file.rs",
                "type": "create"
            }
        });
        let tool_uses = extract_from_tool_use_result(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert!(!tool_uses[0].is_read);
        assert!(tool_uses[0].is_write);
    }

    #[test]
    fn test_tool_use_result_no_path() {
        let data = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "content": "some content",
                "type": "text"
            }
        });
        let tool_uses = extract_from_tool_use_result(&data, test_timestamp());
        assert!(tool_uses.is_empty());
    }

    // =========================================================================
    // Cycle 6: Source 3 - file-history-snapshot (Gap 2) (4 tests)
    // =========================================================================

    #[test]
    fn test_snapshot_single_file() {
        let data = serde_json::json!({
            "type": "file-history-snapshot",
            "snapshot": {
                "trackedFileBackups": {
                    "/path/to/file.rs": {"content": "..."}
                }
            }
        });
        let tool_uses = extract_from_snapshot(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].file_path, Some("/path/to/file.rs".to_string()));
        assert!(tool_uses[0].is_read);
    }

    #[test]
    fn test_snapshot_multiple_files() {
        let data = serde_json::json!({
            "type": "file-history-snapshot",
            "snapshot": {
                "trackedFileBackups": {
                    "/file1.rs": {},
                    "/file2.rs": {},
                    "/file3.rs": {}
                }
            }
        });
        let tool_uses = extract_from_snapshot(&data, test_timestamp());
        assert_eq!(tool_uses.len(), 3);
    }

    #[test]
    fn test_snapshot_empty_backups() {
        let data = serde_json::json!({
            "type": "file-history-snapshot",
            "snapshot": {
                "trackedFileBackups": {}
            }
        });
        let tool_uses = extract_from_snapshot(&data, test_timestamp());
        assert!(tool_uses.is_empty());
    }

    #[test]
    fn test_snapshot_missing_snapshot() {
        let data = serde_json::json!({
            "type": "file-history-snapshot"
        });
        let tool_uses = extract_from_snapshot(&data, test_timestamp());
        assert!(tool_uses.is_empty());
    }

    // =========================================================================
    // Cycle 7: Message Parsing (6 tests)
    // =========================================================================

    #[test]
    fn test_parse_user_message() {
        let line = r#"{"type":"user","toolUseResult":{"filePath":"/file.rs","type":"text"},"timestamp":"2026-01-17T12:00:00Z"}"#;
        let msg = parse_jsonl_line(line).unwrap();
        assert_eq!(msg.msg_type, "user");
        assert_eq!(msg.role, Some("user".to_string()));
        assert_eq!(msg.tool_uses.len(), 1);
        assert_eq!(msg.tool_uses[0].file_path, Some("/file.rs".to_string()));
    }

    #[test]
    fn test_parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/code.rs"}}]},"timestamp":"2026-01-17T12:00:00Z"}"#;
        let msg = parse_jsonl_line(line).unwrap();
        assert_eq!(msg.msg_type, "assistant");
        assert_eq!(msg.role, Some("assistant".to_string()));
        assert_eq!(msg.tool_uses.len(), 1);
        assert_eq!(msg.tool_uses[0].name, "Read");
    }

    #[test]
    fn test_parse_tool_result() {
        let line = r#"{"type":"tool_result","content":"file contents here","timestamp":"2026-01-17T12:00:00Z"}"#;
        let msg = parse_jsonl_line(line).unwrap();
        assert_eq!(msg.msg_type, "tool_result");
        assert!(msg.tool_uses.is_empty()); // tool_result has no tool_uses
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_jsonl_line("").is_none());
        assert!(parse_jsonl_line("   ").is_none());
        assert!(parse_jsonl_line("\t\n").is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_jsonl_line("{not valid json").is_none());
        assert!(parse_jsonl_line("just text").is_none());
    }

    #[test]
    fn test_parse_timestamp_z_suffix() {
        let line = r#"{"type":"tool_result","timestamp":"2026-01-17T14:30:45Z"}"#;
        let msg = parse_jsonl_line(line).unwrap();
        // Verify the timestamp was parsed correctly
        assert_eq!(msg.timestamp.format("%Y-%m-%d").to_string(), "2026-01-17");
        assert_eq!(msg.timestamp.format("%H:%M:%S").to_string(), "14:30:45");
    }

    #[test]
    fn test_parse_timestamp_from_snapshot_nested() {
        // file-history-snapshot records have timestamp at .snapshot.timestamp, NOT root
        // This was a bug: parser only checked root .timestamp, causing all timestamps
        // to fall back to Utc::now() (ingestion time) for these records.
        let line = r#"{"type":"file-history-snapshot","messageId":"test-123","snapshot":{"messageId":"test-123","trackedFileBackups":{"/path/to/file.rs":{"backupFileName":"abc@v1","version":1}},"timestamp":"2026-01-15T15:18:45.931Z"},"isSnapshotUpdate":false}"#;
        let msg = parse_jsonl_line(line).unwrap();

        // Verify timestamp was parsed from .snapshot.timestamp (not fallback to now)
        assert_eq!(msg.timestamp.format("%Y-%m-%d").to_string(), "2026-01-15");
        assert_eq!(msg.timestamp.format("%H:%M:%S").to_string(), "15:18:45");

        // Also verify tool uses were extracted from the snapshot
        assert_eq!(msg.tool_uses.len(), 1);
        assert_eq!(
            msg.tool_uses[0].file_path,
            Some("/path/to/file.rs".to_string())
        );
    }

    #[test]
    fn test_parse_timestamp_prefers_root_over_nested() {
        // If a record has BOTH root timestamp AND nested snapshot.timestamp,
        // root should take precedence (per priority order in parse_timestamp)
        let line = r#"{"type":"file-history-snapshot","timestamp":"2026-02-01T10:00:00Z","snapshot":{"timestamp":"2026-01-15T15:18:45Z"}}"#;
        let msg = parse_jsonl_line(line).unwrap();

        // Should use root timestamp (2026-02-01), not snapshot timestamp (2026-01-15)
        assert_eq!(msg.timestamp.format("%Y-%m-%d").to_string(), "2026-02-01");
    }

    // =========================================================================
    // Cycle 8: Session Aggregation (6 tests)
    // =========================================================================

    fn make_message_with_tool(
        role: &str,
        tool_name: &str,
        file_path: Option<&str>,
        is_read: bool,
        is_write: bool,
        ts: DateTime<Utc>,
    ) -> ParsedMessage {
        ParsedMessage {
            msg_type: role.to_string(),
            role: Some(role.to_string()),
            content: Value::Null,
            timestamp: ts,
            tool_uses: vec![ToolUse {
                id: "test".to_string(),
                name: tool_name.to_string(),
                input: Value::Null,
                timestamp: ts,
                file_path: file_path.map(String::from),
                is_read,
                is_write,
            }],
        }
    }

    #[test]
    fn test_aggregate_dedup_files() {
        // Same file read twice should appear once in files_read
        let ts = test_timestamp();
        let messages = vec![
            make_message_with_tool("assistant", "Read", Some("/file.rs"), true, false, ts),
            make_message_with_tool("assistant", "Read", Some("/file.rs"), true, false, ts),
            make_message_with_tool("assistant", "Read", Some("/other.rs"), true, false, ts),
        ];

        let summary = aggregate_session("test-id", "/project", &messages, 1000);

        // Files should be deduplicated
        assert_eq!(summary.files_read.len(), 2); // /file.rs and /other.rs
        assert!(summary.files_read.contains(&"/file.rs".to_string()));
        assert!(summary.files_read.contains(&"/other.rs".to_string()));
    }

    #[test]
    fn test_aggregate_tool_counts() {
        // Tool counts should NOT be deduplicated - each invocation counts
        let ts = test_timestamp();
        let messages = vec![
            make_message_with_tool("assistant", "Read", Some("/a.rs"), true, false, ts),
            make_message_with_tool("assistant", "Read", Some("/a.rs"), true, false, ts),
            make_message_with_tool("assistant", "Edit", Some("/a.rs"), false, true, ts),
        ];

        let summary = aggregate_session("test-id", "/project", &messages, 1000);

        // Tool counts reflect all invocations
        assert_eq!(summary.tools_used.get("Read"), Some(&2));
        assert_eq!(summary.tools_used.get("Edit"), Some(&1));
    }

    #[test]
    fn test_aggregate_separate_rw() {
        // Read and write tracking should be separate
        let ts = test_timestamp();
        let messages = vec![
            make_message_with_tool("assistant", "Read", Some("/read.rs"), true, false, ts),
            make_message_with_tool("assistant", "Edit", Some("/write.rs"), false, true, ts),
        ];

        let summary = aggregate_session("test-id", "/project", &messages, 1000);

        assert_eq!(summary.files_read.len(), 1);
        assert!(summary.files_read.contains(&"/read.rs".to_string()));
        assert_eq!(summary.files_written.len(), 1);
        assert!(summary.files_written.contains(&"/write.rs".to_string()));
    }

    #[test]
    fn test_aggregate_extract_patterns() {
        // GREP: pseudo-paths should be extracted to grep_patterns
        let ts = test_timestamp();
        let messages = vec![
            make_message_with_tool("assistant", "Grep", Some("GREP:fn main"), true, false, ts),
            make_message_with_tool(
                "assistant",
                "Grep",
                Some("GREP:impl Trait"),
                true,
                false,
                ts,
            ),
        ];

        let summary = aggregate_session("test-id", "/project", &messages, 1000);

        assert_eq!(summary.grep_patterns.len(), 2);
        assert!(summary.grep_patterns.contains(&"fn main".to_string()));
        assert!(summary.grep_patterns.contains(&"impl Trait".to_string()));
        // GREP: paths should NOT be in files_read
        assert!(summary.files_read.is_empty());
    }

    #[test]
    fn test_aggregate_calculate_duration() {
        // Duration should be calculated from min/max timestamps
        let ts1: DateTime<Utc> = "2026-01-17T10:00:00Z".parse().unwrap();
        let ts2: DateTime<Utc> = "2026-01-17T10:30:00Z".parse().unwrap();
        let ts3: DateTime<Utc> = "2026-01-17T11:00:00Z".parse().unwrap();

        let messages = vec![
            make_message_with_tool("assistant", "Read", Some("/a.rs"), true, false, ts2),
            make_message_with_tool("assistant", "Read", Some("/b.rs"), true, false, ts1), // earliest
            make_message_with_tool("assistant", "Read", Some("/c.rs"), true, false, ts3), // latest
        ];

        let summary = aggregate_session("test-id", "/project", &messages, 1000);

        assert_eq!(summary.started_at, ts1);
        assert_eq!(summary.ended_at, ts3);
        assert_eq!(summary.duration_seconds, 3600); // 1 hour
    }

    #[test]
    fn test_aggregate_count_messages() {
        // User and assistant messages should be counted separately
        let ts = test_timestamp();
        let messages = vec![
            ParsedMessage {
                msg_type: "user".to_string(),
                role: Some("user".to_string()),
                content: Value::Null,
                timestamp: ts,
                tool_uses: vec![],
            },
            ParsedMessage {
                msg_type: "assistant".to_string(),
                role: Some("assistant".to_string()),
                content: Value::Null,
                timestamp: ts,
                tool_uses: vec![],
            },
            ParsedMessage {
                msg_type: "assistant".to_string(),
                role: Some("assistant".to_string()),
                content: Value::Null,
                timestamp: ts,
                tool_uses: vec![],
            },
            ParsedMessage {
                msg_type: "tool_result".to_string(),
                role: None, // tool_result has no role
                content: Value::Null,
                timestamp: ts,
                tool_uses: vec![],
            },
        ];

        let summary = aggregate_session("test-id", "/project", &messages, 1000);

        assert_eq!(summary.user_message_count, 1);
        assert_eq!(summary.assistant_message_count, 2);
        assert_eq!(summary.total_messages, 4);
    }

    // =========================================================================
    // Cycle 9: Incremental Sync (4 tests)
    // =========================================================================

    #[test]
    fn test_needs_update_new_session() {
        // Session not in existing map → needs update
        let existing: HashMap<String, i64> = HashMap::new();
        assert!(session_needs_update("new-session-id", 1000, &existing));
    }

    #[test]
    fn test_needs_update_same_size() {
        // Same file size → no update needed
        let mut existing = HashMap::new();
        existing.insert("session-1".to_string(), 5000);
        assert!(!session_needs_update("session-1", 5000, &existing));
    }

    #[test]
    fn test_needs_update_larger() {
        // File got larger → needs update
        let mut existing = HashMap::new();
        existing.insert("session-1".to_string(), 5000);
        assert!(session_needs_update("session-1", 6000, &existing));
    }

    #[test]
    fn test_needs_update_not_found() {
        // Different session ID not in map → needs update
        let mut existing = HashMap::new();
        existing.insert("other-session".to_string(), 5000);
        assert!(session_needs_update("session-1", 5000, &existing));
    }

    // =========================================================================
    // DQ-002: Regression tests for phantom session fix
    // =========================================================================

    #[test]
    fn test_summary_only_session_is_skipped() {
        // aggregate_session with empty messages produces a phantom session
        // (duration=0, files=[], timestamps=Utc::now()). The fix skips these
        // at the sync_sessions level, but we verify the aggregate output here.
        let messages: Vec<ParsedMessage> = vec![];
        let summary = aggregate_session("test-empty", "/test/project", &messages, 1234);

        assert_eq!(summary.total_messages, 0);
        assert!(summary.files_read.is_empty());
        assert!(summary.files_written.is_empty());
        assert_eq!(summary.duration_seconds, 0);
        // This is the phantom pattern: no real data but record exists
    }

    #[test]
    fn test_session_with_tools_retains_timestamp() {
        // Sessions with actual tool_use records should retain their historical
        // timestamps from the JSONL data, not fall back to Utc::now().
        let ts: DateTime<Utc> = "2026-01-15T10:00:00Z".parse().unwrap();
        let messages = vec![ParsedMessage {
            msg_type: "assistant".to_string(),
            role: Some("assistant".to_string()),
            timestamp: ts,
            content: serde_json::Value::Null,
            tool_uses: vec![ToolUse {
                id: "toolu_test1".to_string(),
                name: "Read".to_string(),
                input: serde_json::Value::Null,
                timestamp: ts,
                file_path: Some("/src/main.rs".to_string()),
                is_read: true,
                is_write: false,
            }],
        }];

        let summary = aggregate_session("test-with-data", "/test/project", &messages, 5000);

        assert_eq!(summary.started_at, ts);
        assert_eq!(summary.ended_at, ts);
        assert_eq!(summary.total_messages, 1);
        assert!(summary.files_read.contains(&"/src/main.rs".to_string()));
        assert_eq!(summary.file_size_bytes, 5000);
    }

    // =========================================================================
    // DQ-003: Heat score data quality — snapshot exclusion & Skill extraction
    // =========================================================================

    #[test]
    fn test_snapshot_paths_excluded_from_files_read() {
        // A file read by both a real tool AND a snapshot should appear in files_read.
        // A file ONLY seen via snapshot should NOT appear in files_read.
        let ts = test_timestamp();
        let messages = vec![
            // Real read of /shared.rs
            make_message_with_tool("assistant", "Read", Some("/shared.rs"), true, false, ts),
            // Snapshot sees /shared.rs AND /snapshot_only.rs
            ParsedMessage {
                msg_type: "file-history-snapshot".to_string(),
                role: None,
                content: Value::Null,
                timestamp: ts,
                tool_uses: vec![
                    ToolUse {
                        id: "file-history-snapshot".to_string(),
                        name: "file-history-snapshot".to_string(),
                        input: serde_json::json!({"file_path": "/shared.rs"}),
                        timestamp: ts,
                        file_path: Some("/shared.rs".to_string()),
                        is_read: true,
                        is_write: false,
                    },
                    ToolUse {
                        id: "file-history-snapshot".to_string(),
                        name: "file-history-snapshot".to_string(),
                        input: serde_json::json!({"file_path": "/snapshot_only.rs"}),
                        timestamp: ts,
                        file_path: Some("/snapshot_only.rs".to_string()),
                        is_read: true,
                        is_write: false,
                    },
                ],
            },
        ];

        let summary = aggregate_session("test-snap", "/project", &messages, 1000);

        // /shared.rs kept (real Read), /snapshot_only.rs dropped
        assert!(summary.files_read.contains(&"/shared.rs".to_string()));
        assert!(!summary
            .files_read
            .contains(&"/snapshot_only.rs".to_string()));
        assert_eq!(summary.files_read.len(), 1);
    }

    #[test]
    fn test_snapshot_only_session_has_empty_files_read() {
        // A session with ONLY file-history-snapshot entries should have files_read: []
        let ts = test_timestamp();
        let messages = vec![ParsedMessage {
            msg_type: "file-history-snapshot".to_string(),
            role: None,
            content: Value::Null,
            timestamp: ts,
            tool_uses: vec![
                ToolUse {
                    id: "file-history-snapshot".to_string(),
                    name: "file-history-snapshot".to_string(),
                    input: serde_json::json!({"file_path": "/a.rs"}),
                    timestamp: ts,
                    file_path: Some("/a.rs".to_string()),
                    is_read: true,
                    is_write: false,
                },
                ToolUse {
                    id: "file-history-snapshot".to_string(),
                    name: "file-history-snapshot".to_string(),
                    input: serde_json::json!({"file_path": "/b.rs"}),
                    timestamp: ts,
                    file_path: Some("/b.rs".to_string()),
                    is_read: true,
                    is_write: false,
                },
            ],
        }];

        let summary = aggregate_session("test-snap-only", "/project", &messages, 500);

        assert!(summary.files_read.is_empty());
    }

    #[test]
    fn test_skill_tool_extracts_file_path() {
        let input = serde_json::json!({"skill": "context-package"});
        assert_eq!(
            extract_file_path("Skill", &input),
            Some(".claude/skills/context-package/SKILL.md".to_string())
        );
    }

    #[test]
    fn test_skill_tool_without_skill_field_returns_none() {
        // Skill tool invoked without a "skill" field should return None
        let input = serde_json::json!({"args": "--verbose"});
        assert_eq!(extract_file_path("Skill", &input), None);
    }

    #[test]
    fn test_skill_in_read_tools() {
        assert!(is_read_tool("Skill"));
    }

    // =========================================================================
    // Path Normalization Tests (Spec 02: LIVE-01 fix)
    // =========================================================================

    #[test]
    fn test_normalize_absolute_to_relative() {
        assert_eq!(
            normalize_file_path(
                "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system\\_system\\temp\\foo.md",
                "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system"
            ),
            "_system/temp/foo.md"
        );
    }

    #[test]
    fn test_normalize_already_relative_unchanged() {
        assert_eq!(
            normalize_file_path(
                "_system\\temp\\foo.md",
                "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system"
            ),
            "_system/temp/foo.md"
        );
    }

    #[test]
    fn test_normalize_pseudo_paths_unchanged() {
        assert_eq!(
            normalize_file_path("GREP:some_pattern", "/any/project"),
            "GREP:some_pattern"
        );
        assert_eq!(
            normalize_file_path("GLOB:*.rs", "/any/project"),
            "GLOB:*.rs"
        );
    }

    #[test]
    fn test_normalize_backslash_to_forward_slash() {
        assert_eq!(
            normalize_file_path("src\\capture\\jsonl_parser.rs", "/some/project"),
            "src/capture/jsonl_parser.rs"
        );
    }

    #[test]
    fn test_normalize_outside_project_unchanged() {
        assert_eq!(
            normalize_file_path("D:\\Other\\Project\\file.rs", "C:\\Users\\dietl\\MyProject"),
            "D:/Other/Project/file.rs"
        );
    }

    #[test]
    fn test_normalize_case_insensitive_windows() {
        assert_eq!(
            normalize_file_path(
                "c:\\users\\DIETL\\vscode projects\\taste_systems\\gtm_operating_system\\foo.md",
                "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system"
            ),
            "foo.md"
        );
    }

    #[test]
    fn test_normalize_trailing_separator() {
        assert_eq!(
            normalize_file_path("C:\\Users\\project\\foo.md", "C:\\Users\\project\\"),
            "foo.md"
        );
    }

    #[test]
    fn test_session_dedup_after_normalization() {
        // Integration test: same file via absolute and relative paths should deduplicate
        let ts = test_timestamp();
        let project = "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system";
        let messages = vec![
            make_message_with_tool(
                "assistant",
                "Read",
                Some("C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system\\foo.md"),
                true,
                false,
                ts,
            ),
            make_message_with_tool("assistant", "Read", Some("foo.md"), true, false, ts),
        ];

        let summary = aggregate_session("test-dedup", project, &messages, 1000);

        // Both paths should normalize to "foo.md", so only 1 entry
        assert_eq!(summary.files_read.len(), 1);
        assert!(summary.files_read.contains(&"foo.md".to_string()));
    }
}
