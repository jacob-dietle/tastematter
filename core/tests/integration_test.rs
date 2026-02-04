//! Integration tests for context-os-core
//!
//! These tests run against the real database to verify:
//! 1. Queries work correctly
//! 2. Latency is <100ms

use std::time::Instant;
use tastematter::{
    Database, QueryChainsInput, QueryEngine, QueryFlexInput, QuerySessionsInput, QueryTimelineInput,
};

/// Get the path to the test database (canonical location)
fn get_test_db_path() -> String {
    // Use canonical location: ~/.context-os/context_os_events.db
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".context-os")
        .join("context_os_events.db")
        .to_string_lossy()
        .to_string()
}

#[tokio::test]
async fn test_database_opens() {
    let db_path = get_test_db_path();
    println!("Testing database at: {}", db_path);

    let result = Database::open(&db_path).await;
    assert!(
        result.is_ok(),
        "Failed to open database: {:?}",
        result.err()
    );

    let db = result.unwrap();
    println!("Database opened successfully at: {:?}", db.path());
}

#[tokio::test]
async fn test_query_flex_basic() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let start = Instant::now();
    let result = engine.query_flex(QueryFlexInput::default()).await;
    let elapsed = start.elapsed();

    println!("query_flex took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 100,
        "Query took too long: {:?}",
        elapsed
    );

    assert!(result.is_ok(), "Query failed: {:?}", result.err());
    let result = result.unwrap();

    println!("Result count: {}", result.result_count);
    println!("Receipt ID: {}", result.receipt_id);

    // Note: file_conversation_index may be empty - query should still succeed
    // The important thing is the query executed without error
    assert!(
        !result.receipt_id.is_empty(),
        "Receipt ID should be generated"
    );
}

#[tokio::test]
async fn test_query_flex_with_time_filter() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let input = QueryFlexInput {
        time: Some("7d".to_string()),
        limit: Some(10),
        ..Default::default()
    };

    let start = Instant::now();
    let result = engine.query_flex(input).await;
    let elapsed = start.elapsed();

    println!("query_flex with time filter took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 100,
        "Query took too long: {:?}",
        elapsed
    );

    assert!(result.is_ok(), "Query failed: {:?}", result.err());
    let result = result.unwrap();
    println!("Results with 7d filter: {}", result.result_count);
}

#[tokio::test]
async fn test_query_flex_with_aggregations() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let input = QueryFlexInput {
        agg: vec!["count".to_string(), "recency".to_string()],
        limit: Some(20),
        ..Default::default()
    };

    let start = Instant::now();
    let result = engine.query_flex(input).await;
    let elapsed = start.elapsed();

    println!("query_flex with aggregations took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 100,
        "Query took too long: {:?}",
        elapsed
    );

    let result = result.expect("Query failed");
    assert!(
        result.aggregations.count.is_some(),
        "Count aggregation should be present"
    );

    let count = result.aggregations.count.unwrap();
    println!(
        "Total files: {}, Total accesses: {}",
        count.total_files, count.total_accesses
    );
}

#[tokio::test]
async fn test_query_chains() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let start = Instant::now();
    let result = engine.query_chains(QueryChainsInput::default()).await;
    let elapsed = start.elapsed();

    println!("query_chains took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 100,
        "Query took too long: {:?}",
        elapsed
    );

    assert!(result.is_ok(), "Query failed: {:?}", result.err());
    let result = result.unwrap();

    println!("Total chains: {}", result.total_chains);
    for chain in &result.chains {
        println!(
            "  Chain {}: {} sessions, {} files",
            chain.chain_id, chain.session_count, chain.file_count
        );
    }
}

/// BUG-001: Chains with sessions should have file_count > 0
///
/// Root cause: query_chains reads from stale `chains.files_json` column
/// instead of computing file counts dynamically from session data.
///
/// Fix: Join chain_graph → claude_sessions → json_each(files_read)
/// to compute file counts dynamically.
#[tokio::test]
async fn test_query_chains_file_count_not_zero() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let result = engine.query_chains(QueryChainsInput::default()).await;
    assert!(result.is_ok(), "Query failed: {:?}", result.err());
    let result = result.unwrap();

    // Find chains that have sessions
    let chains_with_sessions: Vec<_> = result
        .chains
        .iter()
        .filter(|c| c.session_count > 0)
        .collect();

    println!("Chains with sessions: {}", chains_with_sessions.len());

    // If we have chains with sessions, at least some should have files
    // (sessions access files, so chains with sessions should have file_count > 0)
    if !chains_with_sessions.is_empty() {
        let chains_with_files: Vec<_> = chains_with_sessions
            .iter()
            .filter(|c| c.file_count > 0)
            .collect();

        println!("Chains with files: {}", chains_with_files.len());

        // BUG-001: This assertion should PASS after fix
        // Currently FAILS because chains.files_json is empty
        assert!(
            !chains_with_files.is_empty(),
            "BUG-001: Chains with sessions should have file_count > 0. \
             Found {} chains with sessions but 0 with files. \
             Root cause: query_chains reads from stale chains.files_json instead of computing dynamically.",
            chains_with_sessions.len()
        );
    }
}

#[tokio::test]
async fn test_query_timeline() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let input = QueryTimelineInput {
        time: "7d".to_string(),
        files: None,
        chain: None,
        limit: Some(30),
    };

    let start = Instant::now();
    let result = engine.query_timeline(input).await;
    let elapsed = start.elapsed();

    println!("query_timeline took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 100,
        "Query took too long: {:?}",
        elapsed
    );

    assert!(result.is_ok(), "Query failed: {:?}", result.err());
    let result = result.unwrap();

    println!("Timeline: {} -> {}", result.start_date, result.end_date);
    println!(
        "Buckets: {}, Files: {}",
        result.buckets.len(),
        result.files.len()
    );
    println!(
        "Summary: {} accesses, peak {} on {}",
        result.summary.total_accesses, result.summary.peak_count, result.summary.peak_day
    );
}

#[tokio::test]
async fn test_query_sessions() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    let input = QuerySessionsInput {
        time: "7d".to_string(),
        chain: None,
        limit: Some(10),
    };

    let start = Instant::now();
    let result = engine.query_sessions(input).await;
    let elapsed = start.elapsed();

    println!("query_sessions took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 200,
        "Query took too long: {:?}",
        elapsed
    );

    assert!(result.is_ok(), "Query failed: {:?}", result.err());
    let result = result.unwrap();

    println!(
        "Sessions: {}, Chains: {}",
        result.sessions.len(),
        result.chains.len()
    );
    println!(
        "Summary: {} sessions, {} files, {} chains",
        result.summary.total_sessions, result.summary.total_files, result.summary.active_chains
    );
}

#[tokio::test]
async fn test_latency_benchmark() {
    let db_path = get_test_db_path();
    let db = Database::open(&db_path)
        .await
        .expect("Failed to open database");
    let engine = QueryEngine::new(db);

    // Run multiple queries and measure average latency
    let mut times = Vec::new();

    for _ in 0..10 {
        let start = Instant::now();
        let _ = engine
            .query_flex(QueryFlexInput {
                time: Some("30d".to_string()),
                limit: Some(50),
                agg: vec!["count".to_string()],
                ..Default::default()
            })
            .await;
        times.push(start.elapsed());
    }

    let avg_ms: f64 = times.iter().map(|t| t.as_millis() as f64).sum::<f64>() / times.len() as f64;
    let max_ms = times.iter().map(|t| t.as_millis()).max().unwrap();
    let min_ms = times.iter().map(|t| t.as_millis()).min().unwrap();

    println!("\n=== Latency Benchmark ===");
    println!("Runs: {}", times.len());
    println!("Average: {:.2}ms", avg_ms);
    println!("Min: {}ms", min_ms);
    println!("Max: {}ms", max_ms);
    println!("Target: <100ms");
    println!("Status: {}", if max_ms < 100 { "PASS" } else { "FAIL" });

    assert!(max_ms < 100, "Max latency exceeded 100ms: {}ms", max_ms);
}

// =============================================================================
// Fresh Install TDD Tests (Phase: Database Write Path)
// =============================================================================

/// Test 2: Query Succeeds After Fresh Install
///
/// Verifies that query_flex() returns empty results (not an error) when
/// executed against a freshly created database with schema but no data.
///
/// This tests the critical user experience: after running `daemon once` on
/// a fresh install, query commands should succeed gracefully rather than
/// crashing with "table not found" errors.
#[tokio::test]
async fn test_query_succeeds_after_fresh_daemon_sync() {
    use tastematter::Database;

    // 1. Create temp database (simulates fresh install after daemon sync)
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("fresh_install.db");

    // 2. Open with write mode and create schema (what daemon sync does)
    let db = Database::open_rw(&db_path)
        .await
        .expect("Should create fresh database");

    db.ensure_schema()
        .await
        .expect("Schema creation should succeed");

    // 3. Re-open in read mode (what query commands do)
    // Note: We can use the existing db since it's already open
    let engine = QueryEngine::new(db);

    // 4. Execute query - should succeed with empty results
    let result = engine
        .query_flex(QueryFlexInput {
            time: Some("1d".to_string()),
            limit: Some(3),
            ..Default::default()
        })
        .await;

    // 5. Assert: Query succeeds (returns Ok, not Err)
    assert!(
        result.is_ok(),
        "Query should succeed on fresh database. Error: {:?}",
        result.err()
    );

    // 6. Assert: Returns empty results (not error)
    let result = result.unwrap();
    assert_eq!(
        result.result_count, 0,
        "Fresh database should return 0 results"
    );
    assert!(
        !result.receipt_id.is_empty(),
        "Receipt ID should still be generated"
    );
}
