//! Temporal edge extraction from file access events.
//!
//! Extracts typed, directed behavioral edges from per-tool-call sequences.
//! Runs as batch job during daemon sync.
//!
//! ## Edge Types
//!
//! - `read_then_edit`: File A read, then B edited within 5 min, same session. A → B.
//! - `read_before`: File A read before B in >50% shared sessions, within 5 min. A → B.
//! - `co_edited`: Both edited in same session. A → B where A < B lexically.
//! - `reference_anchor`: File read in first 2 min of >3 sessions. Self-edge.
//! - `debug_chain`: File read after Bash tool call. (bash context) → File.
//!
//! ## Noise Filters
//!
//! 1. Explore burst detection: >5 consecutive reads in 30s with no writes → exclude.
//! 2. Universal anchor dampening: file in >80% sessions → no `read_before` FROM it.
//! 3. Minimum session count: edges must appear in >= 3 sessions.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::error::CoreError;

/// Minimum sessions an edge must appear in to be stored.
const MIN_SESSION_COUNT: i32 = 2;

/// Minimum lift for edges with session_count < 3 to survive filtering.
/// Lift = (edge_sessions × total) / (source_sessions × target_sessions).
/// 2.0 means the edge appears 2x more often than random chance.
const MIN_LIFT_THRESHOLD: f64 = 2.0;

/// Maximum time delta (seconds) for read_then_edit and read_before edges.
const MAX_TIME_DELTA_SECONDS: f64 = 300.0; // 5 minutes

/// Reference anchor: file must be read within first N seconds of session.
const REFERENCE_ANCHOR_WINDOW_SECONDS: f64 = 120.0; // 2 minutes

/// Minimum sessions for reference_anchor.
const REFERENCE_ANCHOR_MIN_SESSIONS: i32 = 3;

/// Explore burst: more than this many consecutive reads in burst window → burst.
const EXPLORE_BURST_MIN_READS: usize = 5;

/// Explore burst: window in seconds.
const EXPLORE_BURST_WINDOW_SECONDS: f64 = 30.0;

/// Universal anchor threshold: file in >80% of sessions.
const UNIVERSAL_ANCHOR_THRESHOLD: f64 = 0.80;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A raw file access event loaded from the database.
#[derive(Debug, Clone)]
pub struct FileAccessEvent {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub file_path: String,
    pub tool_name: String,
    pub access_type: String, // "read", "write", "other"
    pub sequence_position: i32,
}

/// A candidate edge before cross-session aggregation.
#[derive(Debug, Clone)]
struct EdgeCandidate {
    source_file: String,
    target_file: String,
    edge_type: String,
    session_id: String,
    time_delta_seconds: Option<f64>,
}

/// Aggregated edge ready for DB upsert.
#[derive(Debug, Clone)]
struct AggregatedEdge {
    source_file: String,
    target_file: String,
    edge_type: String,
    session_count: i32,
    total_sessions_with_source: i32,
    avg_time_delta_seconds: Option<f64>,
    confidence: f64,
    lift: f64,
    first_seen: Option<String>,
    last_seen: Option<String>,
}

/// Result of edge extraction batch.
pub struct EdgeExtractionResult {
    pub edges_created: usize,
    pub sessions_processed: usize,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract edges from file_access_events for sessions since last extraction.
///
/// Main entry point. Loads events, filters noise, extracts per-session
/// candidates, aggregates across sessions, applies noise filters, and
/// upserts to the `file_edges` table.
pub async fn extract_file_edges(
    pool: &SqlitePool,
    since: Option<&str>,
) -> Result<EdgeExtractionResult, CoreError> {
    let start = Instant::now();

    // 1. Get session IDs to process
    let sessions = get_sessions_since(pool, since).await?;
    if sessions.is_empty() {
        return Ok(EdgeExtractionResult {
            edges_created: 0,
            sessions_processed: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }
    let session_count = sessions.len();

    // 2. Process sessions in batches to balance query count vs memory
    let mut all_candidates: Vec<EdgeCandidate> = Vec::new();
    for chunk in sessions.chunks(50) {
        let placeholders: Vec<&str> = chunk.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT session_id, timestamp, file_path, tool_name, access_type, sequence_position \
             FROM file_access_events \
             WHERE session_id IN ({}) \
             ORDER BY session_id, sequence_position ASC",
            placeholders.join(",")
        );
        let mut query = sqlx::query_as::<_, (String, String, String, String, String, i32)>(&sql);
        for id in chunk {
            query = query.bind(id);
        }
        let rows = query.fetch_all(pool).await.map_err(CoreError::Database)?;

        // Group by session and extract edges
        let mut by_session: HashMap<&str, Vec<FileAccessEvent>> = HashMap::new();
        let mut events: Vec<FileAccessEvent> = Vec::with_capacity(rows.len());
        for (sid, ts, fp, tn, at, sp) in rows {
            let timestamp = ts.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now());
            events.push(FileAccessEvent {
                session_id: sid,
                timestamp,
                file_path: fp,
                tool_name: tn,
                access_type: at,
                sequence_position: sp,
            });
        }
        for event in &events {
            by_session
                .entry(event.session_id.as_str())
                .or_default()
                .push(event.clone());
        }
        for session_events in by_session.values() {
            let filtered = filter_explore_bursts(session_events);
            let candidates = extract_session_edges(&filtered);
            all_candidates.extend(candidates);
        }
    }

    // 3. Extract reference anchors (cross-session pattern)
    let anchor_candidates = extract_reference_anchors(pool).await?;
    all_candidates.extend(anchor_candidates);

    // 4. Get global session count for lift calculation
    let total_sessions: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT session_id) FROM file_access_events")
            .fetch_one(pool)
            .await
            .map_err(CoreError::Database)?;

    // 5. Aggregate candidates across sessions
    let edges = aggregate_edge_candidates(&all_candidates, total_sessions.0 as usize);

    // 6. Apply noise filters
    let universal_anchors = get_universal_anchors(pool).await?;
    let filtered_edges = apply_noise_filters(&edges, &universal_anchors);

    // 7. Upsert to file_edges table
    let count = upsert_edges(pool, &filtered_edges).await?;

    // 8. Record extraction timestamp
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR REPLACE INTO _metadata (key, value) VALUES ('last_edge_extraction', ?)")
        .bind(&now)
        .execute(pool)
        .await
        .map_err(CoreError::Database)?;

    Ok(EdgeExtractionResult {
        edges_created: count,
        sessions_processed: session_count,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Get files that appear as source in >80% of all sessions.
///
/// These are "universal anchors" (e.g., CLAUDE.md) — too common to form
/// meaningful `read_before` edges FROM, but valid `reference_anchor` targets.
pub async fn get_universal_anchors(pool: &SqlitePool) -> Result<HashSet<String>, CoreError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT file_path, COUNT(DISTINCT session_id) as session_count
           FROM file_access_events
           WHERE access_type = 'read'
           GROUP BY file_path
           HAVING session_count > (
               SELECT COUNT(DISTINCT session_id) * ?1 FROM file_access_events
           )"#,
    )
    .bind(UNIVERSAL_ANCHOR_THRESHOLD)
    .fetch_all(pool)
    .await
    .map_err(CoreError::Database)?;

    Ok(rows.into_iter().map(|(path, _)| path).collect())
}

// ---------------------------------------------------------------------------
// Internal: DB queries
// ---------------------------------------------------------------------------

/// Get distinct session IDs that have events since the given timestamp.
async fn get_sessions_since(
    pool: &SqlitePool,
    since: Option<&str>,
) -> Result<Vec<String>, CoreError> {
    let rows: Vec<(String,)> = if let Some(since_ts) = since {
        sqlx::query_as(
            "SELECT DISTINCT session_id FROM file_access_events \
             WHERE session_id IN (\
                 SELECT DISTINCT session_id FROM file_access_events \
                 WHERE timestamp > ?1\
             )",
        )
        .bind(since_ts)
        .fetch_all(pool)
        .await
        .map_err(CoreError::Database)?
    } else {
        sqlx::query_as("SELECT DISTINCT session_id FROM file_access_events")
            .fetch_all(pool)
            .await
            .map_err(CoreError::Database)?
    };

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

// ---------------------------------------------------------------------------
// Internal: Noise filtering
// ---------------------------------------------------------------------------

/// Detect explore agent bursts: >5 consecutive reads in 30s with no writes.
///
/// Returns references to events that are NOT part of explore bursts.
fn filter_explore_bursts(events: &[FileAccessEvent]) -> Vec<&FileAccessEvent> {
    if events.is_empty() {
        return Vec::new();
    }

    // Identify burst ranges
    let mut burst_indices: HashSet<usize> = HashSet::new();

    // Sliding window: find runs of consecutive reads
    let mut run_start: Option<usize> = None;

    for (i, event) in events.iter().enumerate() {
        if event.access_type == "read" {
            if run_start.is_none() {
                run_start = Some(i);
            }
        } else {
            // Non-read breaks the run
            if let Some(start) = run_start {
                check_and_mark_burst(events, start, i, &mut burst_indices);
            }
            run_start = None;
        }
    }

    // Handle run that extends to end
    if let Some(start) = run_start {
        check_and_mark_burst(events, start, events.len(), &mut burst_indices);
    }

    events
        .iter()
        .enumerate()
        .filter(|(i, _)| !burst_indices.contains(i))
        .map(|(_, e)| e)
        .collect()
}

/// Check if a run of reads [start..end) is an explore burst and mark indices.
fn check_and_mark_burst(
    events: &[FileAccessEvent],
    start: usize,
    end: usize,
    burst_indices: &mut HashSet<usize>,
) {
    let run_len = end - start;
    if run_len <= EXPLORE_BURST_MIN_READS {
        return;
    }

    // Check time span
    let first_ts = events[start].timestamp;
    let last_ts = events[end - 1].timestamp;
    let span_seconds = (last_ts - first_ts).num_milliseconds() as f64 / 1000.0;

    if span_seconds <= EXPLORE_BURST_WINDOW_SECONDS {
        for i in start..end {
            burst_indices.insert(i);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal: Per-session edge extraction
// ---------------------------------------------------------------------------

/// Extract edge candidates from a single session's (filtered) events.
///
/// Produces candidates for:
/// - `read_then_edit`: every (read, later_write) pair within 5 min
/// - `co_edited`: every pair of writes (lexically ordered)
/// - `debug_chain`: reads after Bash tool calls
fn extract_session_edges(events: &[&FileAccessEvent]) -> Vec<EdgeCandidate> {
    if events.is_empty() {
        return Vec::new();
    }

    let session_id = &events[0].session_id;
    let mut candidates = Vec::new();

    // --- Reduce to unique files (O(n) single pass) ---
    // The aggregation only cares about unique (source, target, edge_type) per session.
    // Operating on unique file pairs instead of event pairs: O(unique_files²) vs O(events²).
    struct FileOccurrence {
        earliest_timestamp: DateTime<Utc>,
        earliest_seq: i32,
    }

    let mut read_files: HashMap<&str, FileOccurrence> = HashMap::new();
    let mut write_files: HashMap<&str, FileOccurrence> = HashMap::new();

    for event in events {
        let map = match event.access_type.as_str() {
            "read" => &mut read_files,
            "write" => &mut write_files,
            _ => continue,
        };
        map.entry(event.file_path.as_str())
            .and_modify(|occ| {
                if event.sequence_position < occ.earliest_seq {
                    occ.earliest_timestamp = event.timestamp;
                    occ.earliest_seq = event.sequence_position;
                }
            })
            .or_insert(FileOccurrence {
                earliest_timestamp: event.timestamp,
                earliest_seq: event.sequence_position,
            });
    }

    // --- read_then_edit ---
    // For each unique (read_file, write_file) pair where write came after read
    for (r_file, r_occ) in &read_files {
        for (w_file, w_occ) in &write_files {
            if w_occ.earliest_seq > r_occ.earliest_seq && *r_file != *w_file {
                let delta = (w_occ.earliest_timestamp - r_occ.earliest_timestamp).num_milliseconds()
                    as f64
                    / 1000.0;
                if (0.0..=MAX_TIME_DELTA_SECONDS).contains(&delta) {
                    candidates.push(EdgeCandidate {
                        source_file: r_file.to_string(),
                        target_file: w_file.to_string(),
                        edge_type: "read_then_edit".to_string(),
                        session_id: session_id.clone(),
                        time_delta_seconds: Some(delta),
                    });
                }
            }
        }
    }

    // --- read_before ---
    // Unique read files sorted by earliest sequence position
    let mut sorted_reads: Vec<(&str, &FileOccurrence)> =
        read_files.iter().map(|(k, v)| (*k, v)).collect();
    sorted_reads.sort_by_key(|(_, occ)| occ.earliest_seq);

    for (i, (a_file, a_occ)) in sorted_reads.iter().enumerate() {
        for (b_file, b_occ) in sorted_reads.iter().skip(i + 1) {
            if *a_file != *b_file {
                let delta = (b_occ.earliest_timestamp - a_occ.earliest_timestamp).num_milliseconds()
                    as f64
                    / 1000.0;
                if (0.0..=MAX_TIME_DELTA_SECONDS).contains(&delta) {
                    candidates.push(EdgeCandidate {
                        source_file: a_file.to_string(),
                        target_file: b_file.to_string(),
                        edge_type: "read_before".to_string(),
                        session_id: session_id.clone(),
                        time_delta_seconds: Some(delta),
                    });
                }
            }
        }
    }

    // --- co_edited ---
    // Unique write files, lexically sorted pairs
    let mut write_list: Vec<&str> = write_files.keys().copied().collect();
    write_list.sort();
    for (i, a) in write_list.iter().enumerate() {
        for b in write_list.iter().skip(i + 1) {
            candidates.push(EdgeCandidate {
                source_file: a.to_string(),
                target_file: b.to_string(),
                edge_type: "co_edited".to_string(),
                session_id: session_id.clone(),
                time_delta_seconds: None,
            });
        }
    }

    // --- debug_chain ---
    // File read after a Bash tool call (unchanged — already O(n) linear scan)
    for (i, event) in events.iter().enumerate() {
        if event.tool_name == "Bash" || event.tool_name == "bash" {
            // Check the immediately next event for a read
            if let Some(next) = events.get(i + 1) {
                if next.access_type == "read" {
                    let delta =
                        (next.timestamp - event.timestamp).num_milliseconds() as f64 / 1000.0;
                    if (0.0..=MAX_TIME_DELTA_SECONDS).contains(&delta) {
                        candidates.push(EdgeCandidate {
                            source_file: event.file_path.clone(),
                            target_file: next.file_path.clone(),
                            edge_type: "debug_chain".to_string(),
                            session_id: session_id.clone(),
                            time_delta_seconds: Some(delta),
                        });
                    }
                }
            }
        }
    }

    candidates
}

/// Extract reference_anchor candidates from cross-session data.
///
/// Files read in the first 2 minutes of >3 sessions are reference anchors.
async fn extract_reference_anchors(pool: &SqlitePool) -> Result<Vec<EdgeCandidate>, CoreError> {
    // Get first timestamp per session
    let session_starts: Vec<(String, String)> = sqlx::query_as(
        "SELECT session_id, MIN(timestamp) as first_ts \
         FROM file_access_events \
         GROUP BY session_id",
    )
    .fetch_all(pool)
    .await
    .map_err(CoreError::Database)?;

    let start_map: HashMap<String, DateTime<Utc>> = session_starts
        .into_iter()
        .filter_map(|(sid, ts)| ts.parse::<DateTime<Utc>>().ok().map(|dt| (sid, dt)))
        .collect();

    // Get all read events
    let all_reads: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT session_id, timestamp, file_path \
         FROM file_access_events \
         WHERE access_type = 'read' \
         ORDER BY session_id, sequence_position",
    )
    .fetch_all(pool)
    .await
    .map_err(CoreError::Database)?;

    // Count files read in first 2 min per session
    let mut file_early_sessions: HashMap<String, HashSet<String>> = HashMap::new();

    for (sid, ts_str, file_path) in &all_reads {
        if let (Some(session_start), Ok(event_ts)) =
            (start_map.get(sid), ts_str.parse::<DateTime<Utc>>())
        {
            let delta = (event_ts - session_start).num_milliseconds() as f64 / 1000.0;
            if delta <= REFERENCE_ANCHOR_WINDOW_SECONDS {
                file_early_sessions
                    .entry(file_path.clone())
                    .or_default()
                    .insert(sid.clone());
            }
        }
    }

    let mut candidates = Vec::new();
    for (file_path, sessions) in &file_early_sessions {
        if sessions.len() as i32 >= REFERENCE_ANCHOR_MIN_SESSIONS {
            // Create one candidate per session to get proper session_count in aggregation
            for sid in sessions {
                candidates.push(EdgeCandidate {
                    source_file: file_path.clone(),
                    target_file: file_path.clone(), // self-edge
                    edge_type: "reference_anchor".to_string(),
                    session_id: sid.clone(),
                    time_delta_seconds: None,
                });
            }
        }
    }

    Ok(candidates)
}

// ---------------------------------------------------------------------------
// Internal: Cross-session aggregation
// ---------------------------------------------------------------------------

/// Aggregate edge candidates across sessions into final edges.
///
/// Groups by (source, target, edge_type) and counts distinct sessions.
/// Computes confidence as session_count / total_sessions_with_source.
fn aggregate_edge_candidates(
    candidates: &[EdgeCandidate],
    total_sessions: usize,
) -> Vec<AggregatedEdge> {
    if candidates.is_empty() {
        return Vec::new();
    }

    // Group by (source, target, edge_type)
    struct CandidateGroup {
        sessions: HashSet<String>,
        time_deltas: Vec<f64>,
    }

    let mut groups: HashMap<(String, String, String), CandidateGroup> = HashMap::new();

    for c in candidates {
        let key = (
            c.source_file.clone(),
            c.target_file.clone(),
            c.edge_type.clone(),
        );
        let group = groups.entry(key).or_insert_with(|| CandidateGroup {
            sessions: HashSet::new(),
            time_deltas: Vec::new(),
        });
        group.sessions.insert(c.session_id.clone());
        if let Some(td) = c.time_delta_seconds {
            group.time_deltas.push(td);
        }
    }

    // Count total sessions per source file (for confidence calculation)
    let mut source_session_counts: HashMap<String, HashSet<String>> = HashMap::new();
    for c in candidates {
        source_session_counts
            .entry(c.source_file.clone())
            .or_default()
            .insert(c.session_id.clone());
    }

    // Count total sessions per target file (for lift calculation)
    let mut target_session_counts: HashMap<String, HashSet<String>> = HashMap::new();
    for c in candidates {
        target_session_counts
            .entry(c.target_file.clone())
            .or_default()
            .insert(c.session_id.clone());
    }

    let total = total_sessions.max(1) as f64;

    groups
        .into_iter()
        .map(|((source, target, edge_type), group)| {
            let session_count = group.sessions.len() as i32;
            let total_sessions_with_source = source_session_counts
                .get(&source)
                .map(|s| s.len() as i32)
                .unwrap_or(1);
            let total_sessions_with_target = target_session_counts
                .get(&target)
                .map(|s| s.len() as i32)
                .unwrap_or(1);

            let confidence = if total_sessions_with_source > 0 {
                session_count as f64 / total_sessions_with_source as f64
            } else {
                0.0
            };

            // Lift: how much more likely this edge is vs random chance
            // lift = (edge_sessions * total) / (source_sessions * target_sessions)
            let lift = (session_count as f64 * total)
                / (total_sessions_with_source as f64 * total_sessions_with_target as f64).max(1.0);

            let avg_time_delta = if group.time_deltas.is_empty() {
                None
            } else {
                let sum: f64 = group.time_deltas.iter().sum();
                Some(sum / group.time_deltas.len() as f64)
            };

            AggregatedEdge {
                source_file: source,
                target_file: target,
                edge_type,
                session_count,
                total_sessions_with_source,
                avg_time_delta_seconds: avg_time_delta,
                confidence,
                lift,
                first_seen: None,
                last_seen: None,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Internal: Noise filters (post-aggregation)
// ---------------------------------------------------------------------------

/// Apply noise filters to aggregated edges:
/// 1. Minimum session count
/// 2. Universal anchor dampening (no read_before FROM universal anchors)
fn apply_noise_filters(
    edges: &[AggregatedEdge],
    universal_anchors: &HashSet<String>,
) -> Vec<AggregatedEdge> {
    edges
        .iter()
        .filter(|e| {
            // Filter 1: absolute minimum session count
            if e.session_count < MIN_SESSION_COUNT {
                return false;
            }

            // Filter 2: lift guard for borderline edges
            // session_count=2 requires high lift to survive
            if e.session_count < 3 && e.lift < MIN_LIFT_THRESHOLD {
                return false;
            }

            // Filter 3: universal anchor dampening
            // Don't create read_before edges FROM universal anchors
            if e.edge_type == "read_before" && universal_anchors.contains(&e.source_file) {
                return false;
            }

            true
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Internal: DB writes
// ---------------------------------------------------------------------------

/// Upsert aggregated edges into the file_edges table.
///
/// Uses INSERT OR REPLACE keyed on the UNIQUE index (source_file, target_file, edge_type).
async fn upsert_edges(pool: &SqlitePool, edges: &[AggregatedEdge]) -> Result<usize, CoreError> {
    if edges.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(CoreError::Database)?;

    for edge in edges {
        sqlx::query(
            "INSERT OR REPLACE INTO file_edges \
             (source_file, target_file, edge_type, session_count, \
              total_sessions_with_source, avg_time_delta_seconds, confidence, \
              lift, first_seen, last_seen) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&edge.source_file)
        .bind(&edge.target_file)
        .bind(&edge.edge_type)
        .bind(edge.session_count)
        .bind(edge.total_sessions_with_source)
        .bind(edge.avg_time_delta_seconds)
        .bind(edge.confidence)
        .bind(edge.lift)
        .bind(&edge.first_seen)
        .bind(&edge.last_seen)
        .execute(&mut *tx)
        .await
        .map_err(CoreError::Database)?;
    }

    tx.commit().await.map_err(CoreError::Database)?;

    Ok(edges.len())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Create a FileAccessEvent for testing.
    fn make_event(
        session_id: &str,
        timestamp_str: &str,
        file_path: &str,
        tool_name: &str,
        access_type: &str,
        seq: i32,
    ) -> FileAccessEvent {
        FileAccessEvent {
            session_id: session_id.to_string(),
            timestamp: timestamp_str
                .parse::<DateTime<Utc>>()
                .expect("invalid test timestamp"),
            file_path: file_path.to_string(),
            tool_name: tool_name.to_string(),
            access_type: access_type.to_string(),
            sequence_position: seq,
        }
    }

    /// Schema SQL for creating test databases.
    const TEST_SCHEMA: &str = r#"
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

        CREATE TABLE IF NOT EXISTS file_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_file TEXT NOT NULL,
            target_file TEXT NOT NULL,
            edge_type TEXT NOT NULL,
            session_count INTEGER NOT NULL DEFAULT 0,
            total_sessions_with_source INTEGER NOT NULL DEFAULT 0,
            avg_time_delta_seconds REAL,
            confidence REAL NOT NULL DEFAULT 0.0,
            lift REAL,
            first_seen TEXT,
            last_seen TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_fe_source ON file_edges(source_file, edge_type);
        CREATE INDEX IF NOT EXISTS idx_fe_target ON file_edges(target_file, edge_type);
        CREATE INDEX IF NOT EXISTS idx_fe_type_conf ON file_edges(edge_type, confidence DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_fe_unique ON file_edges(source_file, target_file, edge_type);

        CREATE TABLE IF NOT EXISTS _metadata (
            key TEXT PRIMARY KEY,
            value TEXT,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
    "#;

    /// Create an in-memory SQLite pool with the temporal tables schema.
    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(TEST_SCHEMA).execute(&pool).await.unwrap();
        pool
    }

    /// Insert a file access event into the test DB.
    async fn insert_event(
        pool: &SqlitePool,
        session_id: &str,
        timestamp: &str,
        file_path: &str,
        tool_name: &str,
        access_type: &str,
        seq: i32,
    ) {
        sqlx::query(
            "INSERT INTO file_access_events \
             (session_id, timestamp, file_path, tool_name, access_type, sequence_position) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(timestamp)
        .bind(file_path)
        .bind(tool_name)
        .bind(access_type)
        .bind(seq)
        .execute(pool)
        .await
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // Unit tests: filter_explore_bursts
    // -----------------------------------------------------------------------

    #[test]
    fn test_filter_explore_bursts_removes_rapid_reads() {
        // 8 reads in 10 seconds → all filtered (>5 reads in <30s)
        let events: Vec<FileAccessEvent> = (0..8)
            .map(|i| {
                make_event(
                    "s1",
                    &format!("2026-02-17T10:00:{:02}.000Z", i),
                    &format!("file{}.rs", i),
                    "Read",
                    "read",
                    i,
                )
            })
            .collect();

        let filtered = filter_explore_bursts(&events);
        assert!(
            filtered.is_empty(),
            "All 8 rapid reads should be filtered as explore burst, got {} events",
            filtered.len()
        );
    }

    #[test]
    fn test_filter_explore_bursts_preserves_normal_reads() {
        // 3 reads over 2 minutes → all preserved (only 3 reads, under threshold)
        let events = vec![
            make_event("s1", "2026-02-17T10:00:00.000Z", "a.rs", "Read", "read", 0),
            make_event("s1", "2026-02-17T10:01:00.000Z", "b.rs", "Read", "read", 1),
            make_event("s1", "2026-02-17T10:02:00.000Z", "c.rs", "Read", "read", 2),
        ];

        let filtered = filter_explore_bursts(&events);
        assert_eq!(filtered.len(), 3, "3 normal reads should all be preserved");
    }

    #[test]
    fn test_filter_explore_bursts_preserves_reads_with_interleaved_writes() {
        // Reads interleaved with writes are not bursts
        let events = vec![
            make_event("s1", "2026-02-17T10:00:00.000Z", "a.rs", "Read", "read", 0),
            make_event("s1", "2026-02-17T10:00:01.000Z", "b.rs", "Read", "read", 1),
            make_event("s1", "2026-02-17T10:00:02.000Z", "c.rs", "Edit", "write", 2),
            make_event("s1", "2026-02-17T10:00:03.000Z", "d.rs", "Read", "read", 3),
            make_event("s1", "2026-02-17T10:00:04.000Z", "e.rs", "Read", "read", 4),
            make_event("s1", "2026-02-17T10:00:05.000Z", "f.rs", "Read", "read", 5),
        ];

        let filtered = filter_explore_bursts(&events);
        // The write at index 2 breaks any run, so no run exceeds 5 consecutive reads
        assert_eq!(
            filtered.len(),
            6,
            "Interleaved writes break bursts — all events preserved"
        );
    }

    #[test]
    fn test_filter_explore_bursts_empty_input() {
        let events: Vec<FileAccessEvent> = Vec::new();
        let filtered = filter_explore_bursts(&events);
        assert!(filtered.is_empty());
    }

    // -----------------------------------------------------------------------
    // Unit tests: extract_session_edges
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_session_edges_finds_read_then_edit() {
        // R(A) at t=0 → R(B) at t=10 → W(C) at t=30
        // Expected: read_then_edit(A→C) and read_then_edit(B→C)
        let events = vec![
            make_event("s1", "2026-02-17T10:00:00.000Z", "a.rs", "Read", "read", 0),
            make_event("s1", "2026-02-17T10:00:10.000Z", "b.rs", "Read", "read", 1),
            make_event("s1", "2026-02-17T10:00:30.000Z", "c.rs", "Edit", "write", 2),
        ];

        let refs: Vec<&FileAccessEvent> = events.iter().collect();
        let candidates = extract_session_edges(&refs);

        let rte: Vec<&EdgeCandidate> = candidates
            .iter()
            .filter(|c| c.edge_type == "read_then_edit")
            .collect();

        assert_eq!(rte.len(), 2, "Should find 2 read_then_edit edges");

        let has_a_to_c = rte
            .iter()
            .any(|c| c.source_file == "a.rs" && c.target_file == "c.rs");
        let has_b_to_c = rte
            .iter()
            .any(|c| c.source_file == "b.rs" && c.target_file == "c.rs");

        assert!(has_a_to_c, "Should have read_then_edit(a.rs → c.rs)");
        assert!(has_b_to_c, "Should have read_then_edit(b.rs → c.rs)");
    }

    #[test]
    fn test_extract_session_edges_finds_read_before() {
        // R(A) at t=0 → R(B) at t=30
        // Expected: read_before(A → B)
        let events = vec![
            make_event("s1", "2026-02-17T10:00:00.000Z", "a.rs", "Read", "read", 0),
            make_event("s1", "2026-02-17T10:00:30.000Z", "b.rs", "Read", "read", 1),
        ];

        let refs: Vec<&FileAccessEvent> = events.iter().collect();
        let candidates = extract_session_edges(&refs);

        let rb: Vec<&EdgeCandidate> = candidates
            .iter()
            .filter(|c| c.edge_type == "read_before")
            .collect();

        assert_eq!(rb.len(), 1, "Should find 1 read_before edge");
        assert_eq!(rb[0].source_file, "a.rs");
        assert_eq!(rb[0].target_file, "b.rs");
    }

    #[test]
    fn test_extract_session_edges_finds_co_edited() {
        // W(A) → W(B) → co_edited(A, B) where A < B lexically
        let events = vec![
            make_event(
                "s1",
                "2026-02-17T10:00:00.000Z",
                "z_file.rs",
                "Edit",
                "write",
                0,
            ),
            make_event(
                "s1",
                "2026-02-17T10:01:00.000Z",
                "a_file.rs",
                "Edit",
                "write",
                1,
            ),
        ];

        let refs: Vec<&FileAccessEvent> = events.iter().collect();
        let candidates = extract_session_edges(&refs);

        let co: Vec<&EdgeCandidate> = candidates
            .iter()
            .filter(|c| c.edge_type == "co_edited")
            .collect();

        assert_eq!(co.len(), 1, "Should find 1 co_edited edge");
        // Lexically ordered: a_file < z_file
        assert_eq!(co[0].source_file, "a_file.rs");
        assert_eq!(co[0].target_file, "z_file.rs");
    }

    #[test]
    fn test_extract_session_edges_read_then_edit_respects_time_window() {
        // R(A) at t=0 → W(B) at t=6min — outside 5-min window
        let events = vec![
            make_event("s1", "2026-02-17T10:00:00.000Z", "a.rs", "Read", "read", 0),
            make_event("s1", "2026-02-17T10:06:00.000Z", "b.rs", "Edit", "write", 1),
        ];

        let refs: Vec<&FileAccessEvent> = events.iter().collect();
        let candidates = extract_session_edges(&refs);

        let rte: Vec<&EdgeCandidate> = candidates
            .iter()
            .filter(|c| c.edge_type == "read_then_edit")
            .collect();

        assert!(
            rte.is_empty(),
            "Should not find read_then_edit outside 5-min window"
        );
    }

    #[test]
    fn test_extract_session_edges_no_self_read_then_edit() {
        // R(A) → W(A) — same file, should NOT produce read_then_edit
        let events = vec![
            make_event("s1", "2026-02-17T10:00:00.000Z", "a.rs", "Read", "read", 0),
            make_event("s1", "2026-02-17T10:00:30.000Z", "a.rs", "Edit", "write", 1),
        ];

        let refs: Vec<&FileAccessEvent> = events.iter().collect();
        let candidates = extract_session_edges(&refs);

        let rte: Vec<&EdgeCandidate> = candidates
            .iter()
            .filter(|c| c.edge_type == "read_then_edit")
            .collect();

        assert!(rte.is_empty(), "Should not create read_then_edit self-edge");
    }

    #[test]
    fn test_extract_session_edges_empty_input() {
        let events: Vec<&FileAccessEvent> = Vec::new();
        let candidates = extract_session_edges(&events);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_extract_session_edges_deduplicates_repeated_file_access() {
        // A session reads types.rs 100 times and writes query.rs 50 times.
        // Should produce exactly 1 read_then_edit candidate, not 5000.
        let base = Utc::now();
        let mut events = Vec::new();
        for i in 0..100 {
            events.push(FileAccessEvent {
                session_id: "s1".into(),
                timestamp: base + chrono::Duration::seconds(i),
                file_path: "types.rs".into(),
                tool_name: "Read".into(),
                access_type: "read".into(),
                sequence_position: i as i32,
            });
        }
        for i in 0..50 {
            events.push(FileAccessEvent {
                session_id: "s1".into(),
                timestamp: base + chrono::Duration::seconds(100 + i),
                file_path: "query.rs".into(),
                tool_name: "Edit".into(),
                access_type: "write".into(),
                sequence_position: (100 + i) as i32,
            });
        }
        let refs: Vec<&FileAccessEvent> = events.iter().collect();
        let candidates = extract_session_edges(&refs);
        let rte: Vec<_> = candidates
            .iter()
            .filter(|c| c.edge_type == "read_then_edit")
            .collect();
        assert_eq!(
            rte.len(),
            1,
            "Should deduplicate to 1 candidate per file pair, got {}",
            rte.len()
        );
        assert_eq!(rte[0].source_file, "types.rs");
        assert_eq!(rte[0].target_file, "query.rs");
    }

    // -----------------------------------------------------------------------
    // Unit tests: aggregate_edge_candidates
    // -----------------------------------------------------------------------

    #[test]
    fn test_aggregate_edge_candidates_counts_sessions() {
        // Same edge from 3 different sessions → session_count = 3
        let candidates = vec![
            EdgeCandidate {
                source_file: "a.rs".to_string(),
                target_file: "b.rs".to_string(),
                edge_type: "read_then_edit".to_string(),
                session_id: "s1".to_string(),
                time_delta_seconds: Some(10.0),
            },
            EdgeCandidate {
                source_file: "a.rs".to_string(),
                target_file: "b.rs".to_string(),
                edge_type: "read_then_edit".to_string(),
                session_id: "s2".to_string(),
                time_delta_seconds: Some(20.0),
            },
            EdgeCandidate {
                source_file: "a.rs".to_string(),
                target_file: "b.rs".to_string(),
                edge_type: "read_then_edit".to_string(),
                session_id: "s3".to_string(),
                time_delta_seconds: Some(30.0),
            },
        ];

        let aggregated = aggregate_edge_candidates(&candidates, 100);
        assert_eq!(aggregated.len(), 1, "Should aggregate to 1 edge");

        let edge = &aggregated[0];
        assert_eq!(edge.session_count, 3);
        assert_eq!(edge.source_file, "a.rs");
        assert_eq!(edge.target_file, "b.rs");
        assert_eq!(edge.edge_type, "read_then_edit");

        // Avg time delta: (10 + 20 + 30) / 3 = 20.0
        let avg = edge.avg_time_delta_seconds.unwrap();
        assert!((avg - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_aggregate_edge_candidates_deduplicates_same_session() {
        // Same edge from same session twice → session_count still 1
        let candidates = vec![
            EdgeCandidate {
                source_file: "a.rs".to_string(),
                target_file: "b.rs".to_string(),
                edge_type: "read_then_edit".to_string(),
                session_id: "s1".to_string(),
                time_delta_seconds: Some(10.0),
            },
            EdgeCandidate {
                source_file: "a.rs".to_string(),
                target_file: "b.rs".to_string(),
                edge_type: "read_then_edit".to_string(),
                session_id: "s1".to_string(),
                time_delta_seconds: Some(20.0),
            },
        ];

        let aggregated = aggregate_edge_candidates(&candidates, 100);
        assert_eq!(aggregated.len(), 1);
        assert_eq!(
            aggregated[0].session_count, 1,
            "Same session should be counted once"
        );
    }

    #[test]
    fn test_aggregate_edge_candidates_empty() {
        let candidates: Vec<EdgeCandidate> = Vec::new();
        let aggregated = aggregate_edge_candidates(&candidates, 100);
        assert!(aggregated.is_empty());
    }

    // -----------------------------------------------------------------------
    // Unit tests: noise filters
    // -----------------------------------------------------------------------

    #[test]
    fn test_noise_filter_min_session_count() {
        // Edge in 1 session → always filtered (< MIN_SESSION_COUNT = 2)
        let edges = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 1,
            total_sessions_with_source: 5,
            avg_time_delta_seconds: Some(15.0),
            confidence: 0.4,
            lift: 100.0, // Even high lift can't save session_count=1
            first_seen: None,
            last_seen: None,
        }];

        let anchors = HashSet::new();
        let filtered = apply_noise_filters(&edges, &anchors);
        assert!(
            filtered.is_empty(),
            "Edge with session_count=1 should always be filtered"
        );
    }

    #[test]
    fn test_noise_filter_session_2_high_lift_survives() {
        // session_count=2 with high lift → survives
        let edges = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 2,
            total_sessions_with_source: 3,
            avg_time_delta_seconds: Some(15.0),
            confidence: 0.5,
            lift: 8.0, // Highly significant
            first_seen: None,
            last_seen: None,
        }];

        let anchors = HashSet::new();
        let filtered = apply_noise_filters(&edges, &anchors);
        assert_eq!(
            filtered.len(),
            1,
            "session_count=2 with high lift should survive"
        );
    }

    #[test]
    fn test_noise_filter_session_2_low_lift_filtered() {
        // session_count=2 with low lift → filtered (coincidental)
        let edges = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 2,
            total_sessions_with_source: 200,
            avg_time_delta_seconds: Some(15.0),
            confidence: 0.5,
            lift: 0.5, // Coincidental overlap
            first_seen: None,
            last_seen: None,
        }];

        let anchors = HashSet::new();
        let filtered = apply_noise_filters(&edges, &anchors);
        assert!(
            filtered.is_empty(),
            "session_count=2 with low lift should be filtered"
        );
    }

    #[test]
    fn test_noise_filter_session_3_survives_regardless_of_lift() {
        // session_count=3 with low lift → survives (enough sessions)
        let edges = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 3,
            total_sessions_with_source: 500,
            avg_time_delta_seconds: Some(15.0),
            confidence: 0.5,
            lift: 0.3, // Low lift but enough sessions
            first_seen: None,
            last_seen: None,
        }];

        let anchors = HashSet::new();
        let filtered = apply_noise_filters(&edges, &anchors);
        assert_eq!(
            filtered.len(),
            1,
            "session_count>=3 always survives regardless of lift"
        );
    }

    #[test]
    fn test_noise_filter_passes_sufficient_sessions() {
        let edges = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 3,
            total_sessions_with_source: 5,
            avg_time_delta_seconds: Some(15.0),
            confidence: 0.6,
            lift: 1.0,
            first_seen: None,
            last_seen: None,
        }];

        let anchors = HashSet::new();
        let filtered = apply_noise_filters(&edges, &anchors);
        assert_eq!(
            filtered.len(),
            1,
            "Edge with session_count=3 should pass min filter"
        );
    }

    #[test]
    fn test_noise_filter_universal_anchor_dampening() {
        // read_before FROM a universal anchor → filtered
        let edges = vec![AggregatedEdge {
            source_file: "CLAUDE.md".to_string(),
            target_file: "query.rs".to_string(),
            edge_type: "read_before".to_string(),
            session_count: 10,
            total_sessions_with_source: 12,
            avg_time_delta_seconds: Some(5.0),
            confidence: 0.83,
            lift: 1.0,
            first_seen: None,
            last_seen: None,
        }];

        let mut anchors = HashSet::new();
        anchors.insert("CLAUDE.md".to_string());

        let filtered = apply_noise_filters(&edges, &anchors);
        assert!(
            filtered.is_empty(),
            "read_before FROM universal anchor should be filtered"
        );
    }

    #[test]
    fn test_noise_filter_universal_anchor_allows_other_edge_types() {
        // co_edited from universal anchor — NOT filtered (only read_before is dampened)
        let edges = vec![AggregatedEdge {
            source_file: "CLAUDE.md".to_string(),
            target_file: "query.rs".to_string(),
            edge_type: "co_edited".to_string(),
            session_count: 5,
            total_sessions_with_source: 12,
            avg_time_delta_seconds: None,
            confidence: 0.42,
            lift: 1.0,
            first_seen: None,
            last_seen: None,
        }];

        let mut anchors = HashSet::new();
        anchors.insert("CLAUDE.md".to_string());

        let filtered = apply_noise_filters(&edges, &anchors);
        assert_eq!(
            filtered.len(),
            1,
            "co_edited from universal anchor should NOT be filtered"
        );
    }

    // -----------------------------------------------------------------------
    // Async integration tests (with temp DB)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_extract_file_edges_end_to_end() {
        let pool = create_test_pool().await;

        // Insert events for 5 sessions with a consistent pattern:
        // Each session: R(types.rs) → R(storage.rs) → W(query.rs)
        for i in 0..5 {
            let sid = format!("session-{}", i);
            let base_min = i * 10; // spread sessions apart
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:00.000Z", base_min),
                "types.rs",
                "Read",
                "read",
                0,
            )
            .await;
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:30.000Z", base_min),
                "storage.rs",
                "Read",
                "read",
                1,
            )
            .await;
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:00.000Z", base_min + 1),
                "query.rs",
                "Edit",
                "write",
                2,
            )
            .await;
        }

        let result = extract_file_edges(&pool, None).await.unwrap();

        assert_eq!(result.sessions_processed, 5);
        assert!(result.edges_created > 0, "Should have created some edges");

        // Verify edges in DB
        let edge_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM file_edges")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(edge_count.0 > 0, "file_edges table should have rows");

        // Check for read_then_edit(types.rs → query.rs)
        let rte: Vec<(String, String, i32, f64)> = sqlx::query_as(
            "SELECT source_file, target_file, session_count, confidence \
             FROM file_edges \
             WHERE edge_type = 'read_then_edit' \
               AND source_file = 'types.rs' AND target_file = 'query.rs'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            rte.len(),
            1,
            "Should have read_then_edit(types.rs → query.rs)"
        );
        assert_eq!(rte[0].2, 5, "Should appear in 5 sessions");

        // read_before(types.rs → storage.rs) should NOT exist because types.rs
        // is a universal anchor (appears in 100% of sessions > 80% threshold).
        // Universal anchor dampening correctly filters read_before FROM anchors.
        let rb: Vec<(String, String, i32)> = sqlx::query_as(
            "SELECT source_file, target_file, session_count \
             FROM file_edges \
             WHERE edge_type = 'read_before' \
               AND source_file = 'types.rs' AND target_file = 'storage.rs'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            rb.len(),
            0,
            "read_before FROM universal anchor should be dampened"
        );

        // Verify extraction timestamp was recorded
        let meta: (String,) =
            sqlx::query_as("SELECT value FROM _metadata WHERE key = 'last_edge_extraction'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            !meta.0.is_empty(),
            "Should have recorded extraction timestamp"
        );
    }

    #[tokio::test]
    async fn test_extract_file_edges_incremental() {
        let pool = create_test_pool().await;

        // Phase 1: Insert events for 3 sessions
        for i in 0..3 {
            let sid = format!("session-{}", i);
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:00.000Z", i * 10),
                "a.rs",
                "Read",
                "read",
                0,
            )
            .await;
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:30.000Z", i * 10),
                "b.rs",
                "Edit",
                "write",
                1,
            )
            .await;
        }

        // First extraction
        let result1 = extract_file_edges(&pool, None).await.unwrap();
        assert_eq!(result1.sessions_processed, 3);

        // Get the extraction timestamp
        let ts: (String,) =
            sqlx::query_as("SELECT value FROM _metadata WHERE key = 'last_edge_extraction'")
                .fetch_one(&pool)
                .await
                .unwrap();

        // Phase 2: Add 2 more sessions with later timestamps
        for i in 3..5 {
            let sid = format!("session-{}", i);
            insert_event(
                &pool,
                &sid,
                "2026-02-18T10:00:00.000Z",
                "a.rs",
                "Read",
                "read",
                0,
            )
            .await;
            insert_event(
                &pool,
                &sid,
                "2026-02-18T10:00:30.000Z",
                "b.rs",
                "Edit",
                "write",
                1,
            )
            .await;
        }

        // Incremental extraction — only processes sessions with events after last extraction
        let result2 = extract_file_edges(&pool, Some(&ts.0)).await.unwrap();
        assert_eq!(
            result2.sessions_processed, 2,
            "Should only process the 2 new sessions"
        );
    }

    #[tokio::test]
    async fn test_universal_anchor_dampening() {
        let pool = create_test_pool().await;

        // Create 10 sessions. CLAUDE.md is read in all 10 (100% → universal anchor).
        // query.rs is read in all 10 too, but also written.
        for i in 0..10 {
            let sid = format!("session-{}", i);
            // CLAUDE.md read in every session
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:00.000Z", i),
                "CLAUDE.md",
                "Read",
                "read",
                0,
            )
            .await;
            // query.rs read after CLAUDE.md in every session
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:30.000Z", i),
                "query.rs",
                "Read",
                "read",
                1,
            )
            .await;
            // types.rs written in every session
            insert_event(
                &pool,
                &sid,
                &format!("2026-02-17T10:{:02}:00.000Z", i + 1),
                "types.rs",
                "Edit",
                "write",
                2,
            )
            .await;
        }

        let result = extract_file_edges(&pool, None).await.unwrap();
        assert_eq!(result.sessions_processed, 10);

        // CLAUDE.md should be a universal anchor (read in 100% sessions)
        let anchors = get_universal_anchors(&pool).await.unwrap();
        assert!(
            anchors.contains("CLAUDE.md"),
            "CLAUDE.md should be a universal anchor"
        );

        // No read_before FROM CLAUDE.md should exist in file_edges
        let rb_from_claude: Vec<(String, String)> = sqlx::query_as(
            "SELECT source_file, target_file FROM file_edges \
             WHERE edge_type = 'read_before' AND source_file = 'CLAUDE.md'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(
            rb_from_claude.is_empty(),
            "Should have no read_before edges FROM universal anchor CLAUDE.md, got {}",
            rb_from_claude.len()
        );

        // But reference_anchor for CLAUDE.md SHOULD exist (read in first 2 min of >3 sessions)
        let ref_anchors: Vec<(String,)> = sqlx::query_as(
            "SELECT source_file FROM file_edges \
             WHERE edge_type = 'reference_anchor' AND source_file = 'CLAUDE.md'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(
            !ref_anchors.is_empty(),
            "CLAUDE.md should have reference_anchor self-edge"
        );

        // read_then_edit FROM CLAUDE.md should still exist (not dampened)
        let rte_from_claude: Vec<(String, String)> = sqlx::query_as(
            "SELECT source_file, target_file FROM file_edges \
             WHERE edge_type = 'read_then_edit' AND source_file = 'CLAUDE.md'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(
            !rte_from_claude.is_empty(),
            "read_then_edit FROM CLAUDE.md should NOT be dampened"
        );
    }

    #[tokio::test]
    async fn test_extract_file_edges_no_events() {
        let pool = create_test_pool().await;

        let result = extract_file_edges(&pool, None).await.unwrap();
        assert_eq!(result.sessions_processed, 0);
        assert_eq!(result.edges_created, 0);
    }

    #[tokio::test]
    async fn test_upsert_edges_updates_existing() {
        let pool = create_test_pool().await;

        let edges1 = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 3,
            total_sessions_with_source: 5,
            avg_time_delta_seconds: Some(10.0),
            confidence: 0.6,
            lift: 1.0,
            first_seen: None,
            last_seen: None,
        }];

        upsert_edges(&pool, &edges1).await.unwrap();

        // Upsert again with updated values
        let edges2 = vec![AggregatedEdge {
            source_file: "a.rs".to_string(),
            target_file: "b.rs".to_string(),
            edge_type: "read_then_edit".to_string(),
            session_count: 7,
            total_sessions_with_source: 10,
            avg_time_delta_seconds: Some(15.0),
            confidence: 0.7,
            lift: 1.0,
            first_seen: None,
            last_seen: None,
        }];

        upsert_edges(&pool, &edges2).await.unwrap();

        // Should have exactly 1 row
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM file_edges")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1, "Upsert should not duplicate");

        let row: (i32, f64) = sqlx::query_as(
            "SELECT session_count, confidence FROM file_edges \
             WHERE source_file = 'a.rs' AND target_file = 'b.rs'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 7, "session_count should be updated");
        assert!((row.1 - 0.7).abs() < 0.01, "confidence should be updated");
    }
}
