//! Trail configuration — loaded from the `trail` section of `~/.context-os/config.yaml`.

use serde::{Deserialize, Serialize};

/// Trail sync configuration. All fields optional — if `endpoint` is None,
/// trail push is silently skipped.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrailConfig {
    /// Global trail worker URL (e.g. "https://tastematter-trail.jacob-4c8.workers.dev")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Machine identifier (e.g. Tailscale hostname "laptop-2phko1ph")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_id: Option<String>,

    /// CF Access service token — client ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    /// CF Access service token — client secret
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

impl TrailConfig {
    /// Returns true if trail push is fully configured.
    pub fn is_configured(&self) -> bool {
        self.endpoint.is_some()
            && self.machine_id.is_some()
            && self.client_id.is_some()
            && self.client_secret.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_not_configured() {
        let config = TrailConfig::default();
        assert!(!config.is_configured());
    }

    #[test]
    fn test_partial_config_is_not_configured() {
        let config = TrailConfig {
            endpoint: Some("https://example.com".into()),
            machine_id: None,
            client_id: None,
            client_secret: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_full_config_is_configured() {
        let config = TrailConfig {
            endpoint: Some("https://trail.tastematter.dev".into()),
            machine_id: Some("laptop-test".into()),
            client_id: Some("id.access".into()),
            client_secret: Some("secret".into()),
        };
        assert!(config.is_configured());
    }

    #[test]
    fn test_deserialize_from_yaml() {
        let yaml = r#"
endpoint: https://trail.tastematter.dev
machine_id: laptop-2phko1ph
client_id: abc.access
client_secret: def123
"#;
        let config: TrailConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.is_configured());
        assert_eq!(config.endpoint.unwrap(), "https://trail.tastematter.dev");
    }

    #[test]
    fn test_deserialize_empty_yaml() {
        let yaml = "{}";
        let config: TrailConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.is_configured());
    }
}
