# PostHog Telemetry Integration Spec

**Mission:** Add anonymous usage telemetry to tastematter CLI to understand how alpha users interact with the tool.

**Estimated Lines:** ~130
**Gap Type:** PATTERN GAP (follows daemon/config.rs pattern)

---

## Architecture

```
main.rs (instrument commands)
    │
    ▼
src/telemetry/mod.rs
    ├── TelemetryConfig (from ~/.context-os/telemetry.yaml)
    ├── TelemetryClient (wraps posthog-rs)
    └── capture() → PostHog API (fire-and-forget)
```

---

## Type Contracts

### TelemetryConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryConfig {
    /// Enable/disable telemetry (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Anonymous installation UUID (auto-generated on first run)
    #[serde(default = "generate_uuid")]
    pub uuid: String,
}

fn default_enabled() -> bool { true }
fn generate_uuid() -> String { uuid::Uuid::new_v4().to_string() }
```

### TelemetryClient

```rust
pub struct TelemetryClient {
    config: TelemetryConfig,
    client: Option<posthog_rs::Client>,
}

impl TelemetryClient {
    /// Load config from ~/.context-os/telemetry.yaml or create defaults
    /// Check TASTEMATTER_NO_TELEMETRY env var for opt-out
    pub fn init() -> Self;

    /// Fire-and-forget event capture
    /// NEVER blocks CLI, NEVER panics, NEVER fails user operation
    pub fn capture(&self, event: &str, properties: serde_json::Value);
}
```

---

## Files to Create/Modify

### 1. Cargo.toml (ADD)

```toml
posthog-rs = "0.3.5"
```

### 2. src/telemetry/mod.rs (NEW ~80 lines)

```rust
//! Anonymous telemetry for tastematter CLI.
//!
//! Privacy-first design:
//! - Anonymous UUID per installation (no PII)
//! - Command names + timing only (no file paths, no content)
//! - Opt-out via TASTEMATTER_NO_TELEMETRY=1 or config file
//!
//! Events are fire-and-forget - never block or fail CLI operations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const POSTHOG_API_KEY: &str = "phc_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "generate_uuid")]
    pub uuid: String,
}

fn default_enabled() -> bool { true }
fn generate_uuid() -> String { uuid::Uuid::new_v4().to_string() }

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            uuid: generate_uuid(),
        }
    }
}

pub struct TelemetryClient {
    config: TelemetryConfig,
    client: Option<posthog_rs::Client>,
}

impl TelemetryClient {
    pub fn init() -> Self {
        // Check env var opt-out first
        if std::env::var("TASTEMATTER_NO_TELEMETRY").is_ok() {
            return Self {
                config: TelemetryConfig { enabled: false, ..Default::default() },
                client: None,
            };
        }

        // Load or create config
        let config = Self::load_or_create_config();

        // Initialize PostHog client if enabled
        let client = if config.enabled {
            posthog_rs::client(POSTHOG_API_KEY).ok()
        } else {
            None
        };

        Self { config, client }
    }

    fn load_or_create_config() -> TelemetryConfig {
        let path = Self::config_path();

        if path.exists() {
            // Load existing
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_yaml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            // Create new with fresh UUID
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

    /// Fire-and-forget event capture
    pub fn capture(&self, event: &str, properties: serde_json::Value) {
        if let Some(client) = &self.client {
            let mut ev = posthog_rs::Event::new(event, &self.config.uuid);
            if let serde_json::Value::Object(map) = properties {
                for (k, v) in map {
                    ev = ev.insert_prop(k, v).unwrap_or(ev);
                }
            }
            // Fire and forget - ignore errors
            let _ = client.capture(ev);
        }
    }
}
```

### 3. src/lib.rs (ADD export)

```rust
pub mod telemetry;
pub use telemetry::TelemetryClient;
```

### 4. src/main.rs (ADD instrumentation ~30 lines)

```rust
use tastematter::TelemetryClient;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize telemetry (fire-and-forget)
    let telemetry = TelemetryClient::init();
    let start = Instant::now();

    // ... existing command dispatch ...

    // At end of each command handler, capture event:
    // Example for query flex:
    telemetry.capture("command_executed", serde_json::json!({
        "command": "query_flex",
        "duration_ms": start.elapsed().as_millis(),
        "result_count": results.len(),
        "platform": std::env::consts::OS,
        "version": env!("CARGO_PKG_VERSION"),
    }));
}
```

---

## Events to Capture

| Command | Event Name | Properties |
|---------|------------|------------|
| `query flex` | `command_executed` | command, duration_ms, result_count, time_range |
| `query chains` | `command_executed` | command, duration_ms, chain_count |
| `query sessions` | `command_executed` | command, duration_ms, session_count |
| `daemon once` | `sync_completed` | duration_ms, sessions_parsed, commits_synced |
| `daemon start` | `daemon_started` | interval_minutes |
| `daemon install` | `daemon_installed` | platform |
| `parse-sessions` | `sessions_parsed` | count, duration_ms |
| `build-chains` | `chains_built` | count, duration_ms |

---

## Opt-Out Mechanisms

1. **Environment variable:** `TASTEMATTER_NO_TELEMETRY=1`
2. **Config file:** `~/.context-os/telemetry.yaml` with `enabled: false`

---

## Privacy Commitments

**What we collect:**
- Anonymous UUID (random, no PII linkage)
- Command names (e.g., "query flex")
- Timing (duration_ms)
- Aggregate counts (result_count, session_count)
- Platform (windows/macos/linux)
- Version (0.1.0)

**What we NEVER collect:**
- File paths or directory names
- Session content or conversations
- Query results or actual data
- Personal information
- IP addresses (anonymized by PostHog)

---

## Testing

```bash
# Test telemetry disabled via env
TASTEMATTER_NO_TELEMETRY=1 cargo run -- query flex --time 1d
# Verify: No network calls to PostHog

# Test telemetry enabled (check PostHog dashboard)
cargo run -- query flex --time 1d
# Verify: Event appears in PostHog within 30s

# Test config file opt-out
echo "enabled: false" > ~/.context-os/telemetry.yaml
cargo run -- query flex --time 1d
# Verify: No network calls
```

---

## Success Criteria

- [ ] Events appear in PostHog dashboard
- [ ] Opt-out mechanisms work (env var + config)
- [ ] CLI never blocks or fails due to telemetry
- [ ] No file paths or content in events
- [ ] UUID persists across sessions

---

## Prerequisites

1. **PostHog account** - Create project at posthog.com
2. **API key** - Get from PostHog project settings
3. **Replace placeholder** - Update `POSTHOG_API_KEY` constant

---

## Estimated Effort

| Component | Lines | Time |
|-----------|-------|------|
| Cargo.toml | 1 | 1 min |
| telemetry/mod.rs | ~80 | 30 min |
| lib.rs export | 2 | 1 min |
| main.rs instrumentation | ~30 | 20 min |
| Testing | - | 15 min |
| **Total** | **~113** | **~1 hour** |
