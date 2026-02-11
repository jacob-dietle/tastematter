# Stress Testing Architecture Guide

**Mission:** Systematically break tastematter by testing every component at its failure boundaries, not just the parser.

**Methodology:** Spec-driven (architecture → contracts → tests) + TDD (RED → GREEN → REFACTOR)

**Evidence:** Current test density audit reveals critical gaps:

| Module | Lines | Tests | Per 100L | Risk |
|--------|-------|-------|----------|------|
| storage.rs | 922 | 2 | 0.2 | **CRITICAL** |
| query.rs | 2181 | 5 | 0.2 | **CRITICAL** |
| sync.rs (orchestration) | 1190 | 8 | 0.7 | HIGH |
| context_restore.rs | 1120 | 12 | 1.1 | MEDIUM |
| chain_graph.rs | 1172 | 30 | 2.6 | LOW |
| jsonl_parser.rs | 1994 | 69 | 3.5 | LOW (just hardened) |

**Total existing:** 292 tests across 23 files
**Target:** ~100 net-new tests across 6 phases

---

## Phase Map

```
Phase 1: Storage Hardening        → 15 tests  (0.2 → 1.8 per 100L)
Phase 2: Query Engine Adversarial  → 20 tests  (0.2 → 1.1 per 100L)
Phase 3: Sync Orchestration        → 15 tests  (0.7 → 2.0 per 100L)
Phase 4: Context Restore Edge Cases → 12 tests (1.1 → 2.2 per 100L)
Phase 5: Input Resilience (all paths) → 18 tests (cross-cutting)
Phase 6: E2E Pipeline Enhancement  → 8 scenarios (CI)
```

---

## Phase 1: Storage Hardening

**File:** `core/src/storage.rs` (922 lines, 2 tests)
**Why:** Every command depends on SQLite. If storage breaks, everything breaks.

### Test Matrix

| # | Test | Category | What Breaks |
|---|------|----------|-------------|
| 1.1 | Open nonexistent DB path | Error path | Should return `CoreError::Config`, not panic |
| 1.2 | Open DB on read-only filesystem | Permissions | `open_rw` should return clear error |
| 1.3 | `ensure_schema` is idempotent | Schema | Running twice should not corrupt or duplicate tables |
| 1.4 | `ensure_schema` on existing DB with data | Migration | Should not drop existing data |
| 1.5 | `upsert_session` with duplicate session_id | Idempotency | Should update, not error or duplicate |
| 1.6 | `upsert_session` with NULL fields | Robustness | Optional fields should be nullable |
| 1.7 | `upsert_session` with 10KB conversation_excerpt | Large data | SQLite handles TEXT up to 1GB, but does the schema? |
| 1.8 | `persist_chains` with empty chain map | Edge case | Should succeed with 0 inserts |
| 1.9 | `persist_chains` then `persist_chains` again | Idempotency | Should upsert, not duplicate |
| 1.10 | Query after 1000 session inserts | Performance | Must stay under 100ms |
| 1.11 | Batch insert 100 sessions in one call | Throughput | Transaction should be atomic |
| 1.12 | Open two connections to same DB | Concurrency | Read pool should handle gracefully |
| 1.13 | `open_rw` creates parent directories if missing | UX | Fresh install path: `~/.context-os/` may not exist |
| 1.14 | DB path with spaces in name | Platform | Windows `C:\Users\John Doe\.context-os\` |
| 1.15 | DB path with unicode characters | Platform | macOS users with non-ASCII home dirs |

### Implementation Notes

- All tests use `tempfile::tempdir()` — no dependency on canonical DB
- Tests 1.10-1.11 use synthetic session data (not real JSONL)
- Test 1.12 opens two `Database` instances to same file
- Tests 1.14-1.15 create tempdirs with specific names

---

## Phase 2: Query Engine Adversarial

**File:** `core/src/query.rs` (2181 lines, 5 tests)
**Why:** User-facing API. Every CLI command routes through here. SQL injection, malformed input, empty results.

### Test Matrix

| # | Test | Category | What Breaks |
|---|------|----------|-------------|
| 2.1 | `query_flex` with `time: "0d"` | Edge case | Zero-width window — should return empty, not error |
| 2.2 | `query_flex` with `time: "99999d"` | Edge case | 273 years — should not overflow |
| 2.3 | `query_flex` with `time: "abc"` | Validation | Invalid time string — clear error |
| 2.4 | `query_flex` with `time: "-7d"` | Validation | Negative time — clear error |
| 2.5 | `query_flex` with `limit: 0` | Edge case | Zero limit — return empty or error? |
| 2.6 | `query_flex` with `limit: 1000000` | Performance | Huge limit — should cap or handle gracefully |
| 2.7 | `query_heat` with all-zero-access sessions | Logic | Division by zero in RCR/velocity calculations |
| 2.8 | `query_heat` with single session, single file | Minimum | Smallest valid heat dataset |
| 2.9 | `query_chains` on DB with sessions but no chains | Data gap | Sessions exist but chain_graph table empty |
| 2.10 | `query_context` with empty string project | Validation | `tastematter context ""` |
| 2.11 | `query_context` with SQL injection in project name | Security | `tastematter context "'; DROP TABLE sessions;--"` |
| 2.12 | `query_context` with unicode project name | Encoding | `tastematter context "项目"` |
| 2.13 | `query_context` with path traversal in project | Security | `tastematter context "../../etc/passwd"` |
| 2.14 | `query_timeline` with `time: "1h"` | Granularity | Sub-day buckets |
| 2.15 | `query_timeline` with future dates | Logic | Sessions with timestamps in the future |
| 2.16 | `query_sessions` returns consistent data after daemon re-sync | Consistency | Same data after re-indexing |
| 2.17 | All queries return receipt_id even on error | Contract | Receipt must always be present |
| 2.18 | `query_flex` with `project_filter` that matches no sessions | Filter | Empty filter result vs no filter |
| 2.19 | `compute_display_name` with 10KB first_user_message | Truncation | Should truncate without panic |
| 2.20 | `generate_receipt_id` uniqueness over 1000 calls | Collision | Hash should not collide in practice |

### Implementation Notes

- All tests use `tempfile::tempdir()` + `open_rw` + `ensure_schema` + synthetic inserts
- This makes them self-contained (no dependency on real DB)
- SQL injection test (2.11) should verify parameterized queries
- Tests 2.7-2.8 need synthetic sessions with specific file/tool patterns

---

## Phase 3: Sync Orchestration

**File:** `core/src/daemon/sync.rs` (1190 lines, 8 tests)
**Why:** The data pipeline. If sync breaks, DB never gets populated. If it's not idempotent, data corrupts.

### Test Matrix

| # | Test | Category | What Breaks |
|---|------|----------|-------------|
| 3.1 | `run_sync` twice on same data | Idempotency | session count should not double |
| 3.2 | `run_sync` with empty `.claude` directory | Fresh install | Should succeed with 0 sessions |
| 3.3 | `run_sync` where `.claude` directory doesn't exist | Missing data | Should error gracefully, not panic |
| 3.4 | `run_sync` with sessions that have no `cwd` field | Legacy data | Older Claude Code versions |
| 3.5 | `sync_sessions_phase` skips unchanged files | Incremental | File size check should prevent re-parse |
| 3.6 | `sync_sessions_phase` re-parses when file size changes | Incremental | Appended session should be re-indexed |
| 3.7 | Chain building with 0 sessions | Edge case | Should produce 0 chains |
| 3.8 | Chain building with 1 session | Minimum | Should produce 1 chain |
| 3.9 | Chain building with sessions spanning 3 projects | Multi-project | Should produce 3+ chains |
| 3.10 | `enrich_chains_phase` with Intel service down | Graceful degradation | Should log, not crash |
| 3.11 | `run_sync` duration_ms is reasonable | Performance | Should complete in <10s for small datasets |
| 3.12 | `SyncResult` serializes correctly to JSON | Contract | CLI output depends on this |
| 3.13 | Errors from all phases collected in `result.errors` | Error aggregation | No phase error should be lost |
| 3.14 | `sync_git` with no git repos | Expected case | CI runners have no git repos |
| 3.15 | File index phase with non-existent project paths | Stale data | Sessions reference deleted projects |

### Implementation Notes

- Tests 3.1-3.3 need a mock `.claude` directory with JSONL files
- Use `tempfile::tempdir()` for both `.claude` mock and DB
- Test 3.1 (idempotency) is the HIGHEST VALUE test in this phase

---

## Phase 4: Context Restore Edge Cases

**File:** `core/src/context_restore.rs` (1120 lines, 12 tests)
**Why:** The headline feature. What users see when they run `tastematter context`. Bad output = lost trust.

### Test Matrix

| # | Test | Category | What Breaks |
|---|------|----------|-------------|
| 4.1 | `build_executive_summary` with 0 sessions | Empty state | Status should be "unknown" |
| 4.2 | `build_executive_summary` with sessions from 1 year ago | Stale | Status should be "stale" |
| 4.3 | `build_executive_summary` with sessions from 1 hour ago | Fresh | Status should be "healthy" |
| 4.4 | `build_work_clusters` with 1 file accessed once | Minimum | Should produce 1 cluster |
| 4.5 | `build_work_clusters` with 1000 files | Scale | Should not be slow or OOM |
| 4.6 | `build_suggested_reads` with files that no longer exist on disk | Stale files | Should still suggest (DB doesn't know about disk) |
| 4.7 | `merge_synthesis` with mismatched array lengths | Intel mismatch | Should not panic, should degrade gracefully |
| 4.8 | `merge_synthesis` with empty response | Intel failure | All fields stay None |
| 4.9 | `build_synthesis_request` with unicode file paths | Encoding | CJK/emoji in paths |
| 4.10 | `discover_project_context` on empty directory | Missing files | Should return empty, not error |
| 4.11 | `discover_project_context` with symlinks | Platform | macOS/Linux symlinked dirs |
| 4.12 | Full context pipeline: empty DB → context output | Integration | Should produce valid JSON with "unknown" status |

### Implementation Notes

- Tests 4.1-4.3 test pure functions (no DB needed)
- Tests 4.7-4.8 test Intel merge logic with mock responses
- Test 4.12 is a mini-integration test (tempdir DB)

---

## Phase 5: Input Resilience (Cross-Cutting)

**Files:** Multiple — every module that reads external input
**Why:** Tastematter reads untrusted data from Claude Code JSONL, user CLI args, YAML config, filesystem.

### Test Matrix

| # | Test | Target Module | What Breaks |
|---|------|---------------|-------------|
| 5.1 | JSONL file with UTF-8 BOM (EF BB BF prefix) | jsonl_parser | First line parse fails |
| 5.2 | JSONL file with CRLF line endings | jsonl_parser | `\r` in parsed values |
| 5.3 | JSONL file with null bytes in content | jsonl_parser | serde behavior unclear |
| 5.4 | JSONL line > 10MB (base64 image in tool output) | jsonl_parser | Memory / performance |
| 5.5 | JSONL file with 0 bytes | jsonl_parser | `parse_session_file` error path |
| 5.6 | Session file locked by another process | jsonl_parser | Windows file locking |
| 5.7 | YAML workstreams file with non-UTF8 content | sync.rs | `load_workstreams` error path |
| 5.8 | `.claude` directory is a symlink | sync.rs | `find_session_files` traversal |
| 5.9 | Session filename with spaces | jsonl_parser | `extract_session_id` |
| 5.10 | Session filename with unicode characters | jsonl_parser | Path handling |
| 5.11 | `DaemonConfig` with empty string paths | daemon/config | Should use defaults |
| 5.12 | `content` field as JSON array (not string) | jsonl_parser | `msg.content.as_str()` returns None |
| 5.13 | Tool use with `file_path: null` (explicit null) | jsonl_parser | Option handling |
| 5.14 | Chain graph JSONL with duplicate session IDs | chain_graph | Should deduplicate |
| 5.15 | Message with `timestamp: null` | jsonl_parser | Fallback to Utc::now() |
| 5.16 | Message with `timestamp: ""` (empty string) | jsonl_parser | Parse error path |
| 5.17 | Session with 100K messages | jsonl_parser + chain | Memory/performance |
| 5.18 | Git repo with 10K commits | git_sync | Performance ceiling |

### Implementation Notes

- Tests 5.1-5.5 write synthetic JSONL files to tempdir
- Test 5.6 is platform-specific (Windows only — skip on Unix)
- Test 5.12 is HIGH VALUE: Claude Code sends `content: [{type: "text", text: "..."}]` for tool results
- Test 5.17 is a performance boundary test

---

## Phase 6: E2E Pipeline Enhancement

**File:** `.github/workflows/staging.yml` (e2e-test job)
**Why:** Current E2E only tests happy path with 4 small sessions. Real users have diverse patterns.

### New E2E Scenarios

| # | Scenario | What It Tests |
|---|----------|---------------|
| 6.1 | Session with emoji-heavy prompt | UTF-8 in real Claude Code sessions |
| 6.2 | Run `daemon once` twice, assert same result_count | Sync idempotency in production |
| 6.3 | Delete DB between daemon runs | Recovery from data loss |
| 6.4 | Query with `--time 0d` | Zero-width time window handling |
| 6.5 | `context` command on project with no sessions | Empty project path |
| 6.6 | Assert heat query has results | Heat pipeline end-to-end |
| 6.7 | Assert chains query has results | Chain pipeline end-to-end |
| 6.8 | Performance budget: daemon once < 5s | Regression gate |

### Implementation Notes

- Scenarios 6.1-6.3 are new E2E steps added to existing job
- Scenario 6.2 runs daemon twice and compares — tests idempotency with real Claude Code data
- Scenario 6.3 is destructive-recovery: delete DB, re-run daemon, assert data returns
- Scenario 6.8 captures `duration_ms` from daemon JSON output and asserts < 5000

---

## Agent Execution Plan

Phases 1-5 are independent and can be implemented in parallel by different agents.
Phase 6 depends on phases 1-5 being committed (so CI has the new tests).

### Per-Phase Agent Spec

Each agent:
1. Creates tests in RED state (write test, verify it compiles, understand what it tests)
2. If test reveals a bug → fix the bug (GREEN)
3. Commit test + fix together
4. Write phase completion summary

### File Placement

| Phase | Test Location |
|-------|---------------|
| 1 | `core/src/storage.rs` inline `#[cfg(test)]` module |
| 2 | `core/src/query.rs` inline `#[cfg(test)]` module + `core/tests/integration_test.rs` |
| 3 | `core/src/daemon/sync.rs` inline `#[cfg(test)]` module |
| 4 | `core/src/context_restore.rs` inline `#[cfg(test)]` module |
| 5 | Split across target modules (inline tests) |
| 6 | `.github/workflows/staging.yml` (e2e-test job) |

### Success Criteria

- All new tests pass with `cargo test -- --test-threads=2`
- No existing tests broken
- E2E pipeline passes on all 3 platforms
- Agent eval rating improves from 6/10 to 7+/10

---

## Verification

```bash
# Run all tests (must use thread limit per Known Issues)
cd apps/tastematter && cargo test --manifest-path core/Cargo.toml -- --test-threads=2

# Run only new stress tests by naming convention
cargo test --manifest-path core/Cargo.toml -- stress_ --test-threads=2

# Run E2E (triggers on push to master)
git push origin master
```

---

## Cost Estimate

| Phase | Tests | Estimated Time | Dependencies |
|-------|-------|----------------|--------------|
| 1 | 15 | 45 min | None |
| 2 | 20 | 60 min | Phase 1 (needs storage helpers) |
| 3 | 15 | 45 min | None |
| 4 | 12 | 30 min | None |
| 5 | 18 | 45 min | None |
| 6 | 8 | 30 min | Phases 1-5 |
| **Total** | **~88** | **~4 hours** | |
