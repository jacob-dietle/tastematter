//! Query engine for context-os-core
//!
//! Implements the core query functions that replace the Python CLI.
//! Target: <100ms latency for all queries.

use sqlx::Row;
use std::time::Instant;

use crate::error::CoreError;
use crate::storage::Database;
use crate::types::*;

/// Query engine for context-os
///
/// Provides direct SQLite queries with sub-100ms latency.
pub struct QueryEngine {
    db: Database,
}

impl QueryEngine {
    /// Create a new QueryEngine with the given database
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get a reference to the underlying database
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Query files with flexible filters
    ///
    /// This is the main query command, supporting filtering by:
    /// - Time range (7d, 14d, 30d)
    /// - Chain ID
    /// - Session ID
    /// - File path pattern
    ///
    /// Results are aggregated by file path with access counts.
    pub async fn query_flex(&self, input: QueryFlexInput) -> Result<QueryResult, CoreError> {
        let start = Instant::now();

        // Build the query dynamically based on filters
        // Data is in claude_sessions.files_read as JSON array, use json_each() to expand
        // chain_id is in chain_graph table, joined via session_id
        let mut sql = String::from(
            "SELECT
                json_each.value as file_path,
                COUNT(*) as total_access_count,
                MAX(s.started_at) as last_access,
                COUNT(DISTINCT s.session_id) as session_count
             FROM claude_sessions s, json_each(s.files_read)
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             WHERE s.files_read IS NOT NULL AND s.files_read != '[]'"
        );

        let mut bindings: Vec<String> = Vec::new();

        // Add time filter (using session started_at)
        if let Some(ref time) = input.time {
            let days = parse_time_range(time)?;
            sql.push_str(&format!(
                " AND s.started_at >= datetime('now', '-{} days')",
                days
            ));
        }

        // Add chain filter (via chain_graph join)
        if let Some(ref chain) = input.chain {
            sql.push_str(" AND cg.chain_id = ?");
            bindings.push(chain.clone());
        }

        // Add session filter
        if let Some(ref session) = input.session {
            sql.push_str(" AND s.session_id = ?");
            bindings.push(session.clone());
        }

        // Add file pattern filter (LIKE with wildcards)
        if let Some(ref files) = input.files {
            // Convert glob-style pattern to SQL LIKE pattern
            let pattern = files.replace('*', "%").replace('?', "_");
            sql.push_str(" AND json_each.value LIKE ?");
            bindings.push(pattern);
        }

        // Group by file_path
        sql.push_str(" GROUP BY json_each.value");

        // Add sorting
        match input.sort.as_deref() {
            Some("recency") => sql.push_str(" ORDER BY last_access DESC"),
            _ => sql.push_str(" ORDER BY total_access_count DESC"),
        }

        // Add limit
        let limit = input.limit.unwrap_or(20);
        sql.push_str(&format!(" LIMIT {}", limit));

        // Execute query with bindings
        let mut query = sqlx::query(&sql);
        for binding in &bindings {
            query = query.bind(binding);
        }

        let rows = query.fetch_all(self.db.pool()).await?;

        // Transform rows to FileResult
        let results: Vec<FileResult> = rows
            .iter()
            .map(|row| {
                FileResult {
                    file_path: row.get("file_path"),
                    access_count: row.get::<i64, _>("total_access_count") as u32,
                    last_access: row.get::<Option<String>, _>("last_access"),
                    session_count: Some(row.get::<i64, _>("session_count") as u32),
                    sessions: None,
                    chains: None,
                }
            })
            .collect();

        // Compute aggregations
        let aggregations = self.compute_aggregations(&results, &input.agg);

        let elapsed = start.elapsed();
        log::info!("query_flex completed in {:?}", elapsed);

        Ok(QueryResult {
            receipt_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            result_count: results.len(),
            results,
            aggregations,
        })
    }

    /// Query chain metadata
    ///
    /// Returns chains sorted by session count (most active first).
    pub async fn query_chains(&self, input: QueryChainsInput) -> Result<ChainQueryResult, CoreError> {
        let start = Instant::now();
        let limit = input.limit.unwrap_or(20);

        // FIX BUG-001: Compute file_count dynamically by joining to session data
        // instead of reading from stale chains.files_json column
        let sql = format!(
            "SELECT
                cg.chain_id,
                COUNT(DISTINCT cg.session_id) as session_count,
                COUNT(DISTINCT json_each.value) as file_count
             FROM chain_graph cg
             JOIN claude_sessions s ON cg.session_id = s.session_id
             LEFT JOIN json_each(s.files_read) ON s.files_read IS NOT NULL AND s.files_read != '[]'
             GROUP BY cg.chain_id
             ORDER BY session_count DESC
             LIMIT {}",
            limit
        );

        let rows = sqlx::query(&sql)
            .fetch_all(self.db.pool())
            .await?;

        let chains: Vec<ChainData> = rows
            .iter()
            .map(|row| {
                ChainData {
                    chain_id: row.get("chain_id"),
                    session_count: row.get::<i64, _>("session_count") as u32,
                    file_count: row.get::<i64, _>("file_count") as u32,
                    time_range: None, // TODO: Add time range query if needed
                }
            })
            .collect();

        let total_chains = chains.len() as u32;

        let elapsed = start.elapsed();
        log::info!("query_chains completed in {:?}", elapsed);

        Ok(ChainQueryResult { chains, total_chains })
    }

    /// Query timeline data for visualization
    ///
    /// Returns access counts bucketed by day for the specified time range.
    pub async fn query_timeline(&self, input: QueryTimelineInput) -> Result<TimelineData, CoreError> {
        let start = Instant::now();
        let days = parse_time_range(&input.time)?;
        let limit = input.limit.unwrap_or(30);

        // Get daily buckets from claude_sessions.files_read JSON
        let mut bucket_sql = format!(
            "SELECT
                date(s.started_at) as date,
                COUNT(*) as access_count,
                COUNT(DISTINCT json_each.value) as files_touched,
                GROUP_CONCAT(DISTINCT s.session_id) as sessions
             FROM claude_sessions s, json_each(s.files_read)
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             WHERE s.started_at >= datetime('now', '-{} days')
               AND s.files_read IS NOT NULL AND s.files_read != '[]'",
            days
        );

        let mut bucket_bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            bucket_sql.push_str(" AND cg.chain_id = ?");
            bucket_bindings.push(chain.clone());
        }

        bucket_sql.push_str(" GROUP BY date(s.started_at) ORDER BY date DESC");

        let mut bucket_query = sqlx::query(&bucket_sql);
        for binding in &bucket_bindings {
            bucket_query = bucket_query.bind(binding);
        }
        let bucket_rows = bucket_query.fetch_all(self.db.pool()).await?;

        let buckets: Vec<TimeBucket> = bucket_rows
            .iter()
            .filter_map(|row| {
                let date: Option<String> = row.get("date");
                date.map(|date| {
                    let sessions_str: Option<String> = row.get("sessions");
                    let sessions: Vec<String> = sessions_str
                        .map(|s| s.split(',').map(|x| x.to_string()).collect())
                        .unwrap_or_default();

                    // Parse date to get day of week
                    let day_of_week = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                        .map(|d| d.format("%A").to_string())
                        .unwrap_or_else(|_| "Unknown".to_string());

                    TimeBucket {
                        date,
                        day_of_week,
                        access_count: row.get::<i64, _>("access_count") as u32,
                        files_touched: row.get::<i64, _>("files_touched") as u32,
                        sessions,
                    }
                })
            })
            .collect();

        // Get per-file timeline data from claude_sessions.files_read JSON
        let mut file_sql = format!(
            "SELECT
                json_each.value as file_path,
                COUNT(*) as total_accesses,
                MIN(s.started_at) as first_access,
                MAX(s.started_at) as last_access
             FROM claude_sessions s, json_each(s.files_read)
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             WHERE s.started_at >= datetime('now', '-{} days')
               AND s.files_read IS NOT NULL AND s.files_read != '[]'",
            days
        );

        let mut file_bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            file_sql.push_str(" AND cg.chain_id = ?");
            file_bindings.push(chain.clone());
        }

        if let Some(ref files) = input.files {
            let pattern = files.replace('*', "%").replace('?', "_");
            file_sql.push_str(" AND json_each.value LIKE ?");
            file_bindings.push(pattern);
        }

        file_sql.push_str(&format!(
            " GROUP BY json_each.value
             ORDER BY total_accesses DESC
             LIMIT {}",
            limit
        ));

        let mut file_query = sqlx::query(&file_sql);
        for binding in &file_bindings {
            file_query = file_query.bind(binding);
        }
        let file_rows = file_query.fetch_all(self.db.pool()).await?;

        // Query per-file, per-date bucket counts
        // This populates the file.buckets HashMap for heat map rendering
        let mut per_file_bucket_sql = format!(
            "SELECT
                json_each.value as file_path,
                date(s.started_at) as date,
                COUNT(*) as count
             FROM claude_sessions s, json_each(s.files_read)
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             WHERE s.started_at >= datetime('now', '-{} days')
               AND s.files_read IS NOT NULL AND s.files_read != '[]'",
            days
        );

        let mut bucket_bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            per_file_bucket_sql.push_str(" AND cg.chain_id = ?");
            bucket_bindings.push(chain.clone());
        }

        per_file_bucket_sql.push_str(" GROUP BY json_each.value, date(s.started_at)");

        let mut bucket_query = sqlx::query(&per_file_bucket_sql);
        for binding in &bucket_bindings {
            bucket_query = bucket_query.bind(binding);
        }
        let bucket_rows = bucket_query.fetch_all(self.db.pool()).await?;

        // Build HashMap: file_path -> (date -> count)
        let mut per_file_buckets: std::collections::HashMap<String, std::collections::HashMap<String, u32>> =
            std::collections::HashMap::new();

        for row in &bucket_rows {
            let file_path: String = row.get("file_path");
            let date: String = row.get("date");
            let count: i64 = row.get("count");

            per_file_buckets
                .entry(file_path)
                .or_insert_with(std::collections::HashMap::new)
                .insert(date, count as u32);
        }

        let files: Vec<FileTimeline> = file_rows
            .iter()
            .map(|row| {
                let file_path: String = row.get("file_path");
                let buckets = per_file_buckets
                    .get(&file_path)
                    .cloned()
                    .unwrap_or_default();

                FileTimeline {
                    file_path,
                    total_accesses: row.get::<i64, _>("total_accesses") as u32,
                    buckets,
                    first_access: row.get::<Option<String>, _>("first_access").unwrap_or_default(),
                    last_access: row.get::<Option<String>, _>("last_access").unwrap_or_default(),
                }
            })
            .collect();

        // Compute summary
        let total_accesses: u32 = buckets.iter().map(|b| b.access_count).sum();
        let total_files = files.len() as u32;
        let (peak_day, peak_count) = buckets
            .iter()
            .max_by_key(|b| b.access_count)
            .map(|b| (b.date.clone(), b.access_count))
            .unwrap_or_else(|| ("".to_string(), 0));

        let summary = TimelineSummary {
            total_accesses,
            total_files,
            peak_day,
            peak_count,
        };

        // Compute date range
        let end_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let start_date = (chrono::Utc::now() - chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();

        let elapsed = start.elapsed();
        log::info!("query_timeline completed in {:?}", elapsed);

        Ok(TimelineData {
            time_range: input.time.clone(),
            start_date,
            end_date,
            buckets,
            files,
            summary,
        })
    }

    /// Query session-grouped data
    ///
    /// Returns file accesses grouped by session.
    pub async fn query_sessions(&self, input: QuerySessionsInput) -> Result<SessionQueryResult, CoreError> {
        let start = Instant::now();
        let days = parse_time_range(&input.time)?;
        let limit = input.limit.unwrap_or(50);

        // Build query for sessions - use subquery to count files from JSON
        let mut sql = format!(
            "SELECT
                s.session_id,
                cg.chain_id,
                s.started_at,
                s.ended_at,
                CASE
                    WHEN s.files_read IS NULL OR s.files_read = '[]' THEN 0
                    ELSE (SELECT COUNT(*) FROM json_each(s.files_read))
                END as file_count,
                CASE
                    WHEN s.files_read IS NULL OR s.files_read = '[]' THEN 0
                    ELSE (SELECT COUNT(*) FROM json_each(s.files_read))
                END as total_accesses
             FROM claude_sessions s
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             WHERE s.started_at >= datetime('now', '-{} days')",
            days
        );

        let mut bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            sql.push_str(" AND cg.chain_id = ?");
            bindings.push(chain.clone());
        }

        sql.push_str(&format!(
            " GROUP BY s.session_id
             ORDER BY s.started_at DESC
             LIMIT {}",
            limit
        ));

        let mut query = sqlx::query(&sql);
        for binding in &bindings {
            query = query.bind(binding);
        }
        let session_rows = query.fetch_all(self.db.pool()).await?;

        let mut sessions: Vec<SessionData> = Vec::new();
        for row in &session_rows {
            let session_id: String = row.get("session_id");
            let started_at: String = row.get::<Option<String>, _>("started_at").unwrap_or_default();
            let ended_at: Option<String> = row.get("ended_at");

            // Calculate duration if we have both timestamps
            let duration_seconds = if let (Ok(start), Some(Ok(end))) = (
                chrono::DateTime::parse_from_rfc3339(&started_at),
                ended_at.as_ref().map(|e| chrono::DateTime::parse_from_rfc3339(e))
            ) {
                Some((end - start).num_seconds() as u32)
            } else {
                None
            };

            // Get top files for this session from files_read JSON
            let files_sql = "SELECT
                    json_each.value as file_path,
                    1 as access_count,
                    s.started_at as first_accessed_at
                 FROM claude_sessions s, json_each(s.files_read)
                 WHERE s.session_id = ?
                 LIMIT 5";

            let file_rows = sqlx::query(files_sql)
                .bind(&session_id)
                .fetch_all(self.db.pool())
                .await?;

            let top_files: Vec<SessionFile> = file_rows
                .iter()
                .map(|r| SessionFile {
                    file_path: r.get("file_path"),
                    access_count: r.get::<i64, _>("access_count") as u32,
                    access_types: vec![], // TODO: Get from actual data if available
                    last_access: r.get::<Option<String>, _>("first_accessed_at").unwrap_or_default(),
                })
                .collect();

            sessions.push(SessionData {
                session_id,
                chain_id: row.get("chain_id"),
                started_at,
                ended_at,
                duration_seconds,
                file_count: row.get::<i64, _>("file_count") as u32,
                total_accesses: row.get::<i64, _>("total_accesses") as u32,
                files: vec![], // Full file list not included by default
                top_files,
            });
        }

        // Get chain summaries via chain_graph join with claude_sessions JSON
        let chain_sql = format!(
            "SELECT
                cg.chain_id,
                COUNT(DISTINCT cg.session_id) as session_count,
                COUNT(DISTINCT json_each.value) as file_count,
                MAX(s.started_at) as last_active
             FROM chain_graph cg
             JOIN claude_sessions s ON cg.session_id = s.session_id
             LEFT JOIN json_each(s.files_read) ON s.files_read IS NOT NULL AND s.files_read != '[]'
             WHERE s.started_at >= datetime('now', '-{} days')
               AND cg.chain_id IS NOT NULL
             GROUP BY cg.chain_id
             ORDER BY last_active DESC",
            days
        );

        let chain_rows = sqlx::query(&chain_sql)
            .fetch_all(self.db.pool())
            .await?;

        let chains: Vec<ChainSummary> = chain_rows
            .iter()
            .map(|row| ChainSummary {
                chain_id: row.get("chain_id"),
                session_count: row.get::<i64, _>("session_count") as u32,
                file_count: row.get::<i64, _>("file_count") as u32,
                last_active: row.get::<Option<String>, _>("last_active").unwrap_or_default(),
            })
            .collect();

        // Compute summary
        let total_sessions = sessions.len() as u32;
        let total_files: u32 = sessions.iter().map(|s| s.file_count).sum();
        let total_accesses: u32 = sessions.iter().map(|s| s.total_accesses).sum();
        let active_chains = chains.len() as u32;

        let summary = SessionSummary {
            total_sessions,
            total_files,
            total_accesses,
            active_chains,
        };

        let elapsed = start.elapsed();
        log::info!("query_sessions completed in {:?}", elapsed);

        Ok(SessionQueryResult {
            time_range: input.time.clone(),
            sessions,
            chains,
            summary,
        })
    }

    /// Compute aggregations for query results
    fn compute_aggregations(&self, results: &[FileResult], agg_types: &[String]) -> Aggregations {
        compute_aggregations(results, agg_types)
    }
}

/// Compute aggregations for query results (standalone for testing)
pub fn compute_aggregations(results: &[FileResult], agg_types: &[String]) -> Aggregations {
    let mut aggregations = Aggregations::default();

    for agg in agg_types {
        match agg.as_str() {
            "count" => {
                let total_files = results.len() as u32;
                let total_accesses: u32 = results.iter().map(|r| r.access_count).sum();
                aggregations.count = Some(CountAgg {
                    total_files,
                    total_accesses,
                });
            }
            "recency" => {
                if let (Some(newest), Some(oldest)) = (
                    results.iter().filter_map(|r| r.last_access.as_ref()).max(),
                    results.iter().filter_map(|r| r.last_access.as_ref()).min(),
                ) {
                    aggregations.recency = Some(RecencyAgg {
                        newest: newest.clone(),
                        oldest: oldest.clone(),
                    });
                }
            }
            _ => {} // Unknown aggregation type, ignore
        }
    }

    aggregations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_aggregations_count() {
        let results = vec![
            FileResult {
                file_path: "a.rs".to_string(),
                access_count: 10,
                last_access: Some("2026-01-08".to_string()),
                session_count: None,
                sessions: None,
                chains: None,
            },
            FileResult {
                file_path: "b.rs".to_string(),
                access_count: 5,
                last_access: Some("2026-01-07".to_string()),
                session_count: None,
                sessions: None,
                chains: None,
            },
        ];

        let agg = compute_aggregations(&results, &["count".to_string()]);
        assert!(agg.count.is_some());
        let count = agg.count.unwrap();
        assert_eq!(count.total_files, 2);
        assert_eq!(count.total_accesses, 15);
    }

    #[test]
    fn test_compute_aggregations_recency() {
        let results = vec![
            FileResult {
                file_path: "a.rs".to_string(),
                access_count: 10,
                last_access: Some("2026-01-08".to_string()),
                session_count: None,
                sessions: None,
                chains: None,
            },
            FileResult {
                file_path: "b.rs".to_string(),
                access_count: 5,
                last_access: Some("2026-01-05".to_string()),
                session_count: None,
                sessions: None,
                chains: None,
            },
        ];

        let agg = compute_aggregations(&results, &["recency".to_string()]);
        assert!(agg.recency.is_some());
        let recency = agg.recency.unwrap();
        assert_eq!(recency.newest, "2026-01-08");
        assert_eq!(recency.oldest, "2026-01-05");
    }
}
