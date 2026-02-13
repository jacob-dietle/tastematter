# Synthesis: Context Restoration API Readiness Assessment

**Date:** 2026-02-07
**Auditor:** claude-opus-4-6 (synthesis agent)
**Inputs:**
- `audit_rust_core.md` -- Rust query engine function inventory and composition patterns
- `audit_intel_service.md` -- Intel service endpoint/agent inventory and test coverage
- `audit_integration_layer.md` -- IntelClient, cache, daemon sync flow
- `specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md` -- Target API specification
- `specs/canonical/05_INTELLIGENCE_LAYER_ARCHITECTURE.md` -- Intel service architecture spec
- `specs/canonical/03_CORE_ARCHITECTURE.md` -- Core architecture spec

---

## 1. Spec vs Reality Matrix

For each of the 9 response sections defined in Spec 12, this table shows what the spec requires, what exists today, what's missing, and estimated new code.

### 1.1 executive_summary

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `one_liner` (LLM), `status` (healthy/warning/stale/unknown from recency+test state), `confidence` (LLM), `work_tempo` (active/cooling/dormant from session frequency), `last_meaningful_session` (most recent session >5 files) |
| **Existing code** | `query_sessions()` (query.rs:525-713) returns sessions with file counts per session -- can filter for >5 files. `query_heat()` (query.rs:1027-1173) returns recency data via `last_access` timestamps. `classify_heat()` (types.rs:675-685) classifies heat levels. |
| **What's missing (Phase 1)** | New function to derive `status` from recency thresholds. New function to derive `work_tempo` from session timestamps (3d/7d buckets). Filtering sessions for "meaningful" (>5 files). |
| **What's missing (Phase 2)** | `one_liner` synthesis via Intel. `confidence` from LLM self-assessment. |
| **Est. lines (Phase 1)** | ~30 lines (status derivation + tempo derivation + meaningful session filter) |

### 1.2 current_state

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `narrative` (LLM), `key_metrics` (corpus size, test coverage, providers -- extracted from context packages), `evidence[]` (file:line citations) |
| **Existing code** | No context package reading exists in Rust. `query_flex()` (query.rs:100-205) returns file access patterns. No file content reading -- Rust core only queries the SQLite database, not the filesystem. |
| **What's missing (Phase 1)** | Context package discovery via glob on filesystem (new). Context package content reading (new). Structured extraction of metrics/TODOs from markdown (new parser). `key_metrics` can only be populated from context package content. |
| **What's missing (Phase 2)** | `narrative` synthesis via Intel. `evidence[]` grounding via LLM citation. |
| **Est. lines (Phase 1)** | ~50-70 lines (glob discovery + file reading + basic markdown section extraction) |

### 1.3 continuity

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `left_off_at` (latest context package "For Next Agent" section), `pending_items[]` (extracted from TODOs/checkboxes), `chain_context` (chain_id, total packages, timeline, arc) |
| **Existing code** | `query_chains()` (query.rs:210-284) returns chains with display names. `chain_metadata` / `chain_summaries` tables store Intel-generated names and summaries (cache.rs). No filesystem access for context package content. |
| **What's missing (Phase 1)** | Context package section parser ("For Next Agent", "Jobs To Be Done", `- [ ]` patterns). Package counting and timeline extraction. Chain-to-package association (by query string matching file paths). |
| **What's missing (Phase 2)** | `arc` synthesis via LLM (summarize package titles over time). |
| **Est. lines (Phase 1)** | ~40-50 lines (markdown section parser + pending item extractor + chain context assembly) |

### 1.4 work_clusters

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `name` (LLM), `files[]`, `pmi_score`, `interpretation` (LLM), `access_pattern` (derived from access_count vs session_count) |
| **Existing code** | `query_co_access()` (query.rs:920-1015) computes PMI scores for a single anchor file. `query_flex()` (query.rs:100-205) returns access_count and session_count per file. `compute_aggregations()` (query.rs:1714-1743) provides count/recency aggregations. |
| **What's missing (Phase 1)** | Cluster detection algorithm: seed from highest-access files, run co-access per seed, group by PMI threshold. Access pattern derivation (high/low access vs high/low session quadrant). Multi-anchor co-access batching to avoid N+1. |
| **What's missing (Phase 2)** | `name` and `interpretation` via LLM synthesis. |
| **Est. lines (Phase 1)** | ~60-80 lines (cluster detection loop + access pattern classification + result assembly). The co-access N+1 is the main concern -- see Risk Assessment. |

### 1.5 suggested_reads

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `path`, `reason` (LLM), `priority` (ranked by recency+access+surprise), `time_estimate` (file size / reading speed), `surprise` (true if file in co-access but not primary query) |
| **Existing code** | `query_flex()` returns files with access counts and recency. `query_co_access()` returns related files with PMI. No file size reading. |
| **What's missing (Phase 1)** | Priority ranking algorithm (context package first, then README, then surprise co-access files). Surprise detection (present in co-access results but absent from primary flex query). File size measurement for time estimates. |
| **What's missing (Phase 2)** | `reason` explanation via LLM. |
| **Est. lines (Phase 1)** | ~30-40 lines (priority ranker + surprise detector + time estimator) |

### 1.6 timeline

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `recent_focus[]` (period, focus description (LLM), files_touched count), `attention_shift` (detected boolean, description (LLM), interpretation (LLM)) |
| **Existing code** | `query_timeline()` (query.rs:289-520) returns daily buckets with `access_count` and `files_touched`. Temporal bucketing is fully implemented with date aggregation. |
| **What's missing (Phase 1)** | Period grouping (collapse daily buckets into multi-day periods). Attention shift detection (compare file sets across periods -- deterministic overlap calculation). |
| **What's missing (Phase 2)** | `focus` description per period via LLM. `attention_shift.description` and `interpretation` via LLM. |
| **Est. lines (Phase 1)** | ~30-40 lines (period grouper + shift detector based on file set overlap) |

### 1.7 insights

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | Array of insights with `type` (completion/surprise/stale/abandoned/risk), `title`, `description`, `evidence[]`, optional `action` |
| **Existing code** | Intel service already has a full `generateInsights()` agent (insights.ts) and `POST /api/intel/generate-insights` endpoint. Rust `InsightsRequest` and `InsightsResponse` types exist in types.rs. IntelClient does NOT have a `generate_insights()` method -- only `name_chain()` and `summarize_chain()`. |
| **What's missing (Phase 1)** | Deterministic insight detection: `stale` (context package >7d old), `abandoned` (files with old last_access), `surprise` (high-PMI unexpected files). These don't need LLM. |
| **What's missing (Phase 2)** | `completion` and `risk` insights via Intel `generate-insights` endpoint. New `IntelClient::generate_insights()` method. |
| **Est. lines (Phase 1)** | ~30 lines (3 deterministic insight detectors) |

### 1.8 verification

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `receipt_id`, `command` (verify command string), `sources_cited`, `deterministic_data` (files/sessions/co-access counts), `synthesis_data` (model, tokens, cost -- Phase 2 only) |
| **Existing code** | `generate_receipt_id()` (query.rs:17-25) creates hash-based receipt IDs. `query_verify()` (query.rs:1179-1230) verifies receipt IDs against stored data. Receipt pattern is well-established across all queries. |
| **What's missing (Phase 1)** | Assembly of verification block from query results (count files analyzed, sessions analyzed, co-access pairs). Generate receipt for the context restore response. |
| **What's missing (Phase 2)** | `synthesis_data` fields (model, tokens, cost) from Intel response. |
| **Est. lines (Phase 1)** | ~15 lines (verification block assembly) |

### 1.9 quick_start

| Dimension | Detail |
|-----------|--------|
| **Spec requires** | `commands[]` (copy-paste commands from context packages), `expected_results` (from verification sections) |
| **Existing code** | No command extraction exists. This requires reading context package markdown content and finding "Test Commands" or similar sections. |
| **What's missing (Phase 1)** | Context package section parser for commands (reuses the parser from 1.2/1.3). Pattern matching for command blocks in markdown (fenced code blocks after "test", "verify", "run" headings). |
| **What's missing (Phase 2)** | None -- this section is fully deterministic. |
| **Est. lines (Phase 1)** | ~20-25 lines (command block extractor, reusing context package parser infrastructure) |

### Summary Matrix

| Section | Deterministic (P1) | LLM (P2) | P1 Est. Lines | P1 Complexity |
|---------|--------------------|-----------| --------------|---------------|
| executive_summary | status, work_tempo, last_session | one_liner, confidence | ~30 | Low |
| current_state | key_metrics, evidence (from CP files) | narrative | ~50-70 | Medium (file I/O) |
| continuity | left_off_at, pending_items, chain_context | arc | ~40-50 | Medium (markdown parsing) |
| work_clusters | files, pmi_score, access_pattern | name, interpretation | ~60-80 | High (co-access N+1) |
| suggested_reads | path, priority, time_estimate, surprise | reason | ~30-40 | Low |
| timeline | period buckets, files_touched, shift detected | focus, description, interpretation | ~30-40 | Low |
| insights | stale, abandoned, surprise detectors | completion, risk | ~30 | Low |
| verification | receipt_id, counts | synthesis_data | ~15 | Low |
| quick_start | commands, expected_results | -- | ~20-25 | Low |
| **TOTAL** | | | **~305-380** | |

---

## 2. Phase 1 Feasibility: <200 Lines of New Rust Code?

### Verdict: NO -- Approximately 305-380 lines needed.

The 200-line target is not feasible for a complete Phase 1 implementation. However, a **minimal viable Phase 1** scoped to the 4 most valuable sections could fit within ~200 lines.

### Full Phase 1 Composition Pattern

```rust
pub async fn query_context(
    &self,
    query: &str,
    depth: &str,   // "quick" | "medium" | "deep"
) -> Result<ContextRestoreResult, CoreError> {
    let time = "30d";
    let file_pattern = format!("*{}*", query);

    // ---- Parallel DB queries via tokio::join! ----
    let (flex, heat, chains, sessions, timeline) = tokio::join!(
        self.query_flex(QueryFlexInput {
            time: Some(time.into()),
            files: Some(file_pattern.clone()),
            agg: vec!["count".into(), "recency".into(), "sessions".into()],
            limit: Some(50),
            ..Default::default()
        }),
        self.query_heat(QueryHeatInput {
            time: Some(time.into()),
            files: Some(file_pattern.clone()),
            limit: Some(50),
            ..Default::default()
        }),
        self.query_chains(QueryChainsInput { limit: Some(20) }),
        self.query_sessions(QuerySessionsInput {
            time: time.into(),
            chain: None,
            limit: Some(20),
        }),
        self.query_timeline(QueryTimelineInput {
            time: Some(time.into()),
            files: Some(file_pattern.clone()),
            chain: None,
            limit: Some(30),
        }),
    );

    // ---- Sequential: co-access for top-N hot files ----
    let hot_files: Vec<String> = flex?.results.iter()
        .take(5)  // Limit to top 5 to avoid N+1 blowup
        .map(|f| f.file_path.clone())
        .collect();

    let mut co_access_results = Vec::new();
    for file in &hot_files {
        let co = self.query_co_access(QueryCoAccessInput {
            file_path: file.clone(),
            limit: Some(10),
        }).await?;
        co_access_results.push(co);
    }

    // ---- Sequential: filesystem reads for context packages ----
    let context_packages = discover_context_packages(query)?;  // NEW: glob + read
    let latest_package = context_packages.first();  // Most recent

    // ---- Assembly (all deterministic) ----
    let executive_summary = build_executive_summary(&sessions?, &heat?);
    let current_state = build_current_state(latest_package, &flex?);
    let continuity = build_continuity(latest_package, &chains?);
    let work_clusters = build_work_clusters(&flex?, &co_access_results);
    let suggested_reads = build_suggested_reads(&flex?, &co_access_results, latest_package);
    let timeline_section = build_timeline(&timeline?);
    let insights = build_deterministic_insights(&heat?, latest_package);
    let verification = build_verification(&flex?, &sessions?, &co_access_results);
    let quick_start = build_quick_start(latest_package);

    Ok(ContextRestoreResult { ... })
}
```

### Exact Functions to Compose (from existing code)

| Existing Function | Called By | Purpose in Context Restore |
|-------------------|-----------|---------------------------|
| `query_flex()` | query.rs:100 | File access patterns, counts, recency |
| `query_heat()` | query.rs:1027 | Heat classification for status derivation |
| `query_co_access()` | query.rs:920 | PMI-based file clustering |
| `query_chains()` | query.rs:210 | Chain metadata for continuity section |
| `query_sessions()` | query.rs:525 | Session history for tempo + meaningful session |
| `query_timeline()` | query.rs:289 | Temporal bucketing for timeline section |
| `classify_heat()` | types.rs:675 | Heat level classification (reuse) |
| `compute_heat_score()` | types.rs:768 | Heat scoring (reuse) |
| `generate_receipt_id()` | query.rs:17 | Receipt ID for verification section |

### New Functions Needed

| Function | Purpose | Est. Lines | Depends On |
|----------|---------|-----------|------------|
| `query_context()` | Orchestrator -- runs all queries, assembles result | ~40 | All existing query functions |
| `discover_context_packages()` | Glob filesystem for `**/context_packages/*.md` matching query | ~20 | std::fs, glob crate |
| `parse_context_package()` | Extract sections (For Next Agent, TODO, Test Commands) from markdown | ~40 | String parsing |
| `build_executive_summary()` | Derive status + tempo from sessions/heat | ~20 | query_sessions, query_heat results |
| `build_current_state()` | Extract metrics from context package + flex results | ~15 | Context package content + flex |
| `build_continuity()` | Extract pending items + chain context | ~25 | Context package + chains |
| `build_work_clusters()` | Cluster files by PMI, classify access patterns | ~40 | Co-access results + flex |
| `build_suggested_reads()` | Rank files by priority, detect surprises | ~25 | Flex + co-access |
| `build_timeline()` | Group daily buckets into periods, detect shifts | ~25 | Timeline results |
| `build_deterministic_insights()` | Detect stale/abandoned/surprise patterns | ~20 | Heat + context packages |
| `build_verification()` | Assemble receipt + counts | ~10 | All query results |
| `build_quick_start()` | Extract commands from context package | ~15 | Context package content |
| **New types** (ContextRestoreResult, sub-structs) | Serde types for all 9 sections | ~50 | -- |
| **TOTAL** | | **~345** | |

### Minimal Viable Phase 1 (~200 lines)

To fit within 200 lines, scope to these 5 sections only:

1. **executive_summary** (status + tempo from existing queries)
2. **work_clusters** (from existing co-access + flex)
3. **suggested_reads** (from flex + co-access ranking)
4. **timeline** (from existing timeline query + period grouping)
5. **verification** (receipt assembly)

Skip: `current_state`, `continuity`, `quick_start` (these require filesystem I/O for context packages), and `insights` (can be added incrementally).

---

## 3. Intel Service Readiness for Phase 2

### Is the Intel service ready? Partially.

**What exists and is reusable:**

| Component | Location | Reusability |
|-----------|----------|-------------|
| Elysia app scaffolding | intel/src/index.ts | 100% -- add new `.post()` route |
| Correlation middleware | intel/src/middleware/correlation.ts | 100% -- already global |
| `withOperationLogging` middleware | intel/src/index.ts | 100% -- just add config |
| `classifyError` function | intel/src/index.ts | 100% -- works for any Anthropic error |
| Zod validation pattern | intel/src/types/shared.ts | 100% -- add new schemas |
| Structured logger | intel/src/services/file-logger.ts | 100% |
| Anthropic client singleton | intel/src/index.ts `getAnthropicClient()` | 100% |
| `tool_choice` structured output pattern | All 6 agents | 100% -- fill-in-the-blanks |

**What's needed for `POST /api/intel/synthesize-context`:**

1. **New Zod schemas** in `types/shared.ts`:
   - `ContextSynthesisRequestSchema` (deterministic data from Rust)
   - `ContextSynthesisResponseSchema` (synthesized fields)
   - ~40 lines

2. **New agent file** `src/agents/context-synthesis.ts`:
   - System prompt defining context synthesis rules
   - Tool definition (`output_context_synthesis`) with structured schema
   - `buildPrompt()` function
   - `synthesizeContext()` async function
   - Model: Haiku for medium depth, Sonnet for deep
   - ~80-100 lines

3. **New route** in `index.ts`:
   - `POST /api/intel/synthesize-context`
   - Zod validation + operation logging
   - ~15 lines

4. **Tests**:
   - Unit: tool definition, buildPrompt, agent function with mocks (~60 lines)
   - Integration: HTTP endpoint with mocked Anthropic (~40 lines)

**Total new code for Phase 2 Intel side: ~150-200 lines** (following established patterns).

**How much is copy-paste from existing agents?** ~60-70%. The agent file structure, error handling, Anthropic call pattern, tool extraction, and Zod validation are all identical across all 6 existing agents. The system prompt and tool schema are the only truly new content.

### Existing insights agent overlap

The existing `generateInsights()` agent (insights.ts) already handles some of what context synthesis needs (insight_type: `focus-shift`, `co-occurrence`, `continuity`). However, context synthesis has a broader scope -- it needs to generate narratives, name clusters, and interpret patterns in a single LLM call. A new agent is warranted rather than overloading the existing one.

### Rust-side changes needed for Phase 2

Currently `IntelClient` (client.rs) only has `name_chain()` and `summarize_chain()`. A new method is needed:

```rust
pub async fn synthesize_context(
    &self,
    request: &ContextSynthesisRequest
) -> Result<Option<ContextSynthesisResponse>, CoreError>
```

This follows the identical `Ok(None)` graceful degradation pattern already established. ~30 lines of Rust, copy-paste from `name_chain()` with different endpoint/types.

---

## 4. Integration Readiness

### Can IntelClient support context synthesis calls?

**Yes, trivially.** The pattern is well-established:

1. `IntelClient::new()` builds an HTTP client with 10s timeout (client.rs)
2. Each method does: `self.http_client.post(url).json(request).send().await`
3. Graceful degradation: any failure returns `Ok(None)`
4. Correlation ID tracking via `X-Correlation-ID` header

Adding `synthesize_context()` follows this exact pattern. The only concern is **timeout**: context synthesis via LLM may take 5-10 seconds for deep mode, which is close to the hardcoded 10-second timeout. The timeout should be configurable or increased for this endpoint.

### Should Phase 1 use Intel at all?

**No.** Phase 1 should be purely deterministic, reading only from the SQLite database and (optionally) the filesystem for context packages.

**Rationale (aligned with integration audit Section 7 recommendation):**
1. **Latency:** Phase 1 targets <2 seconds. Intel calls add 1-10 seconds.
2. **Availability:** Intel service may not be running. Context restore should always work.
3. **Separation of concerns:** Read path (queries) stays deterministic. Write path (daemon) handles enrichment.
4. **Already cached:** Intel-generated chain names and summaries are already in `chain_metadata` and `chain_summaries` tables, accessible via `LEFT JOIN` in `query_chains()`.

**Phase 1 architecture:**
```
CLI input → QueryEngine::query_context()
              |
              +→ tokio::join!(query_flex, query_heat, query_chains, query_sessions, query_timeline)
              +→ sequential: query_co_access per top-N file
              +→ sequential: filesystem glob + read for context packages
              +→ deterministic assembly of all 9 sections
              |
              → JSON output
```

**Phase 2 adds one step:**
```
              +→ HTTP POST to Intel service with deterministic data
              +→ merge synthesized fields into response
```

---

## 5. Risk Assessment

### Risk 1: Co-Access N+1 Query Blowup

**Severity: HIGH**

`query_co_access()` runs 2 SQL queries per anchor file (query.rs:929-943 and 960-991). The spec's cluster detection algorithm says "start with highest-access file, run co-access, repeat for unclustered files." For 10 anchor files, that's 20 SQL queries. For 50 files (the default limit in the audit's composition sketch), that's 100 SQL queries.

**Mitigation:** Limit to top 5 anchor files in Phase 1 (10 SQL queries, acceptable). Consider a batch co-access function for Phase 2 that accepts multiple anchors in a single SQL query.

### Risk 2: Context Package Filesystem Access

**Severity: MEDIUM**

The Rust core currently has **zero filesystem access** for content reading. All data comes from SQLite. Spec 12 requires reading context package markdown files from disk for `current_state`, `continuity`, and `quick_start` sections.

**Implication:** This introduces a new dependency -- the Rust binary must have access to the filesystem path where context packages live. This path varies by project and is not stored in the database.

**Mitigation:** Accept a `--project-path` CLI argument. Or, derive it from the `query` parameter by searching known project directories. The simplest approach: require the user to run the command from within the project directory.

### Risk 3: Spec Assumes Context Packages Always Exist

**Severity: LOW**

Spec 12 heavily relies on context packages for `current_state.narrative`, `continuity.left_off_at`, and `quick_start.commands`. Not all projects have context packages. The response must gracefully degrade when no packages are found.

**Mitigation:** All context-package-dependent sections should return `null` or empty arrays when no packages exist. The deterministic sections (work_clusters, timeline, executive_summary) work without them.

### Risk 4: query_sessions N+1 Sub-Query

**Severity: LOW** (for context restore)

The audit notes that `query_sessions()` runs a per-session sub-query for top files (query.rs:605-621). With limit=20, that's 20 additional queries. This is already the existing behavior and hasn't been a problem at current data volumes.

**Mitigation:** Monitor. If latency is a concern, the context restore orchestrator can set a lower session limit (e.g., limit=10).

### Risk 5: Spec 12 Model Reference Mismatch

**Severity: LOW**

Spec 12 references `claude-3-5-haiku-latest` for the medium depth model. The Intel service audit shows that 5 of 6 agents use pinned `claude-haiku-4-5-20251001` while `gitops-decision.ts` uses the floating `claude-3-5-haiku-latest`. The spec should be updated to use the pinned version for consistency.

### Risk 6: Timeout for Deep Mode

**Severity: MEDIUM**

Spec 12 allows up to 15 seconds for deep mode (2-3 LLM calls). The `IntelClient` has a hardcoded 10-second HTTP timeout (client.rs). Deep mode would likely time out.

**Mitigation:** Either increase the IntelClient timeout to 30 seconds, or make it configurable per request. Since context restore is a foreground operation, the user expects to wait -- 15 seconds is acceptable UX.

### Contradictions Between Audits

**No material contradictions found.** The three audits are consistent:

1. All three agree that query methods are independent (no cross-query calls).
2. The integration audit's recommendation to keep context restore deterministic-only aligns with the Rust core audit's composition sketch.
3. The Intel audit's "~150-200 lines for new agent" estimate is consistent with the established pattern sizes.

**One minor discrepancy:** The Rust core audit (Section 6) shows `query_sessions` with a `time: String` required field in `QuerySessionsInput`, but the integration audit's composition sketch shows `time: time.to_string()`. This is consistent -- just different notation for the same thing.

---

## 6. Recommended Implementation Path

### Phase 1: Deterministic Foundation

Ordered steps with specific file paths:

#### Step 1: Add types (types.rs)

**File:** `apps/tastematter/core/src/types.rs`

Add the `ContextRestoreResult` struct and sub-structs for all 9 response sections. Phase 1 versions use `Option<String>` for LLM-synthesized fields (always `None` until Phase 2).

```rust
// New types to add (approximately 50 lines)
pub struct ContextRestoreInput {
    pub query: String,
    pub depth: Option<String>,  // "quick" | "medium" | "deep"
}

pub struct ContextRestoreResult {
    pub receipt_id: String,
    pub query: String,
    pub generated_at: String,
    pub executive_summary: ExecutiveSummary,
    pub current_state: Option<CurrentState>,
    pub continuity: Option<Continuity>,
    pub work_clusters: Vec<WorkCluster>,
    pub suggested_reads: Vec<SuggestedRead>,
    pub timeline: TimelineSection,
    pub insights: Vec<ContextInsight>,
    pub verification: Verification,
    pub quick_start: Option<QuickStart>,
}
// ... plus sub-structs
```

**Est. lines:** ~50

#### Step 2: Add query_context orchestrator (query.rs)

**File:** `apps/tastematter/core/src/query.rs`

Add `QueryEngine::query_context()` method. This is the main orchestrator.

```rust
pub async fn query_context(
    &self,
    input: ContextRestoreInput,
) -> Result<ContextRestoreResult, CoreError>
```

**Calls (in parallel via tokio::join!):**
- `self.query_flex()` -- file patterns
- `self.query_heat()` -- heat classification
- `self.query_chains()` -- chain metadata
- `self.query_sessions()` -- session history
- `self.query_timeline()` -- temporal data

**Calls (sequential):**
- `self.query_co_access()` -- per top-5 file (10 SQL queries max)

**Est. lines:** ~40

#### Step 3: Add builder functions (new file: context_restore.rs)

**File:** `apps/tastematter/core/src/context_restore.rs` (NEW)

All `build_*` functions for each section. These are pure functions that transform query results into response sections.

| Function | Est. Lines |
|----------|-----------|
| `build_executive_summary()` | 20 |
| `build_work_clusters()` | 40 |
| `build_suggested_reads()` | 25 |
| `build_timeline()` | 25 |
| `build_deterministic_insights()` | 20 |
| `build_verification()` | 10 |
| **Subtotal** | **~140** |

If context package reading is included in Phase 1:

| Function | Est. Lines |
|----------|-----------|
| `discover_context_packages()` | 20 |
| `parse_context_package()` | 40 |
| `build_current_state()` | 15 |
| `build_continuity()` | 25 |
| `build_quick_start()` | 15 |
| **Subtotal (filesystem)** | **~115** |

#### Step 4: Wire into CLI (main.rs)

**File:** `apps/tastematter/core/src/main.rs`

Add `context` subcommand to Clap hierarchy following the pattern documented in audit_rust_core.md Section 4.

1. Add `Commands::Context { query, depth, format }` variant (~5 lines)
2. Add match arm dispatching to `engine.query_context()` (~10 lines)
3. Add telemetry command name mapping (~1 line)

**Est. lines:** ~16

#### Step 5: Wire into HTTP (http.rs)

**File:** `apps/tastematter/core/src/http.rs`

Add `POST /api/query/context` endpoint following existing pattern.

**Est. lines:** ~12

#### Step 6: Add module declaration

**File:** `apps/tastematter/core/src/lib.rs`

Add `pub mod context_restore;`

**Est. lines:** 1

### Phase 1 Total

| Component | File | Est. Lines |
|-----------|------|-----------|
| Types | types.rs | ~50 |
| Orchestrator | query.rs | ~40 |
| Builders (DB-only sections) | context_restore.rs | ~140 |
| Builders (filesystem sections) | context_restore.rs | ~115 |
| CLI wiring | main.rs | ~16 |
| HTTP wiring | http.rs | ~12 |
| Module decl | lib.rs | 1 |
| **TOTAL (with filesystem)** | | **~374** |
| **TOTAL (DB-only, no context packages)** | | **~259** |

### Recommended Phase 1 Split

**Phase 1a (DB-only, ~259 lines):** executive_summary, work_clusters, suggested_reads, timeline, insights (deterministic), verification. No filesystem access, no context package reading. Returns `null` for current_state, continuity, quick_start.

**Phase 1b (~115 lines):** Add context package discovery and parsing. Fills in current_state, continuity, quick_start.

---

## 7. Open Questions

### Q1: Co-access anchor file limit

Should co-access clustering be limited to top-N files (e.g., 5) to avoid the N+1 SQL blowup?

**Recommendation:** Yes, limit to 5. At 2 SQL queries per anchor, 5 anchors = 10 SQL queries, which should complete in <200ms against the connection pool. The top 5 files by access count are likely the most informative seeds anyway.

**Decision needed:** Is 5 enough? Or should we implement a batch co-access function that accepts multiple anchors in a single SQL query?

### Q2: Context package discovery path

How does the Rust binary know where to find context packages on the filesystem? Options:

1. **`--project-path` CLI argument** -- explicit, but adds cognitive load
2. **CWD-based** -- assume user runs from project root, glob `**/context_packages/*.md`
3. **Store project path in database** -- requires schema change
4. **Git root detection** -- find `.git` directory and search from there

**Recommendation:** Option 2 (CWD-based) for simplicity. The `tastematter` CLI is already assumed to run from the project directory.

### Q3: Should Phase 1 include context package reading at all?

Context package reading adds ~115 lines and introduces filesystem I/O. Without it, 6 of 9 sections are still populated (executive_summary, work_clusters, suggested_reads, timeline, insights, verification). The 3 missing sections (current_state, continuity, quick_start) are the most "narrative" sections.

**Recommendation:** Defer to Phase 1b. Phase 1a (DB-only) is already highly valuable and can ship faster.

### Q4: Response format for Phase 1 LLM-synthesized fields

In Phase 1, LLM-synthesized fields (one_liner, narrative, cluster names, etc.) won't be populated. Options:

1. **null** -- cleaner, signals "not available"
2. **Deterministic placeholder** -- e.g., cluster name = "Files matching *.ts" instead of LLM-generated name
3. **Omit field entirely** -- different schema between Phase 1 and Phase 2

**Recommendation:** Option 1 (null). Use `Option<String>` in Rust types. This keeps the schema stable between phases.

### Q5: Timeout configuration for IntelClient

The hardcoded 10-second timeout may be insufficient for Phase 2 deep mode. Should the timeout be:

1. **Increased globally** to 30 seconds?
2. **Per-method configurable** (name_chain: 10s, synthesize_context: 30s)?
3. **Per-request configurable** via a parameter?

**Recommendation:** Option 2. Most methods are fine at 10s. Only `synthesize_context()` needs longer.

### Q6: Should the `context` command support glob patterns or natural language?

Spec 12 shows both `tastematter context "nickel"` and `tastematter context "what am I working on"`. The first is a file pattern match; the second requires query interpretation (Phase 2, LLM).

**Recommendation:** Phase 1 treats the query as a file glob pattern (`*{query}*`). Natural language query interpretation is a Phase 2 feature via Intel service.

### Q7: Cache strategy for context restore results

Should the `ContextRestoreResult` be cached? Spec 12 mentions "Cache aggressively" and "<100ms cache hit."

**Recommendation:** Defer caching to Phase 3 (per spec). Phase 1 queries are already fast (<500ms total for all parallel queries). Caching adds complexity (invalidation, TTL) that isn't needed at current data volumes.

---

## Appendix: Cross-Reference Index

### Audit Report -> Spec Section Mapping

| Audit Finding | Relevant Spec 12 Section | Impact |
|---------------|-------------------------|--------|
| query_flex supports file patterns (audit_rust_core:3) | work_clusters, suggested_reads | Direct reuse |
| query_co_access single-anchor only (audit_rust_core:119-122) | work_clusters | N+1 bottleneck |
| query_sessions N+1 sub-query (audit_rust_core:106) | executive_summary | Minor latency concern |
| query_heat pure functions (audit_rust_core:124-127) | executive_summary (status) | Direct reuse |
| query_timeline 3-step SQL (audit_rust_core:109-115) | timeline | Direct reuse |
| All queries independent (audit_rust_core:131) | query_context orchestrator | Enables tokio::join! |
| IntelClient graceful degradation (audit_integration:200-223) | Phase 2 integration | Proven pattern |
| Intel tool_choice pattern (audit_intel:140-198) | Phase 2 synthesize-context agent | Fill-in-the-blanks |
| No cache TTL for chain metadata (audit_integration:76-78) | continuity (cached names) | Stale names possible |
| 10s HTTP timeout hardcoded (audit_integration:226-227) | Phase 2 deep mode | Timeout risk |
| CostGuard not wired (audit_intel:369) | Phase 2 budget control | Must wire for production |
| insights agent exists (audit_intel:100-110) | insights section Phase 2 | Partial reuse |
| IntelClient missing generate_insights() (audit_integration:17-23) | insights section Phase 2 | New method needed |
