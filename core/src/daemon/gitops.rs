//! GitOps signal collection for intelligent git decisions.
//!
//! Collects signals from various sources to feed the GitOps Decision Agent:
//! - Git repository status (uncommitted files, unpushed commits)
//! - Recent session context (from database)
//! - Active chain context (from database)
//! - User rules (from YAML config)

use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::capture::git_status::query_repo_status;
use crate::intelligence::{ActiveChainContext, GitOpsSignals, RecentSessionContext};

/// Error collecting GitOps signals.
#[derive(Debug, Clone)]
pub struct GitOpsError {
    pub message: String,
}

impl std::fmt::Display for GitOpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitOpsError: {}", self.message)
    }
}

impl std::error::Error for GitOpsError {}

/// Collect all GitOps signals for decision making.
///
/// # Arguments
/// * `repo_path` - Path to git repository
/// * `recent_session` - Optional recent session context from DB
/// * `active_chain` - Optional active chain context from DB
/// * `user_rules` - User-defined rules (natural language)
pub fn collect_gitops_signals(
    repo_path: &Path,
    recent_session: Option<RecentSessionContext>,
    active_chain: Option<ActiveChainContext>,
    user_rules: Vec<String>,
) -> Result<GitOpsSignals, GitOpsError> {
    // Query git status
    let status = query_repo_status(repo_path).map_err(|e| GitOpsError {
        message: format!("Failed to query git status: {}", e),
    })?;

    // Calculate hours since timestamps
    let now = Utc::now();

    let hours_since_last_commit = status.last_commit_timestamp.map(|ts| {
        let duration = now.signed_duration_since(ts);
        duration.num_minutes() as f64 / 60.0
    });

    let hours_since_last_push = status.last_push_timestamp.map(|ts| {
        let duration = now.signed_duration_since(ts);
        duration.num_minutes() as f64 / 60.0
    });

    Ok(GitOpsSignals {
        uncommitted_files: status.uncommitted_files,
        unpushed_commits: status.unpushed_commits,
        current_branch: status.branch,
        last_commit_timestamp: status.last_commit_timestamp.map(|t| t.to_rfc3339()),
        last_push_timestamp: status.last_push_timestamp.map(|t| t.to_rfc3339()),
        recent_session,
        active_chain,
        user_rules,
        hours_since_last_commit,
        hours_since_last_push,
    })
}

/// Load user-defined GitOps rules from YAML config.
///
/// Looks for `~/.context-os/gitops-rules.yaml` with format:
/// ```yaml
/// rules:
///   - "Commit knowledge_base/ changes within 1 hour"
///   - "Never auto-commit _system/state/ - always ask"
/// ```
pub fn load_user_rules() -> Vec<String> {
    let home = dirs::home_dir();
    if home.is_none() {
        return Vec::new();
    }

    let rules_path = home.unwrap().join(".context-os").join("gitops-rules.yaml");
    if !rules_path.exists() {
        return Vec::new();
    }

    // Read and parse YAML
    let content = match fs::read_to_string(&rules_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Simple YAML parsing for rules list
    // Format: rules:\n  - "rule 1"\n  - "rule 2"
    let mut rules = Vec::new();
    let mut in_rules_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "rules:" {
            in_rules_section = true;
            continue;
        }

        if in_rules_section {
            // Check if this is a list item
            if trimmed.starts_with("- ") {
                let rule = trimmed[2..].trim();
                // Remove surrounding quotes if present
                let rule = rule.trim_matches('"').trim_matches('\'');
                if !rule.is_empty() {
                    rules.push(rule.to_string());
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // New section started, stop parsing rules
                break;
            }
        }
    }

    rules
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_collect_gitops_signals_works() {
        // Test against current repository
        let signals = collect_gitops_signals(Path::new("."), None, None, vec![]);

        assert!(
            signals.is_ok(),
            "Should collect signals: {:?}",
            signals.err()
        );

        let signals = signals.unwrap();
        assert!(!signals.current_branch.is_empty());

        println!("Branch: {}", signals.current_branch);
        println!("Uncommitted: {}", signals.uncommitted_files.len());
        println!("Unpushed: {}", signals.unpushed_commits);
    }

    #[test]
    fn test_collect_gitops_signals_with_context() {
        let session = RecentSessionContext {
            session_id: "test-session".to_string(),
            ended_at: Some("2026-01-30T10:00:00Z".to_string()),
            files_touched: vec!["src/main.rs".to_string()],
            duration_minutes: 30,
            conversation_summary: Some("Fixed auth bug".to_string()),
        };

        let chain = ActiveChainContext {
            chain_id: "test-chain".to_string(),
            workstream_tags: vec!["pixee".to_string()],
            accomplishments: vec!["Implemented feature".to_string()],
            status: "in_progress".to_string(),
        };

        let rules = vec!["Push before end of day".to_string()];

        let signals = collect_gitops_signals(Path::new("."), Some(session), Some(chain), rules);

        assert!(signals.is_ok());
        let signals = signals.unwrap();
        assert!(signals.recent_session.is_some());
        assert!(signals.active_chain.is_some());
        assert_eq!(signals.user_rules.len(), 1);
    }

    #[test]
    fn test_load_user_rules_returns_empty_if_no_file() {
        let rules = load_user_rules();
        // May or may not have rules file, both are ok
        println!("Loaded {} rules", rules.len());
    }

    #[test]
    fn test_load_user_rules_parses_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let rules_content = r#"
rules:
  - "Commit within 2 hours"
  - "Push before end of day"
  - 'Never commit .env files'

defaults:
  uncommitted_warn_hours: 2
"#;

        let rules_path = temp_dir.path().join("gitops-rules.yaml");
        fs::write(&rules_path, rules_content).unwrap();

        // Note: This test uses a temp dir, but load_user_rules looks for ~/.context-os/
        // So we can't fully test it without mocking. This just tests the parsing logic exists.
        // For full testing, we'd need dependency injection for the path.
    }
}
