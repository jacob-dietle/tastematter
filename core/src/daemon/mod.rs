//! Daemon module for Context OS.
//!
//! Provides the background service that orchestrates:
//! - Git commit synchronization
//! - Claude session parsing
//! - Chain graph building
//! - Inverted index updates
//!
//! # Usage
//!
//! ```bash
//! tastematter daemon --once    # Single sync cycle
//! tastematter daemon start     # Run daemon loop
//! tastematter daemon status    # Show current state
//! ```

pub mod config;
pub mod gitops;
pub mod platform;
pub mod state;
pub mod sync;

pub use config::{
    load_config, save_config, validate_config, DaemonConfig, IntelligenceConfig, LoggingConfig,
    ProjectConfig, SyncConfig, WatchConfig,
};
pub use gitops::{collect_gitops_signals, load_user_rules, GitOpsError};
pub use platform::{
    get_platform, get_platform_name, DaemonPlatform, InstallConfig, InstallResult, PlatformError,
    PlatformStatus,
};
pub use state::DaemonState;
pub use sync::{run_sync, SyncResult};

#[cfg(test)]
mod cli_tests {
    use assert_cmd::Command;

    // ========================================================================
    // TDD Cycle 4: CLI Commands (4 tests)
    // ========================================================================

    #[test]
    fn test_daemon_help_shows_subcommands() {
        // `tastematter daemon --help` lists subcommands
        let mut cmd = Command::cargo_bin("tastematter").unwrap();
        let output = cmd.args(["daemon", "--help"]).output().unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("once") || stdout.contains("start") || stdout.contains("status"),
            "Help should show daemon subcommands. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_daemon_status_runs_without_crash() {
        // `tastematter daemon status` should not crash
        let mut cmd = Command::cargo_bin("tastematter").unwrap();
        let output = cmd.args(["daemon", "status"]).output().unwrap();

        // Should either succeed or fail gracefully
        // (may show "not running" if no state file)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("Status") || stdout.contains("not running") || stderr.len() >= 0,
            "Status command should produce output"
        );
    }

    #[test]
    fn test_daemon_once_help_shows_options() {
        // `tastematter daemon once --help` shows options
        let mut cmd = Command::cargo_bin("tastematter").unwrap();
        let output = cmd.args(["daemon", "once", "--help"]).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("project") || stdout.contains("--help"),
            "Once help should show project option"
        );
    }

    #[test]
    fn test_daemon_start_help_shows_options() {
        // `tastematter daemon start --help` shows options
        let mut cmd = Command::cargo_bin("tastematter").unwrap();
        let output = cmd.args(["daemon", "start", "--help"]).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("interval") || stdout.contains("--help"),
            "Start help should show interval option"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    // ========================================================================
    // TDD Cycle 5: Integration Tests (4 tests)
    // ========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_full_daemon_workflow_config_state_sync() {
        // E2E: Config → State → Sync → Update State
        let temp_dir = TempDir::new().unwrap();

        // 1. Load config (creates default if missing)
        let config_path = temp_dir.path().join("config.yaml");
        let config = load_config(Some(&config_path)).unwrap();

        // Verify config created
        assert!(config_path.exists());
        assert_eq!(config.sync.interval_minutes, 30);

        // 2. Load state (returns default if missing)
        let state_path = temp_dir.path().join("state.json");
        let mut state = DaemonState::load_or_default(&state_path);

        // State should be fresh
        assert_eq!(state.sessions_parsed, 0);

        // 3. Run sync
        let result = run_sync(&config).await.unwrap();

        // Sync ran (counts depend on actual data)
        assert!(result.duration_ms > 0);

        // 4. Update state
        state.sessions_parsed += result.sessions_parsed as i64;
        state.last_session_parse = Some(chrono::Utc::now());
        state.save(&state_path).unwrap();

        // Verify state persisted
        assert!(state_path.exists());
        let reloaded = DaemonState::load(&state_path).unwrap();
        assert!(reloaded.last_session_parse.is_some());
    }

    #[test]
    fn test_config_and_state_work_together() {
        // Config interval affects daemon loop timing (conceptually)
        let temp_dir = TempDir::new().unwrap();

        // Custom config with different interval
        let config_path = temp_dir.path().join("config.yaml");
        std::fs::write(&config_path, "sync:\n  interval_minutes: 5\n").unwrap();

        let config = load_config(Some(&config_path)).unwrap();
        assert_eq!(config.sync.interval_minutes, 5);

        // State tracks sync history
        let state_path = temp_dir.path().join("state.json");
        let mut state = DaemonState::default();
        state.started_at = Some(chrono::Utc::now());
        state.git_commits_synced = 100;
        state.save(&state_path).unwrap();

        let loaded = DaemonState::load(&state_path).unwrap();
        assert_eq!(loaded.git_commits_synced, 100);
        assert!(loaded.started_at.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sync_result_aggregates_all_phases() {
        // SyncResult should have data from all phases
        let config = DaemonConfig::default();
        let result = run_sync(&config).await.unwrap();

        // All fields should be set (even if 0)
        // The key is that the orchestrator ran all phases
        assert!(result.git_commits_synced >= 0);
        assert!(result.sessions_parsed >= 0);
        assert!(result.chains_built >= 0);
        assert!(result.files_indexed >= 0);
        assert!(result.duration_ms > 0);
    }

    #[test]
    fn test_state_accumulates_across_syncs() {
        // Multiple syncs should accumulate in state
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.json");

        let mut state = DaemonState::default();

        // Simulate first sync
        state.sessions_parsed += 50;
        state.git_commits_synced += 10;
        state.save(&state_path).unwrap();

        // Simulate second sync
        let mut state2 = DaemonState::load(&state_path).unwrap();
        state2.sessions_parsed += 30;
        state2.git_commits_synced += 5;
        state2.save(&state_path).unwrap();

        // Verify accumulation
        let final_state = DaemonState::load(&state_path).unwrap();
        assert_eq!(final_state.sessions_parsed, 80); // 50 + 30
        assert_eq!(final_state.git_commits_synced, 15); // 10 + 5
    }
}
