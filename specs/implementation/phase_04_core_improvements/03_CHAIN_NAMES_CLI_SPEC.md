# Chain Names CLI Specification

**Status:** Proposed
**Priority:** P0
**Bug ID:** LIVE-02
**Estimated Effort:** 2-3 hours
**Dependencies:** Spec 05 (Schema Unification) — `chain_metadata` must have unified schema before chain names can be reliably queried.

---

## Problem Statement

The CLI `query chains` command outputs only hex chain IDs (MD5 hashes like `a1b2c3d4e5f6...`). The `chain_metadata` table has a `generated_name` column populated by the Intelligence layer, but the CLI output does not surface it in a human-readable way. The `query sessions` command similarly shows raw `chain_id` with no name.

**Evidence:** `tastematter query chains --format json` returns:
```json
{
  "chains": [
    {
      "chain_id": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
      "session_count": 12,
      "file_count": 87,
      "generated_name": "Codebase Audit and Bug Triage"
    }
  ]
}
```

The `generated_name` is present in the JSON but:
1. There is no `display_name` fallback when `generated_name` is NULL
2. The JSON does not include `summary` from `chain_metadata`
3. The `query sessions` output shows `chain_id` but no chain name
4. There is no table format for `query chains` — only raw JSON via the generic `output()` function

[VERIFIED: `core/src/query.rs:154-204` — `query_chains()` selects `cm.generated_name` but does not select `cm.summary`]
[VERIFIED: `core/src/query.rs:416-520` — `query_sessions()` has no join to `chain_metadata`]
[VERIFIED: `core/src/main.rs:635-639` — chains CLI handler uses generic `output()`, no table format]
[VERIFIED: `core/src/types.rs:232-243` — `ChainData` has `generated_name` but no `display_name`]
[VERIFIED: `core/src/types.rs:395-413` — `SessionData` has `chain_id` but no `chain_name`]

---

## Implementation Steps

### Step 1: Add `display_name` and `summary` to `ChainData` (types.rs)

**File:** `core/src/types.rs` (line 232-243)

```rust
// BEFORE
pub struct ChainData {
    pub chain_id: String,
    pub session_count: u32,
    pub file_count: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<ChainTimeRange>,

    /// AI-generated human-readable name for the chain (from Intel service)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_name: Option<String>,
}

// AFTER
pub struct ChainData {
    pub chain_id: String,

    /// Human-readable name: generated_name → first_user_message (truncated) → chain_id[:12]+"..."
    pub display_name: String,

    pub session_count: u32,
    pub file_count: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<ChainTimeRange>,

    /// AI-generated human-readable name for the chain (from Intel service)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_name: Option<String>,

    /// Chain summary from Intel service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}
```

### Step 2: Add `chain_name` to `SessionData` (types.rs)

**File:** `core/src/types.rs` (line 395-413)

```rust
// BEFORE
pub struct SessionData {
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,

    pub started_at: String,
    // ... rest unchanged
}

// AFTER
pub struct SessionData {
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,

    /// Human-readable chain name (from chain_metadata.generated_name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_name: Option<String>,

    pub started_at: String,
    // ... rest unchanged
}
```

### Step 3: Compute `display_name` with fallback chain in `query_chains()` (query.rs)

**File:** `core/src/query.rs` (line 154-204)

Add `cm.summary` to the SELECT and a subquery for the first user message of the chain's earliest session. Compute `display_name` with fallback priority.

```rust
// Updated SQL — add summary and first_user_message subquery
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

// Updated row mapping — compute display_name
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
```

### Step 4: Add `compute_display_name()` helper (query.rs)

**File:** `core/src/query.rs` (new function, near top of impl or as standalone fn)

```rust
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
            // Truncate at word boundary near 60 chars
            if let Some(pos) = trimmed[..57].rfind(' ') {
                return format!("{}...", &trimmed[..pos]);
            }
            return format!("{}...", &trimmed[..57]);
        }
    }

    // Final fallback: truncated hex ID
    if chain_id.len() > 12 {
        format!("{}...", &chain_id[..12])
    } else {
        chain_id.to_string()
    }
}
```

### Step 5: Add `chain_name` to `query_sessions()` (query.rs)

**File:** `core/src/query.rs` (line 416-520)

Add a LEFT JOIN to `chain_metadata` in the sessions query to pull `generated_name` as `chain_name`.

```rust
// Updated SQL — add LEFT JOIN to chain_metadata
let mut sql = format!(
    "SELECT
        s.session_id,
        cg.chain_id,
        cm.generated_name as chain_name,
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
     LEFT JOIN chain_metadata cm ON cg.chain_id = cm.chain_id
     WHERE s.started_at >= datetime('now', '-{} days')",
    days
);

// In the session construction (line 510-520), add chain_name:
sessions.push(SessionData {
    session_id,
    chain_id: row.get("chain_id"),
    chain_name: row.get("chain_name"),
    started_at,
    ended_at,
    duration_seconds,
    file_count: row.get::<i64, _>("file_count") as u32,
    total_accesses: row.get::<i64, _>("total_accesses") as u32,
    files: vec![],
    top_files,
});
```

### Step 6: Add `output_chains_table()` in main.rs

**File:** `core/src/main.rs` (new function, after `output_heat_table` around line 1376)

```rust
/// Output chain results as a formatted table
fn output_chains_table(result: &tastematter::ChainQueryResult) {
    // Header
    println!(
        "{:<40} {:>8} {:>8}  {}",
        "CHAIN", "SESSIONS", "FILES", "ID"
    );
    println!("{}", "-".repeat(90));

    // Rows
    for chain in &result.chains {
        // Truncate display_name if needed
        let name = if chain.display_name.len() > 38 {
            format!("{}...", &chain.display_name[..35])
        } else {
            chain.display_name.clone()
        };

        // Show truncated chain_id for reference
        let short_id = if chain.chain_id.len() > 12 {
            &chain.chain_id[..12]
        } else {
            &chain.chain_id
        };

        println!(
            "{:<40} {:>8} {:>8}  {}",
            name,
            chain.session_count,
            chain.file_count,
            short_id,
        );
    }

    // Summary
    println!("{}", "-".repeat(90));
    println!("Total: {} chains", result.total_chains);
}
```

### Step 7: Wire table format into chains CLI handler (main.rs)

**File:** `core/src/main.rs` (line 635-639)

```rust
// BEFORE
QueryCommands::Chains { limit, format } => {
    let input = QueryChainsInput { limit: Some(limit) };
    let query_result = engine.query_chains(input).await?;
    result_count = Some(query_result.chains.len() as u32);
    output(&query_result, &format)?;
}

// AFTER
QueryCommands::Chains { limit, format } => {
    let input = QueryChainsInput { limit: Some(limit) };
    let query_result = engine.query_chains(input).await?;
    result_count = Some(query_result.chains.len() as u32);
    match format.as_str() {
        "table" => output_chains_table(&query_result),
        _ => output(&query_result, &format)?,
    }
}
```

---

## Fallback Chain

Display name priority:

| Priority | Source | Example |
|----------|--------|---------|
| 1 | `chain_metadata.generated_name` | "Codebase Audit and Bug Triage" |
| 2 | `first_user_message` (truncated to 60 chars) | "Can you help me refactor the authentication module to..." |
| 3 | `chain_id[:12]` + `"..."` | "a1b2c3d4e5f6..." |

Truncation rules for `first_user_message`:
- If <= 60 chars: use as-is (trimmed)
- If > 60 chars: find last space before char 57, append "..."
- If no space found in first 57 chars: hard-truncate at 57, append "..."

---

## Table Format Design

### `query chains --format table`

```
CHAIN                                    SESSIONS    FILES  ID
------------------------------------------------------------------------------------------
Codebase Audit and Bug Triage                  12       87  a1b2c3d4e5f6
Can you help me refactor the auth...            8       34  b2c3d4e5f6a7
f7e8d9c0a1b2...                                 3       12  f7e8d9c0a1b2
------------------------------------------------------------------------------------------
Total: 3 chains
```

### `query chains --format json` (enhanced)

```json
{
  "chains": [
    {
      "chain_id": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
      "display_name": "Codebase Audit and Bug Triage",
      "session_count": 12,
      "file_count": 87,
      "generated_name": "Codebase Audit and Bug Triage",
      "summary": "Multi-agent audit of tastematter codebase with cross-verification"
    }
  ],
  "total_chains": 3
}
```

### `query sessions --format json` (enhanced)

```json
{
  "sessions": [
    {
      "session_id": "abc-123-def",
      "chain_id": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
      "chain_name": "Codebase Audit and Bug Triage",
      "started_at": "2026-02-06T10:00:00Z",
      "file_count": 15,
      "total_accesses": 42
    }
  ]
}
```

---

## Dependency: Schema Unification (Spec 05)

This spec depends on Spec 05 (Schema Unification) resolving the `chain_metadata` table conflict documented in XCHECK-1.

**Current state:** Two competing `CREATE TABLE IF NOT EXISTS chain_metadata` definitions exist:

| Column | storage.rs (line 207-214) | cache.rs (line 403-411) |
|--------|--------------------------|------------------------|
| `chain_id` | PRIMARY KEY | PRIMARY KEY |
| `generated_name` | YES | YES |
| `summary` | YES | NO |
| `key_topics` | YES | NO |
| `category` | NO | YES |
| `confidence` | NO | YES |
| `generated_at` | NO | YES |
| `model_used` | NO | YES |
| `created_at` | YES | YES |
| `updated_at` | YES | NO |

**Why it matters:** If `storage.rs` runs first (which it does in practice), the table has `summary` and `key_topics` but lacks `category`, `confidence`, `generated_at`, `model_used`. The Intelligence layer writes to these missing columns, which may fail silently. After Spec 05 unifies the schema, this spec can safely query all columns.

**Mitigation for independent implementation:** This spec only reads `generated_name` and `summary`, which both exist in the `storage.rs` schema (which wins the initialization race). So this spec CAN be implemented before Spec 05, but chain names from the Intelligence layer may not be reliably populated until the schema is unified.

---

## TDD Test Plan

### Test 1: `test_compute_display_name_with_generated_name`

```rust
#[test]
fn test_compute_display_name_with_generated_name() {
    let result = compute_display_name(
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
        Some("Codebase Audit"),
        Some("help me fix bugs"),
    );
    assert_eq!(result, "Codebase Audit");
}
```

Verifies that `generated_name` takes priority over all other sources.

### Test 2: `test_compute_display_name_fallback_first_message`

```rust
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
    let result = compute_display_name(
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
        None,
        Some(long_msg),
    );
    assert!(result.len() <= 60);
    assert!(result.ends_with("..."));

    // Empty generated_name falls through
    let result = compute_display_name(
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
        Some(""),
        Some("fallback message"),
    );
    assert_eq!(result, "fallback message");
}
```

Verifies fallback to `first_user_message` when `generated_name` is NULL or empty, including truncation behavior.

### Test 3: `test_compute_display_name_fallback_hex_id`

```rust
#[test]
fn test_compute_display_name_fallback_hex_id() {
    let result = compute_display_name(
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
        None,
        None,
    );
    assert_eq!(result, "a1b2c3d4e5f6...");

    // Empty first_user_message also falls through
    let result = compute_display_name(
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
        None,
        Some(""),
    );
    assert_eq!(result, "a1b2c3d4e5f6...");

    // Short chain_id (edge case)
    let result = compute_display_name("short", None, None);
    assert_eq!(result, "short");
}
```

Verifies final fallback to truncated hex chain ID.

### Test 4: `test_session_includes_chain_name`

```rust
#[tokio::test]
async fn test_session_includes_chain_name() {
    // Setup: create test DB with claude_sessions, chain_graph, chain_metadata
    let db = setup_test_db().await;

    // Insert a session
    sqlx::query("INSERT INTO claude_sessions (session_id, started_at) VALUES ('sess-1', datetime('now'))")
        .execute(db.pool()).await.unwrap();

    // Insert chain_graph mapping
    sqlx::query("INSERT INTO chain_graph (session_id, chain_id) VALUES ('sess-1', 'chain-abc')")
        .execute(db.pool()).await.unwrap();

    // Insert chain_metadata with generated_name
    sqlx::query("INSERT INTO chain_metadata (chain_id, generated_name) VALUES ('chain-abc', 'Test Chain')")
        .execute(db.pool()).await.unwrap();

    // Query sessions
    let engine = QueryEngine::new(db);
    let result = engine.query_sessions(QuerySessionsInput {
        time: "30d".to_string(),
        chain: None,
        limit: None,
    }).await.unwrap();

    // Verify chain_name is populated
    assert_eq!(result.sessions.len(), 1);
    assert_eq!(result.sessions[0].chain_name, Some("Test Chain".to_string()));
}
```

Verifies that `query_sessions` populates `chain_name` from the `chain_metadata` JOIN.

### Test 5: `test_chains_table_output_format`

```rust
#[test]
fn test_chains_table_output_format() {
    let result = ChainQueryResult {
        chains: vec![
            ChainData {
                chain_id: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4".to_string(),
                display_name: "Codebase Audit".to_string(),
                session_count: 12,
                file_count: 87,
                time_range: None,
                generated_name: Some("Codebase Audit".to_string()),
                summary: None,
            },
        ],
        total_chains: 1,
    };

    // Capture stdout and verify table format
    // (Use a buffer capture pattern or snapshot testing)
    // Key assertions:
    // - Header contains "CHAIN", "SESSIONS", "FILES", "ID"
    // - Row shows "Codebase Audit" not the hex ID
    // - Summary line shows "Total: 1 chains"
}
```

Verifies the table output format renders display names instead of raw hex IDs.

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `core/src/types.rs` | Add `display_name`, `summary` to `ChainData`; add `chain_name` to `SessionData` | ~232-243, ~395-413 |
| `core/src/query.rs` | Add `compute_display_name()` fn; update `query_chains()` SQL and mapping; update `query_sessions()` SQL and mapping | ~154-204, ~416-520 |
| `core/src/main.rs` | Add `output_chains_table()` fn; update chains CLI handler to use table format | ~635-639, new fn after ~1376 |

---

## Success Criteria

1. `tastematter query chains --format json` includes `display_name` and `summary` fields
2. `tastematter query chains --format table` shows human-readable chain names, not hex IDs
3. `tastematter query sessions --format json` includes `chain_name` when available
4. Chains without Intel-generated names fall back to first user message or truncated ID
5. All existing tests continue to pass (`cargo test`)
6. New tests for `compute_display_name` pass with all three fallback levels

---

## Handoff Checklist

- [ ] Read this spec fully before starting implementation
- [ ] Read Spec 05 (Schema Unification) for dependency context
- [ ] Run `cargo test` to baseline test state
- [ ] Implement `compute_display_name()` with tests FIRST (pure function, no DB needed)
- [ ] Update `ChainData` and `SessionData` structs in types.rs
- [ ] Update `query_chains()` SQL and row mapping in query.rs
- [ ] Update `query_sessions()` SQL and row mapping in query.rs
- [ ] Add `output_chains_table()` in main.rs
- [ ] Wire table format in chains CLI handler
- [ ] Run `cargo test` — all tests pass
- [ ] Manual verification: `tastematter query chains --format table` shows names
- [ ] Manual verification: `tastematter query chains --format json` shows display_name + summary
- [ ] Manual verification: `tastematter query sessions --format json` shows chain_name

---

**Created:** 2026-02-06
**Author:** Codebase audit LIVE-02 fix specification
