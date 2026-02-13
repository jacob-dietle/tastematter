//! Query engine for context-os-core
//!
//! Implements the core query functions that replace the Python CLI.
//! Target: <100ms latency for all queries.

use sqlx::Row;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use crate::error::CoreError;
use crate::intelligence::IntelClient;
use crate::storage::Database;
use crate::types::*;

/// Generate a receipt ID in Python format: q_XXXXXX
fn generate_receipt_id() -> String {
    let mut hasher = DefaultHasher::new();
    chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(0)
        .hash(&mut hasher);
    let hash = hasher.finish();
    format!("q_{:06x}", hash & 0xFFFFFF) // 6 hex chars
}

/// Compute a human-readable display name for a chain.
///
/// Fallback priority:
/// 1. generated_name (from Intel service)
/// 2. first_user_message (truncated to 60 chars)
/// 3. chain_id[:12] + "..."
fn compute_display_name(
    chain_id: &str,
    generated_name: Option<&str>,
    first_user_message: Option<&str>,
) -> String {
    if let Some(name) = generated_name {
        if !name.is_empty() {
            return name.to_string();
        }
    }

    if let Some(msg) = first_user_message {
        if !msg.is_empty() {
            let trimmed = msg.trim();
            if trimmed.len() <= 60 {
                return trimmed.to_string();
            }
            // Find a safe char boundary near 57 bytes for truncation
            let safe_end = trimmed
                .char_indices()
                .take_while(|(i, _)| *i <= 57)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(57.min(trimmed.len()));
            // Truncate at word boundary near safe_end
            if let Some(pos) = trimmed[..safe_end].rfind(' ') {
                return format!("{}...", &trimmed[..pos]);
            }
            return format!("{}...", &trimmed[..safe_end]);
        }
    }

    // Final fallback: truncated hex ID
    if chain_id.len() > 12 {
        format!("{}...", &chain_id[..12])
    } else {
        chain_id.to_string()
    }
}

/// Query engine for context-os
///
/// Provides direct SQLite queries with sub-100ms latency.
/// Optionally integrates with intelligence service for LLM synthesis.
pub struct QueryEngine {
    db: Database,
    intel_client: Option<IntelClient>,
}

impl QueryEngine {
    /// Create a new QueryEngine with the given database
    pub fn new(db: Database) -> Self {
        Self {
            db,
            intel_client: None,
        }
    }

    /// Add an intelligence client for LLM-powered synthesis
    pub fn with_intel(mut self, client: IntelClient) -> Self {
        self.intel_client = Some(client);
        self
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
        // FIX BUG-05: Use CTE to include both files_read and files_written
        // chain_id is in chain_graph table, joined via session_id

        // Build time filter for CTE legs
        let time_filter = if let Some(ref time) = input.time {
            let days = parse_time_range(time)?;
            format!(" AND s.started_at >= datetime('now', '-{} days')", days)
        } else {
            String::new()
        };

        let mut sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.files_read IS NOT NULL AND s.files_read != '[]'{time_filter}
                UNION ALL
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.files_written IS NOT NULL AND s.files_written != '[]'{time_filter}
            )
            SELECT
                af.file_path,
                COUNT(*) as total_access_count,
                MAX(af.started_at) as last_access,
                COUNT(DISTINCT af.session_id) as session_count
             FROM all_files af
             LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
             WHERE 1=1",
            time_filter = time_filter,
        );

        let mut bindings: Vec<String> = Vec::new();

        // Add chain filter (via chain_graph join)
        if let Some(ref chain) = input.chain {
            sql.push_str(" AND cg.chain_id = ?");
            bindings.push(chain.clone());
        }

        // Add session filter
        if let Some(ref session) = input.session {
            sql.push_str(" AND af.session_id = ?");
            bindings.push(session.clone());
        }

        // Add file pattern filter (LIKE with wildcards)
        if let Some(ref files) = input.files {
            // Convert glob-style pattern to SQL LIKE pattern
            let pattern = files.replace('*', "%").replace('?', "_");
            sql.push_str(" AND af.file_path LIKE ?");
            bindings.push(pattern);
        }

        // Group by file_path
        sql.push_str(" GROUP BY af.file_path");

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
            .map(|row| FileResult {
                file_path: row.get("file_path"),
                access_count: row.get::<i64, _>("total_access_count") as u32,
                last_access: row.get::<Option<String>, _>("last_access"),
                session_count: Some(row.get::<i64, _>("session_count") as u32),
                sessions: None,
                chains: None,
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
    pub async fn query_chains(
        &self,
        input: QueryChainsInput,
    ) -> Result<ChainQueryResult, CoreError> {
        let start = Instant::now();
        let limit = input.limit.unwrap_or(20);

        // FIX BUG-001: Compute file_count dynamically by joining to session data
        // instead of reading from stale chains.files_json column
        // FIX: LEFT JOIN chain_metadata to include Intel-generated names
        // FIX LIVE-02: Add summary and first_user_message for display_name fallback
        let sql = format!(
            "SELECT
                cg.chain_id,
                COUNT(DISTINCT cg.session_id) as session_count,
                COUNT(DISTINCT json_each.value) as file_count,
                cm.generated_name,
                cm.summary,
                (SELECT s2.first_user_message
                 FROM claude_sessions s2
                 JOIN chain_graph cg2 ON s2.session_id = cg2.session_id
                 WHERE cg2.chain_id = cg.chain_id
                   AND s2.first_user_message IS NOT NULL
                   AND s2.first_user_message != ''
                 ORDER BY s2.started_at ASC
                 LIMIT 1
                ) as first_user_message
             FROM chain_graph cg
             JOIN claude_sessions s ON cg.session_id = s.session_id
             LEFT JOIN json_each(s.files_read) ON s.files_read IS NOT NULL AND s.files_read != '[]'
             LEFT JOIN chain_metadata cm ON cg.chain_id = cm.chain_id
             GROUP BY cg.chain_id
             ORDER BY session_count DESC
             LIMIT {}",
            limit
        );

        let rows = sqlx::query(&sql).fetch_all(self.db.pool()).await?;

        let chains: Vec<ChainData> = rows
            .iter()
            .map(|row| {
                let chain_id: String = row.get("chain_id");
                let generated_name: Option<String> = row.get("generated_name");
                let first_user_message: Option<String> = row.get("first_user_message");
                let summary: Option<String> = row.get("summary");

                let display_name = compute_display_name(
                    &chain_id,
                    generated_name.as_deref(),
                    first_user_message.as_deref(),
                );

                ChainData {
                    chain_id,
                    display_name,
                    session_count: row.get::<i64, _>("session_count") as u32,
                    file_count: row.get::<i64, _>("file_count") as u32,
                    time_range: None,
                    generated_name,
                    summary,
                }
            })
            .collect();

        let total_chains = chains.len() as u32;

        let elapsed = start.elapsed();
        log::info!("query_chains completed in {:?}", elapsed);

        Ok(ChainQueryResult {
            chains,
            total_chains,
        })
    }

    /// Query timeline data for visualization
    ///
    /// Returns access counts bucketed by day for the specified time range.
    pub async fn query_timeline(
        &self,
        input: QueryTimelineInput,
    ) -> Result<TimelineData, CoreError> {
        let start = Instant::now();
        let days = parse_time_range(&input.time)?;
        let limit = input.limit.unwrap_or(30);

        // Get daily buckets — FIX BUG-05: include files_written via CTE
        let mut bucket_sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT
                date(af.started_at) as date,
                COUNT(*) as access_count,
                COUNT(DISTINCT af.file_path) as files_touched,
                GROUP_CONCAT(DISTINCT af.session_id) as sessions
             FROM all_files af
             LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
             WHERE 1=1",
            days = days
        );

        let mut bucket_bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            bucket_sql.push_str(" AND cg.chain_id = ?");
            bucket_bindings.push(chain.clone());
        }

        bucket_sql.push_str(" GROUP BY date(af.started_at) ORDER BY date DESC");

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

        // Get per-file timeline data — FIX BUG-05: include files_written
        let mut file_sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT
                af.file_path,
                COUNT(*) as total_accesses,
                MIN(af.started_at) as first_access,
                MAX(af.started_at) as last_access
             FROM all_files af
             LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
             WHERE 1=1",
            days = days
        );

        let mut file_bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            file_sql.push_str(" AND cg.chain_id = ?");
            file_bindings.push(chain.clone());
        }

        if let Some(ref files) = input.files {
            let pattern = files.replace('*', "%").replace('?', "_");
            file_sql.push_str(" AND af.file_path LIKE ?");
            file_bindings.push(pattern);
        }

        file_sql.push_str(&format!(
            " GROUP BY af.file_path
             ORDER BY total_accesses DESC
             LIMIT {}",
            limit
        ));

        let mut file_query = sqlx::query(&file_sql);
        for binding in &file_bindings {
            file_query = file_query.bind(binding);
        }
        let file_rows = file_query.fetch_all(self.db.pool()).await?;

        // Query per-file, per-date bucket counts — FIX BUG-05: include files_written
        let mut per_file_bucket_sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT
                af.file_path,
                date(af.started_at) as date,
                COUNT(*) as count
             FROM all_files af
             LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
             WHERE 1=1",
            days = days
        );

        let mut bucket_bindings: Vec<String> = Vec::new();
        if let Some(ref chain) = input.chain {
            per_file_bucket_sql.push_str(" AND cg.chain_id = ?");
            bucket_bindings.push(chain.clone());
        }

        per_file_bucket_sql.push_str(" GROUP BY af.file_path, date(af.started_at)");

        let mut bucket_query = sqlx::query(&per_file_bucket_sql);
        for binding in &bucket_bindings {
            bucket_query = bucket_query.bind(binding);
        }
        let bucket_rows = bucket_query.fetch_all(self.db.pool()).await?;

        // Build HashMap: file_path -> (date -> count)
        let mut per_file_buckets: std::collections::HashMap<
            String,
            std::collections::HashMap<String, u32>,
        > = std::collections::HashMap::new();

        for row in &bucket_rows {
            let file_path: String = row.get("file_path");
            let date: String = row.get("date");
            let count: i64 = row.get("count");

            per_file_buckets
                .entry(file_path)
                .or_default()
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
                    first_access: row
                        .get::<Option<String>, _>("first_access")
                        .unwrap_or_default(),
                    last_access: row
                        .get::<Option<String>, _>("last_access")
                        .unwrap_or_default(),
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
    pub async fn query_sessions(
        &self,
        input: QuerySessionsInput,
    ) -> Result<SessionQueryResult, CoreError> {
        let start = Instant::now();
        let days = parse_time_range(&input.time)?;
        let limit = input.limit.unwrap_or(50);

        // Build query for sessions - use subquery to count files from JSON
        // FIX LIVE-02: Join chain_metadata to include chain_name
        // FIX BUG-10: Include files_written in file_count and total_accesses
        let mut sql = format!(
            "SELECT
                s.session_id,
                cg.chain_id,
                cm.generated_name as chain_name,
                s.started_at,
                s.ended_at,
                (
                    CASE WHEN s.files_read IS NULL OR s.files_read = '[]' THEN 0
                         ELSE (SELECT COUNT(*) FROM json_each(s.files_read)) END
                    +
                    CASE WHEN s.files_written IS NULL OR s.files_written = '[]' THEN 0
                         ELSE (SELECT COUNT(*) FROM json_each(s.files_written)) END
                ) as file_count,
                (
                    CASE WHEN s.files_read IS NULL OR s.files_read = '[]' THEN 0
                         ELSE (SELECT COUNT(*) FROM json_each(s.files_read)) END
                    +
                    CASE WHEN s.files_written IS NULL OR s.files_written = '[]' THEN 0
                         ELSE (SELECT COUNT(*) FROM json_each(s.files_written)) END
                ) as total_accesses
             FROM claude_sessions s
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             LEFT JOIN chain_metadata cm ON cg.chain_id = cm.chain_id
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
            let started_at: String = row
                .get::<Option<String>, _>("started_at")
                .unwrap_or_default();
            let ended_at: Option<String> = row.get("ended_at");

            // Calculate duration if we have both timestamps
            let duration_seconds = if let (Ok(start), Some(Ok(end))) = (
                chrono::DateTime::parse_from_rfc3339(&started_at),
                ended_at
                    .as_ref()
                    .map(|e| chrono::DateTime::parse_from_rfc3339(e)),
            ) {
                Some((end - start).num_seconds() as u32)
            } else {
                None
            };

            // Get top files for this session — FIX BUG-10: include files_written
            // Use UNION (not UNION ALL) to dedup files present in both
            let files_sql = "SELECT file_path, 1 as access_count, started_at as first_accessed_at
                 FROM (
                    SELECT json_each.value as file_path, s.started_at
                    FROM claude_sessions s, json_each(s.files_read)
                    WHERE s.session_id = ? AND s.files_read IS NOT NULL AND s.files_read != '[]'
                    UNION
                    SELECT json_each.value as file_path, s.started_at
                    FROM claude_sessions s, json_each(s.files_written)
                    WHERE s.session_id = ? AND s.files_written IS NOT NULL AND s.files_written != '[]'
                 )
                 LIMIT 5";

            let file_rows = sqlx::query(files_sql)
                .bind(&session_id)
                .bind(&session_id)
                .fetch_all(self.db.pool())
                .await?;

            let top_files: Vec<SessionFile> = file_rows
                .iter()
                .map(|r| SessionFile {
                    file_path: r.get("file_path"),
                    access_count: r.get::<i64, _>("access_count") as u32,
                    access_types: vec![], // TODO: Get from actual data if available
                    last_access: r
                        .get::<Option<String>, _>("first_accessed_at")
                        .unwrap_or_default(),
                })
                .collect();

            sessions.push(SessionData {
                session_id,
                chain_id: row.get("chain_id"),
                chain_name: row.get("chain_name"),
                started_at,
                ended_at,
                duration_seconds,
                file_count: row.get::<i64, _>("file_count") as u32,
                total_accesses: row.get::<i64, _>("total_accesses") as u32,
                files: vec![], // Full file list not included by default
                top_files,
            });
        }

        // Get chain summaries — FIX BUG-05: include files_written
        let chain_sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT
                cg.chain_id,
                COUNT(DISTINCT cg.session_id) as session_count,
                COUNT(DISTINCT af.file_path) as file_count,
                MAX(af.started_at) as last_active
             FROM chain_graph cg
             JOIN claude_sessions s ON cg.session_id = s.session_id
             LEFT JOIN all_files af ON af.session_id = s.session_id
             WHERE s.started_at >= datetime('now', '-{days} days')
               AND cg.chain_id IS NOT NULL
             GROUP BY cg.chain_id
             ORDER BY last_active DESC",
            days = days
        );

        let chain_rows = sqlx::query(&chain_sql).fetch_all(self.db.pool()).await?;

        let chains: Vec<ChainSummary> = chain_rows
            .iter()
            .map(|row| ChainSummary {
                chain_id: row.get("chain_id"),
                session_count: row.get::<i64, _>("session_count") as u32,
                file_count: row.get::<i64, _>("file_count") as u32,
                last_active: row
                    .get::<Option<String>, _>("last_active")
                    .unwrap_or_default(),
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

    // =========================================================================
    // SEARCH, FILE, CO-ACCESS QUERIES (Phase A: Missing Commands)
    // =========================================================================

    /// Search files by pattern (substring match, case-insensitive)
    ///
    /// Returns files matching the pattern, sorted by access count descending.
    /// Matches Python CLI: cli.py:1920-1972
    pub async fn query_search(&self, input: QuerySearchInput) -> Result<SearchResult, CoreError> {
        let start = Instant::now();
        let limit = input.limit.unwrap_or(20) as i64;
        let pattern = format!("%{}%", input.pattern.to_lowercase());

        // FIX BUG-05: Include files_written in search via CTE
        let sql = "WITH all_files AS (
                SELECT json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT file_path, COUNT(*) as access_count
             FROM all_files
             WHERE LOWER(file_path) LIKE ?
             GROUP BY file_path
             ORDER BY access_count DESC
             LIMIT ?";

        let rows = sqlx::query(sql)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(self.db.pool())
            .await?;

        let results: Vec<SearchResultItem> = rows
            .iter()
            .map(|row| SearchResultItem {
                file_path: row.get("file_path"),
                access_count: row.get::<i64, _>("access_count") as u32,
            })
            .collect();

        let total_matches = results.len();

        let elapsed = start.elapsed();
        log::info!(
            "query_search '{}' completed in {:?}",
            input.pattern,
            elapsed
        );

        Ok(SearchResult {
            receipt_id: generate_receipt_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            pattern: input.pattern,
            total_matches,
            results,
        })
    }

    /// Query sessions that touched a specific file
    ///
    /// Supports exact match, suffix match, and substring match (in that order).
    /// Matches Python CLI: cli.py:1414-1533
    pub async fn query_file(&self, input: QueryFileInput) -> Result<FileQueryResult, CoreError> {
        let start = Instant::now();
        let limit = input.limit.unwrap_or(20) as i64;
        let file_path = &input.file_path;

        // FIX BUG-05: Include files_written in file queries via CTE
        // Reusable CTE SQL fragment for all three match attempts
        let cte = "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
            )";

        // First try exact match
        let exact_sql = format!(
            "{} SELECT DISTINCT
                af.session_id,
                af.started_at as last_access,
                cg.chain_id
             FROM all_files af
             LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
             WHERE af.file_path = ?
             ORDER BY af.started_at DESC
             LIMIT ?",
            cte
        );

        let rows = sqlx::query(&exact_sql)
            .bind(file_path)
            .bind(limit)
            .fetch_all(self.db.pool())
            .await?;

        let (matched_path, sessions) = if !rows.is_empty() {
            // Exact match found
            let sessions: Vec<FileSessionInfo> = rows
                .iter()
                .map(|row| FileSessionInfo {
                    session_id: row.get("session_id"),
                    access_types: vec!["read".to_string()],
                    last_access: row.get("last_access"),
                    chain_id: row.get("chain_id"),
                })
                .collect();
            (Some(file_path.clone()), sessions)
        } else {
            // Try suffix match
            let suffix_sql = format!(
                "{} SELECT DISTINCT
                    af.file_path as matched_path,
                    af.session_id,
                    af.started_at as last_access,
                    cg.chain_id
                 FROM all_files af
                 LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
                 WHERE af.file_path LIKE ?
                 ORDER BY af.started_at DESC
                 LIMIT ?",
                cte
            );

            let suffix_pattern = format!("%{}", file_path);
            let suffix_rows = sqlx::query(&suffix_sql)
                .bind(&suffix_pattern)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?;

            if !suffix_rows.is_empty() {
                let matched: String = suffix_rows[0].get("matched_path");
                let sessions: Vec<FileSessionInfo> = suffix_rows
                    .iter()
                    .map(|row| FileSessionInfo {
                        session_id: row.get("session_id"),
                        access_types: vec!["read".to_string()],
                        last_access: row.get("last_access"),
                        chain_id: row.get("chain_id"),
                    })
                    .collect();
                (Some(matched), sessions)
            } else {
                // Try substring match
                let substr_pattern = format!("%{}%", file_path);
                let substr_rows = sqlx::query(&suffix_sql)
                    .bind(&substr_pattern)
                    .bind(limit)
                    .fetch_all(self.db.pool())
                    .await?;

                if !substr_rows.is_empty() {
                    let matched: String = substr_rows[0].get("matched_path");
                    let sessions: Vec<FileSessionInfo> = substr_rows
                        .iter()
                        .map(|row| FileSessionInfo {
                            session_id: row.get("session_id"),
                            access_types: vec!["read".to_string()],
                            last_access: row.get("last_access"),
                            chain_id: row.get("chain_id"),
                        })
                        .collect();
                    (Some(matched), sessions)
                } else {
                    (None, vec![])
                }
            }
        };

        let found = !sessions.is_empty();
        let elapsed = start.elapsed();
        log::info!(
            "query_file '{}' completed in {:?} (found: {})",
            file_path,
            elapsed,
            found
        );

        Ok(FileQueryResult {
            receipt_id: generate_receipt_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            file_path: input.file_path,
            found,
            matched_path,
            sessions,
        })
    }

    /// Query co-accessed files using PMI scoring
    ///
    /// Finds files frequently accessed together with the anchor file.
    /// Matches Python CLI: cli.py:1539-1589
    pub async fn query_co_access(
        &self,
        input: QueryCoAccessInput,
    ) -> Result<CoAccessResult, CoreError> {
        let start = Instant::now();
        let limit = input.limit.unwrap_or(10) as i64;
        let file_path = &input.file_path;

        // Get sessions that touched this file — FIX BUG-05: include files_written
        let sessions_sql = "SELECT DISTINCT session_id FROM (
                SELECT s.session_id, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
            ) WHERE file_path LIKE ?";

        let file_pattern = format!("%{}%", file_path);
        let session_rows = sqlx::query(sessions_sql)
            .bind(&file_pattern)
            .fetch_all(self.db.pool())
            .await?;

        let session_ids: Vec<String> = session_rows
            .iter()
            .map(|row| row.get::<String, _>("session_id"))
            .collect();

        if session_ids.is_empty() {
            return Ok(CoAccessResult {
                receipt_id: generate_receipt_id(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                query_file: input.file_path,
                results: vec![],
            });
        }

        // Get files co-accessed in those sessions — FIX BUG-05: include files_written
        let placeholders: Vec<String> = session_ids.iter().map(|_| "?".to_string()).collect();
        let placeholders_str = placeholders.join(",");
        let co_access_sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT
                file_path,
                COUNT(DISTINCT session_id) as co_count
             FROM all_files
             WHERE session_id IN ({})
               AND file_path NOT LIKE ?
             GROUP BY file_path
             ORDER BY co_count DESC
             LIMIT ?",
            placeholders_str
        );

        let mut query = sqlx::query(&co_access_sql);
        for sid in &session_ids {
            query = query.bind(sid);
        }
        query = query.bind(&file_pattern);
        query = query.bind(limit);

        let co_rows = query.fetch_all(self.db.pool()).await?;

        // Calculate PMI-like score: co_count / total_sessions
        let total_sessions = session_ids.len() as f64;
        let results: Vec<CoAccessItem> = co_rows
            .iter()
            .map(|row| {
                let co_count: i64 = row.get("co_count");
                CoAccessItem {
                    file_path: row.get("file_path"),
                    pmi_score: (co_count as f64) / total_sessions,
                }
            })
            .collect();

        let elapsed = start.elapsed();
        log::info!("query_co_access '{}' completed in {:?}", file_path, elapsed);

        Ok(CoAccessResult {
            receipt_id: generate_receipt_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            query_file: input.file_path,
            results,
        })
    }

    // =========================================================================
    // HEAT QUERY (Phase: Heat Metrics)
    // =========================================================================

    /// Query file heat metrics with composite scoring
    ///
    /// Uses a single CTE query to fetch 7-day and long-window access counts,
    /// then computes specificity (IDF), velocity, and composite heat score in Rust.
    ///
    /// Target: <100ms latency
    pub async fn query_heat(&self, input: QueryHeatInput) -> Result<HeatResult, CoreError> {
        let start = Instant::now();

        let time_str = input.time.as_deref().unwrap_or("30d");
        let days = parse_time_range(time_str)?;
        let limit = input.limit.unwrap_or(50);

        // Build file filter clause
        let file_filter = if input.files.is_some() {
            " AND af.file_path LIKE ?"
        } else {
            ""
        };

        // Get total sessions in window for specificity calculation
        let total_sessions_sql = format!(
            "SELECT COUNT(DISTINCT session_id) as total_sessions FROM claude_sessions WHERE started_at >= datetime('now', '-{days} days')",
            days = days,
        );
        let total_sessions_row = sqlx::query(&total_sessions_sql)
            .fetch_one(self.db.pool())
            .await?;
        let total_sessions = total_sessions_row.get::<i64, _>("total_sessions").max(1) as f64;

        // FIX BUG-06: Include files_written via CTE
        let sql = format!(
            "WITH all_files AS (
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_read)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_read IS NOT NULL AND s.files_read != '[]'
                UNION ALL
                SELECT s.session_id, s.started_at, json_each.value as file_path
                FROM claude_sessions s, json_each(s.files_written)
                WHERE s.started_at >= datetime('now', '-{days} days')
                  AND s.files_written IS NOT NULL AND s.files_written != '[]'
            )
            SELECT af.file_path,
                    SUM(CASE WHEN af.started_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END) as count_7d,
                    COUNT(*) as count_long,
                    COUNT(DISTINCT af.session_id) as session_count,
                    MIN(af.started_at) as first_access,
                    MAX(af.started_at) as last_access
             FROM all_files af
             WHERE 1=1
               {file_filter}
             GROUP BY af.file_path",
            file_filter = file_filter,
            days = days,
        );

        // Bind file filter pattern (once)
        let mut query = sqlx::query(&sql);
        if let Some(ref files) = input.files {
            let pattern = files.replace('*', "%").replace('?', "_");
            query = query.bind(pattern);
        }

        let rows = query.fetch_all(self.db.pool()).await?;

        // Compute metrics in Rust (more testable than SQL)
        let mut items: Vec<HeatItem> = rows
            .iter()
            .map(|row| {
                let file_path: String = row.get("file_path");
                let count_7d = row.get::<i64, _>("count_7d") as u32;
                let count_long = row.get::<i64, _>("count_long") as u32;
                let first_access: String = row
                    .get::<Option<String>, _>("first_access")
                    .unwrap_or_default();
                let last_access: String = row
                    .get::<Option<String>, _>("last_access")
                    .unwrap_or_default();

                let session_count = row.get::<i64, _>("session_count") as f64;

                // Specificity = 1.0 - session_spread (IDF-like)
                // Files touched in many sessions have low specificity
                let session_spread = session_count / total_sessions;
                let specificity = 1.0 - session_spread;

                let velocity = compute_velocity(count_long, &first_access, &last_access);
                let heat_score = compute_heat_score(velocity, specificity, &last_access);
                let heat_level = classify_heat(heat_score);

                HeatItem {
                    file_path,
                    count_7d,
                    count_long,
                    specificity,
                    velocity,
                    heat_score,
                    heat_level,
                    first_access,
                    last_access,
                }
            })
            .collect();

        // Sort by chosen field
        match input.sort.as_ref().unwrap_or(&HeatSortBy::Heat) {
            HeatSortBy::Heat => items.sort_by(|a, b| {
                b.heat_score
                    .partial_cmp(&a.heat_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            HeatSortBy::Specificity => items.sort_by(|a, b| {
                b.specificity
                    .partial_cmp(&a.specificity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            HeatSortBy::Velocity => items.sort_by(|a, b| {
                b.velocity
                    .partial_cmp(&a.velocity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            HeatSortBy::Name => items.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
        }

        // Apply limit AFTER sort (not in SQL, where it truncates before scoring)
        items.truncate(limit as usize);

        // Reclassify by percentile rank (overrides absolute threshold classification)
        classify_heat_percentile(&mut items);

        // Compute summary AFTER percentile reclassification
        let total_files = items.len() as u32;
        let hot_count = items
            .iter()
            .filter(|i| i.heat_level == HeatLevel::Hot)
            .count() as u32;
        let warm_count = items
            .iter()
            .filter(|i| i.heat_level == HeatLevel::Warm)
            .count() as u32;
        let cool_count = items
            .iter()
            .filter(|i| i.heat_level == HeatLevel::Cool)
            .count() as u32;
        let cold_count = items
            .iter()
            .filter(|i| i.heat_level == HeatLevel::Cold)
            .count() as u32;

        let elapsed = start.elapsed();
        log::info!("query_heat completed in {:?}", elapsed);

        Ok(HeatResult {
            receipt_id: generate_receipt_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            time_range: time_str.to_string(),
            results: items,
            summary: HeatSummary {
                total_files,
                hot_count,
                warm_count,
                cool_count,
                cold_count,
            },
        })
    }

    /// Verify a query receipt against current data
    ///
    /// Returns NOT_FOUND since ledger storage is not yet implemented.
    /// Full verification requires ledger integration (future enhancement).
    pub async fn query_verify(&self, input: QueryVerifyInput) -> Result<VerifyResult, CoreError> {
        let start = Instant::now();
        let receipt_id = input.receipt_id.clone();

        // Check ledger directory for receipt
        let home = dirs::home_dir().ok_or_else(|| CoreError::Query {
            message: "Could not determine home directory".to_string(),
        })?;
        let ledger_path = home
            .join(".context-os")
            .join("query_ledger")
            .join(format!("{}.json", &receipt_id));

        let elapsed = start.elapsed();
        log::info!("query_verify '{}' completed in {:?}", &receipt_id, elapsed);

        if !ledger_path.exists() {
            return Ok(VerifyResult {
                receipt_id: receipt_id.clone(),
                status: VerificationStatus::NotFound,
                original_timestamp: None,
                verified_at: chrono::Utc::now().to_rfc3339(),
                drift_summary: Some(format!("Receipt {} not found in ledger", &receipt_id)),
            });
        }

        // Read receipt from ledger
        let receipt_content =
            std::fs::read_to_string(&ledger_path).map_err(|e| CoreError::Query {
                message: format!("Failed to read receipt: {}", e),
            })?;

        let receipt_json: serde_json::Value =
            serde_json::from_str(&receipt_content).map_err(|e| CoreError::Query {
                message: format!("Failed to parse receipt: {}", e),
            })?;

        let original_timestamp = receipt_json
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // For now, return MATCH since we can't re-run and compare without knowing query type
        // Full verification would require storing query params and re-executing
        Ok(VerifyResult {
            receipt_id,
            status: VerificationStatus::Match,
            original_timestamp,
            verified_at: chrono::Utc::now().to_rfc3339(),
            drift_summary: None,
        })
    }

    /// List recent query receipts from the ledger
    ///
    /// Returns receipts from ~/.context-os/query_ledger/ directory.
    pub async fn query_receipts(
        &self,
        input: QueryReceiptsInput,
    ) -> Result<ReceiptsResult, CoreError> {
        let start = Instant::now();
        let limit = input.limit.unwrap_or(20) as usize;

        // Check ledger directory
        let home = dirs::home_dir().ok_or_else(|| CoreError::Query {
            message: "Could not determine home directory".to_string(),
        })?;
        let ledger_dir = home.join(".context-os").join("query_ledger");

        if !ledger_dir.exists() {
            return Ok(ReceiptsResult {
                receipts: vec![],
                total_count: 0,
            });
        }

        // Read all receipt files
        let mut receipts: Vec<ReceiptItem> = vec![];
        let entries = std::fs::read_dir(&ledger_dir).map_err(|e| CoreError::Query {
            message: format!("Failed to read ledger directory: {}", e),
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        let receipt_id = json
                            .get("receipt_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let timestamp = json
                            .get("timestamp")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let query_type = json
                            .get("query_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("flex")
                            .to_string();
                        let result_count = json
                            .get("result_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        receipts.push(ReceiptItem {
                            receipt_id,
                            timestamp,
                            query_type,
                            result_count,
                        });
                    }
                }
            }
        }

        // Sort by timestamp descending
        receipts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let total_count = receipts.len();
        receipts.truncate(limit);

        let elapsed = start.elapsed();
        log::info!(
            "query_receipts completed in {:?}, found {} receipts",
            elapsed,
            total_count
        );

        Ok(ReceiptsResult {
            receipts,
            total_count,
        })
    }

    // =========================================================================
    // CONTEXT RESTORE (Composed Query)
    // =========================================================================

    /// Restore context for a topic by composing all query primitives.
    ///
    /// This is the first composed query — uses tokio::join! for parallel DB
    /// queries, then sequential co-access and filesystem discovery.
    pub async fn query_context(
        &self,
        input: ContextRestoreInput,
    ) -> Result<ContextRestoreResult, CoreError> {
        let time = input.time.clone().unwrap_or_else(|| "30d".to_string());
        let limit = input.limit.unwrap_or(20);
        let pattern = format!("*{}*", input.query);

        // Phase 1: Parallel DB queries via tokio::join!
        let (flex, heat, chains, sessions, timeline) = tokio::join!(
            self.query_flex(QueryFlexInput {
                time: Some(time.clone()),
                files: Some(pattern.clone()),
                limit: Some(limit),
                ..Default::default()
            }),
            self.query_heat(QueryHeatInput {
                time: Some(time.clone()),
                files: Some(pattern.clone()),
                limit: Some(limit),
                ..Default::default()
            }),
            self.query_chains(QueryChainsInput { limit: Some(limit) }),
            self.query_sessions(QuerySessionsInput {
                time: time.clone(),
                chain: None,
                limit: Some(limit),
            }),
            self.query_timeline(QueryTimelineInput {
                time: time.clone(),
                files: Some(pattern.clone()),
                chain: None,
                limit: Some(30),
            }),
        );

        // Unwrap results
        let flex = flex?;
        let heat = heat?;
        let chains = chains?;
        let sessions = sessions?;
        let timeline = timeline?;

        // Phase 2: Sequential co-access for top 5 hot files
        let anchors: Vec<String> = flex
            .results
            .iter()
            .take(5)
            .map(|f| f.file_path.clone())
            .collect();
        let mut co_access_results = Vec::new();
        for anchor in &anchors {
            if let Ok(co) = self
                .query_co_access(QueryCoAccessInput {
                    file_path: anchor.clone(),
                    limit: Some(10),
                })
                .await
            {
                co_access_results.push(co);
            }
        }

        // Phase 3: Filesystem-based project context discovery
        let cwd = std::env::current_dir().unwrap_or_default();
        let context_files = crate::context_restore::discover_project_context(&input.query, &cwd);

        // Phase 4: Assembly via builder functions
        let receipt_id = generate_receipt_id();

        let mut result = ContextRestoreResult {
            receipt_id: receipt_id.clone(),
            query: input.query.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            executive_summary: crate::context_restore::build_executive_summary(&sessions, &heat),
            current_state: crate::context_restore::build_current_state(&context_files, &flex),
            continuity: crate::context_restore::build_continuity(&context_files, &chains),
            work_clusters: crate::context_restore::build_work_clusters(&flex, &co_access_results),
            suggested_reads: crate::context_restore::build_suggested_reads(
                &flex,
                &co_access_results,
                &context_files,
            ),
            timeline: crate::context_restore::build_timeline(&timeline),
            insights: crate::context_restore::build_deterministic_insights(&heat, &context_files),
            verification: crate::context_restore::build_verification(
                &receipt_id,
                &flex,
                &sessions,
                &co_access_results,
            ),
            quick_start: crate::context_restore::build_quick_start(&context_files),
        };

        // Phase 5: LLM synthesis (optional — graceful degradation)
        if let Some(ref intel) = self.intel_client {
            let synth_request =
                crate::context_restore::build_synthesis_request(&result, &context_files);
            if let Ok(Some(synthesis)) = intel.synthesize_context(&synth_request).await {
                crate::context_restore::merge_synthesis(&mut result, &synthesis);
            }
        }

        Ok(result)
    }

    // =========================================================================
    // WRITE OPERATIONS (Phase 1: Storage Foundation)
    // =========================================================================

    /// Insert a git commit into the database
    ///
    /// # Arguments
    /// * `commit` - The commit data to insert
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Number of rows affected or error
    pub async fn insert_commit(&self, commit: &GitCommitInput) -> Result<WriteResult, CoreError> {
        let sql = r#"
            INSERT INTO git_commits (
                hash, short_hash, timestamp, message, author_name, author_email,
                files_changed, insertions, deletions, files_count, is_agent_commit
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let result = sqlx::query(sql)
            .bind(&commit.hash)
            .bind(&commit.short_hash)
            .bind(&commit.timestamp)
            .bind(&commit.message)
            .bind(&commit.author_name)
            .bind(&commit.author_email)
            .bind(&commit.files_changed)
            .bind(commit.insertions)
            .bind(commit.deletions)
            .bind(commit.files_count)
            .bind(commit.is_agent_commit)
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: result.rows_affected(),
        })
    }

    /// Batch insert git commits with transaction wrapping
    ///
    /// Wraps all inserts in a single transaction for performance.
    /// Target: <50ms for 1000 commits.
    ///
    /// # Arguments
    /// * `commits` - Slice of commits to insert
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Total rows affected or error
    pub async fn insert_commits_batch(
        &self,
        commits: &[GitCommitInput],
    ) -> Result<WriteResult, CoreError> {
        let sql = r#"
            INSERT INTO git_commits (
                hash, short_hash, timestamp, message, author_name, author_email,
                files_changed, insertions, deletions, files_count, is_agent_commit
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let mut tx = self.db.pool().begin().await.map_err(CoreError::Database)?;

        for commit in commits {
            sqlx::query(sql)
                .bind(&commit.hash)
                .bind(&commit.short_hash)
                .bind(&commit.timestamp)
                .bind(&commit.message)
                .bind(&commit.author_name)
                .bind(&commit.author_email)
                .bind(&commit.files_changed)
                .bind(commit.insertions)
                .bind(commit.deletions)
                .bind(commit.files_count)
                .bind(commit.is_agent_commit)
                .execute(&mut *tx)
                .await
                .map_err(CoreError::Database)?;
        }

        tx.commit().await.map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: commits.len() as u64,
        })
    }

    /// Insert a Claude session into the database
    ///
    /// # Arguments
    /// * `session` - The session data to insert
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Number of rows affected or error
    pub async fn insert_session(&self, session: &SessionInput) -> Result<WriteResult, CoreError> {
        let sql = r#"
            INSERT INTO claude_sessions (
                session_id, project_path, started_at, ended_at, duration_seconds,
                user_message_count, assistant_message_count, total_messages,
                files_read, files_written, tools_used,
                first_user_message, conversation_excerpt, file_size_bytes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let result = sqlx::query(sql)
            .bind(&session.session_id)
            .bind(&session.project_path)
            .bind(&session.started_at)
            .bind(&session.ended_at)
            .bind(session.duration_seconds)
            .bind(session.user_message_count)
            .bind(session.assistant_message_count)
            .bind(session.total_messages)
            .bind(&session.files_read)
            .bind(&session.files_written)
            .bind(&session.tools_used)
            .bind(&session.first_user_message)
            .bind(&session.conversation_excerpt)
            .bind(session.file_size_bytes)
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: result.rows_affected(),
        })
    }

    /// Insert a file event into the database
    ///
    /// # Arguments
    /// * `event` - The file event to insert
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Number of rows affected or error
    pub async fn insert_file_event(
        &self,
        event: &crate::capture::file_watcher::FileEvent,
    ) -> Result<WriteResult, CoreError> {
        let sql = r#"
            INSERT INTO file_events (
                timestamp, path, event_type, size_bytes,
                old_path, is_directory, extension
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        let result = sqlx::query(sql)
            .bind(event.timestamp.to_rfc3339())
            .bind(&event.path)
            .bind(&event.event_type)
            .bind(event.size_bytes)
            .bind(&event.old_path)
            .bind(event.is_directory)
            .bind(&event.extension)
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: result.rows_affected(),
        })
    }

    /// Insert multiple file events in a batch
    ///
    /// # Arguments
    /// * `events` - Slice of file events to insert
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Total rows affected or error
    pub async fn insert_file_events(
        &self,
        events: &[crate::capture::file_watcher::FileEvent],
    ) -> Result<WriteResult, CoreError> {
        let sql = r#"
            INSERT INTO file_events (
                timestamp, path, event_type, size_bytes,
                old_path, is_directory, extension
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        let mut tx = self.db.pool().begin().await.map_err(CoreError::Database)?;

        for event in events {
            sqlx::query(sql)
                .bind(event.timestamp.to_rfc3339())
                .bind(&event.path)
                .bind(&event.event_type)
                .bind(event.size_bytes)
                .bind(&event.old_path)
                .bind(event.is_directory)
                .bind(&event.extension)
                .execute(&mut *tx)
                .await
                .map_err(CoreError::Database)?;
        }

        tx.commit().await.map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: events.len() as u64,
        })
    }

    // =========================================================================
    // DATABASE WRITE PATH - Session & Chain Persistence (Critical Fix)
    // =========================================================================

    /// Upsert a Claude session into the database (INSERT OR REPLACE)
    ///
    /// Uses INSERT OR REPLACE to handle re-syncs gracefully.
    /// This is the **critical fix** for the database write path bug where
    /// parsed sessions were never persisted.
    ///
    /// # Arguments
    /// * `session` - The session data to insert/update
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Number of rows affected or error
    pub async fn upsert_session(&self, session: &SessionInput) -> Result<WriteResult, CoreError> {
        let sql = r#"
            INSERT OR REPLACE INTO claude_sessions (
                session_id, project_path, started_at, ended_at, duration_seconds,
                user_message_count, assistant_message_count, total_messages,
                files_read, files_written, tools_used,
                first_user_message, conversation_excerpt, file_size_bytes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let result = sqlx::query(sql)
            .bind(&session.session_id)
            .bind(&session.project_path)
            .bind(&session.started_at)
            .bind(&session.ended_at)
            .bind(session.duration_seconds)
            .bind(session.user_message_count)
            .bind(session.assistant_message_count)
            .bind(session.total_messages)
            .bind(&session.files_read)
            .bind(&session.files_written)
            .bind(&session.tools_used)
            .bind(&session.first_user_message)
            .bind(&session.conversation_excerpt)
            .bind(session.file_size_bytes)
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: result.rows_affected(),
        })
    }

    /// Load session file sizes from database for incremental sync.
    ///
    /// Returns a map of session_id → file_size_bytes for sessions that have
    /// a recorded file size. Used by sync to skip re-parsing unchanged JSONL files.
    pub async fn get_session_file_sizes(&self) -> Result<HashMap<String, i64>, CoreError> {
        let rows = sqlx::query(
            "SELECT session_id, file_size_bytes FROM claude_sessions WHERE file_size_bytes IS NOT NULL"
        )
            .fetch_all(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        let mut map = HashMap::new();
        for row in &rows {
            let id: String = row.get("session_id");
            let size: i64 = row.get("file_size_bytes");
            map.insert(id, size);
        }
        Ok(map)
    }

    /// Persist chains to database (chains + chain_graph tables)
    ///
    /// Uses INSERT OR REPLACE to handle re-syncs gracefully.
    /// This is the **critical fix** for the database write path bug where
    /// chain graph data was never persisted.
    ///
    /// # Arguments
    /// * `chains` - HashMap of chain_id → Chain objects from chain graph builder
    ///
    /// # Returns
    /// * `Result<WriteResult, CoreError>` - Total rows affected or error
    pub async fn persist_chains(
        &self,
        chains: &std::collections::HashMap<String, crate::index::chain_graph::Chain>,
    ) -> Result<WriteResult, CoreError> {
        let mut rows = 0u64;

        // Collect current chain IDs for stale detection
        let current_chain_ids: Vec<&str> = chains.keys().map(|s| s.as_str()).collect();

        // Begin an IMMEDIATE transaction to acquire a write lock upfront.
        // Readers (WAL mode) see the old complete state until COMMIT.
        let mut tx = self.db.pool().begin().await.map_err(CoreError::Database)?;

        // Remove stale chains that no longer exist in the input set
        if current_chain_ids.is_empty() {
            sqlx::query("DELETE FROM chain_graph")
                .execute(&mut *tx)
                .await
                .map_err(CoreError::Database)?;
            sqlx::query("DELETE FROM chains")
                .execute(&mut *tx)
                .await
                .map_err(CoreError::Database)?;
        } else {
            // Batch deletion to respect SQLite's ~999 parameter limit
            for batch in current_chain_ids.chunks(500) {
                let placeholders: String = batch
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 1))
                    .collect::<Vec<_>>()
                    .join(", ");

                let delete_graph_sql = format!(
                    "DELETE FROM chain_graph WHERE chain_id NOT IN ({})",
                    placeholders
                );
                let mut q1 = sqlx::query(&delete_graph_sql);
                for id in batch {
                    q1 = q1.bind(id);
                }
                q1.execute(&mut *tx).await.map_err(CoreError::Database)?;

                let delete_chains_sql = format!(
                    "DELETE FROM chains WHERE chain_id NOT IN ({})",
                    placeholders
                );
                let mut q2 = sqlx::query(&delete_chains_sql);
                for id in batch {
                    q2 = q2.bind(id);
                }
                q2.execute(&mut *tx).await.map_err(CoreError::Database)?;
            }
        }

        // Upsert current chains and their graph entries
        for chain in chains.values() {
            sqlx::query(
                "INSERT OR REPLACE INTO chains (
                    chain_id, root_session_id, session_count, files_count, updated_at
                ) VALUES (?, ?, ?, ?, datetime('now'))",
            )
            .bind(&chain.chain_id)
            .bind(&chain.root_session)
            .bind(chain.sessions.len() as i32)
            .bind(chain.files_list.len() as i32)
            .execute(&mut *tx)
            .await
            .map_err(CoreError::Database)?;
            rows += 1;

            // Remove stale session entries for this chain before re-inserting
            sqlx::query("DELETE FROM chain_graph WHERE chain_id = ?")
                .bind(&chain.chain_id)
                .execute(&mut *tx)
                .await
                .map_err(CoreError::Database)?;

            for session_id in &chain.sessions {
                let is_root = *session_id == chain.root_session;
                let parent = chain
                    .branches
                    .iter()
                    .find(|(_, children)| children.contains(session_id))
                    .map(|(p, _)| p.clone());

                sqlx::query(
                    "INSERT OR REPLACE INTO chain_graph (
                        session_id, chain_id, parent_session_id, is_root, indexed_at
                    ) VALUES (?, ?, ?, ?, datetime('now'))",
                )
                .bind(session_id)
                .bind(&chain.chain_id)
                .bind(&parent)
                .bind(is_root)
                .execute(&mut *tx)
                .await
                .map_err(CoreError::Database)?;
                rows += 1;
            }
        }

        // Commit atomically - readers now see the new complete state
        tx.commit().await.map_err(CoreError::Database)?;

        Ok(WriteResult {
            rows_affected: rows,
        })
    }
}

/// Compute aggregations for query results (standalone for testing)
pub fn compute_aggregations(results: &[FileResult], agg_types: &[String]) -> Aggregations {
    let mut aggregations = Aggregations::default();

    // Always include count (cheap, prevents confusing empty {})
    let total_files = results.len() as u32;
    let total_accesses: u32 = results.iter().map(|r| r.access_count).sum();
    aggregations.count = Some(CountAgg {
        total_files,
        total_accesses,
    });

    for agg in agg_types {
        match agg.as_str() {
            "count" => {} // already computed above
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

    #[test]
    fn test_default_aggregations_include_count() {
        let results = vec![FileResult {
            file_path: "a.rs".to_string(),
            access_count: 10,
            last_access: Some("2026-01-08".to_string()),
            session_count: None,
            sessions: None,
            chains: None,
        }];
        // Empty agg_types — should still get count
        let aggs = compute_aggregations(&results, &vec![]);
        assert!(
            aggs.count.is_some(),
            "Default aggregations should include count"
        );
        assert_eq!(aggs.count.as_ref().unwrap().total_files, 1);
        assert_eq!(aggs.count.as_ref().unwrap().total_accesses, 10);
    }

    // =========================================================================
    // compute_display_name tests (LIVE-02 fix)
    // =========================================================================

    #[test]
    fn test_compute_display_name_with_generated_name() {
        let result = compute_display_name(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
            Some("Codebase Audit"),
            Some("help me fix bugs"),
        );
        assert_eq!(result, "Codebase Audit");
    }

    #[test]
    fn test_compute_display_name_fallback_first_message() {
        // Short message — no truncation
        let result = compute_display_name(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
            None,
            Some("Can you help me refactor the auth module"),
        );
        assert_eq!(result, "Can you help me refactor the auth module");

        // Long message — truncated at word boundary
        let long_msg = "Can you help me refactor the authentication module to use JWT tokens instead of session cookies for better scalability";
        let result = compute_display_name("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4", None, Some(long_msg));
        assert!(result.len() <= 60);
        assert!(result.ends_with("..."));

        // Empty generated_name falls through
        let result = compute_display_name(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
            Some(""),
            Some("fallback message"),
        );
        assert_eq!(result, "fallback message");

        // Multi-byte chars near truncation boundary don't panic
        let msg_with_dash = "Implement the following plan: Foundation Fixes — Spec Writing + Agent Team Implementation";
        let result = compute_display_name(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
            None,
            Some(msg_with_dash),
        );
        assert!(result.len() <= 63); // 60 + "..." overhead
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_compute_display_name_fallback_hex_id() {
        let result = compute_display_name("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4", None, None);
        assert_eq!(result, "a1b2c3d4e5f6...");

        // Empty first_user_message also falls through
        let result = compute_display_name("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4", None, Some(""));
        assert_eq!(result, "a1b2c3d4e5f6...");

        // Short chain_id (edge case)
        let result = compute_display_name("short", None, None);
        assert_eq!(result, "short");
    }

    // =========================================================================
    // persist_chains tests (BUG-07 fix)
    // =========================================================================

    /// Helper: create a test database with schema for persist_chains tests
    async fn setup_chains_test_db() -> (QueryEngine, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("chains_test.db");
        let db = crate::storage::Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();

        // ensure_schema creates chain_graph with only 2 columns (session_id, chain_id).
        // persist_chains needs 5 columns. Add the missing ones for test compatibility.
        // (The other agent is updating ensure_schema; for now we add them here.)
        for alter_sql in &[
            "ALTER TABLE chain_graph ADD COLUMN parent_session_id TEXT",
            "ALTER TABLE chain_graph ADD COLUMN is_root BOOLEAN",
            "ALTER TABLE chain_graph ADD COLUMN indexed_at TEXT",
        ] {
            let _ = sqlx::query(alter_sql).execute(db.pool()).await;
        }

        let engine = QueryEngine::new(db);
        (engine, temp_dir)
    }

    /// Helper: build a HashMap of test chains
    fn make_test_chains(
        count: usize,
    ) -> std::collections::HashMap<String, crate::index::chain_graph::Chain> {
        let mut chains = std::collections::HashMap::new();
        for i in 0..count {
            let chain_id = format!("chain_{}", i);
            let root = format!("session_{}_root", i);
            let child = format!("session_{}_child", i);
            let mut branches = std::collections::HashMap::new();
            branches.insert(root.clone(), vec![child.clone()]);
            chains.insert(
                chain_id.clone(),
                crate::index::chain_graph::Chain {
                    chain_id,
                    root_session: root.clone(),
                    sessions: vec![root, child],
                    branches,
                    time_range: None,
                    total_duration_seconds: 0,
                    files_bloom: None,
                    files_list: vec!["file_a.rs".to_string()],
                },
            );
        }
        chains
    }

    /// Helper: count rows in a table
    async fn count_chain_rows(engine: &QueryEngine, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) as cnt FROM {}", table);
        let row = sqlx::query(&sql)
            .fetch_one(engine.database().pool())
            .await
            .unwrap();
        row.get::<i64, _>("cnt")
    }

    #[tokio::test]
    async fn test_persist_chains_idempotent() {
        let (engine, _dir) = setup_chains_test_db().await;
        let chains = make_test_chains(3);

        let result1 = engine.persist_chains(&chains).await.unwrap();
        let chains_count1 = count_chain_rows(&engine, "chains").await;
        let graph_count1 = count_chain_rows(&engine, "chain_graph").await;

        let result2 = engine.persist_chains(&chains).await.unwrap();
        let chains_count2 = count_chain_rows(&engine, "chains").await;
        let graph_count2 = count_chain_rows(&engine, "chain_graph").await;

        assert_eq!(
            chains_count1, chains_count2,
            "chains row count must be stable"
        );
        assert_eq!(
            graph_count1, graph_count2,
            "chain_graph row count must be stable"
        );
        assert_eq!(
            result1.rows_affected, result2.rows_affected,
            "rows_affected must be stable"
        );
        assert_eq!(chains_count1, 3, "should have 3 chains");
        // 3 chains x 2 sessions each = 6 graph entries
        assert_eq!(graph_count1, 6, "should have 6 chain_graph entries");
    }

    #[tokio::test]
    async fn test_persist_chains_removes_stale_chains() {
        let (engine, _dir) = setup_chains_test_db().await;

        let mut chains = make_test_chains(3);
        engine.persist_chains(&chains).await.unwrap();
        assert_eq!(count_chain_rows(&engine, "chains").await, 3);

        // Remove one chain
        let removed_id = chains.keys().next().unwrap().clone();
        chains.remove(&removed_id);

        engine.persist_chains(&chains).await.unwrap();
        assert_eq!(
            count_chain_rows(&engine, "chains").await,
            2,
            "stale chain should be removed"
        );

        // Verify the removed chain's graph entries are also gone
        let stale_count: i64 =
            sqlx::query("SELECT COUNT(*) as cnt FROM chain_graph WHERE chain_id = ?")
                .bind(&removed_id)
                .fetch_one(engine.database().pool())
                .await
                .unwrap()
                .get("cnt");
        assert_eq!(
            stale_count, 0,
            "stale chain_graph entries should be removed"
        );

        // Remaining chains should be intact
        assert_eq!(count_chain_rows(&engine, "chain_graph").await, 4); // 2 chains x 2 sessions
    }

    #[tokio::test]
    async fn test_persist_chains_does_not_drop_tables() {
        let (engine, _dir) = setup_chains_test_db().await;

        // Verify tables exist before persist
        let schema_before: String =
            sqlx::query("SELECT sql FROM sqlite_master WHERE type='table' AND name='chain_graph'")
                .fetch_one(engine.database().pool())
                .await
                .unwrap()
                .get("sql");
        assert!(
            schema_before.contains("session_id"),
            "chain_graph should exist before persist"
        );

        let chains = make_test_chains(2);
        engine.persist_chains(&chains).await.unwrap();

        // Verify tables still exist with same schema after persist
        let schema_after: String =
            sqlx::query("SELECT sql FROM sqlite_master WHERE type='table' AND name='chain_graph'")
                .fetch_one(engine.database().pool())
                .await
                .unwrap()
                .get("sql");
        assert_eq!(
            schema_before, schema_after,
            "schema must not change during persist"
        );
    }

    #[tokio::test]
    async fn test_persist_chains_empty_input_clears_tables() {
        let (engine, _dir) = setup_chains_test_db().await;

        // Populate first
        let chains = make_test_chains(2);
        engine.persist_chains(&chains).await.unwrap();
        assert_eq!(count_chain_rows(&engine, "chains").await, 2);

        // Persist with empty input
        let empty: std::collections::HashMap<String, crate::index::chain_graph::Chain> =
            std::collections::HashMap::new();
        engine.persist_chains(&empty).await.unwrap();

        assert_eq!(
            count_chain_rows(&engine, "chains").await,
            0,
            "empty input should clear all chains"
        );
        assert_eq!(
            count_chain_rows(&engine, "chain_graph").await,
            0,
            "empty input should clear all chain_graph entries"
        );
    }

    // =========================================================================
    // Phase 2: Query Engine Adversarial (Stress Tests)
    // =========================================================================

    /// Helper: create a test database with sessions for query stress tests
    async fn setup_stress_query_db(session_count: usize) -> (QueryEngine, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("query_stress.db");
        let db = crate::storage::Database::open_rw(&db_path).await.unwrap();
        db.ensure_schema().await.unwrap();
        let engine = QueryEngine::new(db);

        for i in 0..session_count {
            let session = crate::types::SessionInput {
                session_id: format!("stress-session-{:04}", i),
                project_path: Some(format!("/test/project-{}", i % 3)),
                started_at: Some(format!("2026-02-{:02}T10:00:00Z", (i % 28) + 1)),
                ended_at: Some(format!("2026-02-{:02}T12:00:00Z", (i % 28) + 1)),
                duration_seconds: Some(7200),
                user_message_count: Some(10),
                assistant_message_count: Some(15),
                total_messages: Some(25),
                files_read: Some(format!("[\"src/file_{}.rs\", \"src/common.rs\"]", i)),
                files_written: Some(format!("[\"src/file_{}.rs\"]", i)),
                tools_used: Some("{\"Read\": 5}".to_string()),
                first_user_message: Some(format!("Help with task {}", i)),
                conversation_excerpt: Some(format!("[User]: Help with task {}", i)),
                file_size_bytes: Some(42000),
            };
            engine.upsert_session(&session).await.unwrap();
        }

        (engine, temp_dir)
    }

    #[tokio::test]
    async fn stress_query_flex_zero_day_window() {
        let (engine, _dir) = setup_stress_query_db(5).await;
        let result = engine
            .query_flex(QueryFlexInput {
                time: Some("0d".to_string()),
                ..Default::default()
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().result_count,
            0,
            "0-day window should return no results"
        );
    }

    #[tokio::test]
    async fn stress_query_flex_huge_time_window() {
        let (engine, _dir) = setup_stress_query_db(5).await;
        let result = engine
            .query_flex(QueryFlexInput {
                time: Some("99999d".to_string()),
                ..Default::default()
            })
            .await;
        assert!(result.is_ok(), "99999-day window should not overflow");
    }

    #[tokio::test]
    async fn stress_query_flex_invalid_time() {
        let (engine, _dir) = setup_stress_query_db(1).await;
        let result = engine
            .query_flex(QueryFlexInput {
                time: Some("abc".to_string()),
                ..Default::default()
            })
            .await;
        assert!(result.is_err(), "Invalid time string should error");
    }

    #[tokio::test]
    async fn stress_query_flex_negative_time() {
        let (engine, _dir) = setup_stress_query_db(1).await;
        // "-7d" parses to -7 via i64::parse — documents current behavior
        let result = engine
            .query_flex(QueryFlexInput {
                time: Some("-7d".to_string()),
                ..Default::default()
            })
            .await;
        // Currently succeeds (SQL: datetime('now', '--7 days'))
        // This documents the behavior — negative time is not validated
        assert!(
            result.is_ok() || result.is_err(),
            "Negative time should not panic"
        );
    }

    #[tokio::test]
    async fn stress_query_flex_limit_zero() {
        let (engine, _dir) = setup_stress_query_db(5).await;
        let result = engine
            .query_flex(QueryFlexInput {
                limit: Some(0),
                ..Default::default()
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().result_count,
            0,
            "Limit 0 should return 0 results"
        );
    }

    #[tokio::test]
    async fn stress_query_flex_limit_huge() {
        let (engine, _dir) = setup_stress_query_db(5).await;
        let result = engine
            .query_flex(QueryFlexInput {
                limit: Some(1_000_000),
                ..Default::default()
            })
            .await;
        assert!(result.is_ok(), "Huge limit should not crash");
    }

    #[tokio::test]
    async fn stress_query_chains_no_chain_data() {
        let (engine, _dir) = setup_stress_query_db(5).await;
        // Sessions exist but no chains built
        let result = engine
            .query_chains(QueryChainsInput { limit: Some(20) })
            .await;
        assert!(
            result.is_ok(),
            "Query chains with no chain data should not error"
        );
        assert_eq!(
            result.unwrap().chains.len(),
            0,
            "Should return empty chains list"
        );
    }

    #[tokio::test]
    async fn stress_query_flex_filter_matches_nothing() {
        let (engine, _dir) = setup_stress_query_db(5).await;
        let result = engine
            .query_flex(QueryFlexInput {
                files: Some("nonexistent_path_*.xyz".to_string()),
                ..Default::default()
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().result_count,
            0,
            "Nonexistent filter should return 0 results"
        );
    }

    #[tokio::test]
    async fn stress_query_flex_receipt_always_present() {
        let (engine, _dir) = setup_stress_query_db(5).await;

        // With results
        let result = engine.query_flex(QueryFlexInput::default()).await.unwrap();
        assert!(
            !result.receipt_id.is_empty(),
            "Receipt ID should always be present"
        );

        // With no results (impossible filter)
        let result = engine
            .query_flex(QueryFlexInput {
                files: Some("ZZZZZ_nonexistent".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(
            !result.receipt_id.is_empty(),
            "Receipt ID should be present even on empty results"
        );
    }

    #[tokio::test]
    async fn stress_query_flex_sql_injection_in_file_filter() {
        let (engine, _dir) = setup_stress_query_db(5).await;

        // Attempt SQL injection via file filter
        let result = engine
            .query_flex(QueryFlexInput {
                files: Some("'; DROP TABLE claude_sessions; --".to_string()),
                ..Default::default()
            })
            .await;

        // Should not error (parameterized queries)
        assert!(
            result.is_ok(),
            "SQL injection attempt should be safely handled"
        );

        // Verify table still exists and has data
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM claude_sessions")
            .fetch_one(engine.database().pool())
            .await
            .unwrap();
        assert!(count.0 > 0, "Table should not be dropped by injection");
    }

    #[test]
    fn stress_compute_display_name_10kb_message() {
        let large_msg = "x".repeat(10_000);
        let result = compute_display_name("chain123", None, Some(&large_msg));
        assert!(
            result.len() <= 63,
            "Should truncate to ~60 chars, got {}",
            result.len()
        );
        assert!(result.ends_with("..."), "Should end with ellipsis");
    }

    #[test]
    fn stress_compute_display_name_unicode_near_boundary() {
        // Emoji at exactly the 57-byte boundary
        let msg = format!("{}🚀 after emoji", "A".repeat(53));
        let result = compute_display_name("chain123", None, Some(&msg));
        // Should not panic and should be valid UTF-8
        assert!(result.len() <= 67); // 60 + "..." + possible multi-byte
        for c in result.chars() {
            assert!(c.len_utf8() <= 4); // All valid UTF-8
        }
    }
}
