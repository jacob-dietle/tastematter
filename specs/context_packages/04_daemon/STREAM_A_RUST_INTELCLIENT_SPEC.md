# Stream A: Rust IntelClient - Complete TDD Specification

**Mission:** Implement a Rust HTTP client module that calls the TypeScript intelligence service on localhost:3002, with SQLite caching and graceful degradation.

**Methodology:** TDD - Write RED tests first → GREEN implementation → REFACTOR

---

## Prerequisites

Before starting, verify:
- TypeScript intel service exists at `apps/tastematter/intel/` with `/api/intel/name-chain` endpoint
- Rust core exists at `apps/tastematter/core/` with existing patterns in `storage.rs`, `http.rs`, `error.rs`
- 48 tests currently passing in TypeScript intel service
- `reqwest` dependency already in Cargo.toml (line 35)

---

## Critical Reference Files

Read these FIRST before any implementation:

```
apps/tastematter/core/
├── Cargo.toml                    # reqwest already exists
├── src/
│   ├── lib.rs                    # Add `pub mod intelligence;` here
│   ├── storage.rs                # SQLite patterns to follow
│   ├── error.rs                  # CoreError patterns to follow
│   ├── types.rs                  # Type patterns to follow
│   └── http.rs                   # HTTP server patterns (reference)
```

---

## Files to Create

```
apps/tastematter/core/src/intelligence/
├── mod.rs          # Module exports
├── types.rs        # ChainMetadata, CommitAnalysis, etc.
├── client.rs       # IntelClient (reqwest → :3002)
├── cache.rs        # MetadataStore (SQLite cache)
└── cost.rs         # CostTracker (budget) [optional]

apps/tastematter/core/src/migrations/
└── 003_intelligence_metadata.sql  # 5 new tables
```

---

## TDD Execution Order

### Cycle 1: Types (RED → GREEN)

**File:** `core/src/intelligence/types.rs`

**RED Tests First:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_category_serializes_kebab_case() {
        let category = ChainCategory::BugFix;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"bug-fix\"");
    }

    #[test]
    fn chain_category_deserializes_kebab_case() {
        let category: ChainCategory = serde_json::from_str("\"bug-fix\"").unwrap();
        assert_eq!(category, ChainCategory::BugFix);
    }

    #[test]
    fn chain_naming_request_serializes() {
        let request = ChainNamingRequest {
            chain_id: "test-123".to_string(),
            files_touched: vec!["src/main.rs".to_string()],
            session_count: 5,
            recent_sessions: vec![],
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"chain_id\":\"test-123\""));
        assert!(json.contains("\"session_count\":5"));
    }

    #[test]
    fn chain_naming_response_deserializes() {
        let json = r#"{
            "chain_id": "abc",
            "generated_name": "Authentication refactor",
            "category": "refactor",
            "confidence": 0.85,
            "model_used": "claude-haiku-4-5-20251001"
        }"#;
        let response: ChainNamingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.chain_id, "abc");
        assert_eq!(response.category, ChainCategory::Refactor);
        assert!((response.confidence - 0.85).abs() < 0.001);
    }

    #[test]
    fn all_chain_categories_deserialize() {
        let categories = vec![
            ("\"bug-fix\"", ChainCategory::BugFix),
            ("\"feature\"", ChainCategory::Feature),
            ("\"refactor\"", ChainCategory::Refactor),
            ("\"research\"", ChainCategory::Research),
            ("\"cleanup\"", ChainCategory::Cleanup),
            ("\"documentation\"", ChainCategory::Documentation),
            ("\"testing\"", ChainCategory::Testing),
            ("\"unknown\"", ChainCategory::Unknown),
        ];
        for (json, expected) in categories {
            let category: ChainCategory = serde_json::from_str(json).unwrap();
            assert_eq!(category, expected);
        }
    }
}
```

**GREEN Implementation:**
```rust
// core/src/intelligence/types.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Work category for chain naming - matches TypeScript ChainCategorySchema
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChainCategory {
    BugFix,
    Feature,
    Refactor,
    Research,
    Cleanup,
    Documentation,
    Testing,
    Unknown,
}

/// Request to name a conversation chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNamingRequest {
    pub chain_id: String,
    pub files_touched: Vec<String>,
    pub session_count: i32,
    pub recent_sessions: Vec<String>,
}

/// Response from chain naming endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNamingResponse {
    pub chain_id: String,
    pub generated_name: String,
    pub category: ChainCategory,
    pub confidence: f32,
    pub model_used: String,
}

/// Cached chain metadata in SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    pub chain_id: String,
    pub generated_name: Option<String>,
    pub category: Option<ChainCategory>,
    pub confidence: Option<f32>,
    pub generated_at: Option<DateTime<Utc>>,
    pub model_used: Option<String>,
}
```

---

### Cycle 2: HTTP Client (RED → GREEN)

**File:** `core/src/intelligence/client.rs`

**RED Tests First:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_client_creates_with_base_url() {
        let client = IntelClient::new("http://localhost:3002");
        assert_eq!(client.base_url, "http://localhost:3002");
    }

    #[test]
    fn intel_client_default_uses_3002() {
        let client = IntelClient::default();
        assert_eq!(client.base_url, "http://localhost:3002");
    }

    #[tokio::test]
    async fn intel_client_returns_none_when_service_unavailable() {
        // Use port that's definitely not running
        let client = IntelClient::new("http://localhost:59999");
        let request = ChainNamingRequest {
            chain_id: "test".to_string(),
            files_touched: vec![],
            session_count: 1,
            recent_sessions: vec![],
        };
        let result = client.name_chain(&request).await;
        assert!(result.is_ok()); // Doesn't error
        assert!(result.unwrap().is_none()); // Returns None
    }

    #[tokio::test]
    async fn intel_client_has_timeout() {
        let client = IntelClient::new("http://localhost:59999");
        let start = std::time::Instant::now();
        let request = ChainNamingRequest {
            chain_id: "test".to_string(),
            files_touched: vec![],
            session_count: 1,
            recent_sessions: vec![],
        };
        let _ = client.name_chain(&request).await;
        // Should timeout within 15 seconds (default + buffer)
        assert!(start.elapsed().as_secs() < 15);
    }
}
```

**GREEN Implementation:**
```rust
// core/src/intelligence/client.rs
use reqwest::Client;
use log::{info, warn};
use uuid::Uuid;
use std::time::{Duration, Instant};
use crate::intelligence::types::{ChainNamingRequest, ChainNamingResponse};
use crate::error::CoreError;

pub struct IntelClient {
    pub base_url: String,
    http_client: Client,
}

impl Default for IntelClient {
    fn default() -> Self {
        Self::new("http://localhost:3002")
    }
}

impl IntelClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Call chain naming endpoint with graceful degradation
    pub async fn name_chain(
        &self,
        request: &ChainNamingRequest,
    ) -> Result<Option<ChainNamingResponse>, CoreError> {
        let correlation_id = Uuid::new_v4().to_string();
        let start = Instant::now();
        let url = format!("{}/api/intel/name-chain", self.base_url);

        // OBSERVABILITY: Log request start
        info!(
            target: "intelligence",
            correlation_id = %correlation_id,
            operation = "name_chain",
            chain_id = %request.chain_id,
            files_count = request.files_touched.len(),
            "Starting intelligence request"
        );

        let result = self.http_client
            .post(&url)
            .header("X-Correlation-ID", &correlation_id)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await;

        let duration_ms = start.elapsed().as_millis();

        match result {
            Ok(response) if response.status().is_success() => {
                match response.json::<ChainNamingResponse>().await {
                    Ok(data) => {
                        info!(
                            target: "intelligence",
                            correlation_id = %correlation_id,
                            duration_ms = duration_ms,
                            success = true,
                            generated_name = %data.generated_name,
                            "Intelligence request completed"
                        );
                        Ok(Some(data))
                    }
                    Err(e) => {
                        warn!(
                            target: "intelligence",
                            correlation_id = %correlation_id,
                            duration_ms = duration_ms,
                            error = %e,
                            "Failed to parse intelligence response"
                        );
                        Ok(None)
                    }
                }
            }
            Ok(response) => {
                warn!(
                    target: "intelligence",
                    correlation_id = %correlation_id,
                    duration_ms = duration_ms,
                    status = response.status().as_u16(),
                    "Intelligence service returned error status"
                );
                Ok(None) // Graceful degradation
            }
            Err(e) => {
                warn!(
                    target: "intelligence",
                    correlation_id = %correlation_id,
                    duration_ms = duration_ms,
                    error = %e,
                    "Intelligence service unavailable - degrading gracefully"
                );
                Ok(None) // Graceful degradation - don't error, just return None
            }
        }
    }
}
```

---

### Cycle 3: SQLite Cache (RED → GREEN)

**File:** `core/src/intelligence/cache.rs`

**RED Tests First:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_store() -> (MetadataStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = MetadataStore::new(&db_path).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn cache_stores_and_retrieves_chain_metadata() {
        let (store, _tmp) = create_test_store().await;

        let response = ChainNamingResponse {
            chain_id: "test-123".to_string(),
            generated_name: "Authentication refactor".to_string(),
            category: ChainCategory::Refactor,
            confidence: 0.85,
            model_used: "claude-haiku-4-5-20251001".to_string(),
        };

        store.cache_chain_name(&response).await.unwrap();

        let cached = store.get_chain_name("test-123").await.unwrap();
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.generated_name, Some("Authentication refactor".to_string()));
        assert_eq!(cached.category, Some(ChainCategory::Refactor));
    }

    #[tokio::test]
    async fn cache_returns_none_for_missing_chain() {
        let (store, _tmp) = create_test_store().await;
        let cached = store.get_chain_name("nonexistent").await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn cache_overwrites_existing_entry() {
        let (store, _tmp) = create_test_store().await;

        let response1 = ChainNamingResponse {
            chain_id: "test-123".to_string(),
            generated_name: "First name".to_string(),
            category: ChainCategory::Feature,
            confidence: 0.7,
            model_used: "haiku".to_string(),
        };
        store.cache_chain_name(&response1).await.unwrap();

        let response2 = ChainNamingResponse {
            chain_id: "test-123".to_string(),
            generated_name: "Updated name".to_string(),
            category: ChainCategory::Refactor,
            confidence: 0.9,
            model_used: "haiku".to_string(),
        };
        store.cache_chain_name(&response2).await.unwrap();

        let cached = store.get_chain_name("test-123").await.unwrap().unwrap();
        assert_eq!(cached.generated_name, Some("Updated name".to_string()));
    }
}
```

**GREEN Implementation:**
```rust
// core/src/intelligence/cache.rs
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use log::{info, debug};
use std::path::Path;
use chrono::Utc;
use crate::intelligence::types::{ChainMetadata, ChainNamingResponse, ChainCategory};
use crate::error::CoreError;

pub struct MetadataStore {
    pool: Pool<Sqlite>,
}

impl MetadataStore {
    pub async fn new(db_path: &Path) -> Result<Self, CoreError> {
        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Run migration
        sqlx::query(MIGRATION_SQL)
            .execute(&pool)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        info!(
            target: "intelligence.cache",
            db_path = %db_path.display(),
            "MetadataStore initialized"
        );

        Ok(Self { pool })
    }

    pub async fn cache_chain_name(&self, response: &ChainNamingResponse) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();
        let category = serde_json::to_string(&response.category)
            .unwrap_or_else(|_| "\"unknown\"".to_string())
            .trim_matches('"')
            .to_string();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO chain_metadata
            (chain_id, generated_name, category, confidence, generated_at, model_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&response.chain_id)
        .bind(&response.generated_name)
        .bind(&category)
        .bind(response.confidence)
        .bind(&now)
        .bind(&response.model_used)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        debug!(
            target: "intelligence.cache",
            chain_id = %response.chain_id,
            "Cached chain metadata"
        );

        Ok(())
    }

    pub async fn get_chain_name(&self, chain_id: &str) -> Result<Option<ChainMetadata>, CoreError> {
        let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<f32>, Option<String>, Option<String>)>(
            r#"
            SELECT chain_id, generated_name, category, confidence, generated_at, model_used
            FROM chain_metadata
            WHERE chain_id = ?
            "#
        )
        .bind(chain_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        match row {
            Some((chain_id, name, cat_str, conf, gen_at, model)) => {
                let category = cat_str.and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok());
                let generated_at = gen_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));

                debug!(
                    target: "intelligence.cache",
                    chain_id = %chain_id,
                    cache_hit = true,
                    "Cache hit"
                );

                Ok(Some(ChainMetadata {
                    chain_id,
                    generated_name: name,
                    category,
                    confidence: conf,
                    generated_at,
                    model_used: model,
                }))
            }
            None => {
                debug!(
                    target: "intelligence.cache",
                    chain_id = %chain_id,
                    cache_hit = false,
                    "Cache miss"
                );
                Ok(None)
            }
        }
    }
}

const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    category TEXT,
    confidence REAL,
    generated_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS commit_analysis (
    commit_hash TEXT PRIMARY KEY,
    is_agent_commit INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    risk_level TEXT,
    review_focus TEXT,
    related_files TEXT,
    analyzed_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_summaries (
    session_id TEXT PRIMARY KEY,
    summary TEXT,
    key_files TEXT,
    focus_area TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS insights_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    insight_type TEXT,
    title TEXT,
    description TEXT,
    evidence TEXT,
    action TEXT,
    generated_at TEXT,
    expires_at TEXT,
    model_used TEXT
);

CREATE TABLE IF NOT EXISTS intelligence_costs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL,
    model TEXT NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TEXT DEFAULT (datetime('now'))
);
"#;
```

---

### Cycle 4: Module Integration (RED → GREEN)

**File:** `core/src/intelligence/mod.rs`

```rust
// core/src/intelligence/mod.rs
mod types;
mod client;
mod cache;

pub use types::*;
pub use client::IntelClient;
pub use cache::MetadataStore;
```

**Add to `core/src/lib.rs`:**
```rust
pub mod intelligence;
```

---

### Cycle 5: Error Types

**Add to `core/src/error.rs`:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    // ... existing variants ...

    #[error("Intelligence service unavailable")]
    IntelServiceUnavailable,

    #[error("Intelligence service error: {0}")]
    IntelServiceError(String),
}
```

---

## Observability Requirements

### Log Event Schema

All intelligence operations MUST log structured events:

```rust
// Request start
info!(
    target: "intelligence",
    correlation_id = %id,
    operation = "name_chain",
    chain_id = %request.chain_id,
    files_count = request.files_touched.len(),
    "Starting intelligence request"
);

// Request complete (success)
info!(
    target: "intelligence",
    correlation_id = %id,
    operation = "name_chain",
    duration_ms = duration,
    success = true,
    generated_name = %response.generated_name,
    "Intelligence request completed"
);

// Request failed/degraded
warn!(
    target: "intelligence",
    correlation_id = %id,
    operation = "name_chain",
    duration_ms = duration,
    error = %e,
    "Intelligence service unavailable - degrading gracefully"
);
```

---

## Completion Criteria

- [ ] `cargo build --release` succeeds with intelligence module
- [ ] `cargo test --lib intelligence` passes (12+ tests)
- [ ] All 4 TDD cycles complete (types, client, cache, mod)
- [ ] Graceful degradation works (returns None, doesn't error)
- [ ] Structured logging implemented for all operations
- [ ] Correlation IDs passed to TypeScript service

---

## Verification Commands

```bash
cd apps/tastematter/core

# Build
cargo build --release

# Run all intelligence tests
cargo test --lib intelligence -- --nocapture

# Verify logging (run with RUST_LOG=debug)
RUST_LOG=intelligence=debug cargo test --lib intelligence -- --nocapture

# Check SQLite schema was created (after running tests)
sqlite3 /tmp/test.db ".schema chain_metadata"
```

---

## Do NOT

- Do NOT modify TypeScript code (Stream B handles that)
- Do NOT add CLI commands yet (Phase 5)
- Do NOT implement remaining agent types (commit analysis, etc.) - just types for now
- Do NOT implement cost tracking (optional, skip for MVP)
