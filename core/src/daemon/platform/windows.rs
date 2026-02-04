//! Windows platform implementation.
//!
//! Uses Startup folder shortcut (no admin required) as primary method.
//! The shortcut launches the daemon on user login.

use super::{DaemonPlatform, InstallConfig, InstallResult, PlatformError, PlatformStatus};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Task name used in Windows Task Scheduler (fallback).
const TASK_NAME: &str = "tastematter-daemon";

/// Shortcut filename in Startup folder.
const SHORTCUT_NAME: &str = "tastematter-daemon.vbs";

/// Windows platform implementation.
pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> Self {
        Self
    }

    /// Get the Startup folder path for the current user.
    fn startup_folder() -> Result<PathBuf, PlatformError> {
        // Use APPDATA to find user's Startup folder
        // Typically: C:\Users\<user>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup
        let appdata = std::env::var("APPDATA").map_err(|_| {
            PlatformError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "APPDATA environment variable not set",
            ))
        })?;

        Ok(PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup"))
    }

    /// Get the path to the startup script.
    fn startup_script_path() -> Result<PathBuf, PlatformError> {
        Ok(Self::startup_folder()?.join(SHORTCUT_NAME))
    }

    /// Generate VBS script content that runs the daemon hidden (no console window).
    fn generate_startup_script(config: &InstallConfig) -> String {
        let binary = config
            .binary_path
            .display()
            .to_string()
            .replace("\\", "\\\\");
        format!(
            r#"' Tastematter daemon startup script
' Runs the daemon hidden (no console window)
Set WshShell = CreateObject("WScript.Shell")
WshShell.Run """{}"" daemon start --interval {}", 0, False
"#,
            binary, config.interval_minutes
        )
    }

    /// Build the command line for the daemon (used for status display).
    #[allow(dead_code)]
    fn build_daemon_command(config: &InstallConfig) -> String {
        let binary = config.binary_path.display();
        format!(
            "\"{}\" daemon start --interval {}",
            binary, config.interval_minutes
        )
    }

    /// Parse schtasks CSV output to extract status info.
    fn parse_query_output(output: &str) -> Option<TaskInfo> {
        // schtasks /query /fo csv /v returns CSV with headers
        // "TaskName","Next Run Time","Status","Last Run Time",...
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() < 2 {
            return None;
        }

        // Find our task in the output
        for line in lines.iter().skip(1) {
            if line.contains(TASK_NAME) {
                return Some(parse_csv_line(line));
            }
        }
        None
    }

    /// Check if installed via Task Scheduler (legacy/admin install).
    fn is_task_scheduler_installed() -> bool {
        Command::new("schtasks")
            .args(["/query", "/tn", TASK_NAME])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonPlatform for WindowsPlatform {
    fn install(&self, config: &InstallConfig) -> Result<InstallResult, PlatformError> {
        // Verify binary exists
        if !config.binary_path.exists() {
            return Err(PlatformError::BinaryNotFound(
                config.binary_path.display().to_string(),
            ));
        }

        let script_path = Self::startup_script_path()?;
        let startup_folder = Self::startup_folder()?;

        // Ensure Startup folder exists
        if !startup_folder.exists() {
            fs::create_dir_all(&startup_folder)?;
        }

        // Generate and write the VBS startup script
        let script_content = Self::generate_startup_script(config);
        fs::write(&script_path, script_content)?;

        Ok(InstallResult {
            success: true,
            message: "Daemon installed to Startup folder".to_string(),
            details: Some(format!("Script: {}", script_path.display())),
        })
    }

    fn uninstall(&self) -> Result<(), PlatformError> {
        let script_path = Self::startup_script_path()?;

        // Remove startup script if it exists
        if script_path.exists() {
            fs::remove_file(&script_path)?;
        }

        // Also try to remove Task Scheduler task if it exists (legacy cleanup)
        if Self::is_task_scheduler_installed() {
            let _ = Command::new("schtasks")
                .args(["/delete", "/tn", TASK_NAME, "/f"])
                .output();
        }

        // Check if anything was actually installed
        if !script_path.exists() && !Self::is_task_scheduler_installed() {
            // Nothing was installed, but that's fine - return success
        }

        Ok(())
    }

    fn is_installed(&self) -> Result<bool, PlatformError> {
        let script_path = Self::startup_script_path()?;

        // Check both Startup folder and Task Scheduler (legacy)
        Ok(script_path.exists() || Self::is_task_scheduler_installed())
    }

    fn status(&self) -> Result<PlatformStatus, PlatformError> {
        let mut status = PlatformStatus {
            platform_name: "Windows (Startup folder)".to_string(),
            ..Default::default()
        };

        let script_path = Self::startup_script_path()?;

        // Check Startup folder first (preferred method)
        if script_path.exists() {
            status.installed = true;
            status.message = format!("Startup script: {}", script_path.display());

            // Check if daemon process is currently running
            let output = Command::new("tasklist")
                .args(["/fi", "imagename eq tastematter.exe", "/fo", "csv", "/nh"])
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("tastematter.exe") {
                status.running = true;
                status.message = "Running (started from Startup folder)".to_string();
            } else {
                status.message = "Registered (will start on next login)".to_string();
            }

            return Ok(status);
        }

        // Fall back to checking Task Scheduler (legacy installs)
        if Self::is_task_scheduler_installed() {
            status.platform_name = "Windows (Task Scheduler)".to_string();
            status.installed = true;

            let output = Command::new("schtasks")
                .args(["/query", "/tn", TASK_NAME, "/fo", "csv", "/v"])
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(info) = Self::parse_query_output(&stdout) {
                status.running = info.status == "Running";
                status.message = format!("Status: {}", info.status);

                if !info.last_run.is_empty() && info.last_run != "N/A" {
                    status.last_run = parse_windows_datetime(&info.last_run);
                }
            }

            return Ok(status);
        }

        status.message = "Not installed".to_string();
        Ok(status)
    }
}

/// Parsed task information from schtasks output.
#[derive(Debug, Default)]
#[allow(dead_code)]
struct TaskInfo {
    status: String,
    last_run: String,
    next_run: String,
}

/// Parse a CSV line from schtasks output.
/// Format: "TaskName","Next Run Time","Status","Logon Mode","Last Run Time",...
fn parse_csv_line(line: &str) -> TaskInfo {
    let fields: Vec<&str> = line
        .split(',')
        .map(|s| s.trim_matches('"').trim())
        .collect();

    // CSV columns vary by Windows version, but common fields:
    // 0: TaskName
    // 1: Next Run Time
    // 2: Status
    // 4 or later: Last Run Time
    TaskInfo {
        next_run: fields.get(1).unwrap_or(&"").to_string(),
        status: fields.get(2).unwrap_or(&"Unknown").to_string(),
        last_run: fields.get(4).unwrap_or(&"").to_string(),
    }
}

/// Parse Windows datetime format (e.g., "1/23/2026 10:30:00 AM").
fn parse_windows_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Windows uses locale-specific formats, this is a best-effort parse
    // Common US format: M/D/YYYY H:MM:SS AM/PM
    use chrono::{NaiveDateTime, TimeZone};

    // Try common formats
    let formats = [
        "%m/%d/%Y %I:%M:%S %p", // US: 1/23/2026 10:30:00 AM
        "%d/%m/%Y %H:%M:%S",    // EU: 23/01/2026 10:30:00
        "%Y-%m-%d %H:%M:%S",    // ISO: 2026-01-23 10:30:00
    ];

    for fmt in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(chrono::Utc.from_utc_datetime(&dt));
        }
    }

    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_daemon_command() {
        let config = InstallConfig {
            binary_path: std::path::PathBuf::from("C:\\Users\\test\\.local\\bin\\tastematter.exe"),
            interval_minutes: 15,
            service_name: "tastematter".into(),
            description: "Test".into(),
        };
        let cmd = WindowsPlatform::build_daemon_command(&config);
        assert!(cmd.contains("tastematter.exe"));
        assert!(cmd.contains("daemon start"));
        assert!(cmd.contains("--interval 15"));
    }

    #[test]
    fn test_parse_csv_line() {
        let line = r#""tastematter-daemon","1/23/2026 10:00:00 AM","Ready","Interactive/Background","1/22/2026 9:00:00 AM""#;
        let info = parse_csv_line(line);
        assert_eq!(info.status, "Ready");
        assert!(info.next_run.contains("2026"));
        assert!(info.last_run.contains("2026"));
    }

    #[test]
    fn test_parse_windows_datetime_us_format() {
        use chrono::Datelike;
        let dt = parse_windows_datetime("1/23/2026 10:30:00 AM");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2026);
    }

    #[test]
    fn test_parse_windows_datetime_invalid() {
        let dt = parse_windows_datetime("N/A");
        assert!(dt.is_none());

        let dt = parse_windows_datetime("invalid");
        assert!(dt.is_none());
    }

    #[test]
    fn test_task_name_constant() {
        assert_eq!(TASK_NAME, "tastematter-daemon");
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod integration_tests {
    use super::*;

    /// Integration test: Full install/status/uninstall cycle.
    /// Run with: cargo test --release -- --ignored test_full_install_cycle
    #[test]
    #[ignore]
    fn test_full_install_cycle() {
        let platform = WindowsPlatform::new();

        // Clean up any existing installation
        let _ = platform.uninstall();

        // Verify not installed
        assert!(!platform.is_installed().unwrap());

        // Install with current binary
        let config = InstallConfig::default();
        let result = platform.install(&config);

        // If install failed due to binary not found, skip test
        if let Err(PlatformError::BinaryNotFound(_)) = &result {
            println!("Skipping test: binary not found in expected location");
            return;
        }

        let result = result.unwrap();
        assert!(result.success);

        // Check status
        let status = platform.status().unwrap();
        assert!(status.installed);

        // Uninstall
        platform.uninstall().unwrap();

        // Verify uninstalled
        assert!(!platform.is_installed().unwrap());
    }

    /// Integration test: Status when not installed.
    #[test]
    #[ignore]
    fn test_status_not_installed() {
        let platform = WindowsPlatform::new();

        // Ensure not installed
        let _ = platform.uninstall();

        let status = platform.status().unwrap();
        assert!(!status.installed);
        assert!(status.message.contains("Not installed"));
    }
}
