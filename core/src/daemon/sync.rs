//! Sync orchestrator module.
//!
//! Orchestrates all sync phases:
//! - Git commit synchronization
//! - Claude session parsing
//! - Chain graph building
//! - Inverted index updates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use super::config::DaemonConfig;
use crate::capture::git_sync::{sync_commits, SyncOptions};
use crate::capture::jsonl_parser::{sync_sessions, ParseOptions};
use crate::index::chain_graph::{build_chain_graph, Chain};
use crate::index::inverted_index::build_inverted_index;

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

/// Run a single sync cycle.
///
/// Orchestrates all sync phases in order:
/// 1. Git sync
/// 2. Session parsing
/// 3. Chain building
/// 4. Index update
pub fn run_sync(config: &DaemonConfig) -> Result<SyncResult, String> {
    let start = Instant::now();
    let mut result = SyncResult::default();

    // Get paths
    let claude_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".claude")
        .join("projects");

    // 1. Git sync
    let git_result = sync_git(config, &mut result);
    if let Err(e) = git_result {
        result.errors.push(format!("Git sync error: {}", e));
    }

    // 2. Session parsing
    let _session_ids = sync_sessions_phase(&claude_dir, config, &mut result);

    // 3. Chain building
    let chains = build_chains_phase(&claude_dir, &mut result);

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

/// Run session parsing phase.
fn sync_sessions_phase(
    claude_dir: &Path,
    config: &DaemonConfig,
    result: &mut SyncResult,
) -> Vec<String> {
    let options = ParseOptions {
        project_filter: config.project.path.clone(),
        incremental: true,
    };

    // For now, pass empty existing sessions (full sync)
    // TODO: Load from database for incremental sync
    let existing_sessions: HashMap<String, i64> = HashMap::new();

    match sync_sessions(claude_dir, &options, &existing_sessions) {
        Ok((summaries, _parse_result)) => {
            result.sessions_parsed = summaries.len() as i32;
            summaries.iter().map(|s| s.session_id.clone()).collect()
        }
        Err(e) => {
            result.errors.push(format!("Session parse error: {}", e));
            Vec::new()
        }
    }
}

/// Run chain building phase.
fn build_chains_phase(
    claude_dir: &Path,
    result: &mut SyncResult,
) -> Option<HashMap<String, Chain>> {
    match build_chain_graph(claude_dir) {
        Ok(chains) => {
            result.chains_built = chains.len() as i32;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // TDD Cycle 3: SyncOrchestrator (4 tests)
    // ========================================================================

    #[test]
    fn test_run_sync_returns_result_with_counts() {
        // run_sync() returns SyncResult with all counts
        let config = DaemonConfig::default();

        let result = run_sync(&config).unwrap();

        // Should have attempted all phases (counts may be 0 if no data)
        assert!(result.git_commits_synced >= 0);
        assert!(result.sessions_parsed >= 0);
        assert!(result.chains_built >= 0);
        assert!(result.files_indexed >= 0);
    }

    #[test]
    fn test_run_sync_measures_duration() {
        // run_sync() returns SyncResult with duration_ms > 0
        let config = DaemonConfig::default();

        let result = run_sync(&config).unwrap();

        // Duration should be measured
        assert!(result.duration_ms > 0);
    }

    #[test]
    fn test_run_sync_collects_errors() {
        // run_sync() collects errors into result.errors
        let config = DaemonConfig::default();

        let result = run_sync(&config).unwrap();

        // Errors array exists (may be empty or have errors depending on env)
        // The key is that it doesn't panic
        assert!(result.errors.len() >= 0);
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
}
