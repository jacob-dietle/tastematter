---
title: "Tastematter Context Package 38"
package_number: 38
date: 2026-02-17
status: current
previous_package: "[[37_2026-02-10_CONTEXT_RESTORE_PHASE2_COMPLETE]]"
related:
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[core/src/storage.rs]]"
  - "[[core/src/types.rs]]"
  - "[[apps/codegraph-teardown/]]"
  - "[[specs/canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]"
tags:
  - context-package
  - tastematter
  - temporal-edges
  - codegraph-teardown
  - architecture
  - design-research
---

# Tastematter - Context Package 38

## Executive Summary

Deep architectural analysis session comparing CodeGraph (AST-based code graph) with tastematter (behavioral work intelligence). Identified a major design opportunity: **typed, directed temporal edges** extracted from per-tool-call timestamps in session JSONL data. The data already exists in the parser — it's being destroyed during session summarization. This package captures the full design thesis, feasibility analysis, and the empirical validation needed before implementation.

## Global Context

### CodeGraph Teardown (apps/codegraph-teardown/)

Cloned https://github.com/colbymchenry/codegraph for architectural analysis. ~20K LOC TypeScript. Key findings:

**What it is:** AST-based code graph using tree-sitter to parse 17 languages, extract symbols (functions, classes, methods, types) and relationships (calls, imports, extends), store in per-project SQLite, serve via MCP server to Claude Code's Explore agents.

**Three unique engineering principles identified:**
1. **Relationships are first-class, not derived** — typed, directional edges (`calls`, `imports`, `extends`), not statistical correlations or embedding proximity
2. **Extraction and resolution are separate phases** — dumb fast extraction → smart resolution → honest tracking of what couldn't be resolved (`unresolved_refs` table)
3. **Pre-computed structure, instant query** — expensive work at index time, <10ms queries at runtime

**Where it excels:** Codebases with explicit static relationships — TypeScript, Java, C# with strong typing, class hierarchies, import graphs.

**Where it fails:** Dynamic dispatch, event-driven architectures, config-driven wiring, metaprogramming, cross-repo relationships. Also: zero temporal awareness — treats all code equally regardless of work patterns.

### Strategic Positioning: Three Approaches to Code Intelligence

| Approach | How | Good at | Bad at |
|----------|-----|---------|--------|
| **Lexical/agentic** (grep, glob, Read) | Text search + file reading | Finding string matches | Relationships, token waste |
| **Semantic/vector** (embeddings) | Cosine similarity in vector space | Fuzzy meaning match | Precision, structural relationships, noise |
| **Structural/graph** (CodeGraph) | AST → typed edge graph | Call chains, impact analysis | Dynamic behavior, temporal patterns |

**Tastematter occupies none of these.** It's a **behavioral/temporal** system — it indexes work patterns, not code structure. This is a fundamentally different information source. User's thesis: "Vector databases are a dead end for agents."

### The Temporal Edges Design Thesis

**Core insight:** CodeGraph makes structural relationships first-class. Tastematter should make **behavioral relationships** first-class — not as statistical correlations (PMI co-access), but as **typed, directed edges extracted deterministically from temporal ordering of tool calls within sessions.**

**The data exists but is being destroyed.** The JSONL parser extracts per-tool-call data:

```rust
// Already exists in capture/jsonl_parser.rs:35-50
pub struct ToolUse {
    pub id: String,                    // unique tool invocation ID
    pub name: String,                  // "Read", "Edit", "Grep"
    pub timestamp: DateTime<Utc>,      // ← PER-CALL TIMESTAMP (millisecond precision)
    pub file_path: Option<String>,     // which file
    pub is_read: bool,                 // true for Read, Grep, Glob
    pub is_write: bool,                // true for Edit, Write
}
```

But during session summarization (`jsonl_parser.rs:562-695`), this collapses to:

```rust
// What gets stored in claude_sessions table:
files_read: Vec<String>,      // DEDUPLICATED SET — no order, no timestamps
files_written: Vec<String>,   // DEDUPLICATED SET — no order, no timestamps
tools_used: HashMap<String, i32>,  // COUNTS ONLY — no sequence
```

**The entire within-session ordering is lost.** The database knows "session touched A, B, C" but not "A was read at 14:30:01, then B at 14:30:15, then C was edited at 14:31:02."

## The Design: Three-Layer Rollup Architecture

### Layer 1: file_access_events (~190K rows, stored at parse time)

```sql
file_access_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    file_path TEXT NOT NULL,
    tool_name TEXT NOT NULL,      -- Read, Edit, Write, Grep, Glob
    access_type TEXT NOT NULL,    -- read, write, search
    sequence_position INTEGER     -- ordinal within session (0-indexed)
)
-- Indexes: (session_id), (file_path), (session_id, sequence_position)
```

**Change required:** In `daemon/sync.rs`, after inserting `claude_sessions`, also insert individual `ToolUse` records into this table. The parser already extracts them — they just need to be persisted instead of discarded.

### Layer 2: file_edges (~10K-50K rows, extracted in batch during daemon sync)

```sql
file_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_file TEXT NOT NULL,
    target_file TEXT NOT NULL,
    edge_type TEXT NOT NULL,       -- read_before, read_then_edit, co_edited, reference_anchor, debug_chain
    session_count INTEGER,         -- how many sessions show this pattern
    total_sessions_with_source INTEGER, -- denominator for confidence
    avg_time_delta_seconds REAL,   -- average time between source and target access
    confidence REAL,               -- session_count / total_sessions_with_source
    first_seen TEXT,
    last_seen TEXT
)
-- Indexes: (source_file, edge_type), (target_file, edge_type), (edge_type, confidence DESC)
```

**Edge type extraction rules (deterministic, not ML):**
- `read_before`: file A read before file B in >50% of shared sessions, within 5-minute window
- `read_then_edit`: file A read, then file B edited within same session (directional: A informed B)
- `co_edited`: both files edited in same session (mutual dependency, undirected)
- `reference_anchor`: file read in first 2 minutes of >3 sessions (entry point / navigation file)
- `debug_chain`: file read after a Bash tool call (pattern: error → investigate → fix)

**Runs as batch job during daemon sync**, after chain building. Same pattern as existing `build_chain_graph` and `build_inverted_index`. Incremental: only process new sessions since last edge extraction.

### Layer 3: work_patterns (~3-8 patterns per context query, computed at query time)

**The agent never sees Layer 1 or Layer 2.** At query time, `query_context()` adds one step:

```sql
-- Fetch edges only for files matching query pattern
SELECT source_file, target_file, edge_type, session_count, confidence
FROM file_edges
WHERE (source_file LIKE '%pixee%' OR target_file LIKE '%pixee%')
  AND session_count >= 3
  AND confidence >= 0.5
ORDER BY session_count DESC
LIMIT 50
```

50 rows max → lightweight Rust pattern extractor:
1. **Find entry points**: files consistently `source` in `read_before` edges (reference/anchor files)
2. **Find work targets**: files consistently `target` in `read_then_edit` edges (what you edit)
3. **Find typical sequences**: topological sort of `read_before` chain (A→B→C ordering)
4. **Find incomplete sequences**: compare last session's order against typical sequence

**Output enhancement to existing context result (~50-100 extra tokens):**

```json
{
  "work_clusters": [
    {
      "files": ["query.rs", "types.rs", "storage.rs", "main.rs"],
      "pmi_score": 0.55,
      "entry_points": ["types.rs"],
      "work_targets": ["query.rs"],
      "typical_sequence": ["types.rs", "storage.rs", "query.rs", "main.rs"],
      "sequence_confidence": 0.72
    }
  ],
  "continuity": {
    "incomplete_sequence": {
      "observed": ["types.rs → storage.rs → query.rs"],
      "typical_next": "main.rs (4/5 prior completions)",
      "confidence": 0.80
    }
  }
}
```

### Noise Filtering Strategy

Three sources of noise, three deterministic filters:

1. **Explore agent spam** (20-40 files read in rapid succession): Filter by velocity — >5 file reads in 30 seconds with no edits = exploration, not intentional work. Tag during extraction, exclude from edge computation.

2. **Universal entry points** (CLAUDE.md read in every session): Files appearing as `source` in >80% of sessions get tagged as `universal_anchor` — track separately, don't create edges to every downstream file. Same principle as IDF in information retrieval.

3. **Coincidental co-access**: The `session_count >= 3` threshold handles this. A coincidence happens once. A pattern repeats. Same principle as CodeGraph's reference resolution.

## Why This Matters for Markdown / Knowledge Graphs

User insight: "A large portion of what tastematter is used for is context operating systems where it's not code — it's markdown files, which is now code because of LLM-based agents."

CodeGraph's AST approach cannot parse markdown relationships. There's no tree-sitter grammar that finds "this CLAUDE.md references this positioning doc." But temporal edges CAN:

```
CLAUDE.md --read_before(8/10 sessions)--> _synthesis/foundation-summary.md
_synthesis/foundation-summary.md --read_then_edit(5/8 sessions)--> 00_foundation/positioning/value-prop.md
```

This tells you: "CLAUDE.md is the navigation anchor, synthesis is the reference layer, positioning doc is the work target." Discovered purely from behavioral patterns, no semantic understanding needed.

**This is tastematter's unique advantage over CodeGraph:** it works on ANY file type because the intelligence comes from human attention patterns, not from parsing code structure.

## Feasibility Assessment

### Data Availability: CONFIRMED ✅

Every tool call in session JSONL has a timestamp [VERIFIED: [[core/src/capture/jsonl_parser.rs]]:33-50]. The parser already extracts `ToolUse { timestamp, file_path, is_read, is_write }` [VERIFIED: [[core/src/capture/jsonl_parser.rs]]:294-342]. ~190K tool uses across all session history [VERIFIED: MEMORY.md, audited 2026-02-05].

### Storage Overhead: MINIMAL ✅

190K rows in SQLite with 5 indexed columns = ~50-80MB. SQLite handles millions of rows trivially. Query time with proper indexes: <1ms.

### Compute Overhead: LOW ✅

Edge extraction runs as batch job during daemon sync (every 30 min). Same architecture as existing `build_chain_graph`. Incremental: only process new sessions since last extraction.

### Accuracy: NEEDS EMPIRICAL VALIDATION ⚠️

**The critical open question:** Does the temporal ordering in JSONL actually encode meaningful work patterns, or is it mostly Explore agent noise?

**Proposed validation:** Sample 10 sessions, manually trace tool call sequences, check: "Is there a pattern? Would knowing this ordering help restore context?"

If signal in 7/10 sessions → build it.
If mostly noise → the filtering strategy needs to be more aggressive, or the thesis is wrong.

## What Changed Since Package 37

Based on git history and heat data (post Feb 10):

| Commit | What |
|--------|------|
| `281d7ff` | Intel Rust Port Phase 1 — embedded context synthesis, E2E tests, `tastematter intel setup` |
| `7584298` | Phase 1 Context Alerting — alert-worker (CF Worker) + web-app MVP (Svelte) |
| `d146032` | Fix heat formula — session-spread + exponential decay |
| `55aabb6` + `7a2c169` | 65 stress tests + 8 E2E stress scenarios |
| `89cec71` | E2E user experience pipeline + UTF-8 parser fix |
| `3a67d36` | cargo fmt + staging narrative assertion fix |

**New components (HOT, last 2 days):**
- `alert-worker/` — CF Worker for context change alerting
- `download-alert-worker/` — Download alert worker
- `web-app/` — Svelte app (SvelteKit + Knock notifications) at app.tastematter.dev
- `specs/canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md` — Product architecture

## Full Trace: `tastematter context` Command

### CLI Entry (main.rs:1389-1406)

```
tastematter context "query" --time 30d --limit 20
  → Creates ContextRestoreInput { query, time, limit }
  → Calls engine.query_context(input)
```

### Query Engine (query.rs:1350-1454) — Five Phases

```
Phase 1: tokio::join! parallel DB queries
  ├─ query_flex     → files matching *query* pattern, sorted by access count
  ├─ query_heat     → heat scores (specificity, velocity, percentile classification)
  ├─ query_chains   → all chains with session counts
  ├─ query_sessions → recent sessions within time window
  └─ query_timeline → weekly bucketed access data

Phase 2: Sequential co-access for top 5 hot files
  └─ For each anchor: query_co_access → PMI-based co-occurrence

Phase 3: Filesystem discovery (context_restore::discover_project_context)
  └─ walkdir glob: specs/, context_packages/, CLAUDE.md, etc.

Phase 4: Assembly via pure builder functions
  ├─ build_executive_summary  → status/tempo from recency + heat
  ├─ build_current_state      → metrics + evidence from filesystem
  ├─ build_continuity         → left_off_at + pending items
  ├─ build_work_clusters      → PMI co-access groups (UNDIRECTED, UNORDERED)
  ├─ build_suggested_reads    → ranked file list
  ├─ build_timeline           → weekly buckets
  └─ build_insights           → abandoned file detection

Phase 5: Optional LLM synthesis (Haiku, <$0.0003/req)
  └─ build_synthesis_request → intel.synthesize_context → merge_synthesis
```

### What the Temporal Edges Would Change

**Phase 2 enhancement:** Add edge query alongside co-access:
```sql
SELECT source_file, target_file, edge_type, session_count, confidence
FROM file_edges WHERE (source/target LIKE pattern) AND session_count >= 3
```

**Phase 4 enhancement:** New builder `build_work_patterns(edges)` that:
- Identifies entry_points, work_targets from edge directionality
- Computes typical_sequence via topological sort
- Detects incomplete_sequence by comparing last session to pattern

**Output:** Existing `work_clusters` and `continuity` fields gain 4-6 new subfields. ~50-100 extra tokens in final output. Agent gets directed workflow info, not just file bags.

## Database Schema: Current vs Proposed

### Current (storage.rs:148-165)

```sql
claude_sessions (
    session_id TEXT PRIMARY KEY,
    files_read TEXT,           -- JSON array (deduplicated set, NO ORDER)
    files_written TEXT,        -- JSON array (deduplicated set, NO ORDER)
    tools_used TEXT,           -- JSON object (counts only, NO SEQUENCE)
    ...
)
```

### Proposed Addition

```sql
file_access_events (           -- NEW TABLE: preserves temporal ordering
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    file_path TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    access_type TEXT NOT NULL,  -- read, write, search
    sequence_position INTEGER
)

file_edges (                   -- NEW TABLE: directed behavioral edges
    id INTEGER PRIMARY KEY,
    source_file TEXT NOT NULL,
    target_file TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    session_count INTEGER,
    confidence REAL,
    avg_time_delta_seconds REAL,
    last_seen TEXT
)
```

## Local Problem Set

### Completed This Session

- [X] Cloned and analyzed CodeGraph (apps/codegraph-teardown/) [VERIFIED: directory exists]
- [X] Identified three transferable design principles from CodeGraph [VERIFIED: this package]
- [X] Full trace of `tastematter context` from CLI → core → DB [VERIFIED: [[core/src/query.rs]]:1350-1454, [[core/src/context_restore.rs]]:110-187]
- [X] Confirmed temporal data exists in parser but is destroyed during summarization [VERIFIED: [[core/src/capture/jsonl_parser.rs]]:35-50 vs 688-694]
- [X] Designed three-layer rollup architecture (events → edges → patterns) [VERIFIED: this package]
- [X] Identified noise filtering strategy (explore spam, universal anchors, coincidence) [VERIFIED: this package]

### Jobs To Be Done (Next Session)

1. [ ] **EMPIRICAL VALIDATION (DO THIS FIRST)** — Sample 10 sessions from actual JSONL data, trace per-tool-call sequences, evaluate signal quality
   - Success criteria: 7/10 sessions show meaningful temporal patterns
   - Method: Read raw JSONL for 10 sessions, extract tool call sequences with timestamps, manually inspect ordering
   - If signal is weak → revisit thesis before building anything

2. [ ] **Schema migration** — Add `file_access_events` and `file_edges` tables to storage.rs
   - Depends on: empirical validation passing
   - Complexity: Low (two CREATE TABLE statements + indexes)

3. [ ] **Parser integration** — Modify daemon sync to persist individual ToolUse records alongside session summaries
   - Depends on: schema migration
   - Key file: [[core/src/daemon/sync.rs]]
   - Complexity: Medium (batch insert of tool uses during session sync)

4. [ ] **Edge extraction module** — New module implementing deterministic edge type rules
   - Depends on: parser integration
   - New file: `core/src/index/temporal_edges.rs` or `core/src/index/file_edges.rs`
   - Complexity: Medium-High (the noise filtering is the hard part)

5. [ ] **Context restore integration** — Add edge query to Phase 2, pattern builder to Phase 4
   - Depends on: edge extraction module
   - Key files: [[core/src/query.rs]], [[core/src/context_restore.rs]]
   - Complexity: Medium (new builder function, enhanced work_clusters/continuity types)

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/main.rs]] | CLI entry, `Commands::Context` dispatch | Reference |
| [[core/src/query.rs]]:1350-1454 | `query_context()` five-phase pipeline | Reference — will be enhanced |
| [[core/src/context_restore.rs]]:110-187 | `build_work_clusters()` PMI-based | Reference — will be enhanced |
| [[core/src/capture/jsonl_parser.rs]]:35-50 | `ToolUse` struct with per-call timestamps | Reference — data source |
| [[core/src/capture/jsonl_parser.rs]]:562-695 | `build_session_summary()` where ordering is lost | Reference — no change needed |
| [[core/src/storage.rs]]:134-242 | Database schema (tables + indexes) | Will be modified |
| [[core/src/daemon/sync.rs]] | Daemon sync pipeline | Will be modified |
| [[core/src/types.rs]] | API types (WorkCluster, Continuity, etc.) | Will be modified |
| [[apps/codegraph-teardown/]] | CodeGraph clone for analysis | Reference only |
| [[apps/codegraph-teardown/src/extraction/tree-sitter.ts]] | AST extraction (~2560 LOC) | Reference for design patterns |
| [[apps/codegraph-teardown/src/resolution/index.ts]] | Reference resolution with cache warming | Reference for design patterns |
| [[apps/codegraph-teardown/src/graph/traversal.ts]] | BFS/DFS graph traversal | Reference for design patterns |

## Test State

- **Rust core:** 330+ tests passing (`cargo test -- --test-threads=2`) [VERIFIED: CLAUDE.md]
- **Intel TS:** All passing (`cd intel && bun test`) [VERIFIED: CLAUDE.md]
- **No new tests written this session** (design/research session, not implementation)

### Test Commands for Next Agent

```bash
# Verify core still compiles and passes
cd apps/tastematter/core && cargo check
cd apps/tastematter/core && cargo test -- --test-threads=2

# EMPIRICAL VALIDATION: Sample session JSONL for temporal patterns
# Read a raw JSONL session file and trace tool call sequence:
# Look in ~/.claude/projects/*/  for *.jsonl files
# Each line is a JSON record with "type", "timestamp", and tool_use content blocks
# Extract: timestamp, tool_name, file_path for each tool call
# Check: is the ordering meaningful? Would it help an agent restore context?
```

## For Next Agent

**Context Chain:**
- Previous: [[37_2026-02-10_CONTEXT_RESTORE_PHASE2_COMPLETE]] (LLM synthesis shipped)
- This package: Temporal edges design thesis from CodeGraph teardown
- Next action: **Empirical validation of temporal signal quality in session data**

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[core/src/capture/jsonl_parser.rs]]:35-50 to understand ToolUse struct
3. Read [[core/src/query.rs]]:1350-1454 to understand current context pipeline
4. **Run empirical validation:** Sample 10 sessions from `~/.claude/projects/`, extract per-tool-call sequences, evaluate whether temporal ordering encodes meaningful work patterns
5. If validation passes (7/10 sessions show signal), proceed to schema migration

**Do NOT:**
- Build any of the edge extraction infrastructure before validating the signal
- Run `cargo test` without `--test-threads=2` (crashes VS Code)
- Edit existing context packages (append-only)
- Assume the temporal ordering is useful — PROVE IT with data first

**Key insight:**
The JSONL parser already extracts per-tool-call timestamps and file paths. This temporal ordering is destroyed during session summarization. If this ordering encodes meaningful work patterns (empirically validated), it becomes a fundamentally unique data source — typed, directed behavioral edges that work on ANY file type (code, markdown, YAML) because the intelligence comes from human attention patterns, not code structure. [VERIFIED: [[core/src/capture/jsonl_parser.rs]]:35-50 vs 688-694]

**Design principle stolen from CodeGraph:**
"Extract typed, directional relationships from deterministic data and make them queryable as a graph." CodeGraph does this for code (AST → symbol graph). Tastematter should do this for work (session events → behavior graph).
