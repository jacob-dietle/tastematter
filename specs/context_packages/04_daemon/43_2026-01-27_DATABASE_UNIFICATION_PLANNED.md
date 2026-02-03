---
title: "Tastematter Intel Service - Context Package 43"
package_number: 43
date: 2026-01-27
status: current
previous_package: "[[42_2026-01-26_PRODUCTION_OBSERVABILITY_IMPLEMENTED]]"
related:
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/intelligence/cache.rs]]"
  - "[[core/src/query.rs]]"
  - "[[intel/src/index.ts]]"
tags:
  - context-package
  - tastematter
  - intel-service
  - database-unification
---

# Tastematter Intel Service - Context Package 43

## Executive Summary

Completed observability implementation (Task 3: daemon→Intel wiring). Fixed Intel service API key loading. Diagnosed two-database anti-pattern preventing enriched chain names in query output. Next session: unify databases.

## Global Context

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    TASTEMATTER ARCHITECTURE                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Rust Core (port 3001)              TypeScript Intel (port 3002)│
│  ┌─────────────────────┐            ┌─────────────────────┐     │
│  │ CLI: tastematter    │───HTTP────►│ Intel Service       │     │
│  │ - query chains      │            │ - name-chain        │     │
│  │ - intel health      │            │ - analyze-commit    │     │
│  │ - intel name-chain  │            │ - summarize-session │     │
│  └─────────────────────┘            └─────────────────────┘     │
│           │                                   │                  │
│           ▼                                   ▼                  │
│  ┌─────────────────────┐            ┌─────────────────────┐     │
│  │ context_os_events.db│            │ Anthropic API       │     │
│  │ (main database)     │            │ (Claude Haiku)      │     │
│  └─────────────────────┘            └─────────────────────┘     │
│           ▲                                                      │
│           │ PROBLEM: intel_cache.db would be SEPARATE            │
│           │ Can't JOIN chain_graph with chain_metadata           │
│           ▼                                                      │
│  ┌─────────────────────┐                                        │
│  │ intel_cache.db      │ ◄── ANTI-PATTERN (not yet created)     │
│  │ (would store names) │                                        │
│  └─────────────────────┘                                        │
└─────────────────────────────────────────────────────────────────┘
```

### Key Files

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/daemon/sync.rs]] | Daemon sync with Intel enrichment | Modified - wired in |
| [[core/src/intelligence/cache.rs]] | MetadataStore - uses separate DB | NEEDS FIX |
| [[core/src/intelligence/client.rs]] | IntelClient HTTP calls | Complete |
| [[core/src/query.rs]] | query_chains - no enrichment join | NEEDS FIX |
| [[core/src/types.rs]] | ChainData struct - no generated_name | NEEDS FIX |
| [[intel/src/index.ts]] | Intel service - API key fixed | Modified |
| [[intel/.env]] | API key config | Created |

## Local Problem Set

### Completed This Session

- [X] Task 3: Daemon → Intel wiring [VERIFIED: [[sync.rs]]:62-65, 152-234]
  - Added `enrich_chains_phase()` function (~80 lines)
  - Wired into `run_sync()` between chain building and index building
  - 3 new tests, all passing

- [X] API key fix [VERIFIED: [[intel/src/index.ts]]:80-91]
  - Created `.env` file with `ANTHROPIC_API_KEY`
  - Fixed client initialization: `new Anthropic({ apiKey })`
  - Tested via CLI and curl - working

- [X] Diagnosed two-database anti-pattern [VERIFIED: context-gap-analysis]
  - Main DB: `~/.context-os/context_os_events.db`
  - Intel cache: would be `~/.context-os/intel_cache.db` (separate)
  - Can't JOIN = can't show enriched names in queries

### In Progress

None - session ending for context package.

### Jobs To Be Done (Next Session)

**PRIMARY: Unify Databases (~100 lines total)**

1. [ ] Add intel tables to main DB schema
   - File: [[core/src/intelligence/cache.rs]]
   - Change: Migration runs against main DB, not separate file
   - Lines: ~10 to change path

2. [ ] Update MetadataStore to use main DB path
   - File: [[core/src/daemon/sync.rs]]
   - Change: Line 183 `intel_cache.db` → `context_os_events.db`
   - Lines: ~5

3. [ ] Add `generated_name` to ChainData struct
   - File: [[core/src/types.rs]]
   - Change: Add `generated_name: Option<String>` to ChainData
   - Lines: ~5

4. [ ] Update query_chains to LEFT JOIN chain_metadata
   - File: [[core/src/query.rs]]
   - Change: Add LEFT JOIN, select generated_name
   - Lines: ~20

5. [ ] Test end-to-end
   - Run daemon sync with Intel service
   - Query chains, verify names appear
   - Success: `tastematter query chains` shows human-readable names

## Test State

**Rust Core:**
- Tests: 208 passing, 0 failing
- Command: `cargo test --lib`
- Last run: 2026-01-27

**TypeScript Intel:**
- Tests: 151 passing (unit tests)
- Command: `cd intel && bun test`

**Intel Service Status:**
- Running: Yes (background process)
- Health: `curl http://localhost:3002/api/intel/health` → `{"status":"ok"}`
- Chain naming: Working (tested with chains 93a22459, 42958ab7)

## Evidence: Chain Naming Working

```bash
# Test 1: Via curl
curl -X POST http://localhost:3002/api/intel/name-chain \
  -H "Content-Type: application/json" \
  -d '{"chain_id":"93a22459","files_touched":["main.rs","sync.rs"],"session_count":337}'

# Response:
{"chain_id":"93a22459","generated_name":"Cross-platform sync and indexing integration","category":"feature","confidence":0.75}

# Test 2: Via CLI
cargo run --release -- intel name-chain 42958ab7 --files "query.rs,types.rs" --session-count 149

# Response:
{"generated_name":"Query and type system refactor","category":"refactor","confidence":0.82}
```

## For Next Agent

**Context Chain:**
- Previous: [[42_2026-01-26_PRODUCTION_OBSERVABILITY_IMPLEMENTED]] - observability done
- This package: API key fixed, database anti-pattern diagnosed
- Next action: Implement database unification

**Start here:**
1. Kill any running Intel service: Check for bun processes on port 3002
2. Read [[core/src/intelligence/cache.rs]] - understand MetadataStore
3. Read [[core/src/query.rs]] lines 152-192 - understand query_chains
4. Implement the 4 changes listed in Jobs To Be Done

**Key insight:**
The fix is ~100 lines across 4 files. The main change is making MetadataStore
use the same database as QueryEngine, then adding a LEFT JOIN in query_chains.

**Do NOT:**
- Create a new separate database file
- Use a complex caching layer
- Over-engineer the solution

**Verification commands:**
```bash
# Start Intel service
cd apps/tastematter/intel && bun src/index.ts

# Run daemon sync (will call Intel service)
cd apps/tastematter/core && cargo run --release -- daemon sync

# Query chains (should show names after fix)
cargo run --release -- query chains --limit 5
```

## Session Metrics

- Duration: ~2 hours
- Files modified: 3 (sync.rs, index.ts, .env created)
- Tests added: 3 (enrichment phase tests)
- Tests total: 208 Rust + 151 TypeScript
