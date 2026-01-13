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
}
