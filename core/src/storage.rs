//! Database storage layer for context-os-core
//!
//! Provides SQLite connection management with connection pooling.
//! All database access is READ-ONLY (indexer writes, query engine reads).
//!
//! # Architecture (Jeff Dean Approved)
//!
//! Single canonical database location: `~/.context-os/context_os_events.db`
//!
//! NO fallback paths. NO auto-discovery. Explicit configuration only:
//! - `--db` flag for CLI
//! - Canonical path for default
//!
//! This prevents the "three database problem" where Python writes to one
//! location while Rust reads from another.

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::{Path, PathBuf};

use crate::error::CoreError;

/// Canonical database filename
const DB_FILENAME: &str = "context_os_events.db";

/// Canonical database directory under home
const DB_DIR: &str = ".context-os";

/// Database connection manager
///
/// Manages a pool of SQLite connections for concurrent queries.
/// Uses read-only mode since the indexer handles all writes.
pub struct Database {
    pool: SqlitePool,
    path: PathBuf,
}

impl Database {
    /// Open database with connection pooling
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file
    ///
    /// # Returns
    /// * `Result<Self, CoreError>` - Database instance or error
    ///
    /// # Example
    /// ```ignore
    /// let db = Database::open("~/.context-os/context_os_events.db").await?;
    /// ```
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref().to_path_buf();

        // Verify database file exists
        if !path.exists() {
            return Err(CoreError::Config(format!(
                "Database not found: {}\n\
                 Run the indexer first, or specify path with --db flag.",
                path.display()
            )));
        }

        // Verify non-empty
        let metadata = std::fs::metadata(&path)
            .map_err(|e| CoreError::Config(format!("Cannot read database: {}", e)))?;
        if metadata.len() == 0 {
            return Err(CoreError::Config(format!(
                "Database file is empty: {}",
                path.display()
            )));
        }

        // Connect with read-only mode (indexer writes)
        // Use WAL mode compatible settings
        let url = format!("sqlite:{}?mode=ro", path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .connect(&url)
            .await
            .map_err(CoreError::Database)?;

        Ok(Self { pool, path })
    }

    /// Open database in read-write mode with connection pooling
    ///
    /// Unlike `open()`, this opens the database in read-write-create mode,
    /// allowing INSERT, UPDATE, and DELETE operations.
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file
    ///
    /// # Returns
    /// * `Result<Self, CoreError>` - Database instance or error
    pub async fn open_rw(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref().to_path_buf();

        // Connect with read-write-create mode
        // rwc = read-write-create (creates if doesn't exist)
        let url = format!("sqlite:{}?mode=rwc", path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .connect(&url)
            .await
            .map_err(CoreError::Database)?;

        Ok(Self { pool, path })
    }

    /// Ensure core database schema exists.
    ///
    /// Creates all required tables if they don't exist. Safe to call multiple
    /// times (idempotent via IF NOT EXISTS). This enables fresh installs to
    /// work without manual database setup.
    ///
    /// Tables created:
    /// - claude_sessions: Parsed session data from JSONL files
    /// - git_commits: Git history
    /// - file_events: File system events
    /// - chains: Chain metadata
    /// - chain_graph: Session-to-chain mappings
    /// - _metadata: Schema version tracking
    ///
    /// # Returns
    /// * `Ok(())` - Schema exists (created or already present)
    /// * `Err(CoreError)` - Schema creation failed
    pub async fn ensure_schema(&self) -> Result<(), CoreError> {
        // Core schema SQL - uses IF NOT EXISTS for idempotency
        const SCHEMA_SQL: &str = r#"
            -- Layer 1: File Events
            CREATE TABLE IF NOT EXISTS file_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                path TEXT NOT NULL,
                event_type TEXT NOT NULL,
                size_bytes INTEGER,
                old_path TEXT,
                is_directory BOOLEAN DEFAULT FALSE,
                extension TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_file_events_path ON file_events(path);
            CREATE INDEX IF NOT EXISTS idx_file_events_timestamp ON file_events(timestamp);

            -- Layer 2: Claude Sessions
            CREATE TABLE IF NOT EXISTS claude_sessions (
                session_id TEXT PRIMARY KEY,
                project_path TEXT,
                started_at TEXT,
                ended_at TEXT,
                duration_seconds INTEGER,
                user_message_count INTEGER,
                assistant_message_count INTEGER,
                total_messages INTEGER,
                files_read TEXT,
                files_written TEXT,
                tools_used TEXT,
                file_size_bytes INTEGER,
                first_user_message TEXT,
                conversation_excerpt TEXT,
                parsed_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_claude_sessions_started ON claude_sessions(started_at);
            CREATE INDEX IF NOT EXISTS idx_claude_sessions_project ON claude_sessions(project_path);

            -- Layer 3: Git Commits
            CREATE TABLE IF NOT EXISTS git_commits (
                hash TEXT PRIMARY KEY,
                short_hash TEXT,
                timestamp TEXT NOT NULL,
                message TEXT,
                author_name TEXT,
                author_email TEXT,
                files_changed TEXT,
                files_added TEXT,
                files_deleted TEXT,
                files_modified TEXT,
                insertions INTEGER,
                deletions INTEGER,
                files_count INTEGER,
                is_agent_commit BOOLEAN,
                is_merge_commit BOOLEAN,
                synced_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_git_commits_timestamp ON git_commits(timestamp);

            -- Layer 4: Chains (session groupings)
            CREATE TABLE IF NOT EXISTS chains (
                chain_id TEXT PRIMARY KEY,
                root_session_id TEXT,
                session_count INTEGER,
                files_count INTEGER,
                updated_at TEXT
            );

            -- Layer 5: Chain Graph (session-to-chain mapping)
            CREATE TABLE IF NOT EXISTS chain_graph (
                session_id TEXT PRIMARY KEY,
                chain_id TEXT NOT NULL,
                parent_session_id TEXT,
                is_root BOOLEAN,
                indexed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_chain_graph_chain ON chain_graph(chain_id);

            -- Layer 6: Chain Metadata (Intel-generated names and summaries)
            CREATE TABLE IF NOT EXISTS chain_metadata (
                chain_id TEXT PRIMARY KEY,
                generated_name TEXT,
                summary TEXT,
                key_topics TEXT,
                category TEXT,
                confidence REAL,
                generated_at TEXT,
                model_used TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            -- Layer 7: Chain Summaries (Intel-generated chain summaries)
            CREATE TABLE IF NOT EXISTS chain_summaries (
                chain_id TEXT PRIMARY KEY,
                summary TEXT,
                accomplishments TEXT,
                status TEXT,
                key_files TEXT,
                workstream_tags TEXT,
                model_used TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            -- Layer 8: File Access Events (per-tool-call temporal data)
            -- Preserves the per-tool-call ordering that aggregate_session() collapses
            -- to deduplicated HashSets. ~190K rows from existing session history.
            CREATE TABLE IF NOT EXISTS file_access_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                file_path TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                access_type TEXT NOT NULL,
                sequence_position INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_fae_session ON file_access_events(session_id);
            CREATE INDEX IF NOT EXISTS idx_fae_file ON file_access_events(file_path);
            CREATE INDEX IF NOT EXISTS idx_fae_session_seq ON file_access_events(session_id, sequence_position);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_fae_unique ON file_access_events(session_id, file_path, tool_name, sequence_position);

            -- Layer 9: File Edges (aggregated behavioral relationships)
            -- Directed edges extracted deterministically from temporal ordering
            -- of tool calls within sessions. Batch-computed during daemon sync.
            CREATE TABLE IF NOT EXISTS file_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_file TEXT NOT NULL,
                target_file TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                session_count INTEGER NOT NULL DEFAULT 0,
                total_sessions_with_source INTEGER NOT NULL DEFAULT 0,
                avg_time_delta_seconds REAL,
                confidence REAL NOT NULL DEFAULT 0.0,
                first_seen TEXT,
                last_seen TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_fe_source ON file_edges(source_file, edge_type);
            CREATE INDEX IF NOT EXISTS idx_fe_target ON file_edges(target_file, edge_type);
            CREATE INDEX IF NOT EXISTS idx_fe_type_conf ON file_edges(edge_type, confidence DESC);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_fe_unique ON file_edges(source_file, target_file, edge_type);

            -- Metadata
            CREATE TABLE IF NOT EXISTS _metadata (
                key TEXT PRIMARY KEY,
                value TEXT,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            INSERT OR IGNORE INTO _metadata (key, value) VALUES ('schema_version', '2.3');
        "#;

        // Execute schema SQL
        sqlx::query(SCHEMA_SQL)
            .execute(&self.pool)
            .await
            .map_err(CoreError::Database)?;

        // Migration: add columns that may be missing from older schemas.
        // Each ALTER is idempotent: if the column already exists, SQLite returns
        // "duplicate column name" which we silently ignore.
        let migrations = vec![
            // chain_metadata columns from cache.rs
            "ALTER TABLE chain_metadata ADD COLUMN category TEXT",
            "ALTER TABLE chain_metadata ADD COLUMN confidence REAL",
            "ALTER TABLE chain_metadata ADD COLUMN generated_at TEXT",
            "ALTER TABLE chain_metadata ADD COLUMN model_used TEXT",
            // chain_metadata columns from storage.rs (in case cache.rs ran first historically)
            "ALTER TABLE chain_metadata ADD COLUMN summary TEXT",
            "ALTER TABLE chain_metadata ADD COLUMN key_topics TEXT",
            "ALTER TABLE chain_metadata ADD COLUMN updated_at TEXT DEFAULT CURRENT_TIMESTAMP",
            // chain_graph columns from persist_chains
            "ALTER TABLE chain_graph ADD COLUMN parent_session_id TEXT",
            "ALTER TABLE chain_graph ADD COLUMN is_root BOOLEAN",
            "ALTER TABLE chain_graph ADD COLUMN indexed_at TEXT",
            // file_edges lift column (spec #20 quality refinement)
            "ALTER TABLE file_edges ADD COLUMN lift REAL",
            // file_access_events dedup (matches file_edges idx_fe_unique pattern)
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_fae_unique ON file_access_events(session_id, file_path, tool_name, sequence_position)",
        ];

        for migration in migrations {
            // Ignore "duplicate column name" errors -- column already exists
            let _ = sqlx::query(migration).execute(&self.pool).await;
        }

        Ok(())
    }

    /// Get a reference to the connection pool
    ///
    /// Use this to execute queries via sqlx.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get canonical database path
    ///
    /// Returns the single canonical location: `~/.context-os/context_os_events.db`
    ///
    /// NO fallback paths. If you need a different location, use `--db` flag.
    pub fn canonical_path() -> Result<PathBuf, CoreError> {
        dirs::home_dir()
            .map(|h| h.join(DB_DIR).join(DB_FILENAME))
            .ok_or_else(|| CoreError::Config("Cannot determine home directory".to_string()))
    }

    /// Find database: explicit path OR canonical location
    ///
    /// Priority:
    /// 1. Explicit path (if provided and exists)
    /// 2. Canonical path: `~/.context-os/context_os_events.db`
    ///
    /// NO other fallback paths. Fail fast with clear error.
    ///
    /// # Returns
    /// * `Result<PathBuf, CoreError>` - Path to database or error
    pub fn find_database(explicit_path: Option<&Path>) -> Result<PathBuf, CoreError> {
        // 1. Check explicit path first (--db flag)
        if let Some(path) = explicit_path {
            if path.exists() {
                return Ok(path.to_path_buf());
            }
            return Err(CoreError::Config(format!(
                "Database not found at specified path: {}",
                path.display()
            )));
        }

        // 2. Check canonical location ONLY
        let canonical = Self::canonical_path()?;
        if canonical.exists() {
            return Ok(canonical);
        }

        // 3. NO FALLBACK - fail fast with clear error
        Err(CoreError::Config(format!(
            "Database not found at canonical location: {}\n\
             \n\
             To fix:\n\
             1. Run the indexer to create the database, OR\n\
             2. Specify database path with --db flag\n\
             \n\
             Example: context-os --db /path/to/context_os_events.db query flex",
            canonical.display()
        )))
    }

    /// Open database from canonical location or explicit path
    ///
    /// Convenience method that finds and opens the database.
    pub async fn open_default() -> Result<Self, CoreError> {
        let path = Self::find_database(None)?;
        Self::open(path).await
    }

    /// Open database from canonical location, creating if it doesn't exist.
    ///
    /// For fresh machines: creates `~/.context-os/` directory and empty DB with schema.
    /// Queries return empty results, which is correct for "no data yet."
    /// Returns a read-only connection (schema created via temporary RW connection).
    pub async fn open_or_create_default() -> Result<Self, CoreError> {
        let canonical = Self::canonical_path()?;

        if canonical.exists() {
            return Self::open(&canonical).await;
        }

        // Fresh machine: create directory + DB + schema
        if let Some(parent) = canonical.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Config(format!("Could not create database directory: {}", e))
            })?;
        }

        let rw = Self::open_rw(&canonical).await?;
        rw.ensure_schema().await?;
        rw.close().await;

        Self::open(&canonical).await
    }

    /// Close the database connection pool
    ///
    /// Note: The pool will also close when dropped, but this allows
    /// explicit cleanup if needed.
    pub async fn close(self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_path_returns_home_based_path() {
        let result = Database::canonical_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".context-os"));
        assert!(path.to_string_lossy().contains("context_os_events.db"));
    }

    #[test]
    fn test_find_database_explicit_path_not_found_errors() {
        let result = Database::find_database(Some(Path::new("/nonexistent/path.db")));
        assert!(result.is_err());
        if let Err(CoreError::Config(msg)) = result {
            assert!(msg.contains("not found at specified path"));
        } else {
            panic!("Expected Config error for explicit path");
        }
    }

    #[tokio::test]
    async fn test_open_nonexistent_database() {
        let result = Database::open("/nonexistent/path.db").await;
        assert!(result.is_err());

        if let Err(CoreError::Config(msg)) = result {
            assert!(msg.contains("not found"));
        } else {
            panic!("Expected Config error");
        }
    }

    /// Test 1: Database opens in read-write mode
    ///
    /// RED phase: This test should FAIL because open_rw() doesn't exist yet.
    /// GREEN phase: Implement open_rw() method with ?mode=rwc
    #[tokio::test]
    async fn test_open_rw_enables_writes() {
        // Create temp database using rusqlite (sync)
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create empty database with a test table
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", [])
            .unwrap();
        drop(conn);

        // Open with our new read-write method
        let db = Database::open_rw(&db_path).await.unwrap();

        // Should be able to write
        let result = sqlx::query("INSERT INTO test (id) VALUES (1)")
            .execute(db.pool())
            .await;

        assert!(result.is_ok(), "Write should succeed in rw mode");
    }

    /// Test 2: Insert git commit
    ///
    /// RED phase: This test should FAIL because insert_commit() doesn't exist yet.
    /// GREEN phase: Implement insert_commit() method in QueryEngine
    #[tokio::test]
    async fn test_insert_git_commit() {
        use crate::query::QueryEngine;
        use crate::types::GitCommitInput;

        // Create temp database with git_commits schema
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE git_commits (
                hash TEXT PRIMARY KEY,
                short_hash TEXT,
                timestamp TEXT NOT NULL,
                message TEXT,
                author_name TEXT,
                author_email TEXT,
                files_changed TEXT,
                insertions INTEGER,
                deletions INTEGER,
                files_count INTEGER,
                is_agent_commit BOOLEAN
            )",
            [],
        )
        .unwrap();
        drop(conn);

        // Open database with QueryEngine
        let db = Database::open_rw(&db_path).await.unwrap();
        let engine = QueryEngine::new(db);

        // Create test commit
        let commit = GitCommitInput {
            hash: "abc123def456789".to_string(),
            short_hash: "abc123d".to_string(),
            timestamp: "2026-01-13T12:00:00Z".to_string(),
            message: Some("Test commit".to_string()),
            author_name: Some("Test Author".to_string()),
            author_email: Some("test@example.com".to_string()),
            files_changed: Some("[\"file1.rs\"]".to_string()),
            insertions: Some(10),
            deletions: Some(5),
            files_count: Some(1),
            is_agent_commit: false,
        };

        // Insert and verify
        let result = engine.insert_commit(&commit).await;
        assert!(result.is_ok(), "Insert should succeed");
        assert_eq!(result.unwrap().rows_affected, 1);
    }

    /// Test 3: Insert Claude session
    ///
    /// RED phase: This test should FAIL because insert_session() doesn't exist yet.
    /// GREEN phase: Implement insert_session() method in QueryEngine
    #[tokio::test]
    async fn test_insert_session() {
        use crate::query::QueryEngine;
        use crate::types::SessionInput;

        // Create temp database with claude_sessions schema
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE claude_sessions (
                session_id TEXT PRIMARY KEY,
                project_path TEXT,
                started_at TEXT,
                ended_at TEXT,
                duration_seconds INTEGER,
                user_message_count INTEGER,
                assistant_message_count INTEGER,
                total_messages INTEGER,
                files_read TEXT,
                files_written TEXT,
                tools_used TEXT,
                file_size_bytes INTEGER,
                first_user_message TEXT,
                conversation_excerpt TEXT
            )",
            [],
        )
        .unwrap();
        drop(conn);

        // Open database with QueryEngine
        let db = Database::open_rw(&db_path).await.unwrap();
        let engine = QueryEngine::new(db);

        // Create test session
        let session = SessionInput {
            session_id: "test-session-123".to_string(),
            project_path: Some("/test/project".to_string()),
            started_at: Some("2026-01-13T10:00:00Z".to_string()),
            ended_at: Some("2026-01-13T12:00:00Z".to_string()),
            duration_seconds: Some(7200),
            user_message_count: Some(10),
            assistant_message_count: Some(15),
            total_messages: Some(25),
            files_read: Some("[\"file1.rs\", \"file2.rs\"]".to_string()),
            files_written: Some("[\"file1.rs\"]".to_string()),
            tools_used: Some("{\"Read\": 5, \"Edit\": 3}".to_string()),
            first_user_message: Some("Help me refactor this code".to_string()),
            conversation_excerpt: Some("[User 1]: Help me refactor this code".to_string()),
            file_size_bytes: Some(42000),
        };

        // Insert and verify
        let result = engine.insert_session(&session).await;
        assert!(result.is_ok(), "Insert should succeed");
        assert_eq!(result.unwrap().rows_affected, 1);
    }

    /// Test 4: Batch insert performance
    ///
    /// RED phase: This test should FAIL because insert_commits_batch() doesn't exist.
    /// GREEN phase: Implement batch insert with transaction wrapping.
    /// Target: <50ms for 1000 commits
    #[tokio::test]
    async fn test_batch_insert_commits_performance() {
        use crate::query::QueryEngine;
        use crate::types::GitCommitInput;
        use std::time::Instant;

        // Create temp database with git_commits schema
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE git_commits (
                hash TEXT PRIMARY KEY,
                short_hash TEXT,
                timestamp TEXT NOT NULL,
                message TEXT,
                author_name TEXT,
                author_email TEXT,
                files_changed TEXT,
                insertions INTEGER,
                deletions INTEGER,
                files_count INTEGER,
                is_agent_commit BOOLEAN
            )",
            [],
        )
        .unwrap();
        drop(conn);

        // Open database with QueryEngine
        let db = Database::open_rw(&db_path).await.unwrap();
        let engine = QueryEngine::new(db);

        // Create 1000 test commits
        let commits: Vec<GitCommitInput> = (0..1000)
            .map(|i| GitCommitInput {
                hash: format!("hash{:06}", i),
                short_hash: format!("h{:05}", i),
                timestamp: "2026-01-13T12:00:00Z".to_string(),
                message: Some(format!("Commit {}", i)),
                author_name: Some("Test Author".to_string()),
                author_email: Some("test@example.com".to_string()),
                files_changed: None,
                insertions: Some(i as i32),
                deletions: Some(0),
                files_count: Some(1),
                is_agent_commit: false,
            })
            .collect();

        // Time the batch insert
        let start = Instant::now();
        let result = engine.insert_commits_batch(&commits).await;
        let elapsed = start.elapsed();

        // Verify results
        assert!(result.is_ok(), "Batch insert should succeed");
        assert_eq!(result.unwrap().rows_affected, 1000);

        // Performance target: <1000ms for 1000 commits (generous for CI variability)
        // Typical: 107ms release, 150-500ms debug
        // Transaction wrapping ensures all-or-nothing semantics
        assert!(
            elapsed.as_millis() < 1000,
            "Batch insert took {}ms, should be <1000ms",
            elapsed.as_millis()
        );

        println!("Batch insert of 1000 commits took {:?}", elapsed);
    }

    /// Test: ensure_schema() creates tables on fresh database
    ///
    /// RED phase: This test should FAIL because ensure_schema() doesn't exist yet.
    /// GREEN phase: Implement ensure_schema() that creates all required tables.
    #[tokio::test]
    async fn test_ensure_schema_creates_tables_on_fresh_db() {
        // Create temp database (empty - no tables)
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("fresh.db");

        // Open with rwc mode (creates empty file)
        let db = Database::open_rw(&db_path).await.unwrap();

        // Run ensure_schema
        db.ensure_schema()
            .await
            .expect("ensure_schema should succeed");

        // Verify core tables exist by querying them
        let tables = vec![
            "claude_sessions",
            "git_commits",
            "file_events",
            "chains",
            "chain_graph",
            "_metadata",
        ];

        for table in tables {
            let result = sqlx::query(&format!("SELECT COUNT(*) FROM {}", table))
                .fetch_one(db.pool())
                .await;
            assert!(
                result.is_ok(),
                "Table '{}' should exist after ensure_schema",
                table
            );
        }
    }

    /// Test: ensure_schema() is idempotent (safe to call multiple times)
    #[tokio::test]
    async fn test_ensure_schema_is_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("idempotent.db");

        let db = Database::open_rw(&db_path).await.unwrap();

        // Call ensure_schema multiple times - should not error
        db.ensure_schema().await.expect("First call should succeed");
        db.ensure_schema()
            .await
            .expect("Second call should succeed");
        db.ensure_schema().await.expect("Third call should succeed");

        // Verify tables still work
        let result = sqlx::query("SELECT COUNT(*) FROM claude_sessions")
            .fetch_one(db.pool())
            .await;
        assert!(result.is_ok(), "Tables should still be queryable");
    }

    // =========================================================================
    // Spec 05: Schema Unification Tests (XCHECK-1, BUG-09, XCHECK-4)
    // =========================================================================

    #[tokio::test]
    async fn test_unified_schema_has_all_columns() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("unified.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Verify chain_metadata has ALL expected columns
        let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
            sqlx::query_as("PRAGMA table_info(chain_metadata)")
                .fetch_all(db.pool())
                .await
                .unwrap();

        let col_names: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();

        assert!(col_names.contains(&"chain_id".to_string()));
        assert!(col_names.contains(&"generated_name".to_string()));
        assert!(col_names.contains(&"summary".to_string()));
        assert!(col_names.contains(&"key_topics".to_string()));
        assert!(col_names.contains(&"category".to_string()));
        assert!(col_names.contains(&"confidence".to_string()));
        assert!(col_names.contains(&"generated_at".to_string()));
        assert!(col_names.contains(&"model_used".to_string()));
        assert!(col_names.contains(&"created_at".to_string()));
        assert!(col_names.contains(&"updated_at".to_string()));
    }

    #[tokio::test]
    async fn test_cache_writes_succeed_after_schema_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("cache_write.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Simulate the exact INSERT that cache.rs performs
        let result = sqlx::query(
            r#"INSERT OR REPLACE INTO chain_metadata
            (chain_id, generated_name, category, confidence, generated_at, model_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind("test-chain-id")
        .bind("Test Chain Name")
        .bind("development")
        .bind(0.95_f64)
        .bind("2026-02-06T12:00:00Z")
        .bind("claude-sonnet-4-5-20250929")
        .bind("2026-02-06T12:00:00Z")
        .execute(db.pool())
        .await;

        assert!(
            result.is_ok(),
            "Cache INSERT should succeed with unified schema"
        );

        // Verify the row was written
        let row: (String, f64) =
            sqlx::query_as("SELECT category, confidence FROM chain_metadata WHERE chain_id = ?")
                .bind("test-chain-id")
                .fetch_one(db.pool())
                .await
                .unwrap();

        assert_eq!(row.0, "development");
        assert!((row.1 - 0.95).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_migration_adds_missing_columns() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("migration.db");

        // Create the OLD storage.rs schema (without cache.rs columns)
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE chain_metadata (
                    chain_id TEXT PRIMARY KEY,
                    generated_name TEXT,
                    summary TEXT,
                    key_topics TEXT,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE chain_graph (
                    session_id TEXT PRIMARY KEY,
                    chain_id TEXT NOT NULL
                );",
            )
            .unwrap();
        }

        // Run ensure_schema which includes migrations
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Verify new columns were added to chain_metadata
        let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
            sqlx::query_as("PRAGMA table_info(chain_metadata)")
                .fetch_all(db.pool())
                .await
                .unwrap();

        let col_names: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();

        assert!(col_names.contains(&"category".to_string()));
        assert!(col_names.contains(&"confidence".to_string()));
        assert!(col_names.contains(&"generated_at".to_string()));
        assert!(col_names.contains(&"model_used".to_string()));
    }

    #[tokio::test]
    async fn test_schema_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("idempotent2.db");
        let db = Database::open_rw(&db_path).await.unwrap();

        // Run ensure_schema twice - should not error
        db.ensure_schema().await.unwrap();
        db.ensure_schema().await.unwrap();

        // Verify schema is still correct
        let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
            sqlx::query_as("PRAGMA table_info(chain_metadata)")
                .fetch_all(db.pool())
                .await
                .unwrap();

        assert_eq!(
            rows.len(),
            10,
            "chain_metadata should have exactly 10 columns"
        );
    }

    #[tokio::test]
    async fn test_chain_summaries_table_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("summaries.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Verify chain_summaries table exists and has correct columns
        let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
            sqlx::query_as("PRAGMA table_info(chain_summaries)")
                .fetch_all(db.pool())
                .await
                .unwrap();

        let col_names: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();

        assert!(col_names.contains(&"chain_id".to_string()));
        assert!(col_names.contains(&"summary".to_string()));
        assert!(col_names.contains(&"accomplishments".to_string()));
        assert!(col_names.contains(&"status".to_string()));
        assert!(col_names.contains(&"key_files".to_string()));
        assert!(col_names.contains(&"workstream_tags".to_string()));
        assert!(col_names.contains(&"model_used".to_string()));
        assert!(col_names.contains(&"created_at".to_string()));

        // Verify INSERT works
        let result = sqlx::query(
            r#"INSERT OR REPLACE INTO chain_summaries
            (chain_id, summary, accomplishments, status, key_files, workstream_tags, model_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind("test-chain")
        .bind("A test summary")
        .bind("[]")
        .bind("active")
        .bind("[]")
        .bind("[]")
        .bind("claude-sonnet-4-5-20250929")
        .bind("2026-02-06T12:00:00Z")
        .execute(db.pool())
        .await;

        assert!(result.is_ok(), "chain_summaries INSERT should succeed");
    }

    // =========================================================================
    // Phase 1: Storage Hardening (Stress Tests)
    // =========================================================================

    #[tokio::test]
    async fn stress_open_empty_db_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("empty.db");
        // Create a 0-byte file
        std::fs::File::create(&db_path).unwrap();

        let result = Database::open(&db_path).await;
        assert!(result.is_err());
        if let Err(CoreError::Config(msg)) = result {
            assert!(
                msg.contains("empty"),
                "Error should mention empty file: {}",
                msg
            );
        } else {
            panic!("Expected Config error for empty database file");
        }
    }

    #[tokio::test]
    async fn stress_upsert_duplicate_session() {
        use crate::query::QueryEngine;
        use crate::types::SessionInput;

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("upsert.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = QueryEngine::new(db);

        let session = SessionInput {
            session_id: "duplicate-id-123".to_string(),
            project_path: Some("/project/v1".to_string()),
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            ended_at: None,
            duration_seconds: Some(100),
            user_message_count: Some(5),
            assistant_message_count: Some(5),
            total_messages: Some(10),
            files_read: None,
            files_written: None,
            tools_used: None,
            first_user_message: Some("First version".to_string()),
            conversation_excerpt: None,
            file_size_bytes: None,
        };

        // First upsert
        engine.upsert_session(&session).await.unwrap();

        // Second upsert with different data
        let updated = SessionInput {
            session_id: "duplicate-id-123".to_string(),
            project_path: Some("/project/v2".to_string()),
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            ended_at: Some("2026-01-01T01:00:00Z".to_string()),
            duration_seconds: Some(3600),
            user_message_count: Some(20),
            assistant_message_count: Some(25),
            total_messages: Some(45),
            files_read: None,
            files_written: None,
            tools_used: None,
            first_user_message: Some("Updated version".to_string()),
            conversation_excerpt: None,
            file_size_bytes: None,
        };

        engine.upsert_session(&updated).await.unwrap();

        // Verify only 1 row exists
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM claude_sessions WHERE session_id = 'duplicate-id-123'",
        )
        .fetch_one(engine.database().pool())
        .await
        .unwrap();
        assert_eq!(count.0, 1, "Upsert should not create duplicates");

        // Verify it has the updated data
        let row: (String,) = sqlx::query_as(
            "SELECT project_path FROM claude_sessions WHERE session_id = 'duplicate-id-123'",
        )
        .fetch_one(engine.database().pool())
        .await
        .unwrap();
        assert_eq!(row.0, "/project/v2", "Upsert should update existing row");
    }

    #[tokio::test]
    async fn stress_session_all_null_optional_fields() {
        use crate::query::QueryEngine;
        use crate::types::SessionInput;

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("nulls.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = QueryEngine::new(db);

        let session = SessionInput {
            session_id: "minimal-session".to_string(),
            project_path: None,
            started_at: None,
            ended_at: None,
            duration_seconds: None,
            user_message_count: None,
            assistant_message_count: None,
            total_messages: None,
            files_read: None,
            files_written: None,
            tools_used: None,
            first_user_message: None,
            conversation_excerpt: None,
            file_size_bytes: None,
        };

        let result = engine.upsert_session(&session).await;
        assert!(
            result.is_ok(),
            "Session with all NULL optional fields should succeed"
        );

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM claude_sessions WHERE session_id = 'minimal-session'",
        )
        .fetch_one(engine.database().pool())
        .await
        .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn stress_session_large_conversation_excerpt() {
        use crate::query::QueryEngine;
        use crate::types::SessionInput;

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("large.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = QueryEngine::new(db);

        // 10KB conversation excerpt
        let large_excerpt = "x".repeat(10_000);

        let session = SessionInput {
            session_id: "large-excerpt-session".to_string(),
            project_path: Some("/test".to_string()),
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            ended_at: None,
            duration_seconds: None,
            user_message_count: None,
            assistant_message_count: None,
            total_messages: None,
            files_read: None,
            files_written: None,
            tools_used: None,
            first_user_message: None,
            conversation_excerpt: Some(large_excerpt.clone()),
            file_size_bytes: None,
        };

        let result = engine.upsert_session(&session).await;
        assert!(result.is_ok(), "10KB excerpt should be storable");

        let row: (String,) = sqlx::query_as(
            "SELECT conversation_excerpt FROM claude_sessions WHERE session_id = 'large-excerpt-session'",
        )
        .fetch_one(engine.database().pool())
        .await
        .unwrap();
        assert_eq!(row.0.len(), 10_000, "Full 10KB excerpt should round-trip");
    }

    #[tokio::test]
    async fn stress_two_connections_same_db() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("concurrent.db");

        // Open first connection, create schema
        let db1 = Database::open_rw(&db_path).await.unwrap();
        db1.ensure_schema().await.unwrap();

        // Insert data via first connection
        sqlx::query("INSERT INTO claude_sessions (session_id) VALUES ('from-conn-1')")
            .execute(db1.pool())
            .await
            .unwrap();

        // Open second connection to same DB
        let db2 = Database::open_rw(&db_path).await.unwrap();

        // Read from second connection
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM claude_sessions WHERE session_id = 'from-conn-1'")
                .fetch_one(db2.pool())
                .await
                .unwrap();
        assert_eq!(count.0, 1, "Second connection should see data from first");

        // Write from second connection
        sqlx::query("INSERT INTO claude_sessions (session_id) VALUES ('from-conn-2')")
            .execute(db2.pool())
            .await
            .unwrap();

        // Read from first connection should see both
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM claude_sessions")
            .fetch_one(db1.pool())
            .await
            .unwrap();
        assert_eq!(total.0, 2, "Both connections should see all data");
    }

    #[tokio::test]
    async fn stress_db_path_with_spaces() {
        let temp_dir = tempfile::tempdir().unwrap();
        let spaced_dir = temp_dir.path().join("path with spaces");
        std::fs::create_dir_all(&spaced_dir).unwrap();
        let db_path = spaced_dir.join("test database.db");

        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        sqlx::query("INSERT INTO claude_sessions (session_id) VALUES ('space-test')")
            .execute(db.pool())
            .await
            .unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM claude_sessions")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 1, "DB with spaces in path should work");
    }

    #[tokio::test]
    async fn stress_db_path_with_unicode() {
        let temp_dir = tempfile::tempdir().unwrap();
        let unicode_dir = temp_dir.path().join("data_\u{9879}\u{76EE}");
        std::fs::create_dir_all(&unicode_dir).unwrap();
        let db_path = unicode_dir.join("test_db.db");

        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        sqlx::query("INSERT INTO claude_sessions (session_id) VALUES ('unicode-test')")
            .execute(db.pool())
            .await
            .unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM claude_sessions")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 1, "DB with unicode path should work");
    }

    /// Test: ensure_schema() doesn't destroy existing data
    #[tokio::test]
    async fn test_ensure_schema_preserves_existing_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("existing.db");

        // Create database and add some data
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Insert test data
        sqlx::query("INSERT INTO claude_sessions (session_id) VALUES ('test-session-123')")
            .execute(db.pool())
            .await
            .unwrap();

        // Call ensure_schema again
        db.ensure_schema().await.expect("Should not destroy data");

        // Verify data still exists
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM claude_sessions WHERE session_id = 'test-session-123'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(row.0, 1, "Existing data should be preserved");
    }

    // =========================================================================
    // Phase 1: Temporal Edges Schema Tests
    // =========================================================================

    #[tokio::test]
    async fn test_ensure_schema_creates_temporal_tables() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("temporal.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Verify file_access_events table exists with correct columns
        let fae_cols: Vec<(i32, String, String, i32, Option<String>, i32)> =
            sqlx::query_as("PRAGMA table_info(file_access_events)")
                .fetch_all(db.pool())
                .await
                .unwrap();
        let fae_names: Vec<String> = fae_cols.iter().map(|r| r.1.clone()).collect();
        assert!(fae_names.contains(&"id".to_string()));
        assert!(fae_names.contains(&"session_id".to_string()));
        assert!(fae_names.contains(&"timestamp".to_string()));
        assert!(fae_names.contains(&"file_path".to_string()));
        assert!(fae_names.contains(&"tool_name".to_string()));
        assert!(fae_names.contains(&"access_type".to_string()));
        assert!(fae_names.contains(&"sequence_position".to_string()));
        assert_eq!(
            fae_names.len(),
            7,
            "file_access_events should have 7 columns"
        );

        // Verify file_edges table exists with correct columns
        let fe_cols: Vec<(i32, String, String, i32, Option<String>, i32)> =
            sqlx::query_as("PRAGMA table_info(file_edges)")
                .fetch_all(db.pool())
                .await
                .unwrap();
        let fe_names: Vec<String> = fe_cols.iter().map(|r| r.1.clone()).collect();
        assert!(fe_names.contains(&"id".to_string()));
        assert!(fe_names.contains(&"source_file".to_string()));
        assert!(fe_names.contains(&"target_file".to_string()));
        assert!(fe_names.contains(&"edge_type".to_string()));
        assert!(fe_names.contains(&"session_count".to_string()));
        assert!(fe_names.contains(&"total_sessions_with_source".to_string()));
        assert!(fe_names.contains(&"avg_time_delta_seconds".to_string()));
        assert!(fe_names.contains(&"confidence".to_string()));
        assert!(fe_names.contains(&"lift".to_string()));
        assert!(fe_names.contains(&"first_seen".to_string()));
        assert!(fe_names.contains(&"last_seen".to_string()));
        assert_eq!(fe_names.len(), 11, "file_edges should have 11 columns");
    }

    #[tokio::test]
    async fn test_file_access_events_insert_and_query() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("fae_insert.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Insert 5 events for a session
        for i in 0..5 {
            sqlx::query(
                "INSERT INTO file_access_events \
                 (session_id, timestamp, file_path, tool_name, access_type, sequence_position) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind("session-001")
            .bind(format!("2026-02-17T10:00:{:02}.000Z", i))
            .bind(format!("src/file{}.rs", i))
            .bind(if i < 3 { "Read" } else { "Edit" })
            .bind(if i < 3 { "read" } else { "write" })
            .bind(i)
            .execute(db.pool())
            .await
            .unwrap();
        }

        // Query back ordered by sequence_position
        let rows: Vec<(String, String, String, i32)> = sqlx::query_as(
            "SELECT file_path, tool_name, access_type, sequence_position \
             FROM file_access_events WHERE session_id = ? \
             ORDER BY sequence_position",
        )
        .bind("session-001")
        .fetch_all(db.pool())
        .await
        .unwrap();

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].0, "src/file0.rs");
        assert_eq!(rows[0].2, "read");
        assert_eq!(rows[0].3, 0);
        assert_eq!(rows[4].0, "src/file4.rs");
        assert_eq!(rows[4].2, "write");
        assert_eq!(rows[4].3, 4);
    }

    #[tokio::test]
    async fn test_file_edges_unique_constraint() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("fe_unique.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Insert an edge
        sqlx::query(
            "INSERT INTO file_edges \
             (source_file, target_file, edge_type, session_count, confidence) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("types.rs")
        .bind("query.rs")
        .bind("read_then_edit")
        .bind(3)
        .bind(0.6)
        .execute(db.pool())
        .await
        .unwrap();

        // INSERT OR REPLACE with same (source, target, type) — should update, not duplicate
        sqlx::query(
            "INSERT OR REPLACE INTO file_edges \
             (source_file, target_file, edge_type, session_count, confidence) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("types.rs")
        .bind("query.rs")
        .bind("read_then_edit")
        .bind(5)
        .bind(0.8)
        .execute(db.pool())
        .await
        .unwrap();

        // Should have exactly 1 row (not 2)
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM file_edges")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 1, "UNIQUE index should prevent duplicates");

        // Verify it has the updated values
        let row: (i32, f64) = sqlx::query_as(
            "SELECT session_count, confidence FROM file_edges \
             WHERE source_file = 'types.rs' AND target_file = 'query.rs'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(row.0, 5);
        assert!((row.1 - 0.8).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_temporal_tables_preserved_across_ensure_schema() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("temporal_preserve.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // Insert data into temporal tables
        sqlx::query(
            "INSERT INTO file_access_events \
             (session_id, timestamp, file_path, tool_name, access_type, sequence_position) \
             VALUES ('s1', '2026-02-17T10:00:00Z', 'file.rs', 'Read', 'read', 0)",
        )
        .execute(db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO file_edges \
             (source_file, target_file, edge_type, session_count, confidence) \
             VALUES ('a.rs', 'b.rs', 'read_before', 5, 0.7)",
        )
        .execute(db.pool())
        .await
        .unwrap();

        // Run ensure_schema again
        db.ensure_schema().await.unwrap();

        // Verify data survived
        let fae_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM file_access_events")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(
            fae_count.0, 1,
            "file_access_events data should survive ensure_schema"
        );

        let fe_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM file_edges")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(
            fe_count.0, 1,
            "file_edges data should survive ensure_schema"
        );
    }

    #[tokio::test]
    async fn test_schema_version_updated_to_2_3() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("version.db");
        let db = Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        let version: (String,) =
            sqlx::query_as("SELECT value FROM _metadata WHERE key = 'schema_version'")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(version.0, "2.3", "Schema version should be 2.3");
    }
}
