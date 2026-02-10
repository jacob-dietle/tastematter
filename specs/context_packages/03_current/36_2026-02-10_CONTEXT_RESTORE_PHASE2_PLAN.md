---
title: "Tastematter Context Package 36"
package_number: 36
date: 2026-02-10
status: current
previous_package: "[[35_2026-02-09_DB_AUTO_INIT_COMPLETE]]"
related:
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/intelligence/types.rs]]"
  - "[[core/src/intelligence/client.rs]]"
  - "[[core/src/intelligence/mod.rs]]"
  - "[[core/src/query.rs]]"
  - "[[intel/src/types/shared.ts]]"
  - "[[intel/src/index.ts]]"
  - "[[intel/src/agents/chain-summary.ts]]"
tags:
  - context-package
  - tastematter
  - context-restore
  - llm-synthesis
  - phase-2
---

# Tastematter - Context Package 36

## Executive Summary

Completed detailed implementation plan for Context Restore Phase 2: LLM Synthesis. This adds intelligence to the 5 `Option<String>` / `None` fields in `tastematter context` output — `one_liner`, `narrative`, `cluster_names`, `cluster_interpretations`, and `suggested_read_reasons`. Plan approved, no code written yet. ~400 lines across 9 files, ~60% follows existing patterns verbatim.

## Global Context

### What is Context Restore Phase 2?

Phase 1 (shipped v0.1.0-alpha.20) returns deterministic JSON from parallel DB queries + filesystem discovery. Five fields are always `None` because they need LLM synthesis:

1. `ExecutiveSummary.one_liner` — e.g. "Nickel transcript worker is production-ready with 4 providers"
2. `CurrentState.narrative` — e.g. "You built a multi-provider ingestion system..."
3. `WorkCluster.name` (per cluster) — e.g. "Core Pipeline", "Type Contracts"
4. `WorkCluster.interpretation` (per cluster) — e.g. "Active development files that move together"
5. `SuggestedRead.reason` (per file) — e.g. "Latest context package — start here to resume"

### Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| LLM calls per request | **1 call, all 5 fields** | Spec says <$0.0003 for medium depth |
| Data sent to LLM | **Curated subset** | Full result ~20KB, LLM needs ~2-4K tokens |
| Array handling | **Index-matched arrays** | Clusters/reads sent as numbered lists, returned same order |
| Model | **Haiku** (`claude-haiku-4-5-20251001`) | Same model as chain-naming, chain-summary |
| Degradation | **Non-negotiable graceful** | Intel down → return deterministic-only (5 fields stay None) |

### Key Existing Patterns to Follow

- **Agent pattern:** `chain-summary.ts` — system prompt + tool_choice for structured output [VERIFIED: [[intel/src/agents/chain-summary.ts]]:1-48]
- **Client pattern:** `summarize_chain()` — POST, correlation_id, graceful Ok(None) on failure [VERIFIED: [[intel/src/intelligence/client.rs]]:124-198]
- **Route pattern:** `POST /api/intel/summarize-chain` with `withOperationLogging` + Zod validation [VERIFIED: [[intel/src/index.ts]]:191-225]
- **Type pattern:** Serde derives with `skip_serializing_if` for optional fields [VERIFIED: [[core/src/intelligence/types.rs]]:1-128]
- **Integration point:** `query_context()` at line 1324 of query.rs — Phase 4 assembly section [VERIFIED: [[core/src/query.rs]]:1394-1416]

## Implementation Plan (8 Steps)

### Step 1: TypeScript Types (`intel/src/types/shared.ts`) — ~40 lines

Add Zod schemas (pure addition):
- `ContextSynthesisRequestSchema`: query, status, work_tempo, clusters[], suggested_reads[], context_package_content?, key_metrics?, evidence_sources[]
- `ContextSynthesisResponseSchema`: one_liner, narrative, cluster_names[], cluster_interpretations[], suggested_read_reasons[], model_used

### Step 2: Agent (`intel/src/agents/context-synthesis.ts`) — ~120 lines NEW

Follow `chain-summary.ts` pattern:
- System prompt with rules for each field (one_liner <120 chars, narrative 2-4 sentences, cluster names 2-4 words, etc.)
- Dynamic system prompt injects cluster/read counts so LLM knows expected array lengths
- `CONTEXT_SYNTHESIS_TOOL` with 5 required output fields
- `buildPrompt(request)` assembles: query, status, tempo, numbered clusters with files, numbered reads with paths, context package content (truncated 3K chars)
- `synthesizeContext()` calls Claude Haiku with `tool_choice: { type: "tool", name: "output_context_synthesis" }`
- Extract tool_use block, validate with Zod

### Step 3: Intel Route (`intel/src/index.ts`) — ~25 lines

Add `POST /api/intel/synthesize-context` using `withOperationLogging`:
- Zod validation on request body
- Input metrics: query, cluster_count, read_count
- Output metrics: model_used, one_liner length

### Step 4: Rust Intel Types (`core/src/intelligence/types.rs`) — ~45 lines

Add matching types with serde derives:
- `ClusterInput { files, access_pattern, pmi_score }`
- `SuggestedReadInput { path, priority, surprise }`
- `ContextSynthesisRequest` / `ContextSynthesisResponse`
- Unit tests: serialize request, deserialize response

### Step 5: Rust IntelClient Method (`core/src/intelligence/client.rs`) — ~50 lines

Add `synthesize_context()` following `summarize_chain()` pattern:
- POST to `/api/intel/synthesize-context`
- Per-request timeout of 15s via `RequestBuilder::timeout()`
- `Ok(Some(response))` on success, `Ok(None)` on any failure
- Structured logging at `intelligence` target

### Step 6: Build Request + Merge (`core/src/context_restore.rs`) — ~60 lines

**`build_synthesis_request(result, context_files) -> ContextSynthesisRequest`:**
- Extract clusters → `Vec<ClusterInput>`
- Extract suggested_reads → `Vec<SuggestedReadInput>` (cap at 10)
- First high-tier context file content, truncate 3000 chars
- Key metrics + evidence sources from current_state

**`merge_synthesis(result: &mut ContextRestoreResult, synthesis)`:**
- Set `executive_summary.one_liner`
- Set `current_state.narrative`
- Iterate clusters by index → set name + interpretation via `.get(i)`
- Iterate reads by index → set reason via `.get(i)`
- Log warning on array length mismatch

### Step 7: QueryEngine Integration (`core/src/query.rs`) — ~40 lines

Builder pattern preserves existing callers:
```rust
pub struct QueryEngine {
    db: Database,
    intel_client: Option<IntelClient>,  // NEW
}

pub fn new(db: Database) -> Self { Self { db, intel_client: None } }
pub fn with_intel(mut self, client: IntelClient) -> Self { ... }
```

In `query_context()`, after deterministic assembly:
1. Build synthesis request
2. Skip if clusters empty AND reads empty AND no context package
3. Call `synthesize_context` if intel_client present
4. Merge on `Ok(Some(...))`

### Step 8: Wire CLI + HTTP (`main.rs`, `http.rs`) — ~15 lines

```rust
let engine = QueryEngine::new(db).with_intel(IntelClient::default());
```

## Parallelization Strategy

Steps 1-3 (TypeScript) and Steps 4-6 (Rust) are **fully independent** and can run in parallel. Step 7-8 depend on both being complete.

## Current State of Files to Modify

| File | Current Lines | What Exists | What's Needed |
|------|--------------|-------------|---------------|
| `intel/src/types/shared.ts` | ~280 | 6 request/response schemas | Add 2 more schemas |
| `intel/src/agents/context-synthesis.ts` | NEW | N/A | New file ~120 lines |
| `intel/src/index.ts` | ~488 | 7 endpoints | Add 1 more endpoint |
| `core/src/intelligence/types.rs` | ~579 | 3 request/response type groups | Add 1 more group |
| `core/src/intelligence/client.rs` | ~328 | 2 methods (name_chain, summarize_chain) | Add 1 more method |
| `core/src/intelligence/mod.rs` | ~47 | Re-exports types + client + cache | No change (wildcard re-export) |
| `core/src/context_restore.rs` | ~769 | 8 builder functions, 5 fields are None | Add build_synthesis_request + merge_synthesis |
| `core/src/query.rs` | ~1445 | `QueryEngine { db }` | Add `intel_client: Option<IntelClient>` |
| `core/src/main.rs` | ~690 | `QueryEngine::new(db)` | `.with_intel(IntelClient::default())` |
| `core/src/http.rs` | ~230 | `QueryEngine` construction | Same one-liner change |

## Verification Commands

```bash
# Rust
cd apps/tastematter/core
cargo build --release
cargo clippy -- -D warnings
cargo test storage::tests intelligence context_restore -- --test-threads=2

# TypeScript
cd apps/tastematter/intel && bun test

# E2E with synthesis (intel service running)
cd apps/tastematter/intel && bun run dev &
cd apps/tastematter/core && ./target/release/tastematter context "nickel" --format json
# → Verify one_liner, narrative, cluster names populated

# E2E graceful degradation (intel service stopped)
./target/release/tastematter context "nickel" --format json
# → Verify 5 fields are null, no errors
```

## Jobs To Be Done (Next Session)

1. [ ] **Implement Steps 1-3 (TypeScript side)** — ~1.25 hr
   - Types, agent, route in intel/
   - Success criteria: `bun test` passes, endpoint responds to POST
2. [ ] **Implement Steps 4-6 (Rust side)** — ~1.25 hr
   - Types, client method, builder functions
   - Success criteria: `cargo test -- --test-threads=2` passes
3. [ ] **Implement Steps 7-8 (Integration)** — ~0.75 hr
   - QueryEngine builder pattern, wire CLI + HTTP
   - Success criteria: `cargo build --release` clean
4. [ ] **E2E testing** — ~1 hr
   - Intel running: 5 fields populated
   - Intel stopped: 5 fields null, no errors
   - Deterministic fields unchanged (receipt_id, status, etc.)
5. [ ] Tag v0.1.0-alpha.22

Steps 1-3 and 4-6 **can run in parallel** (separate languages, no shared files).

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/context_restore.rs]] | Builder functions (5 Phase 2 None markers) | To modify |
| [[core/src/intelligence/types.rs]] | Rust intel types (3 existing groups) | To modify |
| [[core/src/intelligence/client.rs]] | Rust HTTP client (2 existing methods) | To modify |
| [[core/src/intelligence/mod.rs]] | Re-exports (wildcard) | No change needed |
| [[core/src/query.rs]] | QueryEngine (db only currently) | To modify |
| [[core/src/main.rs]] | CLI entry point | To modify (1 line) |
| [[core/src/http.rs]] | HTTP server | To modify (1 line) |
| [[intel/src/types/shared.ts]] | Zod schemas | To modify |
| [[intel/src/agents/context-synthesis.ts]] | New agent | To create |
| [[intel/src/index.ts]] | Elysia routes | To modify |

## Test State

- Storage tests: 15 passing [VERIFIED: package 35]
- Full test suite: 311 tests [VERIFIED: package 35]
- Intel tests: existing pass [VERIFIED: `bun test` pattern from existing agents]
- Command: `cargo test -- --test-threads=2` (ALWAYS use thread limit)

## For Next Agent

**Context Chain:**
- Previous: [[35_2026-02-09_DB_AUTO_INIT_COMPLETE]] (auto-init done, Phase 2 next)
- This package: Phase 2 plan complete, no code written
- Next action: Implement the 8 steps

**Start here:**
1. Read this context package for the full plan
2. Read [[intel/src/agents/chain-summary.ts]] as the template for the new agent
3. Read [[core/src/intelligence/client.rs]] as the template for the new client method
4. Start Steps 1-3 (TS) and 4-6 (Rust) in parallel if using team agents

**Do NOT:**
- Run `cargo test` without `--test-threads=2` (will crash VS Code)
- Send full context restore result (~20KB) to LLM — curate a 2-4K token subset
- Break graceful degradation — intel down must return deterministic-only result
- Change existing `query_context()` return type — only fill in None fields

**Key insight:**
This is a wiring task, not a design task. ~60% of the code follows existing patterns verbatim.
The only novel work is the agent prompt design (Step 2) and the request builder (Step 6).
[INFERRED: from comparison of plan steps vs existing code in chain-summary.ts and client.rs]
