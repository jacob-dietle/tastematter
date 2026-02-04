//! Linux platform implementation using systemd user services.

use super::{DaemonPlatform, InstallConfig, InstallResult, PlatformError, PlatformStatus};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Service name for systemd.
const SERVICE_NAME: &str = "tastematter";

/// Linux platform implementation.
pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }

    /// Get the path to the systemd user service file.
    fn service_path() -> Result<PathBuf, PlatformError> {
        let home = dirs::home_dir().ok_or_else(|| {
            PlatformError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory",
            ))
        })?;
        Ok(home
            .join(".config")
            .join("systemd")
            .join("user")
            .join(format!("{}.service", SERVICE_NAME)))
    }

    /// Generate the systemd unit file content.
    fn generate_unit_file(config: &InstallConfig) -> String {
        let binary_path = config.binary_path.display();
        format!(
            r#"[Unit]
Description={}
After=network.target

[Service]
Type=simple
ExecStart={} daemon start --interval {}
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
"#,
            config.description, binary_path, config.interval_minutes
        )
    }

    /// Run systemctl with --user flag.
    fn systemctl(args: &[&str]) -> Result<std::process::Output, PlatformError> {
        let mut cmd_args = vec!["--user"];
        cmd_args.extend(args);

        Command::new("systemctl")
            .args(&cmd_args)
            .output()
            .map_err(|e| e.into())
    }
}

impl Default for LinuxPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonPlatform for LinuxPlatform {
    fn install(&self, config: &InstallConfig) -> Result<InstallResult, PlatformError> {
        // Verify binary exists
        if !config.binary_path.exists() {
            return Err(PlatformError::BinaryNotFound(
                config.binary_path.display().to_string(),
            ));
        }

        let service_path = Self::service_path()?;

        // Ensure systemd user directory exists
        if let Some(parent) = service_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Stop if running (ignore errors)
        let _ = Self::systemctl(&["stop", &format!("{}.service", SERVICE_NAME)]);

        // Write unit file
        let unit_content = Self::generate_unit_file(config);
        fs::write(&service_path, unit_content)?;

        // Reload systemd
        Self::systemctl(&["daemon-reload"])?;

        // Enable the service (starts on login)
        let output = Self::systemctl(&["enable", &format!("{}.service", SERVICE_NAME)])?;

        if output.status.success() {
            // Start it now too
            let _ = Self::systemctl(&["start", &format!("{}.service", SERVICE_NAME)]);

            Ok(InstallResult {
                success: true,
                message: format!("Service '{}' enabled", SERVICE_NAME),
                details: Some(format!("Unit file: {}", service_path.display())),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::CommandFailed {
                command: "systemctl --user enable".into(),
                message: stderr.trim().to_string(),
            })
        }
    }

    fn uninstall(&self) -> Result<(), PlatformError> {
        let service_path = Self::service_path()?;

        if !service_path.exists() {
            return Err(PlatformError::NotInstalled);
        }

        // Stop the service
        let _ = Self::systemctl(&["stop", &format!("{}.service", SERVICE_NAME)]);

        // Disable the service
        let _ = Self::systemctl(&["disable", &format!("{}.service", SERVICE_NAME)]);

        // Remove the unit file
        fs::remove_file(&service_path)?;

        // Reload systemd
        Self::systemctl(&["daemon-reload"])?;

        Ok(())
    }

    fn is_installed(&self) -> Result<bool, PlatformError> {
        let service_path = Self::service_path()?;
        Ok(service_path.exists())
    }

    fn status(&self) -> Result<PlatformStatus, PlatformError> {
        let mut status = PlatformStatus {
            platform_name: "Linux (systemd)".to_string(),
            ..Default::default()
        };

        let service_path = Self::service_path()?;
        if !service_path.exists() {
            status.message = "Not installed".to_string();
            return Ok(status);
        }

        status.installed = true;

        // Check service status
        let output = Self::systemctl(&["is-active", &format!("{}.service", SERVICE_NAME)])?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        status.running = stdout == "active";
        status.message = format!("Status: {}", stdout);

        // Get more detailed status if needed
        if status.running {
            // Try to get PID and runtime
            let show_output = Self::systemctl(&[
                "show",
                &format!("{}.service", SERVICE_NAME),
                "--property=MainPID,ActiveEnterTimestamp",
            ])?;
            let show_stdout = String::from_utf8_lossy(&show_output.stdout);

            for line in show_stdout.lines() {
                if line.starts_with("MainPID=") {
                    let pid = line.strip_prefix("MainPID=").unwrap_or("?");
                    if pid != "0" {
                        status.message = format!("Running (PID: {})", pid);
                    }
                }
            }
        }

        Ok(status)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unit_file_contains_description() {
        let config = InstallConfig::default();
        let unit = LinuxPlatform::generate_unit_file(&config);
        assert!(unit.contains(&config.description));
    }

    #[test]
    fn test_generate_unit_file_contains_binary() {
        let config = InstallConfig {
            binary_path: PathBuf::from("/home/user/.local/bin/tastematter"),
            ..Default::default()
        };
        let unit = LinuxPlatform::generate_unit_file(&config);
        assert!(unit.contains("/home/user/.local/bin/tastematter"));
    }

    #[test]
    fn test_generate_unit_file_contains_interval() {
        let config = InstallConfig {
            interval_minutes: 15,
            ..Default::default()
        };
        let unit = LinuxPlatform::generate_unit_file(&config);
        assert!(unit.contains("--interval 15"));
    }

    #[test]
    fn test_generate_unit_file_has_restart_policy() {
        let config = InstallConfig::default();
        let unit = LinuxPlatform::generate_unit_file(&config);
        assert!(unit.contains("Restart=on-failure"));
    }

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "tastematter");
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod integration_tests {
    use super::*;

    /// Integration test: Full install/status/uninstall cycle.
    /// Run with: cargo test --release -- --ignored test_full_install_cycle_linux
    #[test]
    #[ignore]
    fn test_full_install_cycle_linux() {
        let platform = LinuxPlatform::new();

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
}
