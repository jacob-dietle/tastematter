//! macOS platform implementation using launchd (launchctl).

use super::{DaemonPlatform, InstallConfig, InstallResult, PlatformError, PlatformStatus};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Service label for launchd.
const SERVICE_LABEL: &str = "dev.tastematter.daemon";

/// macOS platform implementation.
pub struct MacOsPlatform;

impl MacOsPlatform {
    pub fn new() -> Self {
        Self
    }

    /// Get the path to the LaunchAgents plist file.
    fn plist_path() -> Result<PathBuf, PlatformError> {
        let home = dirs::home_dir().ok_or_else(|| {
            PlatformError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory",
            ))
        })?;
        Ok(home
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{}.plist", SERVICE_LABEL)))
    }

    /// Generate the plist XML content.
    fn generate_plist(config: &InstallConfig) -> String {
        let binary_path = config.binary_path.display();
        let log_dir = dirs::home_dir()
            .map(|h| h.join(".context-os"))
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
        <string>start</string>
        <string>--interval</string>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>{}/daemon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{}/daemon.stderr.log</string>
</dict>
</plist>
"#,
            SERVICE_LABEL,
            binary_path,
            config.interval_minutes,
            log_dir.display(),
            log_dir.display()
        )
    }
}

impl Default for MacOsPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonPlatform for MacOsPlatform {
    fn install(&self, config: &InstallConfig) -> Result<InstallResult, PlatformError> {
        // Verify binary exists
        if !config.binary_path.exists() {
            return Err(PlatformError::BinaryNotFound(
                config.binary_path.display().to_string(),
            ));
        }

        let plist_path = Self::plist_path()?;

        // Ensure LaunchAgents directory exists
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Unload if already loaded (ignore errors)
        let _ = Command::new("launchctl")
            .args(["unload", &plist_path.display().to_string()])
            .output();

        // Write plist file
        let plist_content = Self::generate_plist(config);
        fs::write(&plist_path, plist_content)?;

        // Load the service
        let output = Command::new("launchctl")
            .args(["load", &plist_path.display().to_string()])
            .output()?;

        if output.status.success() {
            Ok(InstallResult {
                success: true,
                message: format!("Service '{}' installed", SERVICE_LABEL),
                details: Some(format!("Plist: {}", plist_path.display())),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::CommandFailed {
                command: "launchctl load".into(),
                message: stderr.trim().to_string(),
            })
        }
    }

    fn uninstall(&self) -> Result<(), PlatformError> {
        let plist_path = Self::plist_path()?;

        if !plist_path.exists() {
            return Err(PlatformError::NotInstalled);
        }

        // Unload the service
        let output = Command::new("launchctl")
            .args(["unload", &plist_path.display().to_string()])
            .output()?;

        // Remove the plist file even if unload failed
        let _ = fs::remove_file(&plist_path);

        if output.status.success() || !plist_path.exists() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(PlatformError::CommandFailed {
                command: "launchctl unload".into(),
                message: stderr.trim().to_string(),
            })
        }
    }

    fn is_installed(&self) -> Result<bool, PlatformError> {
        let plist_path = Self::plist_path()?;
        Ok(plist_path.exists())
    }

    fn status(&self) -> Result<PlatformStatus, PlatformError> {
        let mut status = PlatformStatus {
            platform_name: "macOS (launchd)".to_string(),
            ..Default::default()
        };

        let plist_path = Self::plist_path()?;
        if !plist_path.exists() {
            status.message = "Not installed".to_string();
            return Ok(status);
        }

        status.installed = true;

        // Check if service is loaded/running
        let output = Command::new("launchctl").args(["list"]).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains(SERVICE_LABEL) {
            // Parse the output to check running status
            // Format: PID Status Label
            for line in stdout.lines() {
                if line.contains(SERVICE_LABEL) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pid) = parts.first() {
                        if *pid != "-" {
                            status.running = true;
                            status.message = format!("Running (PID: {})", pid);
                        } else {
                            status.message = "Registered (not running)".to_string();
                        }
                    }
                    break;
                }
            }
        } else {
            status.message = "Installed but not loaded".to_string();
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
    fn test_generate_plist_contains_label() {
        let config = InstallConfig::default();
        let plist = MacOsPlatform::generate_plist(&config);
        assert!(plist.contains(SERVICE_LABEL));
        assert!(plist.contains("RunAtLoad"));
        assert!(plist.contains("true"));
    }

    #[test]
    fn test_generate_plist_contains_binary() {
        let config = InstallConfig {
            binary_path: PathBuf::from("/usr/local/bin/tastematter"),
            ..Default::default()
        };
        let plist = MacOsPlatform::generate_plist(&config);
        assert!(plist.contains("/usr/local/bin/tastematter"));
    }

    #[test]
    fn test_generate_plist_contains_interval() {
        let config = InstallConfig {
            interval_minutes: 15,
            ..Default::default()
        };
        let plist = MacOsPlatform::generate_plist(&config);
        assert!(plist.contains("<string>15</string>"));
    }

    #[test]
    fn test_service_label_constant() {
        assert_eq!(SERVICE_LABEL, "dev.tastematter.daemon");
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod integration_tests {
    use super::*;

    /// Integration test: Full install/status/uninstall cycle.
    /// Run with: cargo test --release -- --ignored test_full_install_cycle_macos
    #[test]
    #[ignore]
    fn test_full_install_cycle_macos() {
        let platform = MacOsPlatform::new();

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
