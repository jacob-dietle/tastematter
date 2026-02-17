//! Anonymous telemetry for tastematter CLI.
//!
//! Privacy-first design following Claude Code, Vercel, and HashiCorp patterns:
//! - NEVER: file paths, query content, error messages, user identity
//! - ALWAYS: machine UUID, platform, version, command, duration, success
//! - WITH CARE: result counts, time range buckets, error codes
//!
//! Opt-out via TASTEMATTER_NO_TELEMETRY=1 or config file.
//! Debug via TASTEMATTER_TELEMETRY_DEBUG=1.
//!
//! Events are fire-and-forget - never block or fail CLI operations.

pub mod events;

pub use events::{
    CommandExecutedEvent, ErrorCode, ErrorOccurredEvent, FeatureUsedEvent, SyncCompletedEvent,
    TimeRangeBucket,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const POSTHOG_API_KEY: &str = "phc_viCzBS9wW3iaNF0jG0j9mR6IApVnTc62jDkfxPNGUIP";
const POSTHOG_CAPTURE_URL: &str = "https://us.i.posthog.com/capture/";

/// Telemetry configuration stored in ~/.context-os/telemetry.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable/disable telemetry (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Anonymous installation UUID (auto-generated on first run)
    #[serde(default = "generate_uuid")]
    pub uuid: String,
}

fn default_enabled() -> bool {
    true
}

fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            uuid: generate_uuid(),
        }
    }
}

/// Telemetry client - fire-and-forget async event capture via PostHog HTTP API.
///
/// Uses raw `reqwest::Client` POST instead of `posthog-rs` crate, which panics
/// inside tokio due to its internal `reqwest::blocking::Client`.
pub struct TelemetryClient {
    config: TelemetryConfig,
    client: reqwest::Client,
}

impl TelemetryClient {
    /// Initialize telemetry client
    /// Checks env var and config file for opt-out
    pub fn init() -> Self {
        // Check env var opt-out first (also skip in CI environments)
        if std::env::var("TASTEMATTER_NO_TELEMETRY").is_ok() || std::env::var("CI").is_ok() {
            return Self {
                config: TelemetryConfig {
                    enabled: false,
                    ..Default::default()
                },
                client: reqwest::Client::new(),
            };
        }

        // Load or create config
        let config = Self::load_or_create_config();

        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn load_or_create_config() -> TelemetryConfig {
        let path = Self::config_path();

        if path.exists() {
            // Load existing config
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_yaml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            // Create new config with fresh UUID
            let config = TelemetryConfig::default();
            let _ = Self::save_config(&config);
            config
        }
    }

    fn save_config(config: &TelemetryConfig) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(config).unwrap_or_default();
        std::fs::write(path, yaml)
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".context-os")
            .join("telemetry.yaml")
    }

    /// Fire-and-forget event capture via PostHog HTTP API
    /// NEVER blocks CLI, NEVER panics, NEVER fails user operation
    /// Set TASTEMATTER_TELEMETRY_DEBUG=1 for verbose logging
    pub async fn capture(&self, event: &str, properties: serde_json::Value) {
        let debug = std::env::var("TASTEMATTER_TELEMETRY_DEBUG").is_ok();

        if !self.config.enabled {
            if debug {
                eprintln!("[telemetry] Client not initialized (telemetry disabled)");
            }
            return;
        }

        // Build properties with standard fields
        let mut props = match properties {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        props.insert("$lib".into(), serde_json::json!("tastematter-cli"));
        props.insert("platform".into(), serde_json::json!(std::env::consts::OS));
        props.insert(
            "version".into(),
            serde_json::json!(env!("CARGO_PKG_VERSION")),
        );

        let body = serde_json::json!({
            "api_key": POSTHOG_API_KEY,
            "event": event,
            "distinct_id": self.config.uuid,
            "properties": props,
        });

        if debug {
            eprintln!(
                "[telemetry] {}: {}",
                event,
                serde_json::to_string(&props).unwrap_or_default()
            );
        }

        match self
            .client
            .post(POSTHOG_CAPTURE_URL)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if debug {
                    eprintln!("[telemetry] ✓ Event sent successfully");
                }
            }
            Ok(resp) => {
                if debug {
                    eprintln!("[telemetry] ✗ Event failed: HTTP {}", resp.status());
                }
            }
            Err(e) => {
                if debug {
                    eprintln!("[telemetry] ✗ Event failed: {:?}", e);
                }
            }
        }
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    // ========== Typed Event Helpers ==========

    /// Capture a command execution event
    pub async fn capture_command(&self, event: CommandExecutedEvent) {
        self.capture("command_executed", event.to_properties())
            .await;
    }

    /// Capture a sync completion event
    pub async fn capture_sync(&self, event: SyncCompletedEvent) {
        self.capture("sync_completed", event.to_properties()).await;
    }

    /// Capture an error event (codes only, never messages)
    pub async fn capture_error(&self, event: ErrorOccurredEvent) {
        self.capture("error_occurred", event.to_properties()).await;
    }

    /// Capture a feature usage event
    pub async fn capture_feature(&self, event: FeatureUsedEvent) {
        self.capture("feature_used", event.to_properties()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_uuid() {
        let config = TelemetryConfig::default();
        assert!(config.enabled);
        assert!(!config.uuid.is_empty());
        // UUID should be valid format
        assert!(uuid::Uuid::parse_str(&config.uuid).is_ok());
    }

    #[test]
    fn test_env_var_disables_telemetry() {
        std::env::set_var("TASTEMATTER_NO_TELEMETRY", "1");
        let client = TelemetryClient::init();
        assert!(!client.is_enabled());
        std::env::remove_var("TASTEMATTER_NO_TELEMETRY");
    }
}
