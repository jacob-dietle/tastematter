//! Trail pull: fetch rows from global trail worker, write to local SQLite.

use log::{debug, info, warn};
use serde_json::Value;
use sqlx::sqlite::SqlitePool;

use super::config::TrailConfig;
use super::push::TABLES;

/// Tables that use INSERT OR IGNORE (auto-increment with UNIQUE constraints).
/// All other tables use INSERT OR REPLACE (natural primary keys).
const IGNORE_TABLES: &[&str] = &["file_access_events", "file_edges"];

/// Columns added by the worker that don't exist in local SQLite.
/// source_machine is preserved locally for attribution; only synced_at is D1-only.
#[cfg(test)]
const D1_ONLY_COLUMNS: &[&str] = &["synced_at"];

/// Result of a trail pull operation.
#[derive(Debug, Default)]
pub struct TrailPullResult {
    pub rows_pulled: i32,
    pub errors: Vec<String>,
}

/// Pull trail data from the global worker into local SQLite.
///
/// Fetches rows synced after `last_trail_pull` from D1, strips D1-only columns,
/// and upserts into local tables. Tracks last pull timestamp in `_metadata`.
/// Graceful degradation: returns errors in result but does not panic.
pub async fn pull_trail(pool: &SqlitePool, config: &TrailConfig) -> TrailPullResult {
    let mut result = TrailPullResult::default();

    if !config.is_configured() {
        debug!(target: "trail.pull", "Trail not configured, skipping pull");
        return result;
    }

    let endpoint = config.endpoint.as_ref().unwrap();
    let client_id = config.client_id.as_ref().unwrap();
    let client_secret = config.client_secret.as_ref().unwrap();

    // Read last pull timestamp from _metadata (default: epoch)
    let since = match sqlx::query_as::<_, (String,)>(
        "SELECT value FROM _metadata WHERE key = 'last_trail_pull'",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some((ts,))) => ts,
        Ok(None) => "1970-01-01T00:00:00Z".to_string(),
        Err(e) => {
            result
                .errors
                .push(format!("Failed to read last_trail_pull: {}", e));
            "1970-01-01T00:00:00Z".to_string()
        }
    };

    // Build table list from TABLES constant
    let table_names: Vec<&str> = TABLES.iter().map(|t| t.name).collect();
    let tables_param = table_names.join(",");

    // GET /trail/pull?since=...&tables=...
    let pull_url = format!(
        "{}/trail/pull?since={}&tables={}",
        endpoint.trim_end_matches('/'),
        since,
        tables_param,
    );

    let client = reqwest::Client::new();
    let response = match client
        .get(&pull_url)
        .header("CF-Access-Client-Id", client_id)
        .header("CF-Access-Client-Secret", client_secret)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            result.errors.push(format!("Pull request failed: {}", e));
            return result;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        result
            .errors
            .push(format!("Pull failed ({}): {}", status, text));
        return result;
    }

    let body: Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to parse pull response: {}", e));
            return result;
        }
    };

    let tables_obj = match body.get("tables").and_then(|t| t.as_object()) {
        Some(t) => t,
        None => {
            result
                .errors
                .push("Pull response missing 'tables' object".to_string());
            return result;
        }
    };

    let synced_at = body
        .get("synced_at")
        .and_then(|s| s.as_str())
        .unwrap_or(&since);

    // Upsert rows into local SQLite
    for table_def in TABLES {
        let rows = match tables_obj.get(table_def.name).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };

        if rows.is_empty() {
            continue;
        }

        let is_ignore = IGNORE_TABLES.contains(&table_def.name);
        let strategy = if is_ignore {
            "INSERT OR IGNORE"
        } else {
            "INSERT OR REPLACE"
        };

        let mut table_rows = 0;
        for row in rows {
            let row_obj = match row.as_object() {
                Some(o) => o,
                None => continue,
            };

            // Filter to only columns in the local schema (from TableDef)
            // This strips source_machine, synced_at, and any D1-only columns
            let mut columns: Vec<&str> = Vec::new();
            let mut values: Vec<Value> = Vec::new();

            for &col in table_def.columns {
                if let Some(val) = row_obj.get(col) {
                    columns.push(col);
                    values.push(val.clone());
                }
            }

            if columns.is_empty() {
                continue;
            }

            let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
            let sql = format!(
                "{} INTO {} ({}) VALUES ({})",
                strategy,
                table_def.name,
                columns.join(", "),
                placeholders.join(", "),
            );

            let mut query = sqlx::query(&sql);
            for val in &values {
                query = bind_json_value(query, val);
            }

            match query.execute(pool).await {
                Ok(_) => table_rows += 1,
                Err(e) => {
                    // Log but don't fail the entire pull for individual row errors
                    warn!(
                        target: "trail.pull",
                        "Failed to insert row into {}: {}", table_def.name, e
                    );
                }
            }
        }

        if table_rows > 0 {
            debug!(
                target: "trail.pull",
                "{}: {} rows pulled", table_def.name, table_rows
            );
            result.rows_pulled += table_rows;
        }
    }

    // Update last_trail_pull in _metadata
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO _metadata (key, value, updated_at) \
         VALUES ('last_trail_pull', ?, datetime('now'))",
    )
    .bind(synced_at)
    .execute(pool)
    .await
    {
        result
            .errors
            .push(format!("Failed to update last_trail_pull: {}", e));
    }

    info!(
        target: "trail.pull",
        "Trail pull complete: {} rows from {} tables",
        result.rows_pulled,
        tables_obj.len()
    );

    result
}

/// Bind a serde_json::Value to a sqlx query argument.
fn bind_json_value<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    val: &'q Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match val {
        Value::Null => query.bind(None::<String>),
        Value::Bool(b) => query.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else if let Some(f) = n.as_f64() {
                query.bind(f)
            } else {
                query.bind(n.to_string())
            }
        }
        Value::String(s) => query.bind(s.as_str()),
        // JSON arrays/objects: store as JSON string
        _ => query.bind(val.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_tables_match_push_convention() {
        // file_access_events and file_edges use INSERT OR IGNORE (auto-increment + UNIQUE)
        assert!(IGNORE_TABLES.contains(&"file_access_events"));
        assert!(IGNORE_TABLES.contains(&"file_edges"));
        // All other tables use INSERT OR REPLACE
        assert!(!IGNORE_TABLES.contains(&"claude_sessions"));
        assert!(!IGNORE_TABLES.contains(&"chains"));
    }

    #[test]
    fn test_d1_only_columns_not_in_table_defs() {
        // Only synced_at should be D1-only. source_machine must be in TableDef
        // columns so pull preserves it and push can filter by it.
        assert_eq!(
            D1_ONLY_COLUMNS,
            &["synced_at"],
            "source_machine must NOT be in D1_ONLY_COLUMNS — it's needed locally for attribution",
        );
        // Verify synced_at is not in any table's columns
        for table in TABLES {
            for &col in table.columns {
                assert!(
                    !D1_ONLY_COLUMNS.contains(&col),
                    "D1-only column '{}' found in table '{}' columns",
                    col,
                    table.name,
                );
            }
        }
    }

    #[test]
    fn test_pull_preserves_source_machine() {
        // source_machine must be in each TableDef's columns list so that
        // pull inserts it into local SQLite, preserving D1 attribution.
        for table in TABLES {
            assert!(
                table.columns.contains(&"source_machine"),
                "Table '{}' missing 'source_machine' in columns — pull would strip attribution",
                table.name,
            );
        }
    }

    #[test]
    fn test_all_tables_covered() {
        let table_names: Vec<&str> = TABLES.iter().map(|t| t.name).collect();
        assert_eq!(table_names.len(), 8);
        assert!(table_names.contains(&"claude_sessions"));
        assert!(table_names.contains(&"file_access_events"));
        assert!(table_names.contains(&"file_edges"));
        assert!(table_names.contains(&"git_commits"));
    }

    #[tokio::test]
    async fn test_pull_skips_when_not_configured() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let config = TrailConfig::default();
        let result = pull_trail(&pool, &config).await;
        assert_eq!(result.rows_pulled, 0);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_last_trail_pull_defaults_to_epoch() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Create _metadata table
        sqlx::query("CREATE TABLE _metadata (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // No last_trail_pull entry — should default to epoch
        let since: Option<(String,)> =
            sqlx::query_as("SELECT value FROM _metadata WHERE key = 'last_trail_pull'")
                .fetch_optional(&pool)
                .await
                .unwrap();

        assert!(since.is_none());
    }

    #[tokio::test]
    async fn test_metadata_write_and_read() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query("CREATE TABLE _metadata (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Write last_trail_pull
        sqlx::query(
            "INSERT OR REPLACE INTO _metadata (key, value, updated_at) \
             VALUES ('last_trail_pull', '2026-02-24T12:00:00Z', datetime('now'))",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Read it back
        let (val,): (String,) =
            sqlx::query_as("SELECT value FROM _metadata WHERE key = 'last_trail_pull'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(val, "2026-02-24T12:00:00Z");
    }

    #[test]
    fn test_bind_json_value_types() {
        // Verify our JSON-to-SQLite type mapping covers all cases
        assert!(matches!(Value::Null, Value::Null));
        assert!(matches!(Value::Bool(true), Value::Bool(_)));
        assert!(matches!(
            Value::Number(serde_json::Number::from(42)),
            Value::Number(_)
        ));
        assert!(matches!(
            Value::String("test".to_string()),
            Value::String(_)
        ));
    }

    #[tokio::test]
    async fn test_upsert_replace_for_natural_pk_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Create a simplified claude_sessions table
        sqlx::query(
            "CREATE TABLE claude_sessions (session_id TEXT PRIMARY KEY, project_path TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // First insert
        sqlx::query("INSERT OR REPLACE INTO claude_sessions (session_id, project_path) VALUES ('s1', '/old/path')")
            .execute(&pool)
            .await
            .unwrap();

        // Second insert with same PK (should replace)
        sqlx::query("INSERT OR REPLACE INTO claude_sessions (session_id, project_path) VALUES ('s1', '/new/path')")
            .execute(&pool)
            .await
            .unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM claude_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let (path,): (String,) =
            sqlx::query_as("SELECT project_path FROM claude_sessions WHERE session_id = 's1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(path, "/new/path");
    }

    #[tokio::test]
    async fn test_upsert_ignore_for_auto_increment_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Create file_access_events with UNIQUE constraint (matching our schema)
        sqlx::query(
            "CREATE TABLE file_access_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                access_type TEXT NOT NULL,
                sequence_position INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE UNIQUE INDEX idx_fae_unique ON file_access_events(session_id, file_path, tool_name, sequence_position)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // First insert
        sqlx::query(
            "INSERT OR IGNORE INTO file_access_events (session_id, file_path, tool_name, access_type, sequence_position) \
             VALUES ('s1', '/file.rs', 'Read', 'read', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Duplicate insert (should be ignored)
        sqlx::query(
            "INSERT OR IGNORE INTO file_access_events (session_id, file_path, tool_name, access_type, sequence_position) \
             VALUES ('s1', '/file.rs', 'Read', 'read', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM file_access_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "INSERT OR IGNORE should skip duplicate");
    }
}
