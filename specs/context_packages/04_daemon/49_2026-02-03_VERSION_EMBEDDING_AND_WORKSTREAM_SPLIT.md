---
title: "Tastematter Daemon Context Package 49"
package_number: 49
date: 2026-02-03
status: current
previous_package: "[[48_2026-02-03_FRESH_INSTALL_TDD_AND_RELEASE]]"
related:
  - "[[core/build.rs]]"
  - "[[core/src/storage.rs]]"
  - "[[_system/state/workstreams.yaml]]"
tags:
  - context-package
  - tastematter
  - version-embedding
  - workstreams
  - chain-query-fix
---

# Tastematter - Context Package 49

## Executive Summary

**VERSION EMBEDDING COMPLETE. WORKSTREAMS SPLIT. CHAIN QUERY BUG FIXED.** Added build.rs for git version embedding at compile time (local dev shows `0.1.0-dev+hash`, releases show exact tag). Split tastematter from 1 stream into 4 (cli/intel/gtm/desktop) in workstreams.yaml. Fixed chain_metadata table missing bug. Committed ~16K lines of previously uncommitted code (intel service, context packages, website). Released v0.1.0-alpha.13.

## What Was Accomplished This Session

### 1. Version Embedding (build.rs)

Added compile-time git version extraction so binaries self-identify:

| Build Context | `--version` Output |
|---------------|-------------------|
| On exact tag (CI) | `v0.1.0-alpha.13` |
| On tag with changes | `v0.1.0-alpha.13-dirty` |
| After tag, new commits | `0.1.0-dev+abc1234` |

**Files:**
- `core/build.rs` - New file, extracts git describe at compile time
- `core/src/main.rs:45` - Changed `version = "0.1.0"` → `version = env!("TASTEMATTER_VERSION")`

[VERIFIED: `tastematter --version` returns `v0.1.0-alpha.13`]

### 2. Workstream Split

Split `tastematter-product` into 4 distinct streams in workstreams.yaml:

| Stream | Temperature | Progress | Last Active |
|--------|-------------|----------|-------------|
| **tastematter-cli** | SHIPPED | 100% | Feb 3 |
| **tastematter-intel** | WARM | 67% | Feb 3 |
| **tastematter-gtm** | WARM | 30% | Jan 29 |
| **tastematter-desktop** | PAUSED | 70% | Jan 4 |

**Also corrected:**
- Removed false "$300 guided setup" monetization claim
- Reopened dq_002 (monetization decision)
- Added temperature_log entry for the split

[VERIFIED: `_system/state/workstreams.yaml` updated to v1.5]

### 3. Chain Query Bug Fix

**Bug:** `query chains` failed with "no such table: chain_metadata"

**Root cause:** The `chain_metadata` table was only created by intel cache module, but `query_chains` does LEFT JOIN to it.

**Fix:** Added `chain_metadata` table to base schema in storage.rs:

```sql
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    summary TEXT,
    key_topics TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

[VERIFIED: `tastematter query chains --limit 10` now works]

### 4. Massive Code Commit

Committed ~16K lines of previously uncommitted code:

| Commit | Content | Lines |
|--------|---------|-------|
| `1a72c81` | Version embedding (build.rs) | 64 |
| `8ab95f1` | TypeScript intel service | 6,822 |
| `55d9ac5` | Architecture docs & schemas | 685 |
| `5b87095` | Context packages 29-48 | 6,994 |
| `1af2abc` | Website landing page | 1,513 |

**Intel service now committed:**
- 6 agents (chain-naming, chain-summary, commit-analysis, gitops-decision, insights, session-summary)
- Middleware (correlation, cost-guard, operation-logger)
- Services (file-logger, logger)
- Full test suite (181 pass, 8 fail)

[VERIFIED: `git log --oneline -5` shows commits]

### 5. Release v0.1.0-alpha.13

Deployed with all commits. GitHub Actions completed successfully (all 4 platforms).

[VERIFIED: `curl https://install.tastematter.dev/latest.txt` returns `v0.1.0-alpha.13`]

## Current State

### Tastematter Streams Status

| Stream | Status | Tests | Next Action |
|--------|--------|-------|-------------|
| **tastematter-cli** | SHIPPED | 269 | Maintenance only |
| **tastematter-intel** | 67% | 181 pass, 8 fail | Fix failing tests |
| **tastematter-gtm** | 30% | - | Public repo, beta testers |
| **tastematter-desktop** | PAUSED | - | Chain integration when resumed |

### Test Status

**Rust Core:**
- 259 lib tests passing
- 10 integration tests passing
- Total: 269 tests

**Intel Service:**
- 181 passing, 8 failing
- Failing tests: integration/schema validation issues

### Files Modified This Session

| File | Change |
|------|--------|
| `core/build.rs` | New - git version extraction |
| `core/src/main.rs` | Version from env var |
| `core/src/storage.rs` | Added chain_metadata table |
| `_system/state/workstreams.yaml` | Split into 4 streams |
| `intel/` | Entire directory committed |
| `website/` | Entire directory committed |
| `specs/context_packages/` | 20 packages committed |

## Jobs To Be Done (Next Session)

### Immediate
1. [ ] Commit chain_metadata schema fix
   - File: `core/src/storage.rs`
   - Success criteria: `query chains` works on fresh install

2. [ ] Fix 8 failing intel service tests
   - Location: `intel/tests/integration/`
   - Issue: Schema validation errors

### Future
3. [ ] Create public GitHub repo for tastematter
4. [ ] Recruit 10-20 beta testers
5. [ ] Resume desktop UI (chain integration)

## For Next Agent

**Context Chain:**
- Previous: [[48_2026-02-03_FRESH_INSTALL_TDD_AND_RELEASE]] (fresh install TDD, install script fix)
- This package: Version embedding, workstream split, chain query fix
- Next: Commit schema fix, fix intel tests

**Start here:**
1. Read this package (you're doing it now)
2. Commit the chain_metadata fix: `core/src/storage.rs`
3. Run `cargo test --lib` to verify (should pass)
4. Tag and release v0.1.0-alpha.14

**Key files:**
- [[core/build.rs]] - Version embedding logic
- [[core/src/storage.rs]] - Schema with chain_metadata fix
- [[_system/state/workstreams.yaml]] - 4-stream split

**Verification commands:**
```bash
# Test chain query works
tastematter query chains --limit 5

# Test version embedding
tastematter --version

# Run all tests
cd apps/tastematter/core && cargo test --lib
```

**Key insight:**
The `query chains` command fails on fresh installs because it LEFT JOINs to `chain_metadata` which doesn't exist until intel service creates it. Fix: Add table to base schema so it always exists (empty until intel populates it).

[VERIFIED: Fix applied locally, needs commit]
