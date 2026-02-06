---
title: "Tastematter Daemon Context Package 51"
package_number: 51
date: 2026-02-03
status: current
previous_package: "[[50_2026-02-03_RELEASE_INFRASTRUCTURE_COMPLETE]]"
related:
  - "[[specs/implementation/phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md]]"
  - "[[specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md]]"
  - "[[.claude/skills/context-query/references/heat-metrics-model.md]]"
  - "[[_system/meta/context_os_architecture_2026-02-03.html]]"
tags:
  - context-package
  - tastematter
  - system-meta-review
  - cli-usability
  - heat-metrics
---

# Tastematter - Context Package 51

## Executive Summary

**SYSTEM META-REVIEW COMPLETE. CLI USABILITY AUDIT DONE. HEAT METRICS SPEC WRITTEN.** Ran comprehensive meta-review of the GTM Operating System (41 skills, 236 context packages, 5,246 lines of state). Discovered significant gap between declared system architecture and actual usage via heat metrics analysis. Created heat command spec and skill reference. Audited CLI usability - found data quality bugs (timestamps, duration) but architecture is sound for agent-as-user model.

## What Was Accomplished This Session

### 1. System Meta-Review

Generated comprehensive architecture map at `_system/meta/context_os_architecture_2026-02-03.html`:

| Metric | Count |
|--------|-------|
| Skills | 41 (40 active + 1 deprecated) |
| State Files | 9 operational (5,246 lines) |
| Context Packages | 236 across 16 locations |
| Knowledge Nodes | 177 |
| Applications | 17 active |

**Key Finding:** Massive gap between declared vs actual system usage.

[VERIFIED: Exploration agent inventory + tastematter queries]

### 2. Heat Metrics Model

Developed heat metrics to quantify file usage patterns:

**Core Metrics:**
- **RCR (Recency Concentration Ratio)** = 7d_accesses / 30d_accesses
  - RCR > 0.7 = HOT (active work)
  - RCR 0.4-0.7 = WARM (steady use)
  - RCR < 0.25 = COLD (legacy/reference)

- **Access Velocity** = accesses / days_since_first_access
  - Normalizes for file age

**Files Created:**
1. `apps/tastematter/specs/implementation/phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md`
2. `.claude/skills/context-query/references/heat-metrics-model.md`

[VERIFIED: Files created this session]

### 3. CLI Usability Audit

Tested all 9 query commands for agent-as-user model:

| Command | Data Quality | Useful for Agents? |
|---------|--------------|-------------------|
| `query flex` | ⚠️ | ✅ Yes |
| `query chains` | ✅ | ✅ Yes |
| `query co-access` | ✅ | ✅ Yes |
| `query sessions` | ⚠️ | ⚠️ Partial |
| `query timeline` | ⚠️ | ⚠️ Partial |
| `query search` | ✅ | ✅ Yes |
| `query file` | ⚠️ | ✅ Yes |
| `query verify` | ✅ | ✅ Yes |
| `query receipts` | ✅ | ✅ Yes |

**Revised Assessment:** 7 of 9 commands useful for agents. Main issues are **data quality bugs**, not output design.

[VERIFIED: Live CLI testing this session]

### 4. Data Quality Bugs Identified

**P0 Bugs Found:**

1. **Timestamp Bug:** All `last_access` values return identical timestamps (~2026-02-04T01:29:55.xxx)
   - Blocks: Heat metrics, recency analysis, "when did I work on X"
   - Location: Likely in `parse-sessions` or `storage.rs`

2. **Session Duration Bug:** All sessions show `duration_seconds: 0`
   - Blocks: Identifying long vs short sessions
   - Location: Session boundary detection

3. **Count Verification Needed:** `access_count == session_count` for all files
   - May be correct (one access per session) or may be a bug
   - Needs verification against raw JSONL

[VERIFIED: `tastematter query flex/sessions/file` output inspection]

### 5. Architecture Layering Clarified

```
Layer 3: SYNTHESIS (tastematter context)
    │ consumes
    ▼
Layer 2: DERIVED METRICS (tastematter heat)
    │ composes from
    ▼
Layer 1: PRIMITIVES (flex, co-access, chains, file, search)
    │
    ▼
Layer 0: DATA QUALITY (timestamps, counts, durations)
```

**Key Insight:** Heat can be built with approximations even with timestamp bugs. Context restoration depends on accurate heat.

### 6. Context Restoration API Spec Reviewed

`specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md` defines:
- `tastematter context "<query>"` command
- 9-section response schema
- 3 depth levels (quick/medium/deep)
- Integration with intel service

**Implementation Phases:**
| Phase | Name | Status |
|-------|------|--------|
| 1 | Deterministic Foundation | ⬜ READY (4-6h) |
| 2 | Intelligence Integration | ⬜ BLOCKED on Phase 1 |
| 3 | Depth + Caching | ⬜ BLOCKED on Phase 2 |

## Global Context

### The Vision

"Context intelligence tool where it knows more about the user's context than the user themselves"

**Achieved by:** CLI (raw data) + Skill (interpretation) + Grep/Glob (content) = Context Intelligence

### Architecture Decision: Agent-as-User

The CLI is NOT for human end users. It's for:
- Future Claude Code sessions
- Using context-query skill for interpretation
- Combined with grep/glob/read for content

**Implication:** Raw JSON output, PMI scores, UUIDs are all FINE. The skill provides the interpretation layer.

## Jobs To Be Done (Next Session)

### Priority 1: Data Quality Fixes

1. **Investigate Timestamp Bug**
   - Check `parse-sessions` command - is it preserving timestamps from JSONL?
   - Check `storage.rs` - INSERT statements using correct timestamps?
   - Success criteria: `last_access` values vary across files

2. **Fix Session Duration**
   - Check session boundary detection in parser
   - Success criteria: `duration_seconds > 0` for real sessions

3. **Verify Count Accuracy**
   - Compare CLI counts to raw JSONL event counts
   - Success criteria: Understand if access_count == session_count is correct

### Priority 2: Heat Command Implementation

Per spec at `phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md`:

```bash
tastematter heat [OPTIONS]
  -t, --time <TIME>      Base time window (default: 30d)
  -f, --files <FILES>    Filter by pattern
  -l, --limit <LIMIT>    Max results (default: 50)
  --format <FORMAT>      table | json | csv
```

**Implementation Approach:**
1. Run two flex queries (7d and 30d)
2. Join by file_path
3. Compute RCR, velocity, heat_score
4. Classify and output

### Priority 3: Context Restoration (After P1+P2)

Begin `tastematter context` implementation once:
- Timestamps are accurate
- Heat command provides classification

## Execution Strategy Options

### Option A: Sequential
1. Fix bugs → 2. Build heat → 3. Build context

### Option B: Parallel Streams (Recommended)
```
Stream A: Bug Fixes (Layer 0)
├── Timestamp preservation
├── Session duration
└── Count verification

Stream B: Heat Command (Layer 2)
├── Can work with approximate data
├── Spec already complete
└── Standalone value
```

### Rationale for Parallel
- Heat has standalone value (system health, drift detection)
- Heat can use approximations (oldest access in window for velocity)
- Bug fixes don't block heat spec implementation
- Convergence point: Context restoration uses both

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[specs/implementation/phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md]] | Heat command spec | Created |
| [[.claude/skills/context-query/references/heat-metrics-model.md]] | Manual heat calculation guide | Created |
| [[specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md]] | Context restoration spec | Reviewed |
| [[_system/meta/context_os_architecture_2026-02-03.html]] | System architecture map | Generated |
| [[core/src/storage.rs]] | DB storage layer | Bug investigation target |
| [[core/src/main.rs]] | CLI entry point | Bug investigation target |

## Test State

**Rust Core:** 269 tests (259 lib + 10 integration)
**Python:** 495 tests
**TypeScript Intel:** 181 tests (173 passing, 8 failing)

**Test Commands:**
```bash
# Rust tests
cd apps/tastematter/core && cargo test

# Verify CLI working
tastematter query flex --time 7d --limit 5 --format json
tastematter query chains --limit 5 --format json
```

## For Next Agent

**Context Chain:**
- Previous: [[50_2026-02-03_RELEASE_INFRASTRUCTURE_COMPLETE]] (release workflow done)
- This package: Meta-review + CLI audit + heat spec
- Next: Bug investigation OR heat implementation

**Start Here:**

1. **If fixing bugs:** Read `core/src/storage.rs` and search for INSERT statements, check timestamp handling
2. **If building heat:** Read `phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md`, implement per Option A (build on flex query)
3. **For either:** Run `/context-gap-analysis` first to verify assumptions

**Key Files to Read:**
- [[01_HEAT_COMMAND_SPEC.md]] - Full spec for heat command
- [[heat-metrics-model.md]] - How to calculate manually (until CLI exists)
- [[12_CONTEXT_RESTORATION_API_SPEC.md]] - The bigger vision

**Do NOT:**
- Assume timestamps are working (they're not - investigate first)
- Build context restoration before heat (depends on it)
- Rewrite CLI output format (it's correct for agent-as-user model)

**Key Insight:**
The CLI architecture is sound. The skill provides interpretation. The bugs are in data quality (Layer 0), not in command design (Layer 1). Fix the foundation, then build up.

[VERIFIED: CLI testing + code inspection this session]
