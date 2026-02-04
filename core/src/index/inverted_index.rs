//! Inverted file index for Context OS Intelligence.
//!
//! Maps: file_path -> List[FileAccess]
//! Enables: "Which sessions touched this file?"
//!
//! Algorithm:
//! 1. Parse tool_use blocks from JSONL (Read, Edit, Write, etc.)
//! 2. Extract file paths from tool inputs
//! 3. Filter out pattern tools (Grep, Glob) - they search, not access files
//! 4. Classify access type (read/write/create)
//! 5. Build bidirectional file <-> sessions mapping
//! 6. Deduplicate within session (increment count)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// ============================================================================
// Type Definitions
// ============================================================================

/// Single file access record with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    /// File path (relative or absolute)
    pub file_path: String,
    /// Session that accessed the file
    pub session_id: String,
    /// Chain the session belongs to (optional, populated later)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    /// Access timestamp
    pub timestamp: DateTime<Utc>,
    /// Access type: "read", "write", "create"
    pub access_type: String,
    /// Tool used: Read, Edit, Write, etc.
    pub tool_name: String,
    /// Number of accesses within session (deduplication count)
    pub access_count: i32,
}

/// Bidirectional file <-> session mapping.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvertedIndex {
    /// file_path -> list of accesses (all sessions)
    pub file_to_accesses: HashMap<String, Vec<FileAccess>>,
    /// session_id -> list of file paths accessed
    pub session_to_files: HashMap<String, Vec<String>>,
}

/// Result of building the inverted index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexBuildResult {
    /// Total file accesses indexed
    pub accesses_indexed: i32,
    /// Unique files
    pub unique_files: i32,
    /// Unique sessions
    pub unique_sessions: i32,
}

// ============================================================================
// Access Type Classification (TDD Cycle 1)
// ============================================================================

/// Classify tool name to access type.
///
/// Returns:
/// - Some("read") for Read, WebFetch, WebSearch
/// - Some("write") for Edit, NotebookEdit
/// - Some("create") for Write
/// - None for pattern tools (Grep, Glob) or non-file tools
pub fn classify_access_type(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "Read" | "WebFetch" | "WebSearch" => Some("read"),
        "Edit" | "NotebookEdit" => Some("write"),
        "Write" => Some("create"),
        "Grep" | "Glob" => None, // Pattern-based, not file access
        _ => None,
    }
}

/// Classify toolUseResult.type to access type.
///
/// Maps the result type field to access classification:
/// - "create" -> "create"
/// - "update" -> "write"
/// - "text"   -> "read"
/// - unknown  -> "read" (safe default)
pub fn classify_tool_use_result(result_type: &str) -> &'static str {
    match result_type {
        "create" => "create",
        "update" => "write",
        "text" => "read",
        _ => "read", // Safe default
    }
}

// ============================================================================
// File Path Extraction (TDD Cycle 2)
// ============================================================================

/// Extract file path for inverted index.
/// Skips pattern tools (Grep, Glob) - they search, not access files.
pub fn extract_inverted_file_path(tool_name: &str, input: &Value) -> Option<String> {
    // Skip pattern tools
    if tool_name == "Grep" || tool_name == "Glob" {
        return None;
    }

    // NotebookEdit uses notebook_path
    if tool_name == "NotebookEdit" {
        if let Some(path) = input.get("notebook_path").and_then(|p| p.as_str()) {
            return Some(path.to_string());
        }
    }

    // Most tools use file_path
    if let Some(path) = input.get("file_path").and_then(|p| p.as_str()) {
        return Some(path.to_string());
    }

    // Fallback to path
    input.get("path").and_then(|p| p.as_str()).map(String::from)
}

/// Extract file path from toolUseResult (Gap 1 fix).
///
/// Extraction priority:
/// 1. toolUseResult.filePath (direct)
/// 2. toolUseResult.file.filePath (nested)
pub fn extract_tool_use_result_path(record: &Value) -> Option<String> {
    let result = record.get("toolUseResult")?;

    // Priority 1: Direct filePath
    if let Some(path) = result.get("filePath").and_then(|p| p.as_str()) {
        return Some(path.to_string());
    }

    // Priority 2: Nested in file object
    if let Some(file) = result.get("file") {
        if let Some(path) = file.get("filePath").and_then(|p| p.as_str()) {
            return Some(path.to_string());
        }
    }

    None
}

// ============================================================================
// JSONL Extraction (TDD Cycle 3)
// ============================================================================

/// Extract file accesses from a JSONL session file.
/// Uses 3-source extraction: assistant tool_use + user toolUseResult + file-history-snapshot
pub fn extract_file_accesses(filepath: &Path, session_id: &str) -> Vec<FileAccess> {
    // Track accesses by (file_path, access_type) for deduplication
    let mut access_tracker: HashMap<(String, String), FileAccess> = HashMap::new();

    let file = match File::open(filepath) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = BufReader::new(file);

    for line in reader.lines().filter_map(Result::ok) {
        if line.is_empty() {
            continue;
        }

        let record: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let record_type = record.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let timestamp = parse_timestamp(&record);

        match record_type {
            // Source 1: Assistant tool_use blocks
            "assistant" => {
                if let Some(content) = record
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array())
                {
                    for block in content {
                        if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                            continue;
                        }
                        let tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let input = block.get("input").cloned().unwrap_or(Value::Null);

                        if let Some(access_type) = classify_access_type(tool_name) {
                            if let Some(file_path) = extract_inverted_file_path(tool_name, &input) {
                                add_access(
                                    &mut access_tracker,
                                    file_path,
                                    access_type,
                                    tool_name,
                                    session_id,
                                    timestamp,
                                );
                            }
                        }
                    }
                }
            }
            // Source 2: User toolUseResult (Gap 1 fix)
            "user" => {
                if let Some(file_path) = extract_tool_use_result_path(&record) {
                    let result_type = record
                        .get("toolUseResult")
                        .and_then(|r| r.get("type"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("text");
                    let access_type = classify_tool_use_result(result_type);
                    add_access(
                        &mut access_tracker,
                        file_path,
                        access_type,
                        "toolUseResult",
                        session_id,
                        timestamp,
                    );
                }
            }
            // Source 3: file-history-snapshot (Gap 2 fix)
            "file-history-snapshot" => {
                if let Some(backups) = record
                    .get("snapshot")
                    .and_then(|s| s.get("trackedFileBackups"))
                    .and_then(|b| b.as_object())
                {
                    for file_path in backups.keys() {
                        add_access(
                            &mut access_tracker,
                            file_path.clone(),
                            "read",
                            "file-history-snapshot",
                            session_id,
                            timestamp,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    access_tracker.into_values().collect()
}

/// Helper to add access with deduplication
fn add_access(
    tracker: &mut HashMap<(String, String), FileAccess>,
    file_path: String,
    access_type: &str,
    tool_name: &str,
    session_id: &str,
    timestamp: DateTime<Utc>,
) {
    let key = (file_path.clone(), access_type.to_string());
    if let Some(existing) = tracker.get_mut(&key) {
        existing.access_count += 1;
    } else {
        tracker.insert(
            key,
            FileAccess {
                file_path,
                session_id: session_id.to_string(),
                chain_id: None,
                timestamp,
                access_type: access_type.to_string(),
                tool_name: tool_name.to_string(),
                access_count: 1,
            },
        );
    }
}

fn parse_timestamp(record: &Value) -> DateTime<Utc> {
    record
        .get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| {
            let s = s.replace('Z', "+00:00");
            chrono::DateTime::parse_from_rfc3339(&s).ok()
        })
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

// ============================================================================
// Index Building (TDD Cycle 4)
// ============================================================================

/// Build inverted file index from JSONL directory.
pub fn build_inverted_index(
    jsonl_dir: &Path,
    chains: Option<&HashMap<String, crate::index::chain_graph::Chain>>,
) -> InvertedIndex {
    let mut index = InvertedIndex::default();

    // Build session -> chain lookup
    let session_to_chain: HashMap<String, String> = chains
        .map(|c| {
            c.iter()
                .flat_map(|(chain_id, chain)| {
                    chain
                        .sessions
                        .iter()
                        .map(move |s| (s.clone(), chain_id.clone()))
                })
                .collect()
        })
        .unwrap_or_default();

    // Find all JSONL files (recursive for subagents/)
    let jsonl_files: Vec<_> = walkdir::WalkDir::new(jsonl_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "jsonl")
                .unwrap_or(false)
        })
        .collect();

    for entry in jsonl_files {
        let filepath = entry.path();
        let session_id = filepath
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let accesses = extract_file_accesses(filepath, &session_id);
        let chain_id = session_to_chain.get(&session_id).cloned();

        for mut access in accesses {
            access.chain_id = chain_id.clone();

            // Build session -> files mapping
            index
                .session_to_files
                .entry(session_id.clone())
                .or_default()
                .push(access.file_path.clone());

            // Build file -> accesses mapping
            index
                .file_to_accesses
                .entry(access.file_path.clone())
                .or_default()
                .push(access);
        }
    }

    // Deduplicate session_to_files
    for files in index.session_to_files.values_mut() {
        files.sort();
        files.dedup();
    }

    index
}

// ============================================================================
// Query Functions (TDD Cycle 5)
// ============================================================================

/// Get all sessions that touched a file.
pub fn get_sessions_for_file<'a>(index: &'a InvertedIndex, file_path: &str) -> &'a [FileAccess] {
    index
        .file_to_accesses
        .get(file_path)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// Get all files touched in a session.
pub fn get_files_for_session(index: &InvertedIndex, session_id: &str) -> Vec<String> {
    index
        .session_to_files
        .get(session_id)
        .cloned()
        .unwrap_or_default()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // Helper function for test file creation
    fn create_jsonl_file(dir: &Path, name: &str, lines: &[&str]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
        path
    }

    // ========================================================================
    // Cycle 1: Access Type Classification (4 tests)
    // ========================================================================

    #[test]
    fn test_classify_read_tools_returns_read() {
        assert_eq!(classify_access_type("Read"), Some("read"));
        assert_eq!(classify_access_type("WebFetch"), Some("read"));
        assert_eq!(classify_access_type("WebSearch"), Some("read"));
    }

    #[test]
    fn test_classify_write_tools_returns_write() {
        assert_eq!(classify_access_type("Edit"), Some("write"));
        assert_eq!(classify_access_type("NotebookEdit"), Some("write"));
    }

    #[test]
    fn test_classify_create_tools_returns_create() {
        assert_eq!(classify_access_type("Write"), Some("create"));
    }

    #[test]
    fn test_classify_pattern_and_non_file_tools_returns_none() {
        // Grep, Glob -> None (pattern-based search, not file access)
        assert_eq!(classify_access_type("Grep"), None);
        assert_eq!(classify_access_type("Glob"), None);
        // Non-file tools -> None
        assert_eq!(classify_access_type("Bash"), None);
        assert_eq!(classify_access_type("Task"), None);
        assert_eq!(classify_access_type("Unknown"), None);
    }

    // ========================================================================
    // Cycle 2: File Path Extraction (6 tests)
    // ========================================================================

    #[test]
    fn test_extract_path_from_file_path_field() {
        let input = serde_json::json!({"file_path": "/src/main.rs"});
        assert_eq!(
            extract_inverted_file_path("Read", &input),
            Some("/src/main.rs".to_string())
        );
    }

    #[test]
    fn test_extract_path_from_notebook_path_for_notebook_edit() {
        let input = serde_json::json!({"notebook_path": "/notebook.ipynb"});
        assert_eq!(
            extract_inverted_file_path("NotebookEdit", &input),
            Some("/notebook.ipynb".to_string())
        );
    }

    #[test]
    fn test_extract_path_fallback_to_path_field() {
        let input = serde_json::json!({"path": "/some/path.txt"});
        assert_eq!(
            extract_inverted_file_path("Read", &input),
            Some("/some/path.txt".to_string())
        );
    }

    #[test]
    fn test_extract_path_skips_grep_glob_patterns() {
        let grep_input = serde_json::json!({"pattern": "TODO", "path": "/src"});
        let glob_input = serde_json::json!({"pattern": "**/*.rs"});

        assert_eq!(extract_inverted_file_path("Grep", &grep_input), None);
        assert_eq!(extract_inverted_file_path("Glob", &glob_input), None);
    }

    #[test]
    fn test_extract_tool_use_result_path_direct() {
        let record = serde_json::json!({
            "type": "user",
            "toolUseResult": {"filePath": "/confirmed/file.rs", "type": "text"}
        });
        assert_eq!(
            extract_tool_use_result_path(&record),
            Some("/confirmed/file.rs".to_string())
        );
    }

    #[test]
    fn test_extract_tool_use_result_path_nested() {
        let record = serde_json::json!({
            "type": "user",
            "toolUseResult": {
                "file": {"filePath": "/nested/path.rs"},
                "type": "update"
            }
        });
        assert_eq!(
            extract_tool_use_result_path(&record),
            Some("/nested/path.rs".to_string())
        );
    }

    // ========================================================================
    // Cycle 3: JSONL Extraction with 3 Sources (6 tests)
    // ========================================================================

    #[test]
    fn test_extract_from_assistant_tool_use() {
        let dir = TempDir::new().unwrap();
        let path = create_jsonl_file(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/src/main.rs"}}]}}"#,
            ],
        );

        let accesses = extract_file_accesses(&path, "session-id");
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].file_path, "/src/main.rs");
        assert_eq!(accesses[0].access_type, "read");
        assert_eq!(accesses[0].tool_name, "Read");
    }

    #[test]
    fn test_extract_from_user_tool_use_result() {
        let dir = TempDir::new().unwrap();
        let path = create_jsonl_file(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"user","timestamp":"2026-01-18T10:00:00Z","toolUseResult":{"filePath":"/confirmed.rs","type":"text"}}"#,
            ],
        );

        let accesses = extract_file_accesses(&path, "session-id");
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].file_path, "/confirmed.rs");
        assert_eq!(accesses[0].access_type, "read");
    }

    #[test]
    fn test_extract_from_file_history_snapshot() {
        let dir = TempDir::new().unwrap();
        let path = create_jsonl_file(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"file-history-snapshot","timestamp":"2026-01-18T10:00:00Z","snapshot":{"trackedFileBackups":{"/tracked.rs":{}}}}"#,
            ],
        );

        let accesses = extract_file_accesses(&path, "session-id");
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].file_path, "/tracked.rs");
        assert_eq!(accesses[0].access_type, "read");
    }

    #[test]
    fn test_dedup_within_session_increments_count() {
        let dir = TempDir::new().unwrap();
        let path = create_jsonl_file(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/same.rs"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:01:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/same.rs"}}]}}"#,
            ],
        );

        let accesses = extract_file_accesses(&path, "session-id");
        assert_eq!(accesses.len(), 1); // Deduplicated
        assert_eq!(accesses[0].access_count, 2); // Count incremented
    }

    #[test]
    fn test_preserve_cross_session_as_separate_records() {
        let dir = TempDir::new().unwrap();
        let path = create_jsonl_file(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/file.rs"}}]}}"#,
            ],
        );

        let accesses = extract_file_accesses(&path, "custom-session-id");
        assert_eq!(accesses[0].session_id, "custom-session-id");
    }

    #[test]
    fn test_skip_non_file_tools() {
        let dir = TempDir::new().unwrap();
        let path = create_jsonl_file(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:01:00Z","message":{"content":[{"type":"tool_use","name":"Task","input":{"prompt":"do something"}}]}}"#,
            ],
        );

        let accesses = extract_file_accesses(&path, "session-id");
        assert_eq!(accesses.len(), 0); // No file accesses from Bash/Task
    }

    // ========================================================================
    // Cycle 4: Index Building (4 tests)
    // ========================================================================

    #[test]
    fn test_build_index_single_session() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/file.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);

        assert_eq!(index.file_to_accesses.len(), 1);
        assert!(index.file_to_accesses.contains_key("/file.rs"));
        assert_eq!(index.session_to_files.len(), 1);
        assert!(index.session_to_files.contains_key("session1"));
    }

    #[test]
    fn test_build_index_multiple_sessions() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/shared.rs"}}]}}"#,
            ],
        );
        create_jsonl_file(
            dir.path(),
            "session2.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T11:00:00Z","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/shared.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);

        // Same file accessed by 2 sessions
        let accesses = index.file_to_accesses.get("/shared.rs").unwrap();
        assert_eq!(accesses.len(), 2); // Separate records per session
    }

    #[test]
    fn test_file_to_sessions_lookup() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/target.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);
        let accesses = index.file_to_accesses.get("/target.rs").unwrap();

        assert_eq!(accesses[0].session_id, "session1");
        assert_eq!(accesses[0].access_type, "read");
    }

    #[test]
    fn test_session_to_files_lookup() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/file1.rs"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:01:00Z","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/file2.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);
        let files = index.session_to_files.get("session1").unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&"/file1.rs".to_string()));
        assert!(files.contains(&"/file2.rs".to_string()));
    }

    // ========================================================================
    // Cycle 5: Integration & Parity (4 tests)
    // ========================================================================

    #[test]
    fn test_index_build_result_counts() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/file1.rs"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:01:00Z","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/file2.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);
        let result = IndexBuildResult {
            accesses_indexed: index
                .file_to_accesses
                .values()
                .map(|v| v.len() as i32)
                .sum(),
            unique_files: index.file_to_accesses.len() as i32,
            unique_sessions: index.session_to_files.len() as i32,
        };

        assert_eq!(result.unique_files, 2);
        assert_eq!(result.unique_sessions, 1);
        assert_eq!(result.accesses_indexed, 2);
    }

    #[test]
    fn test_grep_glob_patterns_filtered_out() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Grep","input":{"pattern":"TODO","path":"/src"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:01:00Z","message":{"content":[{"type":"tool_use","name":"Glob","input":{"pattern":"**/*.rs"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:02:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/real.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);

        // Only Read should create an access, Grep/Glob should be filtered
        assert_eq!(index.file_to_accesses.len(), 1);
        assert!(index.file_to_accesses.contains_key("/real.rs"));
        assert!(!index
            .file_to_accesses
            .keys()
            .any(|k| k.starts_with("GREP:") || k.starts_with("GLOB:")));
    }

    #[test]
    fn test_query_file_history() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "session1.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/query/target.rs"}}]}}"#,
            ],
        );
        create_jsonl_file(
            dir.path(),
            "session2.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T11:00:00Z","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/query/target.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);

        // Query: "Which sessions touched /query/target.rs?"
        let accesses = get_sessions_for_file(&index, "/query/target.rs");
        assert_eq!(accesses.len(), 2);

        let session_ids: Vec<_> = accesses.iter().map(|a| &a.session_id).collect();
        assert!(session_ids.contains(&&"session1".to_string()));
        assert!(session_ids.contains(&&"session2".to_string()));
    }

    #[test]
    fn test_query_files_for_session() {
        let dir = TempDir::new().unwrap();
        create_jsonl_file(
            dir.path(),
            "target-session.jsonl",
            &[
                r#"{"type":"assistant","timestamp":"2026-01-18T10:00:00Z","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/a.rs"}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-01-18T10:01:00Z","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"/b.rs"}}]}}"#,
            ],
        );

        let index = build_inverted_index(dir.path(), None);

        // Query: "What files did target-session touch?"
        let files = get_files_for_session(&index, "target-session");
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"/a.rs".to_string()));
        assert!(files.contains(&"/b.rs".to_string()));
    }
}
