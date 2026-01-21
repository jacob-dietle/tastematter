//! Daemon state persistence module.
//!
//! Tracks daemon state across restarts:
//! - Start time
//! - Last sync times
//! - Event counts

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Daemon state persisted to JSON file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonState {
    pub started_at: Option<DateTime<Utc>>,
    pub last_git_sync: Option<DateTime<Utc>>,
    pub last_session_parse: Option<DateTime<Utc>>,
    pub last_chain_build: Option<DateTime<Utc>>,
    #[serde(default)]
    pub file_events_captured: i64,
    #[serde(default)]
    pub git_commits_synced: i64,
    #[serde(default)]
    pub sessions_parsed: i64,
    #[serde(default)]
    pub chains_built: i64,
}

impl DaemonState {
    /// Save state to JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create state directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;

        std::fs::write(path, json).map_err(|e| format!("Failed to write state file: {}", e))?;

        Ok(())
    }

    /// Load state from JSON file, or return fresh state if not found.
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read state file: {}", e))?;

        // Handle empty or corrupted files
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        serde_json::from_str(&content).map_err(|e| {
            // Return default on parse error (corrupted file)
            log::warn!("Failed to parse state file, using default: {}", e);
            format!("Failed to parse state file: {}", e)
        })
    }

    /// Load state from JSON file, returning default on any error.
    /// This is the safer version that never fails.
    pub fn load_or_default(path: &Path) -> Self {
        Self::load(path).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ========================================================================
    // TDD Cycle 2: DaemonState (4 tests)
    // ========================================================================

    #[test]
    fn test_default_state_has_zero_counts() {
        let state = DaemonState::default();

        assert!(state.started_at.is_none());
        assert!(state.last_git_sync.is_none());
        assert_eq!(state.git_commits_synced, 0);
        assert_eq!(state.sessions_parsed, 0);
        assert_eq!(state.chains_built, 0);
    }

    #[test]
    fn test_state_save_creates_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("daemon.state.json");

        let mut state = DaemonState::default();
        state.started_at = Some(Utc::now());
        state.git_commits_synced = 42;

        state.save(&state_path).unwrap();

        assert!(state_path.exists());
        let content = std::fs::read_to_string(&state_path).unwrap();
        assert!(content.contains("42"));
    }

    #[test]
    fn test_state_load_restores_from_json() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("daemon.state.json");

        // Save state
        let mut original = DaemonState::default();
        original.sessions_parsed = 100;
        original.chains_built = 10;
        original.save(&state_path).unwrap();

        // Load state
        let loaded = DaemonState::load(&state_path).unwrap();

        assert_eq!(loaded.sessions_parsed, 100);
        assert_eq!(loaded.chains_built, 10);
    }

    #[test]
    fn test_state_load_returns_default_if_file_missing() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("nonexistent.json");

        let state = DaemonState::load(&state_path).unwrap();

        assert_eq!(state.git_commits_synced, 0);
        assert_eq!(state.sessions_parsed, 0);
    }
}
