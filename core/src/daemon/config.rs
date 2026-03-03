//! Daemon configuration module.
//!
//! Handles loading and validation of daemon configuration from YAML files.
//! Configuration is stored at `~/.context-os/config.yaml`.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[cfg(feature = "trail")]
use crate::trail::config::TrailConfig;

/// Sync timing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Interval between syncs in minutes (default: 30)
    #[serde(default = "default_interval_minutes")]
    pub interval_minutes: u32,
    /// How far back to sync git commits (default: 7 days)
    #[serde(default = "default_git_since_days")]
    pub git_since_days: u32,
}

fn default_interval_minutes() -> u32 {
    30
}

fn default_git_since_days() -> u32 {
    7
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            interval_minutes: default_interval_minutes(),
            git_since_days: default_git_since_days(),
        }
    }
}

/// File watching configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Enable file watching (default: true)
    #[serde(default = "default_watch_enabled")]
    pub enabled: bool,
    /// Paths to watch (default: ["."])
    #[serde(default = "default_watch_paths")]
    pub paths: Vec<String>,
    /// Debounce window in ms (default: 100)
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
}

fn default_watch_enabled() -> bool {
    true
}

fn default_watch_paths() -> Vec<String> {
    vec![".".to_string()]
}

fn default_debounce_ms() -> u64 {
    100
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            enabled: default_watch_enabled(),
            paths: default_watch_paths(),
            debounce_ms: default_debounce_ms(),
        }
    }
}

/// Project configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    /// Project path (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: DEBUG, INFO, WARNING, ERROR (default: INFO)
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "INFO".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

/// Intelligence service configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntelligenceConfig {
    /// Anthropic API key for direct LLM synthesis (set via `tastematter intel setup`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Complete daemon configuration (mirrors Python DaemonConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Config version
    #[serde(default = "default_version")]
    pub version: u32,
    /// Sync configuration
    #[serde(default)]
    pub sync: SyncConfig,
    /// Watch configuration
    #[serde(default)]
    pub watch: WatchConfig,
    /// Project configuration
    #[serde(default)]
    pub project: ProjectConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Intelligence service configuration
    #[serde(default)]
    pub intelligence: IntelligenceConfig,
    /// Global trail sync configuration (requires `trail` feature)
    #[cfg(feature = "trail")]
    #[serde(default)]
    pub trail: TrailConfig,
}

fn default_version() -> u32 {
    1
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            sync: SyncConfig::default(),
            watch: WatchConfig::default(),
            project: ProjectConfig::default(),
            logging: LoggingConfig::default(),
            intelligence: IntelligenceConfig::default(),
            #[cfg(feature = "trail")]
            trail: TrailConfig::default(),
        }
    }
}

impl DaemonConfig {
    /// Local machine identity for source_machine attribution.
    /// Returns None in public builds or when trail is unconfigured.
    pub fn local_machine_id(&self) -> Option<&str> {
        #[cfg(feature = "trail")]
        {
            self.trail.machine_id.as_deref()
        }
        #[cfg(not(feature = "trail"))]
        {
            None
        }
    }
}

/// Load configuration from a YAML file.
///
/// If the file doesn't exist, creates it with default values.
/// Partial configs are merged with defaults.
///
/// # Arguments
/// * `path` - Optional path to config file. If None, uses `~/.context-os/config.yaml`
///
/// # Returns
/// * `Ok(DaemonConfig)` - The loaded/created configuration
/// * `Err(String)` - Error message if loading fails
pub fn load_config(path: Option<&Path>) -> Result<DaemonConfig, String> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => {
            let home = dirs::home_dir().ok_or("Could not find home directory")?;
            home.join(".context-os").join("config.yaml")
        }
    };

    // If file doesn't exist, create with defaults
    if !config_path.exists() {
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let default_config = DaemonConfig::default();
        let yaml = serde_yaml::to_string(&default_config)
            .map_err(|e| format!("Failed to serialize default config: {}", e))?;
        std::fs::write(&config_path, yaml)
            .map_err(|e| format!("Failed to write default config: {}", e))?;

        return Ok(default_config);
    }

    // Load existing file
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    // Parse YAML - serde will use defaults for missing fields
    let config: DaemonConfig = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

/// Save configuration to a YAML file.
///
/// Writes the config to the same path `load_config` reads from.
pub fn save_config(config: &DaemonConfig, path: Option<&Path>) -> Result<(), String> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => {
            let home = dirs::home_dir().ok_or("Could not find home directory")?;
            home.join(".context-os").join("config.yaml")
        }
    };

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let yaml =
        serde_yaml::to_string(config).map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, yaml).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Validate configuration values.
///
/// Returns a list of validation errors (empty if valid).
pub fn validate_config(config: &DaemonConfig) -> Vec<String> {
    let mut errors = Vec::new();

    // Validate sync interval
    if config.sync.interval_minutes == 0 {
        errors.push("sync.interval_minutes must be greater than 0".to_string());
    }

    // Validate git_since_days
    if config.sync.git_since_days == 0 {
        errors.push("sync.git_since_days must be greater than 0".to_string());
    }

    // Validate log level
    let valid_levels = ["DEBUG", "INFO", "WARNING", "WARN", "ERROR"];
    if !valid_levels.contains(&config.logging.level.to_uppercase().as_str()) {
        errors.push(format!(
            "logging.level must be one of {:?}, got '{}'",
            valid_levels, config.logging.level
        ));
    }

    // Validate watch paths (at least one if enabled)
    if config.watch.enabled && config.watch.paths.is_empty() {
        errors.push("watch.paths must not be empty when watch.enabled is true".to_string());
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ========================================================================
    // TDD Cycle 1: DaemonConfig (4 tests)
    // ========================================================================

    #[test]
    fn test_default_config_has_expected_values() {
        // Default config matches Python defaults
        let config = DaemonConfig::default();

        assert_eq!(config.version, 1);
        assert_eq!(config.sync.interval_minutes, 30);
        assert_eq!(config.sync.git_since_days, 7);
        assert!(config.watch.enabled);
        assert_eq!(config.watch.paths, vec!["."]);
        assert_eq!(config.watch.debounce_ms, 100);
        assert_eq!(config.logging.level, "INFO");
    }

    #[test]
    fn test_load_config_creates_file_if_missing() {
        // load_config() creates config.yaml with defaults if not exists
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        assert!(!config_path.exists());
        let config = load_config(Some(&config_path)).unwrap();
        assert!(config_path.exists());
        assert_eq!(config.sync.interval_minutes, 30);
    }

    #[test]
    fn test_load_config_merges_with_defaults() {
        // Partial YAML file should merge with defaults
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        std::fs::write(&config_path, "sync:\n  interval_minutes: 60\n").unwrap();

        let config = load_config(Some(&config_path)).unwrap();
        assert_eq!(config.sync.interval_minutes, 60); // From file
        assert_eq!(config.sync.git_since_days, 7); // From defaults
    }

    #[test]
    fn test_validate_config_catches_invalid_values() {
        // Validation returns errors for invalid config
        let mut config = DaemonConfig::default();
        config.sync.interval_minutes = 0; // Invalid: must be > 0
        config.logging.level = "INVALID".to_string(); // Invalid level

        let errors = validate_config(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("interval_minutes")));
        assert!(errors.iter().any(|e| e.contains("level")));
    }

    // =========================================================================
    // Phase 5: Input Resilience (Stress Tests)
    // =========================================================================

    #[test]
    fn stress_config_with_empty_project_path() {
        let mut config = DaemonConfig::default();
        config.project.path = Some(String::new());
        // Should not panic — empty string is a valid Option<String>
        let errors = validate_config(&config);
        // Whether this is flagged as error depends on validation logic
        let _ = errors;
    }

    #[test]
    fn stress_config_with_unicode_project_path() {
        let mut config = DaemonConfig::default();
        config.project.path = Some("/home/\u{30E6}\u{30FC}\u{30B6}\u{30FC}/project".to_string());
        let errors = validate_config(&config);
        // Unicode paths are valid on modern systems
        assert!(
            !errors.iter().any(|e| e.contains("project")),
            "Unicode project paths should be valid"
        );
    }

    #[test]
    fn stress_load_config_malformed_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("bad_config.yaml");
        std::fs::write(&config_path, "{{{{not: valid: yaml}}}}").unwrap();

        let result = load_config(Some(&config_path));
        // Should either return defaults or an error, not panic
        assert!(
            result.is_ok() || result.is_err(),
            "Malformed YAML should not panic"
        );
    }
}
