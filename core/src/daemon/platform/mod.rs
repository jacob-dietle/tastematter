//! Platform-specific daemon registration.
//!
//! Provides cross-platform support for installing the daemon to run on login:
//! - Windows: Task Scheduler (schtasks.exe)
//! - macOS: launchd (launchctl)
//! - Linux: systemd user services (systemctl --user)

use std::path::PathBuf;
use thiserror::Error;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsPlatform as CurrentPlatform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOsPlatform as CurrentPlatform;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxPlatform as CurrentPlatform;

// ============================================================================
// Types
// ============================================================================

/// Configuration for daemon installation.
#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Path to the tastematter binary.
    pub binary_path: PathBuf,

    /// Sync interval in minutes (default: 30).
    pub interval_minutes: u32,

    /// Service/task name (default: "tastematter").
    pub service_name: String,

    /// Service description.
    pub description: String,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            binary_path: get_default_binary_path(),
            interval_minutes: 30,
            service_name: "tastematter".to_string(),
            description: "Tastematter background sync daemon".to_string(),
        }
    }
}

/// Result of an installation attempt.
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Whether installation succeeded.
    pub success: bool,

    /// Human-readable message.
    pub message: String,

    /// Platform-specific details (e.g., task name, plist path).
    pub details: Option<String>,
}

/// Current platform status.
#[derive(Debug, Clone)]
pub struct PlatformStatus {
    /// Whether daemon is registered with the OS.
    pub installed: bool,

    /// Whether daemon is currently running.
    pub running: bool,

    /// Last run time (if available from OS).
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,

    /// Next scheduled run (if available from OS).
    pub next_run: Option<chrono::DateTime<chrono::Utc>>,

    /// Platform-specific status message.
    pub message: String,

    /// Platform name (e.g., "Windows (Task Scheduler)").
    pub platform_name: String,
}

impl Default for PlatformStatus {
    fn default() -> Self {
        Self {
            installed: false,
            running: false,
            last_run: None,
            next_run: None,
            message: "Not installed".to_string(),
            platform_name: get_platform_name(),
        }
    }
}

/// Platform-specific errors.
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Command failed: {command} - {message}")]
    CommandFailed { command: String, message: String },

    #[error("Not supported on this platform")]
    NotSupported,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Already installed")]
    AlreadyInstalled,

    #[error("Not installed")]
    NotInstalled,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to find binary: {0}")]
    BinaryNotFound(String),
}

// ============================================================================
// Trait
// ============================================================================

/// Platform-specific daemon registration trait.
pub trait DaemonPlatform {
    /// Install daemon to run on user login.
    fn install(&self, config: &InstallConfig) -> Result<InstallResult, PlatformError>;

    /// Remove daemon registration.
    fn uninstall(&self) -> Result<(), PlatformError>;

    /// Check if daemon is registered.
    fn is_installed(&self) -> Result<bool, PlatformError>;

    /// Get detailed registration status.
    fn status(&self) -> Result<PlatformStatus, PlatformError>;
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the default binary path based on platform.
pub fn get_default_binary_path() -> PathBuf {
    // Try to find the binary in common locations
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "windows")]
        let path = home.join(".local").join("bin").join("tastematter.exe");

        #[cfg(not(target_os = "windows"))]
        let path = home.join(".local").join("bin").join("tastematter");

        if path.exists() {
            return path;
        }
    }

    // Fall back to expecting it in PATH
    #[cfg(target_os = "windows")]
    return PathBuf::from("tastematter.exe");

    #[cfg(not(target_os = "windows"))]
    return PathBuf::from("tastematter");
}

/// Get human-readable platform name.
pub fn get_platform_name() -> String {
    #[cfg(target_os = "windows")]
    return "Windows (Task Scheduler)".to_string();

    #[cfg(target_os = "macos")]
    return "macOS (launchd)".to_string();

    #[cfg(target_os = "linux")]
    return "Linux (systemd)".to_string();

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "Unknown".to_string();
}

/// Get the platform implementation for the current OS.
pub fn get_platform() -> CurrentPlatform {
    CurrentPlatform::new()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // TDD Cycle 1: Core Types (4 tests)

    #[test]
    fn test_install_config_defaults() {
        let config = InstallConfig::default();
        assert_eq!(config.interval_minutes, 30);
        assert_eq!(config.service_name, "tastematter");
        assert!(!config.description.is_empty());
    }

    #[test]
    fn test_install_result_success() {
        let result = InstallResult {
            success: true,
            message: "Task created successfully".into(),
            details: Some("Trigger: At logon".into()),
        };
        assert!(result.success);
        assert!(result.details.is_some());
    }

    #[test]
    fn test_platform_status_default() {
        let status = PlatformStatus::default();
        assert!(!status.installed);
        assert!(!status.running);
        assert!(status.last_run.is_none());
        assert!(!status.platform_name.is_empty());
    }

    #[test]
    fn test_platform_error_display() {
        let err = PlatformError::CommandFailed {
            command: "schtasks /create".into(),
            message: "Access denied".into(),
        };
        let display = err.to_string();
        assert!(display.contains("schtasks"));
        assert!(display.contains("Access denied"));
    }

    #[test]
    fn test_get_platform_name_not_empty() {
        let name = get_platform_name();
        assert!(!name.is_empty());
        // Should contain platform-specific info
        #[cfg(target_os = "windows")]
        assert!(name.contains("Windows"));
        #[cfg(target_os = "macos")]
        assert!(name.contains("macOS"));
        #[cfg(target_os = "linux")]
        assert!(name.contains("Linux"));
    }
}
