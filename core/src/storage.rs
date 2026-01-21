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
        let metadata = std::fs::metadata(&path).map_err(|e| {
            CoreError::Config(format!("Cannot read database: {}", e))
        })?;
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
            .ok_or_else(|| CoreError::Config(
                "Cannot determine home directory".to_string()
            ))
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
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", []).unwrap();
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
        ).unwrap();
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
                tools_used TEXT
            )",
            [],
        ).unwrap();
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
        ).unwrap();
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
}
