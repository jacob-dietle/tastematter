//! SQLite cache for intelligence metadata
//!
//! Provides local caching of intelligence service responses to reduce API calls
//! and enable offline operation.

use chrono::Utc;
use log::{debug, info};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::path::Path;

use crate::error::CoreError;
use crate::intelligence::types::{
    ChainCategory, ChainMetadata, ChainNamingResponse, ChainSummaryResponse, WorkStatus,
    WorkstreamTag,
};

/// SQLite-based cache for intelligence metadata
pub struct MetadataStore {
    pool: Pool<Sqlite>,
}

impl MetadataStore {
    /// Create new metadata store, initializing the database schema
    pub async fn new(db_path: &Path) -> Result<Self, CoreError> {
        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .map_err(|e| CoreError::Config(format!("Database connection failed: {}", e)))?;

        // Run migration
        sqlx::query(MIGRATION_SQL)
            .execute(&pool)
            .await
            .map_err(|e| CoreError::Config(format!("Migration failed: {}", e)))?;

        info!(
            target: "intelligence.cache",
            "MetadataStore initialized: db_path={}",
            db_path.display()
        );

        Ok(Self { pool })
    }

    /// Cache a chain naming response
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
            "#,
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
        .map_err(|e| CoreError::Config(format!("Cache write failed: {}", e)))?;

        debug!(
            target: "intelligence.cache",
            "Cached chain metadata: chain_id={}",
            response.chain_id
        );

        Ok(())
    }

    /// Get cached chain name by chain ID
    pub async fn get_chain_name(&self, chain_id: &str) -> Result<Option<ChainMetadata>, CoreError> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<String>,
                Option<f32>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT chain_id, generated_name, category, confidence, generated_at, model_used
            FROM chain_metadata
            WHERE chain_id = ?
            "#,
        )
        .bind(chain_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::Config(format!("Cache read failed: {}", e)))?;

        match row {
            Some((chain_id, name, cat_str, conf, gen_at, model)) => {
                let category: Option<ChainCategory> =
                    cat_str.and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok());
                let generated_at = gen_at.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });

                debug!(
                    target: "intelligence.cache",
                    "Cache hit: chain_id={}",
                    chain_id
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
                    "Cache miss: chain_id={}",
                    chain_id
                );
                Ok(None)
            }
        }
    }

    /// Get all cached chain names
    pub async fn get_all_chain_names(&self) -> Result<Vec<ChainMetadata>, CoreError> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<String>,
                Option<f32>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT chain_id, generated_name, category, confidence, generated_at, model_used
            FROM chain_metadata
            ORDER BY generated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::Config(format!("Cache read failed: {}", e)))?;

        let metadata: Vec<ChainMetadata> = rows
            .into_iter()
            .map(|(chain_id, name, cat_str, conf, gen_at, model)| {
                let category: Option<ChainCategory> =
                    cat_str.and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok());
                let generated_at = gen_at.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });

                ChainMetadata {
                    chain_id,
                    generated_name: name,
                    category,
                    confidence: conf,
                    generated_at,
                    model_used: model,
                }
            })
            .collect();

        Ok(metadata)
    }

    /// Delete cached chain name
    pub async fn delete_chain_name(&self, chain_id: &str) -> Result<bool, CoreError> {
        let result = sqlx::query("DELETE FROM chain_metadata WHERE chain_id = ?")
            .bind(chain_id)
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::Config(format!("Cache delete failed: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear all cached chain names
    pub async fn clear_chain_names(&self) -> Result<u64, CoreError> {
        let result = sqlx::query("DELETE FROM chain_metadata")
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::Config(format!("Cache clear failed: {}", e)))?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // Chain Summary Cache Methods (Phase 3)
    // =========================================================================

    /// Cache a chain summary response
    pub async fn cache_chain_summary(
        &self,
        response: &ChainSummaryResponse,
    ) -> Result<(), CoreError> {
        let now = Utc::now().to_rfc3339();

        // Serialize arrays to JSON for SQLite TEXT columns
        let accomplishments_json =
            serde_json::to_string(&response.accomplishments).unwrap_or_else(|_| "[]".to_string());
        let key_files_json =
            serde_json::to_string(&response.key_files).unwrap_or_else(|_| "[]".to_string());
        let workstream_tags_json =
            serde_json::to_string(&response.workstream_tags).unwrap_or_else(|_| "[]".to_string());
        let status_str = serde_json::to_string(&response.status)
            .unwrap_or_else(|_| "\"in_progress\"".to_string())
            .trim_matches('"')
            .to_string();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO chain_summaries
            (chain_id, summary, accomplishments, status, key_files, workstream_tags, model_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&response.chain_id)
        .bind(&response.summary)
        .bind(&accomplishments_json)
        .bind(&status_str)
        .bind(&key_files_json)
        .bind(&workstream_tags_json)
        .bind(&response.model_used)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::Config(format!("Cache chain summary failed: {}", e)))?;

        debug!(
            target: "intelligence.cache",
            "Cached chain summary: chain_id={}, status={:?}",
            response.chain_id,
            response.status
        );

        Ok(())
    }

    /// Get cached chain summary by chain ID
    pub async fn get_chain_summary(
        &self,
        chain_id: &str,
    ) -> Result<Option<ChainSummaryResponse>, CoreError> {
        let row = sqlx::query_as::<
            _,
            (
                String,                  // chain_id
                Option<String>,          // summary
                Option<String>,          // accomplishments (JSON)
                Option<String>,          // status
                Option<String>,          // key_files (JSON)
                Option<String>,          // workstream_tags (JSON)
                Option<String>,          // model_used
            ),
        >(
            r#"
            SELECT chain_id, summary, accomplishments, status, key_files, workstream_tags, model_used
            FROM chain_summaries
            WHERE chain_id = ?
            "#,
        )
        .bind(chain_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::Config(format!("Cache read failed: {}", e)))?;

        match row {
            Some((chain_id, summary, acc_json, status_str, files_json, tags_json, model)) => {
                // Parse JSON fields
                let accomplishments: Vec<String> = acc_json
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let key_files: Vec<String> = files_json
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let workstream_tags: Vec<WorkstreamTag> = tags_json
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let status: WorkStatus = status_str
                    .as_ref()
                    .and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok())
                    .unwrap_or(WorkStatus::InProgress);

                debug!(
                    target: "intelligence.cache",
                    "Cache hit for chain summary: chain_id={}",
                    chain_id
                );

                Ok(Some(ChainSummaryResponse {
                    chain_id,
                    summary: summary.unwrap_or_default(),
                    accomplishments,
                    status,
                    key_files,
                    workstream_tags,
                    model_used: model.unwrap_or_default(),
                }))
            }
            None => {
                debug!(
                    target: "intelligence.cache",
                    "Cache miss for chain summary: chain_id={}",
                    chain_id
                );
                Ok(None)
            }
        }
    }

    /// Get all cached chain summaries
    pub async fn get_all_chain_summaries(&self) -> Result<Vec<ChainSummaryResponse>, CoreError> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT chain_id, summary, accomplishments, status, key_files, workstream_tags, model_used
            FROM chain_summaries
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::Config(format!("Cache read failed: {}", e)))?;

        let summaries: Vec<ChainSummaryResponse> = rows
            .into_iter()
            .map(
                |(chain_id, summary, acc_json, status_str, files_json, tags_json, model)| {
                    let accomplishments: Vec<String> = acc_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();
                    let key_files: Vec<String> = files_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();
                    let workstream_tags: Vec<WorkstreamTag> = tags_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();
                    let status: WorkStatus = status_str
                        .as_ref()
                        .and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok())
                        .unwrap_or(WorkStatus::InProgress);

                    ChainSummaryResponse {
                        chain_id,
                        summary: summary.unwrap_or_default(),
                        accomplishments,
                        status,
                        key_files,
                        workstream_tags,
                        model_used: model.unwrap_or_default(),
                    }
                },
            )
            .collect();

        Ok(summaries)
    }
}

/// SQL migration for intelligence-only cache tables.
/// Note: chain_metadata and chain_summaries are owned by storage.rs ensure_schema().
const MIGRATION_SQL: &str = r#"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::WorkstreamTagSource;
    use tempfile::TempDir;

    async fn create_test_store() -> (MetadataStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create the prerequisite tables that storage.rs ensure_schema() normally creates.
        // In production, ensure_schema() runs before MetadataStore::new().
        let db = crate::storage::Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

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
        assert_eq!(
            cached.generated_name,
            Some("Authentication refactor".to_string())
        );
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

    #[tokio::test]
    async fn cache_get_all_returns_multiple_entries() {
        let (store, _tmp) = create_test_store().await;

        for i in 0..3 {
            let response = ChainNamingResponse {
                chain_id: format!("chain-{}", i),
                generated_name: format!("Name {}", i),
                category: ChainCategory::Feature,
                confidence: 0.8,
                model_used: "haiku".to_string(),
            };
            store.cache_chain_name(&response).await.unwrap();
        }

        let all = store.get_all_chain_names().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn cache_delete_removes_entry() {
        let (store, _tmp) = create_test_store().await;

        let response = ChainNamingResponse {
            chain_id: "delete-me".to_string(),
            generated_name: "To be deleted".to_string(),
            category: ChainCategory::Cleanup,
            confidence: 0.9,
            model_used: "haiku".to_string(),
        };
        store.cache_chain_name(&response).await.unwrap();

        let deleted = store.delete_chain_name("delete-me").await.unwrap();
        assert!(deleted);

        let cached = store.get_chain_name("delete-me").await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn cache_clear_removes_all_entries() {
        let (store, _tmp) = create_test_store().await;

        for i in 0..5 {
            let response = ChainNamingResponse {
                chain_id: format!("chain-{}", i),
                generated_name: format!("Name {}", i),
                category: ChainCategory::Feature,
                confidence: 0.8,
                model_used: "haiku".to_string(),
            };
            store.cache_chain_name(&response).await.unwrap();
        }

        let cleared = store.clear_chain_names().await.unwrap();
        assert_eq!(cleared, 5);

        let all = store.get_all_chain_names().await.unwrap();
        assert!(all.is_empty());
    }

    // =========================================================================
    // Chain Summary Cache Tests (Phase 3)
    // =========================================================================

    #[tokio::test]
    async fn cache_stores_and_retrieves_chain_summary() {
        let (store, _tmp) = create_test_store().await;

        let response = ChainSummaryResponse {
            chain_id: "test-summary-123".to_string(),
            summary: "Fixed OAuth redirect loop issues".to_string(),
            accomplishments: vec![
                "Identified bug".to_string(),
                "Applied fix".to_string(),
                "Added tests".to_string(),
            ],
            status: WorkStatus::Complete,
            key_files: vec!["src/auth.rs".to_string(), "src/oauth.rs".to_string()],
            workstream_tags: vec![
                WorkstreamTag {
                    tag: "pixee".to_string(),
                    source: WorkstreamTagSource::Existing,
                },
                WorkstreamTag {
                    tag: "oauth-auth".to_string(),
                    source: WorkstreamTagSource::Generated,
                },
            ],
            model_used: "claude-haiku-4-5-20251001".to_string(),
        };

        store.cache_chain_summary(&response).await.unwrap();

        let cached = store.get_chain_summary("test-summary-123").await.unwrap();
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.summary, "Fixed OAuth redirect loop issues");
        assert_eq!(cached.status, WorkStatus::Complete);
        assert_eq!(cached.accomplishments.len(), 3);
        assert_eq!(cached.key_files.len(), 2);
        assert_eq!(cached.workstream_tags.len(), 2);
        assert_eq!(cached.workstream_tags[0].tag, "pixee");
        assert_eq!(
            cached.workstream_tags[0].source,
            WorkstreamTagSource::Existing
        );
    }

    #[tokio::test]
    async fn cache_summary_returns_none_for_missing() {
        let (store, _tmp) = create_test_store().await;
        let cached = store.get_chain_summary("nonexistent").await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn cache_summary_overwrites_existing() {
        let (store, _tmp) = create_test_store().await;

        let response1 = ChainSummaryResponse {
            chain_id: "update-test".to_string(),
            summary: "First summary".to_string(),
            accomplishments: vec!["Task 1".to_string()],
            status: WorkStatus::InProgress,
            key_files: vec![],
            workstream_tags: vec![],
            model_used: "haiku".to_string(),
        };
        store.cache_chain_summary(&response1).await.unwrap();

        let response2 = ChainSummaryResponse {
            chain_id: "update-test".to_string(),
            summary: "Updated summary".to_string(),
            accomplishments: vec!["Task 1".to_string(), "Task 2".to_string()],
            status: WorkStatus::Complete,
            key_files: vec!["file.rs".to_string()],
            workstream_tags: vec![],
            model_used: "haiku".to_string(),
        };
        store.cache_chain_summary(&response2).await.unwrap();

        let cached = store
            .get_chain_summary("update-test")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cached.summary, "Updated summary");
        assert_eq!(cached.status, WorkStatus::Complete);
        assert_eq!(cached.accomplishments.len(), 2);
    }

    #[tokio::test]
    async fn cache_get_all_summaries_returns_multiple() {
        let (store, _tmp) = create_test_store().await;

        for i in 0..3 {
            let response = ChainSummaryResponse {
                chain_id: format!("summary-{}", i),
                summary: format!("Summary {}", i),
                accomplishments: vec![],
                status: WorkStatus::InProgress,
                key_files: vec![],
                workstream_tags: vec![],
                model_used: "haiku".to_string(),
            };
            store.cache_chain_summary(&response).await.unwrap();
        }

        let all = store.get_all_chain_summaries().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn cache_summary_preserves_all_status_values() {
        let (store, _tmp) = create_test_store().await;

        let statuses = vec![
            WorkStatus::InProgress,
            WorkStatus::Complete,
            WorkStatus::Paused,
            WorkStatus::Abandoned,
        ];

        for (i, status) in statuses.iter().enumerate() {
            let response = ChainSummaryResponse {
                chain_id: format!("status-test-{}", i),
                summary: "Test".to_string(),
                accomplishments: vec![],
                status: status.clone(),
                key_files: vec![],
                workstream_tags: vec![],
                model_used: "haiku".to_string(),
            };
            store.cache_chain_summary(&response).await.unwrap();

            let cached = store
                .get_chain_summary(&format!("status-test-{}", i))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(cached.status, *status);
        }
    }

    // =========================================================================
    // PRACTICAL INTEGRATION TEST - Uses real database and Intel service
    // Run manually: cargo test practical_integration --features integration -- --nocapture
    // =========================================================================

    #[tokio::test]
    #[ignore] // Run with: cargo test practical_integration -- --ignored --nocapture
    async fn practical_integration_test_chain_summary_with_real_service() {
        use crate::intelligence::{ChainSummaryRequest, IntelClient};

        // 1. Open real database
        let db_path = dirs::home_dir()
            .expect("Home dir")
            .join(".context-os")
            .join("context_os_events.db");

        println!("Opening database at: {:?}", db_path);

        let store = MetadataStore::new(&db_path)
            .await
            .expect("Failed to open store");

        // 2. Check current state
        let existing_summaries = store.get_all_chain_summaries().await.unwrap();
        println!("Existing chain summaries: {}", existing_summaries.len());
        for summary in &existing_summaries {
            println!(
                "  - {}: {} ({:?})",
                summary.chain_id, summary.summary, summary.status
            );
        }

        // 3. Check Intel service
        let client = IntelClient::default();
        let service_available = client.health_check().await;
        println!("Intel service available: {}", service_available);

        if !service_available {
            println!("SKIP: Intel service not running at localhost:3002");
            return;
        }

        // 4. Generate a fresh summary
        let test_chain_id = format!("practical-test-{}", chrono::Utc::now().timestamp());
        let request = ChainSummaryRequest {
            chain_id: test_chain_id.clone(),
            conversation_excerpt: Some("Help me implement authentication for my app. I want to use JWT tokens. Let me create the auth middleware first...".to_string()),
            files_touched: vec!["src/auth.rs".to_string(), "src/middleware.rs".to_string()],
            session_count: 3,
            duration_seconds: Some(3600),
            existing_workstreams: Some(vec!["tastematter-product".to_string()]),
        };

        println!("Calling summarize_chain for: {}", test_chain_id);
        let response = client
            .summarize_chain(&request)
            .await
            .expect("Request failed");

        match response {
            Some(summary) => {
                println!("✅ Summary generated:");
                println!("   Chain ID: {}", summary.chain_id);
                println!("   Summary: {}", summary.summary);
                println!("   Status: {:?}", summary.status);
                println!("   Accomplishments: {:?}", summary.accomplishments);
                println!("   Key files: {:?}", summary.key_files);
                println!("   Workstream tags: {:?}", summary.workstream_tags);
                println!("   Model: {}", summary.model_used);

                // 5. Cache it
                store
                    .cache_chain_summary(&summary)
                    .await
                    .expect("Cache failed");
                println!("✅ Summary cached");

                // 6. Verify cache
                let cached = store.get_chain_summary(&test_chain_id).await.unwrap();
                assert!(cached.is_some(), "Summary should be cached");
                println!("✅ Cache verified");

                // 7. Check workstream tags
                for tag in &summary.workstream_tags {
                    println!("   Tag: {} (source: {:?})", tag.tag, tag.source);
                }
            }
            None => {
                println!("❌ No summary returned - service may have failed");
            }
        }
    }
}
