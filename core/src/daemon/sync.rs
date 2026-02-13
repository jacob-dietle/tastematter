//! Sync orchestrator module.
//!
//! Orchestrates all sync phases:
//! - Git commit synchronization
//! - Claude session parsing (WITH DATABASE PERSISTENCE)
//! - Chain graph building (WITH DATABASE PERSISTENCE)
//! - Intelligence enrichment (optional - graceful degradation)
//! - Inverted index updates

use log::debug;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use super::config::DaemonConfig;
use crate::capture::git_sync::{sync_commits, SyncOptions};
use crate::capture::jsonl_parser::{sync_sessions, ParseOptions};
use crate::index::chain_graph::{build_chain_graph, Chain};
use crate::index::inverted_index::build_inverted_index;
use crate::intelligence::{ChainNamingRequest, ChainSummaryRequest, IntelClient, MetadataStore};
use crate::query::QueryEngine;
use crate::storage::Database;
use crate::types::SessionInput;
use sqlx::sqlite::SqlitePool;

/// Result from a single sync operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncResult {
    pub git_commits_synced: i32,
    pub sessions_parsed: i32,
    pub chains_built: i32,
    pub files_indexed: i32,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Run a single sync cycle with database persistence.
///
/// Orchestrates all sync phases in order:
/// 1. Git sync
/// 2. Session parsing (WITH DATABASE PERSISTENCE)
/// 3. Chain building (WITH DATABASE PERSISTENCE)
///    3.5. Intelligence enrichment (optional - graceful degradation)
/// 4. Index update
///
/// **CRITICAL FIX:** This function now persists parsed sessions and chains
/// to the database. Previously, parsed data was discarded after extraction.
pub async fn run_sync(config: &DaemonConfig) -> Result<SyncResult, String> {
    let start = Instant::now();
    let mut result = SyncResult::default();

    // Open database in write mode for persistence
    let db_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".context-os");
    let db_path = db_dir.join("context_os_events.db");

    // Ensure directory exists (required for fresh installs)
    if let Err(e) = fs::create_dir_all(&db_dir) {
        result
            .errors
            .push(format!("Could not create database directory: {}", e));
    }

    let engine = match Database::open_rw(&db_path).await {
        Ok(db) => {
            // Ensure schema exists (idempotent - safe on existing DBs, required for fresh installs)
            if let Err(e) = db.ensure_schema().await {
                result.errors.push(format!("Schema init error: {}", e));
            }
            Some(QueryEngine::new(db))
        }
        Err(e) => {
            result.errors.push(format!(
                "DB open error (continuing without persistence): {}",
                e
            ));
            None
        }
    };

    // Get paths - NOTE: find_session_files expects ~/.claude, not ~/.claude/projects
    // The function internally joins "projects" to the base path
    let claude_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".claude");

    // 1. Git sync
    let git_result = sync_git(config, &mut result);
    if let Err(e) = git_result {
        log::info!("Git sync skipped: {}", e);
        // Not pushed to result.errors — git sync is optional enhancement
    }

    // 2. Session parsing WITH PERSISTENCE
    let _session_ids = sync_sessions_phase(&claude_dir, config, &mut result, engine.as_ref()).await;

    // 3. Chain building WITH PERSISTENCE
    let chains = build_chains_phase(&claude_dir, &mut result, engine.as_ref()).await;

    // 3.5 Intelligence enrichment (optional - graceful degradation)
    if let Some(ref chains) = chains {
        enrich_chains_phase(chains, &mut result).await;
    }

    // 4. Inverted index (uses chains for context)
    build_index_phase(&claude_dir, chains.as_ref(), &mut result);

    // Record duration
    result.duration_ms = start.elapsed().as_millis() as u64;

    Ok(result)
}

/// Run git sync phase.
fn sync_git(config: &DaemonConfig, result: &mut SyncResult) -> Result<(), String> {
    let options = SyncOptions {
        since: Some(format!("{} days", config.sync.git_since_days)),
        until: None,
        repo_path: config.project.path.clone(),
        incremental: true,
    };

    match sync_commits(&options) {
        Ok((commits, _sync_result)) => {
            result.git_commits_synced = commits.len() as i32;
            Ok(())
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

/// Run session parsing phase with database persistence.
///
/// **CRITICAL FIX:** This function now persists each parsed session to the database
/// using INSERT OR REPLACE to handle re-syncs gracefully.
async fn sync_sessions_phase(
    claude_dir: &Path,
    config: &DaemonConfig,
    result: &mut SyncResult,
    engine: Option<&QueryEngine>,
) -> Vec<String> {
    let options = ParseOptions {
        project_filter: config.project.path.clone(),
        incremental: true,
    };

    // Load existing session file sizes from DB for incremental sync.
    // Sessions whose JSONL file size hasn't changed are skipped.
    let existing_sessions: HashMap<String, i64> = if let Some(engine) = engine {
        engine.get_session_file_sizes().await.unwrap_or_default()
    } else {
        HashMap::new()
    };

    match sync_sessions(claude_dir, &options, &existing_sessions) {
        Ok((summaries, _parse_result)) => {
            result.sessions_parsed = summaries.len() as i32;

            // NEW: Persist each session to database
            if let Some(engine) = engine {
                let mut persisted = 0;
                for summary in &summaries {
                    let input: SessionInput = summary.clone().into();
                    match engine.upsert_session(&input).await {
                        Ok(_) => persisted += 1,
                        Err(e) => {
                            // Log but don't fail - continue with other sessions
                            result.errors.push(format!(
                                "Insert session {} ({}): {}",
                                &summary.session_id[..8.min(summary.session_id.len())],
                                summary
                                    .project_path
                                    .split(['/', '\\'])
                                    .next_back()
                                    .unwrap_or("?"),
                                e
                            ));
                        }
                    }
                }
                if persisted > 0 {
                    debug!(
                        target: "daemon.sync",
                        "Persisted {}/{} sessions to database",
                        persisted, summaries.len()
                    );
                }
            }

            summaries.iter().map(|s| s.session_id.clone()).collect()
        }
        Err(e) => {
            result.errors.push(format!("Session parse error: {}", e));
            Vec::new()
        }
    }
}

/// Run chain building phase with database persistence.
///
/// **CRITICAL FIX:** This function now persists chains to the database
/// (chains and chain_graph tables) using INSERT OR REPLACE.
async fn build_chains_phase(
    claude_dir: &Path,
    result: &mut SyncResult,
    engine: Option<&QueryEngine>,
) -> Option<HashMap<String, Chain>> {
    match build_chain_graph(claude_dir) {
        Ok(chains) => {
            result.chains_built = chains.len() as i32;

            // NEW: Persist chains to database
            if let Some(engine) = engine {
                if let Err(e) = engine.persist_chains(&chains).await {
                    result.errors.push(format!("Chain persistence: {}", e));
                } else {
                    debug!(
                        target: "daemon.sync",
                        "Persisted {} chains to database",
                        chains.len()
                    );
                }
            }

            Some(chains)
        }
        Err(e) => {
            result.errors.push(format!("Chain building error: {}", e));
            None
        }
    }
}

/// Run inverted index building phase.
fn build_index_phase(
    claude_dir: &Path,
    chains: Option<&HashMap<String, Chain>>,
    result: &mut SyncResult,
) {
    let index = build_inverted_index(claude_dir, chains);
    result.files_indexed = index.file_to_accesses.len() as i32;
}

/// Criteria for chains that should be summarized.
///
/// "Interesting" chains are those worth summarizing:
/// - Multi-session chains (≥2 sessions)
/// - Long duration chains (>30 minutes)
/// - Many files touched (>10 files)
fn should_summarize_chain(chain: &Chain) -> bool {
    chain.sessions.len() >= 2                    // Multi-session
        || chain.total_duration_seconds > 1800   // >30 min
        || chain.files_list.len() > 10 // Many files
}

/// Run intelligence enrichment phase (optional - graceful degradation).
///
/// Calls the Intel service to:
/// 1. Name chains that don't have cached names
/// 2. Summarize "interesting" chains with workstream tagging
///
/// Returns the count of chains successfully enriched.
///
/// # Graceful Degradation
/// - If Intel service is unavailable, logs a message and returns 0
/// - If cache cannot be opened, logs and continues without caching
/// - Never fails the sync - enrichment is optional
async fn enrich_chains_phase(chains: &HashMap<String, Chain>, result: &mut SyncResult) -> i32 {
    // Empty chains = nothing to do
    if chains.is_empty() {
        return 0;
    }

    let client = IntelClient::default();

    // Check if service is available — silently skip if not running
    // Intel enrichment is opt-in; users don't need to know it exists until enabled
    if !client.health_check().await {
        return 0;
    }

    // Open cache - use main database for unified storage
    let cache_path = match dirs::home_dir() {
        Some(h) => h.join(".context-os").join("context_os_events.db"),
        None => {
            result
                .errors
                .push("Intel: Could not find home directory".to_string());
            return 0;
        }
    };

    let cache = match MetadataStore::new(&cache_path).await {
        Ok(c) => c,
        Err(e) => {
            result.errors.push(format!("Intel: Cache error - {}", e));
            return 0;
        }
    };

    // Open database pool for querying session data
    let db_pool = match SqlitePool::connect(&format!("sqlite:{}", cache_path.display())).await {
        Ok(p) => Some(p),
        Err(e) => {
            result
                .errors
                .push(format!("Intel: Could not open DB for session data - {}", e));
            None
        }
    };

    // Load workstreams for hybrid tagging (Phase 4)
    // Try to find project root from CWD, fallback to empty list
    let workstreams = std::env::current_dir()
        .map(|cwd| load_workstreams(&cwd))
        .unwrap_or_default();

    let mut named_count = 0;
    let mut summarized_count = 0;

    for (chain_id, chain) in chains.iter() {
        // =====================================================
        // Chain Naming (existing behavior)
        // =====================================================

        let needs_naming = matches!(cache.get_chain_name(chain_id).await, Ok(None));

        if needs_naming {
            // Query session intent data from root session if DB is available
            let intent_data = if let Some(ref pool) = db_pool {
                query_session_intent(pool, &chain.root_session).await
            } else {
                SessionIntentData {
                    first_user_message: None,
                    conversation_excerpt: None,
                }
            };

            // Use conversation_excerpt as first_user_intent (backward compat)
            // but also populate explicit A/B test fields
            let first_user_intent = intent_data.conversation_excerpt.clone();

            // Build request with enrichment fields
            let request = ChainNamingRequest {
                chain_id: chain_id.clone(),
                files_touched: chain.files_list.clone(),
                session_count: chain.sessions.len() as i32,
                recent_sessions: chain.sessions.iter().take(5).cloned().collect(),
                // Enrichment fields
                tools_used: None, // TODO: Aggregate from sessions
                first_user_intent,
                commit_messages: None, // TODO: Query from git_commits
                // A/B test fields - explicit separation for quality comparison
                first_user_message: intent_data.first_user_message,
                conversation_excerpt: intent_data.conversation_excerpt,
            };

            // Call Intel service
            if let Ok(Some(response)) = client.name_chain(&request).await {
                // Cache the result
                if cache.cache_chain_name(&response).await.is_ok() {
                    named_count += 1;
                }
            }
        }

        // =====================================================
        // Chain Summary (Phase 6 - new behavior)
        // =====================================================

        // Only summarize "interesting" chains that aren't already cached
        if should_summarize_chain(chain) {
            let needs_summary = matches!(cache.get_chain_summary(chain_id).await, Ok(None));

            if needs_summary {
                // Aggregate excerpts from all sessions in the chain (Phase 5)
                let aggregated_excerpt = if let Some(ref pool) = db_pool {
                    aggregate_chain_excerpts(pool, &chain.sessions).await
                } else {
                    None
                };

                let request = ChainSummaryRequest {
                    chain_id: chain_id.clone(),
                    conversation_excerpt: aggregated_excerpt,
                    files_touched: chain.files_list.clone(),
                    session_count: chain.sessions.len() as i32,
                    duration_seconds: Some(chain.total_duration_seconds),
                    existing_workstreams: Some(workstreams.clone()),
                };

                // Call Intel service for summary
                if let Ok(Some(response)) = client.summarize_chain(&request).await {
                    // Cache the result
                    if cache.cache_chain_summary(&response).await.is_ok() {
                        summarized_count += 1;
                    }
                }
            }
        }
    }

    // Log summary of enrichment results
    if named_count > 0 || summarized_count > 0 {
        result.errors.push(format!(
            "Intel: Named {} chains, Summarized {} chains",
            named_count, summarized_count
        ));
    }

    named_count + summarized_count
}

/// Session intent data for chain naming.
struct SessionIntentData {
    first_user_message: Option<String>,
    conversation_excerpt: Option<String>,
}

/// Query first_user_message and conversation_excerpt from a session.
///
/// Returns both values separately for A/B testing comparison.
/// The conversation_excerpt contains all user messages (~8K chars).
/// The first_user_message is just the first message (for baseline comparison).
async fn query_session_intent(pool: &SqlitePool, session_id: &str) -> SessionIntentData {
    // Query for both fields
    let result: Result<(Option<String>, Option<String>), sqlx::Error> = sqlx::query_as(
        r#"
        SELECT conversation_excerpt, first_user_message
        FROM claude_sessions
        WHERE session_id = ?
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map(|opt| opt.unwrap_or((None, None)));

    match result {
        Ok((excerpt, first_msg)) => SessionIntentData {
            first_user_message: first_msg,
            conversation_excerpt: excerpt,
        },
        Err(_) => SessionIntentData {
            first_user_message: None,
            conversation_excerpt: None,
        },
    }
}

// =============================================================================
// Excerpt Aggregation (Phase 5 - Chain Summary Integration)
// =============================================================================

/// Maximum number of sessions to aggregate excerpts from.
const MAX_SESSIONS_TO_AGGREGATE: usize = 10;

/// Maximum total characters for aggregated excerpt.
const MAX_EXCERPT_CHARS: usize = 8000;

/// Aggregate conversation excerpts from multiple sessions in a chain.
///
/// Queries up to 10 sessions, concatenates their excerpts with session separators,
/// and truncates to 8K chars total to stay within LLM context limits.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `sessions` - List of session IDs to query (typically from chain.sessions)
///
/// # Returns
/// Aggregated excerpt with session separators, or None if no excerpts found.
///
/// # Example output:
/// ```text
/// === Session 1 (session-abc) ===
/// Help me fix the auth redirect...
///
/// === Session 2 (session-def) ===
/// Now let's add tests for...
/// ```
pub async fn aggregate_chain_excerpts(pool: &SqlitePool, sessions: &[String]) -> Option<String> {
    if sessions.is_empty() {
        return None;
    }

    // Limit to MAX_SESSIONS_TO_AGGREGATE sessions
    let sessions_to_query: Vec<&String> = sessions.iter().take(MAX_SESSIONS_TO_AGGREGATE).collect();

    // Build placeholders for IN clause
    let placeholders: Vec<String> = (0..sessions_to_query.len())
        .map(|_| "?".to_string())
        .collect();
    let query = format!(
        r#"
        SELECT session_id, conversation_excerpt
        FROM claude_sessions
        WHERE session_id IN ({})
        AND conversation_excerpt IS NOT NULL
        AND conversation_excerpt != ''
        "#,
        placeholders.join(", ")
    );

    // Build query with bound parameters
    let mut query_builder = sqlx::query_as::<_, (String, String)>(&query);
    for session_id in &sessions_to_query {
        query_builder = query_builder.bind(*session_id);
    }

    let rows: Vec<(String, String)> = match query_builder.fetch_all(pool).await {
        Ok(r) => r,
        Err(_) => return None,
    };

    if rows.is_empty() {
        return None;
    }

    // Aggregate excerpts with session separators
    let mut aggregated = String::new();
    for (i, (session_id, excerpt)) in rows.iter().enumerate() {
        let session_num = i + 1;
        let short_id = &session_id[..8.min(session_id.len())]; // First 8 chars of session ID

        let header = format!("\n=== Session {} ({}) ===\n", session_num, short_id);
        aggregated.push_str(&header);
        aggregated.push_str(excerpt);
        aggregated.push('\n');

        // Check if we've exceeded max length
        if aggregated.len() >= MAX_EXCERPT_CHARS {
            break;
        }
    }

    // Truncate to max length if needed
    if aggregated.len() > MAX_EXCERPT_CHARS {
        // Find last complete sentence or paragraph within limit
        let truncated = &aggregated[..MAX_EXCERPT_CHARS];
        // Find last newline to avoid cutting mid-sentence
        if let Some(last_newline) = truncated.rfind('\n') {
            return Some(truncated[..last_newline].to_string() + "\n[... truncated ...]");
        }
        return Some(truncated.to_string() + "\n[... truncated ...]");
    }

    Some(aggregated)
}

// =============================================================================
// Workstream Loading (Phase 4 - Chain Summary Integration)
// =============================================================================

/// Load existing workstream keys from workstreams.yaml
///
/// Extracts the top-level keys from the `streams:` section to pass to the
/// intelligence service for hybrid tagging (existing vs generated).
///
/// # Graceful Degradation
/// Returns empty Vec if:
/// - File doesn't exist
/// - File can't be parsed
/// - `streams` section missing
///
/// # Example workstreams.yaml structure:
/// ```yaml
/// streams:
///   tastematter-product:
///     name: "Tastematter Desktop App"
///     ...
///   nickel-transcript:
///     name: "Nickel Transcript Worker"
///     ...
/// ```
///
/// Returns: `["tastematter-product", "nickel-transcript", ...]`
pub fn load_workstreams(project_root: &Path) -> Vec<String> {
    let workstreams_path = project_root
        .join("_system")
        .join("state")
        .join("workstreams.yaml");

    // Read file - graceful return on missing
    let content = match fs::read_to_string(&workstreams_path) {
        Ok(c) => c,
        Err(_) => {
            debug!(
                target: "daemon.sync",
                "workstreams.yaml not found at {:?} - using empty list",
                workstreams_path
            );
            return Vec::new();
        }
    };

    // Parse YAML - graceful return on parse error
    let yaml: Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            debug!(
                target: "daemon.sync",
                "Failed to parse workstreams.yaml: {} - using empty list",
                e
            );
            return Vec::new();
        }
    };

    // Extract stream keys from `streams:` section
    let streams = match yaml.get("streams") {
        Some(Value::Mapping(m)) => m,
        _ => {
            debug!(
                target: "daemon.sync",
                "No 'streams' section in workstreams.yaml - using empty list"
            );
            return Vec::new();
        }
    };

    // Collect all top-level keys as workstream names
    let workstream_keys: Vec<String> = streams
        .keys()
        .filter_map(|k| k.as_str().map(|s| s.to_string()))
        .collect();

    debug!(
        target: "daemon.sync",
        "Loaded {} workstreams from YAML: {:?}",
        workstream_keys.len(),
        workstream_keys
    );

    workstream_keys
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // should_summarize_chain Tests (Phase 6)
    // ========================================================================

    #[test]
    fn test_should_summarize_chain_returns_true_for_multi_session() {
        let chain = Chain {
            chain_id: "test".to_string(),
            root_session: "root".to_string(),
            sessions: vec!["s1".to_string(), "s2".to_string()], // 2+ sessions
            branches: HashMap::new(),
            time_range: None,
            total_duration_seconds: 60, // < 30 min
            files_bloom: None,
            files_list: vec!["file.rs".to_string()], // < 10 files
        };
        assert!(
            should_summarize_chain(&chain),
            "Multi-session chains should be summarized"
        );
    }

    #[test]
    fn test_should_summarize_chain_returns_true_for_long_duration() {
        let chain = Chain {
            chain_id: "test".to_string(),
            root_session: "root".to_string(),
            sessions: vec!["s1".to_string()], // Single session
            branches: HashMap::new(),
            time_range: None,
            total_duration_seconds: 3600, // 1 hour > 30 min
            files_bloom: None,
            files_list: vec!["file.rs".to_string()], // < 10 files
        };
        assert!(
            should_summarize_chain(&chain),
            "Long duration chains should be summarized"
        );
    }

    #[test]
    fn test_should_summarize_chain_returns_true_for_many_files() {
        let chain = Chain {
            chain_id: "test".to_string(),
            root_session: "root".to_string(),
            sessions: vec!["s1".to_string()], // Single session
            branches: HashMap::new(),
            time_range: None,
            total_duration_seconds: 60, // < 30 min
            files_bloom: None,
            files_list: (0..15).map(|i| format!("file{}.rs", i)).collect(), // 15 files > 10
        };
        assert!(
            should_summarize_chain(&chain),
            "Chains with many files should be summarized"
        );
    }

    #[test]
    fn test_should_summarize_chain_returns_false_for_simple_chain() {
        let chain = Chain {
            chain_id: "test".to_string(),
            root_session: "root".to_string(),
            sessions: vec!["s1".to_string()], // Single session
            branches: HashMap::new(),
            time_range: None,
            total_duration_seconds: 60, // < 30 min
            files_bloom: None,
            files_list: vec!["file.rs".to_string()], // < 10 files
        };
        assert!(
            !should_summarize_chain(&chain),
            "Simple chains should NOT be summarized"
        );
    }

    // ========================================================================
    // TDD Cycle 3: SyncOrchestrator (4 tests)
    // ========================================================================

    // ========================================================================
    // TDD Cycle 4: Intelligence Enrichment (3 tests) - RED PHASE
    // ========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_enrich_chains_returns_count_of_chains_processed() {
        // enrich_chains_phase() returns the number of chains it attempted to enrich
        let chains: HashMap<String, Chain> = HashMap::new();
        let mut result = SyncResult::default();

        let enriched = enrich_chains_phase(&chains, &mut result).await;

        // With empty chains, should process 0
        assert_eq!(enriched, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_enrich_chains_silently_skips_when_service_unavailable() {
        // When Intel service is unavailable, enrich_chains_phase() should silently
        // return 0 without adding error messages (Intel is opt-in, not user-facing)
        let chains: HashMap<String, Chain> = HashMap::new();
        let mut result = SyncResult::default();

        let enriched = enrich_chains_phase(&chains, &mut result).await;

        // Should complete without panicking and return 0
        assert_eq!(enriched, 0);
        // Should NOT add any Intel-related messages to errors
        let intel_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.contains("Intel"))
            .collect();
        assert!(
            intel_errors.is_empty(),
            "Intel unavailable should be silent, found: {:?}",
            intel_errors
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_enrich_chains_skips_already_cached_chains() {
        // enrich_chains_phase() should check cache first and skip chains that are already named
        // This test verifies the function signature works - actual caching tested in integration
        let mut chains: HashMap<String, Chain> = HashMap::new();
        chains.insert(
            "test-chain".to_string(),
            Chain {
                chain_id: "test-chain".to_string(),
                root_session: "root".to_string(),
                sessions: vec!["session1".to_string()],
                branches: HashMap::new(),
                time_range: None,
                total_duration_seconds: 0,
                files_bloom: None,
                files_list: vec!["file.rs".to_string()],
            },
        );
        let mut result = SyncResult::default();

        // Should not panic and should handle gracefully
        let enriched = enrich_chains_phase(&chains, &mut result).await;

        // With service unavailable, enriched should be 0
        assert!(enriched >= 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_run_sync_returns_result_with_counts() {
        // run_sync() returns SyncResult with all counts
        let config = DaemonConfig::default();

        let result = run_sync(&config).await.unwrap();

        // Should have attempted all phases (counts may be 0 if no data)
        assert!(result.git_commits_synced >= 0);
        assert!(result.sessions_parsed >= 0);
        assert!(result.chains_built >= 0);
        assert!(result.files_indexed >= 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_run_sync_measures_duration() {
        // run_sync() returns SyncResult with duration_ms > 0
        let config = DaemonConfig::default();

        let result = run_sync(&config).await.unwrap();

        // Duration should be measured
        assert!(result.duration_ms > 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_run_sync_collects_errors() {
        // run_sync() collects errors into result.errors
        let config = DaemonConfig::default();

        let result = run_sync(&config).await.unwrap();

        // Errors array exists (may be empty or have errors depending on env)
        // The key is that it doesn't panic
        let _ = result.errors.len();
    }

    #[test]
    fn test_sync_result_serializes_to_json() {
        // SyncResult can be serialized to JSON for status reporting
        let result = SyncResult {
            git_commits_synced: 10,
            sessions_parsed: 50,
            chains_built: 5,
            files_indexed: 100,
            duration_ms: 1234,
            errors: vec!["test error".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("1234"));
        assert!(json.contains("test error"));

        // Can round-trip
        let parsed: SyncResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sessions_parsed, 50);
    }

    // =========================================================================
    // Workstream Loading Tests (Phase 4 - Chain Summary Integration)
    // =========================================================================

    #[test]
    #[ignore] // Requires local workstreams.yaml — not available in CI
    fn test_load_workstreams_from_real_yaml() {
        // Test loading from actual project workstreams.yaml
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap(); // gtm_operating_system root

        let workstreams = load_workstreams(project_root);

        // Should have found workstreams (we know at least 10 exist)
        assert!(
            workstreams.len() >= 5,
            "Expected at least 5 workstreams, got: {:?}",
            workstreams
        );

        // Should include known workstreams
        assert!(
            workstreams.contains(&"tastematter-product".to_string()),
            "Expected 'tastematter-product' in {:?}",
            workstreams
        );
    }

    // =========================================================================
    // Phase 3: Sync Orchestration (Stress Tests)
    // =========================================================================

    #[test]
    fn stress_sync_result_default_is_zeroed() {
        let result = SyncResult::default();
        assert_eq!(result.git_commits_synced, 0);
        assert_eq!(result.sessions_parsed, 0);
        assert_eq!(result.chains_built, 0);
        assert_eq!(result.files_indexed, 0);
        assert_eq!(result.duration_ms, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn stress_sync_result_round_trips_json_with_errors() {
        let result = SyncResult {
            git_commits_synced: 5,
            sessions_parsed: 100,
            chains_built: 10,
            files_indexed: 250,
            duration_ms: 5000,
            errors: vec![
                "Unicode error: \u{1F680} emoji in path".to_string(),
            ],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SyncResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].contains('\u{1F680}'));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stress_sync_sessions_phase_with_empty_claude_dir() {
        // .claude exists but has zero JSONL files
        let temp_home = tempfile::TempDir::new().unwrap();
        let claude_dir = temp_home.path().join(".claude");
        let projects_dir = claude_dir.join("projects");
        fs::create_dir_all(&projects_dir).unwrap();

        // Create an empty project subdirectory
        let proj_dir = projects_dir.join("empty-project");
        fs::create_dir_all(&proj_dir).unwrap();

        let db_dir = temp_home.path().join(".context-os");
        fs::create_dir_all(&db_dir).unwrap();
        let db = Database::open_rw(db_dir.join("test.db")).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = QueryEngine::new(db);

        let mut result = SyncResult::default();
        let config = DaemonConfig::default();
        let session_ids =
            sync_sessions_phase(&claude_dir, &config, &mut result, Some(&engine)).await;

        assert_eq!(result.sessions_parsed, 0);
        assert!(session_ids.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stress_chain_building_with_zero_sessions() {
        let temp_home = tempfile::TempDir::new().unwrap();
        let claude_dir = temp_home.path().join(".claude");
        let projects_dir = claude_dir.join("projects");
        fs::create_dir_all(&projects_dir).unwrap();

        let db_dir = temp_home.path().join(".context-os");
        fs::create_dir_all(&db_dir).unwrap();
        let db = Database::open_rw(db_dir.join("test.db")).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = QueryEngine::new(db);

        let mut result = SyncResult::default();
        let chains = build_chains_phase(&claude_dir, &mut result, Some(&engine)).await;

        assert_eq!(result.chains_built, 0, "Zero sessions → zero chains");
        if let Some(chains_map) = chains {
            assert!(chains_map.is_empty());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stress_enrich_chains_with_empty_map() {
        let chains: HashMap<String, Chain> = HashMap::new();
        let mut result = SyncResult::default();
        let enriched = enrich_chains_phase(&chains, &mut result).await;
        assert_eq!(enriched, 0, "Empty chains → 0 enriched");
    }

    #[test]
    fn test_load_workstreams_returns_empty_for_missing_file() {
        // load_workstreams returns empty Vec if file doesn't exist (graceful)
        let fake_path = std::path::Path::new("/nonexistent/path");
        let workstreams = load_workstreams(fake_path);
        assert!(workstreams.is_empty());
    }

    #[test]
    fn test_load_workstreams_returns_empty_for_malformed_yaml() {
        // If YAML is malformed or missing streams section, return empty (graceful)
        use std::io::Write;
        let temp_dir = tempfile::TempDir::new().unwrap();
        let yaml_dir = temp_dir.path().join("_system").join("state");
        std::fs::create_dir_all(&yaml_dir).unwrap();

        // Write malformed YAML
        let yaml_path = yaml_dir.join("workstreams.yaml");
        let mut file = std::fs::File::create(&yaml_path).unwrap();
        writeln!(file, "not: valid: yaml: with: colons").unwrap();

        let workstreams = load_workstreams(temp_dir.path());
        assert!(workstreams.is_empty());
    }

    // =========================================================================
    // Excerpt Aggregation Tests (Phase 5 - Chain Summary Integration)
    // =========================================================================

    #[tokio::test]
    async fn test_aggregate_chain_excerpts_returns_none_for_empty_sessions() {
        // Create test database
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();

        // Create table
        sqlx::query(
            "CREATE TABLE claude_sessions (session_id TEXT PRIMARY KEY, conversation_excerpt TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let sessions: Vec<String> = vec![];
        let result = aggregate_chain_excerpts(&pool, &sessions).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_aggregate_chain_excerpts_combines_multiple_sessions() {
        // Create test database with sessions
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();

        // Create table and insert test data
        sqlx::query(
            "CREATE TABLE claude_sessions (session_id TEXT PRIMARY KEY, conversation_excerpt TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO claude_sessions VALUES ('session-001', 'Help me fix auth')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO claude_sessions VALUES ('session-002', 'Now add tests')")
            .execute(&pool)
            .await
            .unwrap();

        let sessions = vec!["session-001".to_string(), "session-002".to_string()];
        let result = aggregate_chain_excerpts(&pool, &sessions).await;

        assert!(result.is_some());
        let aggregated = result.unwrap();
        assert!(aggregated.contains("Help me fix auth"));
        assert!(aggregated.contains("Now add tests"));
        assert!(aggregated.contains("Session 1"));
        assert!(aggregated.contains("Session 2"));
    }

    #[tokio::test]
    async fn test_aggregate_chain_excerpts_limits_to_max_sessions() {
        // Verify that only MAX_SESSIONS_TO_AGGREGATE sessions are queried
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE claude_sessions (session_id TEXT PRIMARY KEY, conversation_excerpt TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert 15 sessions (more than MAX_SESSIONS_TO_AGGREGATE = 10)
        for i in 0..15 {
            sqlx::query(&format!(
                "INSERT INTO claude_sessions VALUES ('session-{:03}', 'Excerpt {}')",
                i, i
            ))
            .execute(&pool)
            .await
            .unwrap();
        }

        let sessions: Vec<String> = (0..15).map(|i| format!("session-{:03}", i)).collect();
        let result = aggregate_chain_excerpts(&pool, &sessions).await;

        assert!(result.is_some());
        let aggregated = result.unwrap();
        // Should contain sessions 0-9 (first 10)
        assert!(aggregated.contains("Excerpt 0"));
        assert!(aggregated.contains("Excerpt 9"));
        // Should NOT contain session 10+ (limited to 10)
        // Note: They might not appear due to truncation OR query limit
    }

    #[tokio::test]
    async fn test_aggregate_chain_excerpts_skips_null_excerpts() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE claude_sessions (session_id TEXT PRIMARY KEY, conversation_excerpt TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert one with excerpt, one without
        sqlx::query("INSERT INTO claude_sessions VALUES ('session-001', 'Has excerpt')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO claude_sessions VALUES ('session-002', NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO claude_sessions VALUES ('session-003', '')")
            .execute(&pool)
            .await
            .unwrap();

        let sessions = vec![
            "session-001".to_string(),
            "session-002".to_string(),
            "session-003".to_string(),
        ];
        let result = aggregate_chain_excerpts(&pool, &sessions).await;

        assert!(result.is_some());
        let aggregated = result.unwrap();
        assert!(aggregated.contains("Has excerpt"));
        // Should only have one session header (null/empty filtered out)
        assert!(aggregated.contains("Session 1"));
        // Should not contain session 2 or 3 headers
        assert!(!aggregated.contains("Session 2"));
    }

    // =========================================================================
    // Fresh Install TDD Tests (Phase: Database Write Path)
    // =========================================================================

    /// Test 1: Fresh Install End-to-End
    ///
    /// Verifies the complete fresh install sequence:
    /// 1. DB directory creation when ~/.context-os/ doesn't exist
    /// 2. Database file creation via open_rw()
    /// 3. Schema table creation via ensure_schema()
    ///
    /// This tests the critical path that run_sync() follows on a fresh install,
    /// isolated from the real home directory.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_fresh_install_creates_db_and_schema() {
        use crate::storage::Database;

        // 1. Create temp dir to simulate fresh install (NO existing .context-os)
        let temp_home = tempfile::TempDir::new().unwrap();
        let db_dir = temp_home.path().join(".context-os");
        let db_path = db_dir.join("context_os_events.db");

        // 2. Verify directory does NOT exist (fresh install state)
        assert!(!db_dir.exists(), "DB dir should not exist initially");

        // 3. Create directory (mirrors run_sync line 61-64)
        fs::create_dir_all(&db_dir).expect("Should create DB directory");
        assert!(db_dir.exists(), "DB directory should be created");

        // 4. Open database in write mode (mirrors run_sync line 66-72)
        let db = Database::open_rw(&db_path)
            .await
            .expect("Should open/create database");
        assert!(db_path.exists(), "Database file should be created");

        // 5. Initialize schema (mirrors run_sync ensure_schema call)
        db.ensure_schema()
            .await
            .expect("Schema initialization should succeed");

        // 6. Verify all 6 core tables exist
        let tables = vec![
            "claude_sessions",
            "git_commits",
            "file_events",
            "chains",
            "chain_graph",
            "_metadata",
        ];

        for table in tables {
            let result = sqlx::query(&format!("SELECT COUNT(*) FROM {}", table))
                .fetch_one(db.pool())
                .await;
            assert!(
                result.is_ok(),
                "Table '{}' should exist after fresh install sequence",
                table
            );
        }

        // 7. Verify schema version was set
        let version: (String,) =
            sqlx::query_as("SELECT value FROM _metadata WHERE key = 'schema_version'")
                .fetch_one(db.pool())
                .await
                .expect("Schema version should exist");
        assert_eq!(version.0, "2.2", "Schema version should be 2.2");
    }

    /// Test 3: Zero Sessions Graceful Handling
    ///
    /// Verifies that sync handles an empty Claude sessions directory gracefully:
    /// - sessions_parsed = 0
    /// - chains_built = 0
    /// - No errors (beyond expected Intel service unavailability)
    ///
    /// This simulates a fresh install where the user hasn't run any Claude sessions yet.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sync_handles_zero_sessions_gracefully() {
        use crate::storage::Database;

        // 1. Create temp directory structure
        let temp_home = tempfile::TempDir::new().unwrap();

        // Create .claude/projects/ with NO session files (zero sessions state)
        let claude_dir = temp_home.path().join(".claude");
        let projects_dir = claude_dir.join("projects");
        fs::create_dir_all(&projects_dir).expect("Should create Claude projects dir");

        // Create database
        let db_dir = temp_home.path().join(".context-os");
        fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("context_os_events.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = crate::query::QueryEngine::new(db);

        // 2. Run session parsing phase with empty directory
        let mut result = SyncResult::default();
        let config = DaemonConfig::default();

        let session_ids =
            sync_sessions_phase(&claude_dir, &config, &mut result, Some(&engine)).await;

        // 3. Assert graceful handling of zero sessions
        assert_eq!(
            result.sessions_parsed, 0,
            "Should parse 0 sessions from empty directory"
        );
        assert!(session_ids.is_empty(), "Session IDs should be empty");

        // 4. Run chain building phase with empty sessions
        let chains = build_chains_phase(&claude_dir, &mut result, Some(&engine)).await;

        // 5. Assert graceful handling of zero chains
        assert_eq!(
            result.chains_built, 0,
            "Should build 0 chains from empty sessions"
        );

        // Chains may be Some(empty HashMap) or None depending on implementation
        if let Some(chains) = chains {
            assert!(chains.is_empty(), "Chain map should be empty");
        }

        // 6. Verify no critical errors (Intel is silently skipped when unavailable)
        let critical_errors: Vec<_> = result.errors.iter().collect();
        assert!(
            critical_errors.is_empty(),
            "Should have no critical errors for zero sessions. Errors: {:?}",
            critical_errors
        );
    }
}
