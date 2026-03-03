//! Trail push: read local SQLite, normalize paths, POST to global trail worker.

use log::{debug, info, warn};
use serde_json::{json, Map, Value};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use super::config::TrailConfig;
use super::paths::{normalize_json_paths, normalize_path};

/// Column definitions for each table. Only these columns are sent to/from D1.
/// This is the explicit contract between local SQLite and the D1 schema.
///
/// Columns NOT sent (local-only): is_agent_commit, is_merge_commit, is_root
/// Columns added by worker: source_machine, synced_at
pub struct TableDef {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    /// Columns containing single file paths (normalized on push)
    pub path_columns: &'static [&'static str],
    /// Columns containing JSON arrays of file paths (normalized on push)
    pub json_path_columns: &'static [&'static str],
}

pub const TABLES: &[TableDef] = &[
    TableDef {
        name: "claude_sessions",
        columns: &[
            "session_id",
            "project_path",
            "started_at",
            "ended_at",
            "duration_seconds",
            "user_message_count",
            "assistant_message_count",
            "total_messages",
            "files_read",
            "files_written",
            "tools_used",
            "file_size_bytes",
            "first_user_message",
            "conversation_excerpt",
            "parsed_at",
            "source_machine",
        ],
        path_columns: &["project_path"],
        json_path_columns: &["files_read", "files_written"],
    },
    TableDef {
        name: "chain_graph",
        columns: &[
            "session_id",
            "chain_id",
            "parent_session_id",
            "indexed_at",
            "source_machine",
        ],
        path_columns: &[],
        json_path_columns: &[],
    },
    TableDef {
        name: "chain_metadata",
        columns: &[
            "chain_id",
            "generated_name",
            "summary",
            "key_topics",
            "category",
            "confidence",
            "generated_at",
            "model_used",
            "created_at",
            "updated_at",
            "source_machine",
        ],
        path_columns: &[],
        json_path_columns: &[],
    },
    TableDef {
        name: "chain_summaries",
        columns: &[
            "chain_id",
            "summary",
            "accomplishments",
            "status",
            "key_files",
            "workstream_tags",
            "model_used",
            "created_at",
            "source_machine",
        ],
        path_columns: &[],
        json_path_columns: &[],
    },
    TableDef {
        name: "chains",
        columns: &[
            "chain_id",
            "root_session_id",
            "session_count",
            "files_count",
            "updated_at",
            "source_machine",
        ],
        path_columns: &[],
        json_path_columns: &[],
    },
    TableDef {
        name: "file_access_events",
        columns: &[
            "session_id",
            "timestamp",
            "file_path",
            "tool_name",
            "access_type",
            "sequence_position",
            "source_machine",
        ],
        path_columns: &["file_path"],
        json_path_columns: &[],
    },
    TableDef {
        name: "file_edges",
        columns: &[
            "source_file",
            "target_file",
            "edge_type",
            "session_count",
            "total_sessions_with_source",
            "avg_time_delta_seconds",
            "confidence",
            "lift",
            "first_seen",
            "last_seen",
            "source_machine",
        ],
        path_columns: &["source_file", "target_file"],
        json_path_columns: &[],
    },
    TableDef {
        name: "git_commits",
        columns: &[
            "hash",
            "short_hash",
            "timestamp",
            "message",
            "author_name",
            "author_email",
            "files_changed",
            "files_added",
            "files_deleted",
            "files_modified",
            "insertions",
            "deletions",
            "files_count",
            "source_machine",
        ],
        path_columns: &[],
        json_path_columns: &[
            "files_changed",
            "files_added",
            "files_deleted",
            "files_modified",
        ],
    },
];

/// Result of a trail push operation.
#[derive(Debug, Default)]
pub struct TrailPushResult {
    pub rows_pushed: i32,
    pub errors: Vec<String>,
}

/// Push all local trail data to the global trail worker.
///
/// Reads all rows from each table, normalizes paths, and POSTs to the worker.
/// Graceful degradation: returns errors in result but does not panic.
pub async fn push_trail(pool: &SqlitePool, config: &TrailConfig) -> TrailPushResult {
    let mut result = TrailPushResult::default();

    if !config.is_configured() {
        debug!(target: "trail.push", "Trail not configured, skipping push");
        return result;
    }

    let endpoint = config.endpoint.as_ref().unwrap();
    let machine_id = config.machine_id.as_ref().unwrap();
    let client_id = config.client_id.as_ref().unwrap();
    let client_secret = config.client_secret.as_ref().unwrap();

    // Query all tables
    let mut tables_payload: Map<String, Value> = Map::new();

    for table_def in TABLES {
        match query_table(pool, table_def, machine_id).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    debug!(
                        target: "trail.push",
                        "Table {}: {} rows",
                        table_def.name,
                        rows.len()
                    );
                    tables_payload.insert(table_def.name.to_string(), Value::Array(rows));
                }
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Query {}: {}", table_def.name, e));
            }
        }
    }

    if tables_payload.is_empty() {
        debug!(target: "trail.push", "No data to push");
        return result;
    }

    // Build request body
    let body = json!({
        "machine_id": machine_id,
        "tables": tables_payload,
    });

    // POST to worker
    let push_url = format!("{}/trail/push", endpoint.trim_end_matches('/'));
    info!(
        target: "trail.push",
        "Pushing to {} ({} tables)",
        push_url,
        tables_payload.len()
    );

    let client = reqwest::Client::new();
    match client
        .post(&push_url)
        .header("CF-Access-Client-Id", client_id)
        .header("CF-Access-Client-Secret", client_secret)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<Value>().await {
                    Ok(resp_body) => {
                        let rows_synced = resp_body
                            .get("rows_synced")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        result.rows_pushed = rows_synced as i32;
                        info!(
                            target: "trail.push",
                            "Trail push complete: {} rows synced",
                            rows_synced
                        );
                    }
                    Err(e) => {
                        result.errors.push(format!("Parse response: {}", e));
                    }
                }
            } else {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                result
                    .errors
                    .push(format!("Push failed ({}): {}", status, body_text));
            }
        }
        Err(e) => {
            warn!(target: "trail.push", "Push request failed: {}", e);
            result.errors.push(format!("Request failed: {}", e));
        }
    }

    result
}

/// Query local-origin rows from a single table with explicit column selection
/// and path normalization.
///
/// Returns rows where source_machine IS NULL (legacy/pre-attribution) OR
/// source_machine matches the local machine_id (stamped at creation).
/// Rows pulled from D1 with a different source_machine are excluded
/// to prevent re-pushing other machines' data.
async fn query_table(
    pool: &SqlitePool,
    table_def: &TableDef,
    machine_id: &str,
) -> Result<Vec<Value>, String> {
    let columns_sql = table_def.columns.join(", ");
    let query = format!(
        "SELECT {} FROM {} WHERE source_machine IS NULL OR source_machine = ?",
        columns_sql, table_def.name,
    );

    let rows = sqlx::query(&query)
        .bind(machine_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("{}", e))?;

    let mut result = Vec::with_capacity(rows.len());

    for row in &rows {
        let mut obj = Map::new();
        for col in table_def.columns {
            // Try to get value as different types
            let value = get_column_value(row, col);

            // Apply path normalization
            let value = if let Value::String(ref s) = value {
                if table_def.path_columns.contains(col) {
                    Value::String(normalize_path(s))
                } else if table_def.json_path_columns.contains(col) {
                    Value::String(normalize_json_paths(s))
                } else {
                    value
                }
            } else {
                value
            };

            obj.insert(col.to_string(), value);
        }
        result.push(Value::Object(obj));
    }

    Ok(result)
}

/// Extract a column value from a SQLite row as a serde_json::Value.
/// Handles TEXT, INTEGER, REAL, and NULL types.
fn get_column_value(row: &sqlx::sqlite::SqliteRow, col: &str) -> Value {
    // Try string first (covers TEXT)
    if let Ok(v) = row.try_get::<Option<String>, _>(col) {
        return match v {
            Some(s) => Value::String(s),
            None => Value::Null,
        };
    }
    // Try i64 (covers INTEGER)
    if let Ok(v) = row.try_get::<Option<i64>, _>(col) {
        return match v {
            Some(n) => Value::Number(n.into()),
            None => Value::Null,
        };
    }
    // Try f64 (covers REAL)
    if let Ok(v) = row.try_get::<Option<f64>, _>(col) {
        return match v {
            Some(f) => serde_json::Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            None => Value::Null,
        };
    }
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tables_have_expected_count() {
        assert_eq!(TABLES.len(), 8, "Should have 8 tables matching D1 schema");
    }

    #[test]
    fn test_table_names_match_d1() {
        let expected = [
            "claude_sessions",
            "chain_graph",
            "chain_metadata",
            "chain_summaries",
            "chains",
            "file_access_events",
            "file_edges",
            "git_commits",
        ];
        let actual: Vec<&str> = TABLES.iter().map(|t| t.name).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_no_local_only_columns_leaked() {
        // These columns exist in local SQLite but NOT in D1.
        // They must NOT appear in any table's column list.
        let forbidden = ["is_agent_commit", "is_merge_commit", "is_root", "id"];

        for table in TABLES {
            for col in table.columns {
                assert!(
                    !forbidden.contains(col),
                    "Table '{}' has forbidden column '{}' — this would cause D1 INSERT errors",
                    table.name,
                    col
                );
            }
        }
    }

    #[test]
    fn test_claude_sessions_has_path_normalization() {
        let sessions = &TABLES[0];
        assert_eq!(sessions.name, "claude_sessions");
        assert!(sessions.path_columns.contains(&"project_path"));
        assert!(sessions.json_path_columns.contains(&"files_read"));
        assert!(sessions.json_path_columns.contains(&"files_written"));
    }

    #[test]
    fn test_file_edges_has_path_normalization() {
        let edges = TABLES.iter().find(|t| t.name == "file_edges").unwrap();
        assert!(edges.path_columns.contains(&"source_file"));
        assert!(edges.path_columns.contains(&"target_file"));
    }

    #[test]
    fn test_push_result_default() {
        let result = TrailPushResult::default();
        assert_eq!(result.rows_pushed, 0);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_push_skips_when_not_configured() {
        // Create an in-memory SQLite pool for testing
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let config = TrailConfig::default(); // not configured
        let result = push_trail(&pool, &config).await;

        assert_eq!(result.rows_pushed, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_all_tables_have_source_machine_column() {
        // Every synced table must include source_machine so that:
        // 1. Pull preserves attribution from D1
        // 2. Push can filter to only local-origin rows
        for table in TABLES {
            assert!(
                table.columns.contains(&"source_machine"),
                "Table '{}' missing 'source_machine' column — would corrupt D1 attribution on push",
                table.name,
            );
        }
    }

    #[tokio::test]
    async fn test_push_excludes_other_machine_rows() {
        // Push should only send rows where source_machine IS NULL (local-origin)
        // or source_machine matches the configured machine_id.
        // Rows pulled from D1 with a different source_machine must NOT be pushed back.
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Create claude_sessions with all columns from TableDef + source_machine
        sqlx::query(
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
                conversation_excerpt TEXT,
                parsed_at TEXT,
                source_machine TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert a local-origin row (source_machine IS NULL)
        sqlx::query(
            "INSERT INTO claude_sessions (session_id, project_path, source_machine) \
             VALUES ('local-1', '/home/user/project', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert a row from another machine (should be excluded from push)
        sqlx::query(
            "INSERT INTO claude_sessions (session_id, project_path, source_machine) \
             VALUES ('laptop-1', '/home/user/project', 'laptop')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let sessions_def = &TABLES[0];
        assert_eq!(sessions_def.name, "claude_sessions");

        let rows = query_table(&pool, sessions_def).await.unwrap();

        // Should only return the local-origin row, not the laptop row
        assert_eq!(
            rows.len(),
            1,
            "Push should exclude rows from other machines, got: {:?}",
            rows,
        );
        assert_eq!(
            rows[0].get("session_id").and_then(|v| v.as_str()),
            Some("local-1"),
        );
    }
}
