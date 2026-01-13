---
title: "Repository Consolidation Plan"
package_number: 26
date: 2026-01-12
status: current
previous_package: "[[25_2026-01-12_TIMELINE_BUCKETS_FIX]]"
related:
  - "[[apps/context-os/]]"
  - "[[apps/tastematter/]]"
tags:
  - context-package
  - tastematter
  - migration
  - architecture
---

# Repository Consolidation Plan

## Executive Summary

**Decision:** Consolidate fragmented `tastematter/` and `context-os/` into a single unified project.

**New Name:** Tastematter (dropping "context-os" name entirely)

**Key Principle:** Context coherence is #1 priority for agentic development.

**Trigger:** Chain linking bug investigation revealed:
1. Python daemon exists at `apps/context-os/cli/src/context_os_events/daemon/`
2. Rust query engine at `apps/context-os/core/` is READ-ONLY
3. Chain linking broken because Python indexer doesn't parse `leafUuid`
4. Context packages scattered across 4+ locations
5. Agents cannot reliably find authoritative context

---

## Problem Statement

### Current Fragmented State

```
apps/
├── tastematter/              # Svelte + Tauri frontend
│   └── specs/context_packages/  # 26 packages (00-25)
│
└── context-os/               # Backend ecosystem
    ├── core/                 # Rust query engine (READ-ONLY)
    └── cli/                  # Python daemon + indexer (WRITES)
        └── specs/
            ├── tastematter/context_packages/     # 22 more packages!
            ├── context_os_intelligence/context_packages/
            └── event_capture/context_packages/
```

### Why This Is Broken

1. **Context packages in 4 locations** - Agent starting fresh has no idea which to read
2. **Different numbering schemes** - Package 25 in one location vs Package 21 in another
3. **Specs duplicated and divergent** - Architecture decisions made in one place, not reflected elsewhere
4. **No single source of truth** - Which CLAUDE.md is authoritative?

### Chain Linking Bug Evidence

```
Query: tastematter query chains --limit 20

Result:
- Chain fa6b4bf6: 149 sessions
- All sessions have IDENTICAL timestamps (batch import)
- All sessions show file_count: 0

Root cause: Python indexer at chain_graph.py doesn't parse leafUuid
```

---

## Target Architecture

```
apps/tastematter/                    # THE unified project
├── core/                            # Rust query engine
├── indexer/                         # Rust indexer (NEW - replaces Python)
├── frontend/                        # Svelte + Tauri UI
├── specs/
│   ├── canonical/                   # Blessed architecture decisions
│   ├── implementation/              # Phase-based details
│   └── context_packages/            # UNIFIED package chain
└── CLAUDE.md                        # Single navigation guide
```

---

## Migration Phases

| Phase | Action | Risk | Time |
|-------|--------|------|------|
| 1 | Audit all context packages | None | 30 min |
| 2 | Scaffold new directory structure | Low | 10 min |
| 3 | Merge context packages (chronological) | Medium | 1-2 hr |
| 4 | Move canonical specs | Low | 15 min |
| 5 | Move core (Rust) | Low | 10 min |
| 6 | Move frontend | Low | 15 min |
| 7 | Update references | Medium | 30 min |
| 8 | Cleanup old directories | High | 10 min |

**Total:** 3-4 hours

---

## Context Package Merge Strategy

### The Challenge

Multiple package chains with conflicting numbering:
- `tastematter/specs/context_packages/` - Packages 00-25
- `context-os/specs/tastematter/context_packages/` - Packages 00-21
- Other locations with their own numbering

### The Solution

1. **Extract date from each package filename** (YYYY-MM-DD)
2. **Sort ALL packages chronologically**
3. **Renumber sequentially** starting from 00
4. **Add `migrated_from:` field** for traceability

### Example Transformation

```yaml
# BEFORE
---
package_number: 25
previous_package: "[[24_...]]"
---

# AFTER
---
package_number: 47
previous_package: "[[46_...]]"
migrated_from: "apps/tastematter/specs/context_packages/25_..."
---
```

---

## Key Decisions

### Decision 1: Port Indexer to Rust

**Why:** Single language eliminates Python/Rust sync issues
**Impact:** No more daemon as separate process; indexer becomes part of Rust codebase
**Timeline:** After migration complete

### Decision 2: Keep Database

**Why:** Persistence and fast queries outweigh complexity
**Alternative considered:** In-memory JSONL parsing (simpler but slower startup)

### Decision 3: Unified Context Packages

**Why:** Agentic development requires single source of truth
**Method:** Chronological merge with traceability
**Risk mitigation:** Add `migrated_from:` field, document uncertainties in README

---

## Verification Checkpoints

### After Phase 3 (Context Packages)
- [ ] README.md lists all packages chronologically
- [ ] Each package has valid `previous_package` link
- [ ] No broken wiki-links

### After Phase 6 (Code Moves)
- [ ] `cargo build` succeeds in core/
- [ ] `pnpm dev` works in frontend/
- [ ] Tests pass

### Before Phase 8 (Cleanup)
- [ ] All verifications above pass
- [ ] No `grep -r "context-os"` hits in active code
- [ ] Git status shows expected changes

---

## Files to Investigate (Phase 1)

| File | Purpose |
|------|---------|
| `apps/context-os/cli/src/context_os_events/index/chain_graph.py` | Chain linking logic (BROKEN) |
| `apps/context-os/specs/*/context_packages/*.md` | Packages to merge |
| `apps/tastematter/specs/context_packages/*.md` | Packages to merge |

---

## Success Criteria

- [ ] Single `apps/tastematter/` directory
- [ ] Unified context package chain (chronological, navigable)
- [ ] `cargo build` and `pnpm dev` both work
- [ ] CLAUDE.md enables agents to navigate effectively
- [ ] No references to "context-os" in active code

---

## For Next Agent

**Start here:**
1. Read this package (you're doing it now)
2. Run Phase 1: Audit all context_packages directories
3. Create sorted list of all packages by date
4. Begin scaffolding new structure

**Do NOT:**
- Delete any files until Phase 8
- Skip verification checkpoints
- Assume package ordering without checking dates

**Key insight:** This migration is about CONTEXT COHERENCE, not just file organization. The goal is to enable future agents to find authoritative information quickly.

---

## Evidence & Attribution

[VERIFIED: apps/context-os/cli/src/context_os_events/daemon/ EXISTS]
[VERIFIED: Explore agent found daemon at this location]
[VERIFIED: Chain fa6b4bf6 has 149 sessions with identical timestamps]
[INFERRED: Python indexer doesn't parse leafUuid based on observed behavior]
[DECISION: User chose "Port indexer to Rust" and "Keep database"]
