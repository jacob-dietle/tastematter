# Phase 1: Core Foundation - TDD Test Plan

## Philosophy

> "I'm not a great programmer; I'm just a good programmer with great habits." - Kent Beck

**This is a TEST-FIRST implementation.** Every test in this document must be written BEFORE the corresponding implementation code.

**The Red-Green-Refactor Cycle:**
1. **RED** - Write test → Run → Should FAIL (proves test catches the issue)
2. **GREEN** - Write minimal code → Run → Should PASS
3. **REFACTOR** - Clean up → Run → Should still PASS
4. **COMMIT** - Save progress with test reference

**If a test passes before you write the code:** The test is wrong or you misunderstand the requirement.

---

## Test Hierarchy

Following the evidence-based testing pyramid:

```
Level 3: E2E Tests (Real Database)
├── Full query flow against production DB
├── Latency verification (<100ms)
└── Expected bugs found: 10-15
Time: 2 hours | Priority: CRITICAL

Level 2: Integration Tests (Real SQLite)
├── Query engine with test fixture DB
├── Type contract verification
└── Expected bugs found: 4-5
Time: 1 hour | Priority: HIGH

Level 1: Unit Tests (Mocked)
├── Input parsing
├── SQL generation
├── Result transformation
└── Expected bugs found: 0-1
Time: 30 min | Priority: MEDIUM
```

**Evidence from LinkedIn Pipeline:** E2E tests found 3x more bugs than unit tests combined.

---

## Test File Structure

```
apps/context-os-core/
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── types.rs
│   ├── storage.rs
│   └── query.rs
└── tests/
    ├── fixtures/
    │   ├── test.db              # Test SQLite database
    │   └── expected_output.json # Expected JSON for contract tests
    ├── unit/
    │   ├── test_types.rs        # Type serialization tests
    │   ├── test_sql_builder.rs  # SQL generation tests
    │   └── test_time_parser.rs  # Time range parsing tests
    ├── integration/
    │   ├── test_query_flex.rs   # query_flex with test DB
    │   ├── test_query_chains.rs # query_chains with test DB
    │   └── test_contracts.rs    # JSON contract verification
    └── e2e/
        ├── test_real_db.rs      # Tests against ~/.context-os/context.db
        └── test_latency.rs      # Performance verification
```

---

## Level 1: Unit Tests

### Test 1.1: QueryFlexInput Default Values

**Purpose:** Verify default input values are correct
**Write BEFORE:** `src/types.rs` QueryFlexInput implementation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_flex_input_defaults() {
        // Given: Default input
        let input = QueryFlexInput::default();

        // Then: All optional fields are None, agg is empty vec
        assert!(input.files.is_none());
        assert!(input.time.is_none());
        assert!(input.chain.is_none());
        assert!(input.session.is_none());
        assert!(input.agg.is_empty());
        assert!(input.limit.is_none());
        assert!(input.sort.is_none());
    }
}
```

**Red:** Run test → Should fail (QueryFlexInput doesn't exist yet)
**Green:** Implement QueryFlexInput with #[derive(Default)]
**Commit:** `test(types): QueryFlexInput default values`

---

### Test 1.2: Time Range Parsing

**Purpose:** Verify time range strings parse correctly to days
**Write BEFORE:** `src/types.rs` parse_time_range function

```rust
#[test]
fn test_parse_time_range_standard() {
    // Given: Standard time ranges
    // When/Then: Parse correctly
    assert_eq!(parse_time_range("7d").unwrap(), 7);
    assert_eq!(parse_time_range("14d").unwrap(), 14);
    assert_eq!(parse_time_range("30d").unwrap(), 30);
}

#[test]
fn test_parse_time_range_custom() {
    // Given: Custom time range
    // When/Then: Parse correctly
    assert_eq!(parse_time_range("3d").unwrap(), 3);
    assert_eq!(parse_time_range("90d").unwrap(), 90);
}

#[test]
fn test_parse_time_range_invalid() {
    // Given: Invalid time ranges
    // When/Then: Return error
    assert!(parse_time_range("invalid").is_err());
    assert!(parse_time_range("7").is_err());      // Missing 'd' suffix
    assert!(parse_time_range("d7").is_err());     // Wrong order
    assert!(parse_time_range("").is_err());       // Empty
}
```

**Red:** Run test → Should fail (parse_time_range doesn't exist)
**Green:** Implement parse_time_range
**Commit:** `test(types): time range parsing`

---

### Test 1.3: FileResult Optional Field Serialization

**Purpose:** Verify None fields are NOT serialized (serde skip_serializing_if)
**Write BEFORE:** `src/types.rs` FileResult implementation

```rust
#[test]
fn test_file_result_skips_none_fields() {
    // Given: FileResult with all optional fields as None
    let file = FileResult {
        file_path: "src/main.rs".to_string(),
        access_count: 10,
        last_access: None,
        session_count: None,
        sessions: None,
        chains: None,
    };

    // When: Serialize to JSON
    let json = serde_json::to_string(&file).unwrap();

    // Then: None fields are NOT in JSON
    assert!(json.contains("file_path"));
    assert!(json.contains("access_count"));
    assert!(!json.contains("last_access"));
    assert!(!json.contains("session_count"));
    assert!(!json.contains("sessions"));
    assert!(!json.contains("chains"));
}

#[test]
fn test_file_result_includes_some_fields() {
    // Given: FileResult with some optional fields set
    let file = FileResult {
        file_path: "src/main.rs".to_string(),
        access_count: 10,
        last_access: Some("2026-01-08".to_string()),
        session_count: Some(3),
        sessions: None,
        chains: None,
    };

    // When: Serialize to JSON
    let json = serde_json::to_string(&file).unwrap();

    // Then: Some fields ARE in JSON, None fields are NOT
    assert!(json.contains("last_access"));
    assert!(json.contains("session_count"));
    assert!(!json.contains("sessions"));
    assert!(!json.contains("chains"));
}
```

**Red:** Run test → Should fail (FileResult doesn't exist)
**Green:** Implement FileResult with #[serde(skip_serializing_if = "Option::is_none")]
**Commit:** `test(types): FileResult optional field serialization`

---

### Test 1.4: CoreError to CommandError Conversion

**Purpose:** Verify error conversion produces correct error codes
**Write BEFORE:** `src/error.rs` From<CoreError> for CommandError

```rust
#[test]
fn test_database_error_conversion() {
    // Given: Database error (simulated)
    let core_err = CoreError::Database(sqlx::Error::RowNotFound);

    // When: Convert to CommandError
    let cmd_err: CommandError = core_err.into();

    // Then: Correct error code and message
    assert_eq!(cmd_err.code, "DATABASE_ERROR");
    assert_eq!(cmd_err.message, "Database operation failed");
    assert!(cmd_err.details.is_some());
}

#[test]
fn test_query_error_conversion() {
    // Given: Query error
    let core_err = CoreError::Query {
        message: "Invalid filter".to_string(),
    };

    // When: Convert to CommandError
    let cmd_err: CommandError = core_err.into();

    // Then: Correct error code and message
    assert_eq!(cmd_err.code, "QUERY_ERROR");
    assert_eq!(cmd_err.message, "Invalid filter");
    assert!(cmd_err.details.is_none());
}

#[test]
fn test_config_error_conversion() {
    // Given: Config error
    let core_err = CoreError::Config("Database not found".to_string());

    // When: Convert to CommandError
    let cmd_err: CommandError = core_err.into();

    // Then: Correct error code and message
    assert_eq!(cmd_err.code, "CONFIG_ERROR");
    assert_eq!(cmd_err.message, "Database not found");
}
```

**Red:** Run test → Should fail (CoreError doesn't exist)
**Green:** Implement CoreError and From<CoreError> for CommandError
**Commit:** `test(error): CoreError to CommandError conversion`

---

## Level 2: Integration Tests

### Test 2.1: Database Connection

**Purpose:** Verify database opens successfully in read-only mode
**Write BEFORE:** `src/storage.rs` Database::open

**Setup:** Create test fixture database first:
```bash
# Create test fixture
mkdir -p tests/fixtures
sqlite3 tests/fixtures/test.db <<EOF
CREATE TABLE file_accesses (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    session_id TEXT NOT NULL,
    chain_id TEXT,
    access_type TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

INSERT INTO file_accesses VALUES
(1, 'src/main.rs', 'session-1', 'chain-1', 'read', 'Read', '2026-01-08T10:00:00Z'),
(2, 'src/main.rs', 'session-1', 'chain-1', 'write', 'Edit', '2026-01-08T10:05:00Z'),
(3, 'src/lib.rs', 'session-1', 'chain-1', 'read', 'Read', '2026-01-08T10:10:00Z'),
(4, 'src/main.rs', 'session-2', 'chain-1', 'read', 'Read', '2026-01-08T11:00:00Z'),
(5, 'README.md', 'session-2', 'chain-2', 'read', 'Read', '2026-01-08T12:00:00Z');

CREATE TABLE chains (
    chain_id TEXT PRIMARY KEY,
    root_session_id TEXT NOT NULL,
    session_count INTEGER,
    files_json TEXT,
    files_bloom BLOB
);

INSERT INTO chains VALUES
('chain-1', 'session-1', 2, '["src/main.rs", "src/lib.rs"]', NULL),
('chain-2', 'session-2', 1, '["README.md"]', NULL);

CREATE TABLE claude_sessions (
    session_id TEXT PRIMARY KEY,
    started_at TEXT,
    total_messages INTEGER
);

INSERT INTO claude_sessions VALUES
('session-1', '2026-01-08T10:00:00Z', 10),
('session-2', '2026-01-08T11:00:00Z', 5);
EOF
```

```rust
// tests/integration/test_storage.rs
use context_os_core::Database;

#[tokio::test]
async fn test_database_opens_successfully() {
    // Given: Path to test fixture
    let db_path = "tests/fixtures/test.db";

    // When: Open database
    let result = Database::open(db_path).await;

    // Then: Opens successfully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_database_read_only() {
    // Given: Open database
    let db = Database::open("tests/fixtures/test.db").await.unwrap();

    // When: Try to write (should fail because read-only)
    let result = sqlx::query("INSERT INTO file_accesses VALUES (999, 'test', 'test', 'test', 'test', 'test', 'test')")
        .execute(db.pool())
        .await;

    // Then: Write fails (read-only mode)
    assert!(result.is_err());
}

#[tokio::test]
async fn test_database_not_found() {
    // Given: Non-existent path
    let db_path = "tests/fixtures/nonexistent.db";

    // When: Try to open
    let result = Database::open(db_path).await;

    // Then: Returns error
    assert!(result.is_err());
}
```

**Red:** Run test → Should fail (Database doesn't exist)
**Green:** Implement Database::open with sqlx
**Commit:** `test(storage): database connection`

---

### Test 2.2: query_flex Basic Query

**Purpose:** Verify query_flex returns correct results from test DB
**Write BEFORE:** `src/query.rs` QueryEngine::query_flex

```rust
// tests/integration/test_query_flex.rs
use context_os_core::{Database, QueryEngine, QueryFlexInput};

#[tokio::test]
async fn test_query_flex_returns_files() {
    // Given: Database with test data
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query with defaults
    let result = engine.query_flex(QueryFlexInput::default()).await.unwrap();

    // Then: Returns files sorted by access count
    assert!(result.result_count > 0);
    assert!(!result.results.is_empty());

    // src/main.rs has 3 accesses, should be first
    assert_eq!(result.results[0].file_path, "src/main.rs");
    assert_eq!(result.results[0].access_count, 3);
}

#[tokio::test]
async fn test_query_flex_has_receipt_id() {
    // Given: Database and engine
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query
    let result = engine.query_flex(QueryFlexInput::default()).await.unwrap();

    // Then: Has valid receipt_id (UUID format)
    assert!(!result.receipt_id.is_empty());
    assert!(result.receipt_id.len() == 36); // UUID format: 8-4-4-4-12
}

#[tokio::test]
async fn test_query_flex_has_timestamp() {
    // Given: Database and engine
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query
    let result = engine.query_flex(QueryFlexInput::default()).await.unwrap();

    // Then: Has valid timestamp (RFC3339 format)
    assert!(!result.timestamp.is_empty());
    assert!(result.timestamp.contains('T')); // ISO format contains T
}
```

**Red:** Run test → Should fail (QueryEngine doesn't exist)
**Green:** Implement QueryEngine::query_flex
**Commit:** `test(query): query_flex basic query`

---

### Test 2.3: query_flex Filters

**Purpose:** Verify query_flex respects filter parameters
**Write BEFORE:** Filter handling in QueryEngine::query_flex

```rust
#[tokio::test]
async fn test_query_flex_chain_filter() {
    // Given: Database with multiple chains
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query with chain filter
    let input = QueryFlexInput {
        chain: Some("chain-1".to_string()),
        ..Default::default()
    };
    let result = engine.query_flex(input).await.unwrap();

    // Then: Only chain-1 files returned
    for file in &result.results {
        // All files should be from chain-1 (src/main.rs, src/lib.rs)
        assert!(file.file_path.starts_with("src/"));
    }
    // README.md is in chain-2, should NOT be in results
    assert!(!result.results.iter().any(|f| f.file_path == "README.md"));
}

#[tokio::test]
async fn test_query_flex_limit() {
    // Given: Database with multiple files
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query with limit
    let input = QueryFlexInput {
        limit: Some(1),
        ..Default::default()
    };
    let result = engine.query_flex(input).await.unwrap();

    // Then: Only 1 result returned
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.result_count, 1);
}

#[tokio::test]
async fn test_query_flex_sort_by_recency() {
    // Given: Database with test data
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query sorted by recency
    let input = QueryFlexInput {
        sort: Some("recency".to_string()),
        ..Default::default()
    };
    let result = engine.query_flex(input).await.unwrap();

    // Then: Results sorted by last_access DESC
    // README.md (12:00) should be first, then src/main.rs (11:00)
    assert_eq!(result.results[0].file_path, "README.md");
}
```

**Red:** Run test → Should fail (filters not implemented)
**Green:** Implement filter handling in query_flex
**Commit:** `test(query): query_flex filters`

---

### Test 2.4: query_flex Aggregations

**Purpose:** Verify aggregations compute correctly
**Write BEFORE:** Aggregation handling in QueryEngine::query_flex

```rust
#[tokio::test]
async fn test_query_flex_count_aggregation() {
    // Given: Database with test data
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query with count aggregation
    let input = QueryFlexInput {
        agg: vec!["count".to_string()],
        ..Default::default()
    };
    let result = engine.query_flex(input).await.unwrap();

    // Then: Count aggregation present and correct
    let count = result.aggregations.count.expect("count aggregation missing");
    assert_eq!(count.total_files, 3);       // main.rs, lib.rs, README.md
    assert_eq!(count.total_accesses, 5);    // 5 total accesses in test DB
}

#[tokio::test]
async fn test_query_flex_recency_aggregation() {
    // Given: Database with test data
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query with recency aggregation
    let input = QueryFlexInput {
        agg: vec!["recency".to_string()],
        ..Default::default()
    };
    let result = engine.query_flex(input).await.unwrap();

    // Then: Recency aggregation present
    let recency = result.aggregations.recency.expect("recency aggregation missing");
    assert!(!recency.newest.is_empty());
    assert!(!recency.oldest.is_empty());
}

#[tokio::test]
async fn test_query_flex_no_aggregations() {
    // Given: Database with test data
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query with no aggregations
    let input = QueryFlexInput {
        agg: vec![],
        ..Default::default()
    };
    let result = engine.query_flex(input).await.unwrap();

    // Then: No aggregations computed
    assert!(result.aggregations.count.is_none());
    assert!(result.aggregations.recency.is_none());
}
```

**Red:** Run test → Should fail (aggregations not implemented)
**Green:** Implement compute_aggregations
**Commit:** `test(query): query_flex aggregations`

---

### Test 2.5: query_chains

**Purpose:** Verify query_chains returns chain data correctly
**Write BEFORE:** `src/query.rs` QueryEngine::query_chains

```rust
#[tokio::test]
async fn test_query_chains_returns_chains() {
    // Given: Database with chains
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query chains
    let result = engine.query_chains(QueryChainsInput::default()).await.unwrap();

    // Then: Returns chains sorted by session_count
    assert_eq!(result.total_chains, 2);
    assert_eq!(result.chains.len(), 2);

    // chain-1 has 2 sessions, should be first
    assert_eq!(result.chains[0].chain_id, "chain-1");
    assert_eq!(result.chains[0].session_count, 2);
}

#[tokio::test]
async fn test_query_chains_file_count() {
    // Given: Database with chains
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query chains
    let result = engine.query_chains(QueryChainsInput::default()).await.unwrap();

    // Then: File counts correct (parsed from files_json)
    let chain1 = result.chains.iter().find(|c| c.chain_id == "chain-1").unwrap();
    assert_eq!(chain1.file_count, 2); // ["src/main.rs", "src/lib.rs"]
}

#[tokio::test]
async fn test_query_chains_limit() {
    // Given: Database with chains
    let db = Database::open("tests/fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query chains with limit
    let input = QueryChainsInput { limit: Some(1) };
    let result = engine.query_chains(input).await.unwrap();

    // Then: Only 1 chain returned
    assert_eq!(result.chains.len(), 1);
    // total_chains should still reflect actual count
    assert_eq!(result.total_chains, 1);
}
```

**Red:** Run test → Should fail (query_chains doesn't exist)
**Green:** Implement QueryEngine::query_chains
**Commit:** `test(query): query_chains`

---

### Test 2.6: JSON Contract Verification

**Purpose:** Verify JSON output matches frontend expectations EXACTLY
**Write BEFORE:** Final review of all types

**CRITICAL:** This test prevents type mismatch bugs between Rust and frontend.

```rust
// tests/integration/test_contracts.rs
use context_os_core::*;

#[test]
fn test_query_result_json_structure() {
    // Given: QueryResult matching what frontend expects
    let result = QueryResult {
        receipt_id: "test-123".to_string(),
        timestamp: "2026-01-08T12:00:00Z".to_string(),
        result_count: 1,
        results: vec![FileResult {
            file_path: "src/main.rs".to_string(),
            access_count: 10,
            last_access: Some("2026-01-08".to_string()),
            session_count: Some(2),
            sessions: None,
            chains: None,
        }],
        aggregations: Aggregations {
            count: Some(CountAgg {
                total_files: 1,
                total_accesses: 10,
            }),
            recency: None,
        },
    };

    // When: Serialize to JSON
    let json = serde_json::to_value(&result).unwrap();

    // Then: Structure matches frontend expectations
    // Top-level fields
    assert!(json["receipt_id"].is_string());
    assert!(json["timestamp"].is_string());
    assert!(json["result_count"].is_number());
    assert!(json["results"].is_array());
    assert!(json["aggregations"].is_object());

    // FileResult fields
    let file = &json["results"][0];
    assert!(file["file_path"].is_string());
    assert!(file["access_count"].is_number());
    assert!(file["last_access"].is_string());
    assert!(file["session_count"].is_number());
    // None fields should NOT exist
    assert!(file.get("sessions").is_none());
    assert!(file.get("chains").is_none());

    // Aggregations
    let agg = &json["aggregations"];
    assert!(agg["count"]["total_files"].is_number());
    assert!(agg["count"]["total_accesses"].is_number());
    assert!(agg.get("recency").is_none()); // None should be skipped
}

#[test]
fn test_chain_data_json_structure() {
    // Given: ChainData
    let chain = ChainData {
        chain_id: "chain-123".to_string(),
        session_count: 5,
        file_count: 10,
        time_range: Some(ChainTimeRange {
            start: "2026-01-01".to_string(),
            end: "2026-01-08".to_string(),
        }),
    };

    // When: Serialize to JSON
    let json = serde_json::to_value(&chain).unwrap();

    // Then: Structure correct
    assert_eq!(json["chain_id"], "chain-123");
    assert_eq!(json["session_count"], 5);
    assert_eq!(json["file_count"], 10);
    assert_eq!(json["time_range"]["start"], "2026-01-01");
    assert_eq!(json["time_range"]["end"], "2026-01-08");
}

#[test]
fn test_command_error_json_structure() {
    // Given: CommandError
    let err = CommandError {
        code: "DATABASE_ERROR".to_string(),
        message: "Connection failed".to_string(),
        details: Some("timeout".to_string()),
    };

    // When: Serialize to JSON
    let json = serde_json::to_value(&err).unwrap();

    // Then: Structure matches frontend error handling
    assert_eq!(json["code"], "DATABASE_ERROR");
    assert_eq!(json["message"], "Connection failed");
    assert_eq!(json["details"], "timeout");
}
```

**Red:** Run test → Should fail until types are fully implemented
**Green:** Verify all serde attributes are correct
**Commit:** `test(contracts): JSON structure verification`

---

## Level 3: E2E Tests (Real Database)

### Test 3.1: Real Database Query

**Purpose:** Verify queries work against actual user database
**Write BEFORE:** Final integration verification

**IMPORTANT:** These tests use the REAL database at `~/.context-os/context.db`

```rust
// tests/e2e/test_real_db.rs
use context_os_core::{Database, QueryEngine, QueryFlexInput};
use std::path::PathBuf;

fn get_real_db_path() -> Option<PathBuf> {
    // Try standard locations
    let candidates = [
        dirs::home_dir().map(|h| h.join(".context-os/context.db")),
        Some(PathBuf::from("../../context_os_events/data/context_os_events.db")),
    ];

    for path in candidates.into_iter().flatten() {
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_real_db_query_flex() {
    // Skip if no real database
    let db_path = match get_real_db_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: No real database found");
            return;
        }
    };

    // Given: Real database
    let db = Database::open(&db_path).await.unwrap();
    let engine = QueryEngine::new(db);

    // When: Query
    let result = engine.query_flex(QueryFlexInput {
        time: Some("7d".to_string()),
        limit: Some(10),
        ..Default::default()
    }).await.unwrap();

    // Then: Returns data (actual counts depend on user's data)
    println!("Real DB query returned {} files", result.result_count);
    assert!(result.receipt_id.len() == 36);  // Valid UUID
    // Don't assert on counts - they vary
}

#[tokio::test]
#[ignore]
async fn test_real_db_query_chains() {
    let db_path = match get_real_db_path() {
        Some(p) => p,
        None => return,
    };

    let db = Database::open(&db_path).await.unwrap();
    let engine = QueryEngine::new(db);

    let result = engine.query_chains(QueryChainsInput::default()).await.unwrap();

    println!("Real DB has {} chains", result.total_chains);
    // Just verify it doesn't crash and returns valid structure
}
```

**Run:** `cargo test --ignored` (requires real database)
**Commit:** `test(e2e): real database queries`

---

### Test 3.2: Latency Verification

**Purpose:** Verify all queries complete in <100ms
**Write BEFORE:** Performance optimization

**CRITICAL:** This is the primary success metric for Phase 1.

```rust
// tests/e2e/test_latency.rs
use context_os_core::{Database, QueryEngine, QueryFlexInput, QueryChainsInput};
use std::time::{Duration, Instant};

const MAX_LATENCY: Duration = Duration::from_millis(100);

#[tokio::test]
#[ignore]
async fn test_query_flex_latency() {
    let db_path = match get_real_db_path() {
        Some(p) => p,
        None => return,
    };

    let db = Database::open(&db_path).await.unwrap();
    let engine = QueryEngine::new(db);

    // Warm-up query (first query may be slower due to connection setup)
    let _ = engine.query_flex(QueryFlexInput::default()).await;

    // Measure 10 queries
    let mut total_time = Duration::ZERO;
    for _ in 0..10 {
        let start = Instant::now();
        let _ = engine.query_flex(QueryFlexInput {
            time: Some("7d".to_string()),
            limit: Some(20),
            agg: vec!["count".to_string()],
            ..Default::default()
        }).await.unwrap();
        total_time += start.elapsed();
    }

    let avg_latency = total_time / 10;
    println!("Average query_flex latency: {:?}", avg_latency);

    // CRITICAL: Must be under 100ms
    assert!(
        avg_latency < MAX_LATENCY,
        "query_flex too slow: {:?} (max: {:?})",
        avg_latency,
        MAX_LATENCY
    );
}

#[tokio::test]
#[ignore]
async fn test_query_chains_latency() {
    let db_path = match get_real_db_path() {
        Some(p) => p,
        None => return,
    };

    let db = Database::open(&db_path).await.unwrap();
    let engine = QueryEngine::new(db);

    // Warm-up
    let _ = engine.query_chains(QueryChainsInput::default()).await;

    // Measure
    let start = Instant::now();
    for _ in 0..10 {
        let _ = engine.query_chains(QueryChainsInput { limit: Some(20) }).await.unwrap();
    }
    let avg_latency = start.elapsed() / 10;

    println!("Average query_chains latency: {:?}", avg_latency);
    assert!(avg_latency < MAX_LATENCY);
}

#[tokio::test]
#[ignore]
async fn test_cold_start_latency() {
    // Purpose: Measure time from Database::open to first query result
    // This simulates app startup

    let db_path = match get_real_db_path() {
        Some(p) => p,
        None => return,
    };

    let start = Instant::now();

    // Cold start: open + query
    let db = Database::open(&db_path).await.unwrap();
    let engine = QueryEngine::new(db);
    let _ = engine.query_flex(QueryFlexInput::default()).await.unwrap();

    let cold_start = start.elapsed();
    println!("Cold start latency: {:?}", cold_start);

    // Cold start should be under 500ms (includes connection setup)
    assert!(
        cold_start < Duration::from_millis(500),
        "Cold start too slow: {:?}",
        cold_start
    );
}
```

**Run:** `cargo test --ignored test_latency`
**Commit:** `test(e2e): latency verification`

---

## Test Execution Order

Follow this order for TDD implementation:

### Phase A: Setup (15 min)
1. Create test fixture database
2. Create tests/ directory structure
3. Add test dependencies to Cargo.toml

### Phase B: Unit Tests (30 min)
Run in order:
```bash
cargo test test_query_flex_input_defaults -- --nocapture
cargo test test_parse_time_range -- --nocapture
cargo test test_file_result -- --nocapture
cargo test test_error_conversion -- --nocapture
```

### Phase C: Integration Tests (1 hour)
Run in order:
```bash
cargo test test_database -- --nocapture
cargo test test_query_flex -- --nocapture
cargo test test_query_chains -- --nocapture
cargo test test_contracts -- --nocapture
```

### Phase D: E2E Tests (30 min)
```bash
# Requires real database
cargo test --ignored -- --nocapture
```

---

## Success Criteria

**Phase 1 is complete when:**

- [ ] All unit tests pass (`cargo test` - ~15 tests)
- [ ] All integration tests pass (`cargo test` - ~20 tests)
- [ ] E2E tests pass with real DB (`cargo test --ignored` - ~5 tests)
- [ ] Average query latency < 100ms (measured)
- [ ] Cold start latency < 500ms (measured)
- [ ] JSON contracts verified against CONTRACTS.rs

**Test Quality Gates:**

- [ ] Every test was written BEFORE the code it tests
- [ ] Every test failed initially (red) then passed (green)
- [ ] Tests are minimal (one assertion or closely related assertions)
- [ ] Tests have clear docstrings explaining purpose

---

## Common Testing Pitfalls

### Pitfall 1: Writing Tests After Code

**Problem:** Tests become biased, test what code does not what it should do.
**Prevention:** Strict TDD discipline. Write test → Run → Should FAIL → Implement → Run → Should PASS

### Pitfall 2: Trusting Success Logs Without Verification

**Problem:** Code logs "success" but data isn't actually correct.
**Prevention:** Always assert on actual values, not just absence of errors.

```rust
// BAD: Only checks no error
assert!(result.is_ok());

// GOOD: Verifies actual data
let result = result.unwrap();
assert_eq!(result.results[0].file_path, "expected/path.rs");
assert_eq!(result.results[0].access_count, 10);
```

### Pitfall 3: Mock-Heavy Tests Missing Real Integration Bugs

**Problem:** Mocks pass but real database fails.
**Prevention:** Integration tests with real SQLite fixture. E2E tests with production database.

### Pitfall 4: Ignoring Latency Until End

**Problem:** Discover performance issues too late.
**Prevention:** Run latency tests early and often. Target is <100ms.

### Pitfall 5: Not Testing JSON Serialization

**Problem:** Rust types serialize differently than frontend expects.
**Prevention:** Contract tests verify exact JSON structure matches CONTRACTS.rs.

---

## References

- **CONTRACTS.rs** - Type definitions that tests verify
- **SPEC.md** - Implementation specification
- `03_CORE_ARCHITECTURE.md` - Architecture decisions
- Kent Beck's TDD principles (test-driven-execution skill)

---

**Spec Version:** 1.0
**Created:** 2026-01-08
**Phase:** 1 of 8
**Test Count:** ~40 tests across 3 levels
