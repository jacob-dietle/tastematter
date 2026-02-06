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
             WHERE s.files_read IS NOT NULL AND s.files_read != '[]'",
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
        let sql = format!(
            "SELECT
                cg.chain_id,
                COUNT(DISTINCT cg.session_id) as session_count,
                COUNT(DISTINCT json_each.value) as file_count,
                cm.generated_name
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
                ChainData {
                    chain_id: row.get("chain_id"),
                    session_count: row.get::<i64, _>("session_count") as u32,
                    file_count: row.get::<i64, _>("file_count") as u32,
                    time_range: None, // TODO: Add time range query if needed
                    generated_name: row.get::<Option<String>, _>("generated_name"),
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
                    last_access: r
                        .get::<Option<String>, _>("first_accessed_at")
                        .unwrap_or_default(),
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

        // Query file_accesses table (or claude_sessions.files_read JSON)
        // Using claude_sessions.files_read to match Python behavior
        let sql = "SELECT
                json_each.value as file_path,
                COUNT(*) as access_count
             FROM claude_sessions s, json_each(s.files_read)
             WHERE s.files_read IS NOT NULL
               AND s.files_read != '[]'
               AND LOWER(json_each.value) LIKE ?
             GROUP BY json_each.value
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

        // First try exact match
        let exact_sql = "SELECT DISTINCT
                s.session_id,
                s.started_at as last_access,
                cg.chain_id
             FROM claude_sessions s, json_each(s.files_read)
             LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
             WHERE json_each.value = ?
             ORDER BY s.started_at DESC
             LIMIT ?";

        let rows = sqlx::query(exact_sql)
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
                    access_types: vec!["read".to_string()], // Default to read
                    last_access: row.get("last_access"),
                    chain_id: row.get("chain_id"),
                })
                .collect();
            (Some(file_path.clone()), sessions)
        } else {
            // Try suffix match
            let suffix_sql = "SELECT DISTINCT
                    json_each.value as matched_path,
                    s.session_id,
                    s.started_at as last_access,
                    cg.chain_id
                 FROM claude_sessions s, json_each(s.files_read)
                 LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
                 WHERE json_each.value LIKE ?
                 ORDER BY s.started_at DESC
                 LIMIT ?";

            let suffix_pattern = format!("%{}", file_path);
            let suffix_rows = sqlx::query(suffix_sql)
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
                let substr_rows = sqlx::query(suffix_sql)
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

        // Get sessions that touched this file
        let sessions_sql = "SELECT DISTINCT s.session_id
             FROM claude_sessions s, json_each(s.files_read)
             WHERE json_each.value LIKE ?";

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

        // Get files co-accessed in those sessions with frequency
        // Simplified PMI: count co-occurrences / total sessions for file
        let placeholders: Vec<String> = session_ids.iter().map(|_| "?".to_string()).collect();
        let co_access_sql = format!(
            "SELECT
                json_each.value as file_path,
                COUNT(DISTINCT s.session_id) as co_count
             FROM claude_sessions s, json_each(s.files_read)
             WHERE s.session_id IN ({})
               AND json_each.value NOT LIKE ?
             GROUP BY json_each.value
             ORDER BY co_count DESC
             LIMIT ?",
            placeholders.join(",")
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
    /// then computes RCR, velocity, and composite heat score in Rust.
    ///
    /// Target: <100ms latency
    pub async fn query_heat(&self, input: QueryHeatInput) -> Result<HeatResult, CoreError> {
        let start = Instant::now();

        let time_str = input.time.as_deref().unwrap_or("30d");
        let days = parse_time_range(time_str)?;
        let limit = input.limit.unwrap_or(50);

        // Build file filter clause
        let file_filter = if input.files.is_some() {
            " AND json_each.value LIKE ?"
        } else {
            ""
        };

        // Single-scan approach: compute both 7d and long-window counts in one pass
        // using conditional SUM for the short window (avoids double table scan)
        let sql = format!(
            "SELECT json_each.value as file_path,
                    SUM(CASE WHEN s.started_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END) as count_7d,
                    COUNT(*) as count_long,
                    MIN(s.started_at) as first_access,
                    MAX(s.started_at) as last_access
             FROM claude_sessions s, json_each(s.files_read)
             WHERE s.started_at >= datetime('now', '-{days} days')
               AND s.files_read IS NOT NULL AND s.files_read != '[]'
               {file_filter}
             GROUP BY json_each.value
             LIMIT {limit}",
            file_filter = file_filter,
            days = days,
            limit = limit,
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

                // RCR = count_7d / count_long (safe division)
                let rcr = if count_long > 0 {
                    count_7d as f64 / count_long as f64
                } else {
                    0.0
                };

                let velocity = compute_velocity(count_long, &first_access, &last_access);
                let heat_score = compute_heat_score(velocity, rcr, &last_access);
                let heat_level = classify_heat(heat_score);

                HeatItem {
                    file_path,
                    count_7d,
                    count_long,
                    rcr,
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
            HeatSortBy::Rcr => items.sort_by(|a, b| {
                b.rcr
                    .partial_cmp(&a.rcr)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            HeatSortBy::Velocity => items.sort_by(|a, b| {
                b.velocity
                    .partial_cmp(&a.velocity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            HeatSortBy::Name => items.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
        }

        // Compute summary
        let total_files = items.len() as u32;
        let hot_count = items.iter().filter(|i| i.heat_level == HeatLevel::Hot).count() as u32;
        let warm_count = items.iter().filter(|i| i.heat_level == HeatLevel::Warm).count() as u32;
        let cool_count = items.iter().filter(|i| i.heat_level == HeatLevel::Cool).count() as u32;
        let cold_count = items.iter().filter(|i| i.heat_level == HeatLevel::Cold).count() as u32;

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

        // Drop and recreate tables to avoid FK constraint issues from old Python schema
        // The old Python schema had FK constraints that cause issues during Rust sync
        sqlx::query("DROP TABLE IF EXISTS chain_graph")
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        sqlx::query("DROP TABLE IF EXISTS chains")
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;

        // Recreate tables WITHOUT foreign key constraints
        sqlx::query(
            "CREATE TABLE chains (
                chain_id TEXT PRIMARY KEY,
                root_session_id TEXT,
                session_count INTEGER,
                files_count INTEGER,
                updated_at TEXT
            )",
        )
        .execute(self.db.pool())
        .await
        .map_err(CoreError::Database)?;

        sqlx::query(
            "CREATE TABLE chain_graph (
                session_id TEXT PRIMARY KEY,
                chain_id TEXT,
                parent_session_id TEXT,
                is_root BOOLEAN,
                indexed_at TEXT
            )",
        )
        .execute(self.db.pool())
        .await
        .map_err(CoreError::Database)?;

        for chain in chains.values() {
            // Insert chain metadata
            sqlx::query(
                "INSERT OR REPLACE INTO chains (
                    chain_id, root_session_id, session_count, files_count, updated_at
                ) VALUES (?, ?, ?, ?, datetime('now'))",
            )
            .bind(&chain.chain_id)
            .bind(&chain.root_session)
            .bind(chain.sessions.len() as i32)
            .bind(chain.files_list.len() as i32)
            .execute(self.db.pool())
            .await
            .map_err(CoreError::Database)?;
            rows += 1;

            // Insert session memberships
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
                .execute(self.db.pool())
                .await
                .map_err(CoreError::Database)?;
                rows += 1;
            }
        }

        Ok(WriteResult {
            rows_affected: rows,
        })
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
