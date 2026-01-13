---
title: "Migration Execution Guide - Detailed Handoff"
package_number: 27
date: 2026-01-12
status: current
previous_package: "[[26_2026-01-12_REPOSITORY_CONSOLIDATION_PLAN]]"
related:
  - "[[apps/context-os/]]"
  - "[[apps/tastematter/]]"
  - "[[~/.claude/plans/synchronous-coalescing-harbor.md]]"
tags:
  - context-package
  - tastematter
  - migration
  - execution-guide
  - handoff
---

# Migration Execution Guide - Detailed Handoff for Next Agent

## Executive Summary

**Purpose:** This package provides COMPLETE execution instructions for consolidating the fragmented Tastematter repository. The next agent should be able to execute this migration without asking questions.

**Key Insight from User:** Do NOT do a flat chronological merge. First subdivide packages by APP AREA, understand the logical groupings, THEN consider how to merge.

**Total Packages to Migrate:** 61 (across 4 source directories)
**Target:** Single `apps/tastematter/` with unified context

---

## CRITICAL: Information Architecture First

### The User's Guidance

> "First want them subdivided by the part of the app they were related to and only after we have solid grouping by chronological/app area then can we even consider all in total merge - that is probably not the best information architecture/hierarchy"

### Why This Matters

A flat chronological merge (00-60) would:
- Lose the logical narrative of each component's evolution
- Make it hard to understand "how did the query engine evolve?"
- Mix unrelated concerns (UI work interleaved with backend work)

The RIGHT approach:
1. Understand what each package CHAIN represents
2. Group by component/domain
3. Preserve internal chronology within groups
4. Create a hierarchy that aids understanding

---

## Package Chain Analysis (By App Area)

### Chain 1: Query Engine Foundation (context_os_intelligence)
**Location:** `apps/context-os/specs/context_os_intelligence/context_packages/`
**Count:** 11 packages (00-10)
**Date Range:** 2025-12-16 to 2025-12-24
**Theme:** Building the Python query engine, indexes, chain detection

| # | Date | Title | Summary |
|---|------|-------|---------|
| 00 | 2025-12-16 | PHASE6_COMPLETE | Initial query infrastructure |
| 01 | 2025-12-16 | AGENT_HANDOFF | Agent coordination setup |
| 02 | 2025-12-18 | PRACTICAL_TESTING | Testing methodology |
| 03 | 2025-12-21 | IMPLEMENTATION_A | First implementation phase |
| 04 | 2025-12-21 | IMPLEMENTATION_B | Continued implementation |
| 05 | 2025-12-21 | IMPLEMENTATION_C | Further implementation |
| 06 | 2025-12-21 | IMPLEMENTATION_D | Final implementation phase |
| 07 | 2025-12-21 | SKILL_REWRITE | Skills system rewrite |
| 08 | 2025-12-22 | SKILL_VALIDATION | Validating skills |
| 09 | 2025-12-22 | HANDOFF | Handoff documentation |
| 10 | 2025-12-24 | FORENSIC_ANALYSIS | Deep analysis of system |

**Narrative:** This chain tells the story of building the Python-based query engine that became the foundation for Tastematter's data layer.

---

### Chain 2: Tastematter UI Foundation (context-os/tastematter)
**Location:** `apps/context-os/specs/tastematter/context_packages/`
**Count:** 22 packages (00-21)
**Date Range:** 2025-12-28 to 2026-01-04
**Theme:** Building the Svelte/Tauri frontend, TDD, design system

| # | Date | Title | Summary |
|---|------|-------|---------|
| 00 | 2025-12-28 | PHASE0_COMPLETE | Initial UI scaffolding |
| 01 | 2025-12-28 | PHASE1_COMPLETE | Core UI components |
| 02 | 2025-12-29 | PHASE2_COMPLETE | Data integration |
| 03 | 2025-12-29 | PHASE3_COMPLETE | Views implementation |
| 04 | 2025-12-30 | PHASE4_SPEC_CREATED | Phase 4 planning |
| 05 | 2025-12-30 | PHASE5_SPEC_CREATED | Phase 5 planning |
| 06 | 2025-12-30 | SHARED_ARCHITECTURE | Architecture decisions |
| 07 | 2025-12-30 | TDD_PLAN_READY | TDD methodology setup |
| 08 | 2025-12-30 | PHASE4_TDD_RED | TDD Red phase |
| 09 | 2025-12-30 | PHASE4_TDD_GREEN | TDD Green phase |
| 10 | 2025-12-30 | PHASE4_INTEGRATION_DARKMODE | Dark mode integration |
| 11 | 2025-12-30 | VISUAL_DESIGN_AUDIT | Design audit |
| 12 | 2025-12-30 | DESIGN_SYSTEM_CLEANUP | Design system work |
| 13 | 2025-12-30 | DESIGN_SYSTEM_COMPLETE | Design system done |
| 14 | 2025-12-31 | PHASE5_TDD_IN_PROGRESS | Phase 5 TDD work |
| 15 | 2026-01-02 | PHASE5_CYCLES_1_2_COMPLETE | TDD cycles |
| 16 | 2026-01-02 | PHASE5_CYCLES_3_4_COMPLETE | More TDD cycles |
| 17 | 2026-01-02 | PHASE5_COMPLETE_LOGGING_SPEC | Logging spec |
| 18 | 2026-01-02 | AGENT1_EVENT_LOGGER_COMPLETE | Event logger |
| 19 | 2026-01-02 | AGENT2_STATE_SNAPSHOTS_COMPLETE | State snapshots |
| 20 | 2026-01-02 | AGENT3_AGENT_CONTEXT_COMMAND_COMPLETE | Agent context |
| 21 | 2026-01-04 | CHAIN_FIXES_TASTEMATTER_ENHANCEMENT | Chain bug fixes |

**Narrative:** This chain documents building the Tauri desktop app from scratch using TDD, including the design system and multi-agent implementation.

---

### Chain 3: Tastematter Current (tastematter/specs)
**Location:** `apps/tastematter/specs/context_packages/`
**Count:** 27 packages (00-26)
**Date Range:** 2026-01-05 to 2026-01-12
**Theme:** Performance optimization, Rust port, architecture refinement, bug fixes

| # | Date | Title | Summary |
|---|------|-------|---------|
| 00 | 2026-01-05 | UNIFIED_DATA_ARCHITECTURE | Data architecture |
| 01 | 2026-01-05 | LOGGING_SERVICE | Logging implementation |
| 02 | 2026-01-05 | PERF_OPTIMIZATION_HANDOFF | Performance work |
| 03 | 2026-01-06 | PHASE2_IN_PROGRESS | Ongoing work |
| 04 | 2026-01-06 | PERF_OPTIMIZATION_COMPLETE | Performance done |
| 05 | 2026-01-07 | VISION_FOUNDATION | Vision documents |
| 06 | 2026-01-07 | CANONICAL_ENRICHMENT | Canonical docs |
| 07 | 2026-01-07 | ARCHITECTURE_SKILL_CREATION | Architecture skill |
| 08 | 2026-01-07 | SKILL_COMPLETE_PHASE0_READY | Skills ready |
| 09 | 2026-01-08 | UNIFIED_CORE_ARCHITECTURE | Core architecture |
| 10 | 2026-01-08 | IMPLEMENTATION_SPECS_COMPLETE | Impl specs |
| 11 | 2026-01-08 | DIRECTORY_REORG_COMPLETE | Dir reorganization |
| 12 | 2026-01-08 | PHASE1_CORE_COMPLETE | Rust core complete |
| 13 | 2026-01-09 | PHASE2_DATA_SOURCE_FIX | Data source fix |
| 14 | 2026-01-09 | PHASE2B_TAURI_ALIGNMENT | Tauri alignment |
| 15 | 2026-01-09 | PHASE0_COMPLETE | Phase 0 done |
| 16 | 2026-01-09 | ARCHITECTURE_DOC_UPDATE | Doc updates |
| 17 | 2026-01-09 | TRANSPORT_ARCHITECTURE_SPEC | Transport spec |
| 18 | 2026-01-09 | HTTP_SERVER_COMPLETE | HTTP server |
| 19 | 2026-01-09 | TRANSPORT_ABSTRACTION_IN_PROGRESS | Transport work |
| 20 | 2026-01-10 | QUICK_WINS_COMPLETE | Quick wins |
| 21 | 2026-01-10 | INTELLIGENCE_LAYER_SPEC | Intelligence layer |
| 22 | 2026-01-11 | CHAIN_LINKAGE_BUG_RCA | Bug RCA |
| 23 | 2026-01-11 | BUG_FIXES_COMPLETE | Bug fixes |
| 24 | 2026-01-12 | DATABASE_ARCHITECTURE_FIX | DB architecture |
| 25 | 2026-01-12 | TIMELINE_BUCKETS_FIX | Timeline fix |
| 26 | 2026-01-12 | REPOSITORY_CONSOLIDATION_PLAN | This migration |

**Narrative:** This chain documents the Rust port, performance optimization, HTTP server addition, bug fixes, and architecture refinement.

---

### Chain 4: Event Capture/Daemon (event_capture)
**Location:** `apps/context-os/specs/event_capture/context_packages/`
**Count:** 1 package (00)
**Date Range:** 2026-01-12
**Theme:** Chain linking bug investigation, daemon architecture

| # | Date | Title | Summary |
|---|------|-------|---------|
| 00 | 2026-01-12 | CHAIN_LINKING_BUG_INVESTIGATION | Chain linking bug |

**Narrative:** Single package documenting the chain linking bug discovery (broken leafUuid parsing).

---

## Recommended Information Architecture

### Option A: Preserve Chain Identity (RECOMMENDED)

Instead of a flat 00-60 merge, create **subdirectories by chain**:

```
apps/tastematter/specs/context_packages/
├── README.md                           # Master index
├── 01_query_engine/                    # Chain 1: context_os_intelligence
│   ├── README.md
│   ├── 00_2025-12-16_PHASE6_COMPLETE.md
│   ├── ...
│   └── 10_2025-12-24_FORENSIC_ANALYSIS.md
│
├── 02_ui_foundation/                   # Chain 2: tastematter UI
│   ├── README.md
│   ├── 00_2025-12-28_PHASE0_COMPLETE.md
│   ├── ...
│   └── 21_2026-01-04_CHAIN_FIXES.md
│
├── 03_current/                         # Chain 3: Current work
│   ├── README.md
│   ├── 00_2026-01-05_UNIFIED_DATA.md
│   ├── ...
│   └── 27_2026-01-12_MIGRATION_GUIDE.md
│
└── 04_daemon/                          # Chain 4: Event capture
    ├── README.md
    └── 00_2026-01-12_CHAIN_LINKING.md
```

**Pros:**
- Preserves narrative of each component
- Easy to understand "how did X evolve?"
- Internal numbering stays valid (no renumbering!)
- Clear hierarchy

**Cons:**
- Multiple README files to maintain
- Cross-chain references need full paths

---

### Option B: Flat Merge with Prefixes

Single directory, but prefix filenames with chain identifier:

```
apps/tastematter/specs/context_packages/
├── README.md
├── QE_00_2025-12-16_PHASE6_COMPLETE.md      # Query Engine
├── QE_01_2025-12-16_AGENT_HANDOFF.md
├── ...
├── UI_00_2025-12-28_PHASE0_COMPLETE.md      # UI Foundation
├── UI_01_2025-12-28_PHASE1_COMPLETE.md
├── ...
├── CUR_00_2026-01-05_UNIFIED_DATA.md        # Current
├── ...
└── DC_00_2026-01-12_CHAIN_LINKING.md        # Daemon/Capture
```

**Pros:**
- Single directory (simpler)
- Clear prefixes show origin
- No nested READMEs

**Cons:**
- Harder to browse by date
- Prefixes make filenames long

---

### Option C: Flat Chronological (NOT RECOMMENDED)

The original plan of 00-60 flat numbering:

```
apps/tastematter/specs/context_packages/
├── 00_2025-12-16_PHASE6_COMPLETE.md         # Was QE 00
├── 01_2025-12-16_AGENT_HANDOFF.md           # Was QE 01
├── ...
├── 33_2026-01-05_UNIFIED_DATA.md            # Was CUR 00
├── ...
└── 60_2026-01-12_MIGRATION_GUIDE.md         # Was CUR 27
```

**Cons:**
- Loses narrative structure
- Interleaves unrelated work
- Hard to follow component evolution
- Requires renumbering all `previous_package` links

---

## Execution Plan (For Next Agent)

### Phase 0: Pre-Flight Checks

```bash
# 1. Verify git status is clean
cd "C:/Users/dietl/VSCode Projects/taste_systems/gtm_operating_system"
git status

# 2. Verify no processes using the directories
# (No context-os daemon should be running)

# 3. Create backup branch
git checkout -b backup/pre-migration-2026-01-12
git checkout main
```

### Phase 1: Create Target Structure (SCAFFOLD)

```bash
# Create the new directory hierarchy
mkdir -p apps/tastematter/specs/context_packages_new/01_query_engine
mkdir -p apps/tastematter/specs/context_packages_new/02_ui_foundation
mkdir -p apps/tastematter/specs/context_packages_new/03_current
mkdir -p apps/tastematter/specs/context_packages_new/04_daemon
mkdir -p apps/tastematter/core
mkdir -p apps/tastematter/indexer/src
mkdir -p apps/tastematter/frontend
mkdir -p apps/tastematter/data

# Verify
ls -la apps/tastematter/specs/context_packages_new/
```

### Phase 2: Copy Context Packages (By Chain)

**RULE:** Copy, don't move. Only delete originals AFTER verification.

#### Copy Chain 1 (Query Engine)

```bash
# Copy all 11 packages
cp apps/context-os/specs/context_os_intelligence/context_packages/*.md \
   apps/tastematter/specs/context_packages_new/01_query_engine/

# Verify count
ls apps/tastematter/specs/context_packages_new/01_query_engine/*.md | wc -l
# Should be 11 (plus README if exists)
```

#### Copy Chain 2 (UI Foundation)

```bash
# Copy all 22 packages
cp apps/context-os/specs/tastematter/context_packages/*.md \
   apps/tastematter/specs/context_packages_new/02_ui_foundation/

# Verify count
ls apps/tastematter/specs/context_packages_new/02_ui_foundation/*.md | wc -l
# Should be 22 (plus README if exists)
```

#### Copy Chain 3 (Current)

```bash
# Copy all 27 packages (including this one!)
cp apps/tastematter/specs/context_packages/*.md \
   apps/tastematter/specs/context_packages_new/03_current/

# Verify count
ls apps/tastematter/specs/context_packages_new/03_current/*.md | wc -l
# Should be 28 (including README)
```

#### Copy Chain 4 (Daemon)

```bash
# Copy 1 package
cp apps/context-os/specs/event_capture/context_packages/*.md \
   apps/tastematter/specs/context_packages_new/04_daemon/

# Verify count
ls apps/tastematter/specs/context_packages_new/04_daemon/*.md | wc -l
# Should be 1
```

### Phase 3: Update Frontmatter

For each package, add `migrated_from:` field to frontmatter:

```yaml
---
title: "Original Title"
package_number: NN
date: YYYY-MM-DD
status: current
previous_package: "[[NN-1_...]]"
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/NN_..."
related:
  - ...
tags:
  - context-package
  - tastematter
---
```

**Script approach (if automating):**
```python
# Pseudo-code for adding migrated_from
for each package:
    read file
    parse YAML frontmatter
    add migrated_from: original_path
    write file
```

### Phase 4: Create Chain READMEs

Create README.md for each subdirectory:

#### 01_query_engine/README.md

```markdown
# Query Engine Foundation (Chain 1)

Context packages documenting the Python query engine development.

**Date Range:** 2025-12-16 to 2025-12-24
**Package Count:** 11

## Timeline

| # | Date | Title |
|---|------|-------|
| 00 | 2025-12-16 | PHASE6_COMPLETE |
| ... | ... | ... |
| 10 | 2025-12-24 | FORENSIC_ANALYSIS |

## Narrative

This chain documents building the Python-based query engine including:
- Index structures (inverted, temporal, chain_graph)
- Query execution engine
- 375 tests passing
- Skills system integration

## Next Chain

Work continued in [[02_ui_foundation/]] starting 2025-12-28.
```

(Create similar for each chain)

### Phase 5: Create Master README

Create `apps/tastematter/specs/context_packages_new/README.md`:

```markdown
# Tastematter Context Packages

Unified context package archive for the Tastematter project.

## Package Chains

| Chain | Directory | Count | Date Range | Theme |
|-------|-----------|-------|------------|-------|
| 1 | [[01_query_engine/]] | 11 | 2025-12-16 to 2025-12-24 | Python query engine |
| 2 | [[02_ui_foundation/]] | 22 | 2025-12-28 to 2026-01-04 | Svelte/Tauri UI |
| 3 | [[03_current/]] | 28 | 2026-01-05 to 2026-01-12 | Rust port, fixes |
| 4 | [[04_daemon/]] | 1 | 2026-01-12 | Chain linking |

**Total:** 62 packages (61 + this migration guide)

## Migration History

Packages consolidated on 2026-01-12 from:
- `apps/context-os/specs/context_os_intelligence/context_packages/`
- `apps/context-os/specs/tastematter/context_packages/`
- `apps/tastematter/specs/context_packages/`
- `apps/context-os/specs/event_capture/context_packages/`

Original locations preserved in `migrated_from:` frontmatter field.

## How to Navigate

1. **To understand component evolution:** Read chain README, then packages in order
2. **To find specific work:** Check chain themes, then search within
3. **To continue current work:** Go to [[03_current/]], read latest package
4. **To load context:** Run `/context-foundation` (will read master chain)

## Chronological Cross-Reference

For a pure date-ordered view, see: [[CHRONOLOGICAL_INDEX.md]]
```

### Phase 6: Move Code Directories

```bash
# Move Rust core
cp -r apps/context-os/core/* apps/tastematter/core/

# Verify build
cd apps/tastematter/core
cargo build --release

# Move frontend (from current tastematter root)
# This is trickier - need to restructure
cp -r apps/tastematter/src apps/tastematter/frontend/
cp -r apps/tastematter/src-tauri apps/tastematter/frontend/
cp apps/tastematter/package.json apps/tastematter/frontend/
cp apps/tastematter/vite.config.ts apps/tastematter/frontend/
cp apps/tastematter/tsconfig.json apps/tastematter/frontend/
# ... (all frontend files)

# Verify frontend
cd apps/tastematter/frontend
pnpm install
pnpm dev
```

### Phase 7: Swap Context Packages Directory

```bash
# Only after ALL verification passes
mv apps/tastematter/specs/context_packages apps/tastematter/specs/context_packages_old
mv apps/tastematter/specs/context_packages_new apps/tastematter/specs/context_packages

# Verify
ls apps/tastematter/specs/context_packages/
# Should show: 01_query_engine/ 02_ui_foundation/ 03_current/ 04_daemon/ README.md
```

### Phase 8: Update CLAUDE.md

Create `apps/tastematter/CLAUDE.md`:

```markdown
# Tastematter - Agent Navigation Guide

## Quick Start

1. Read `specs/context_packages/03_current/` latest package
2. Run `/context-foundation` to load full context

## Project Structure

```
apps/tastematter/
├── core/           # Rust query engine
├── indexer/        # Rust indexer (TODO: implement)
├── frontend/       # Svelte + Tauri UI
├── specs/
│   ├── canonical/  # Architecture decisions
│   └── context_packages/
│       ├── 01_query_engine/
│       ├── 02_ui_foundation/
│       ├── 03_current/
│       └── 04_daemon/
└── data/           # Runtime data (gitignored)
```

## Key Commands

```bash
# Build core
cd core && cargo build --release

# Run frontend
cd frontend && pnpm dev

# Query CLI
./core/target/release/context-os query flex --time 7d
```
```

### Phase 9: Cleanup (ONLY AFTER VERIFICATION)

```bash
# Final verification checklist:
# [ ] cargo build works in core/
# [ ] pnpm dev works in frontend/
# [ ] All 62 packages present in new location
# [ ] All chain READMEs created
# [ ] Master README created
# [ ] CLAUDE.md created

# ONLY THEN delete old directories:
rm -rf apps/context-os/
rm -rf apps/tastematter/specs/context_packages_old/
rm -rf apps/tastematter/src/  # (moved to frontend/)
rm -rf apps/tastematter/src-tauri/  # (moved to frontend/)

# Git commit
git add -A
git commit -m "feat(tastematter): Consolidate repository structure

- Merge context-os into tastematter
- Unified context packages (4 chains, 62 packages)
- core/, indexer/, frontend/ structure
- Drop 'context-os' name entirely

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Rollback Procedures

### If Phase 2-3 Fails (Package Copy)

```bash
# Simply delete the new directory
rm -rf apps/tastematter/specs/context_packages_new/
# Originals untouched
```

### If Phase 6 Fails (Code Move)

```bash
# Delete copied directories
rm -rf apps/tastematter/core/
rm -rf apps/tastematter/frontend/
# Originals still in context-os/ and tastematter root
```

### If Phase 7-8 Fails (Swap/CLAUDE.md)

```bash
# Restore old context_packages
mv apps/tastematter/specs/context_packages apps/tastematter/specs/context_packages_new
mv apps/tastematter/specs/context_packages_old apps/tastematter/specs/context_packages
```

### Nuclear Rollback (Full Restore)

```bash
# Checkout the backup branch
git checkout backup/pre-migration-2026-01-12
# Or reset to before migration
git reset --hard HEAD~1
```

---

## Rules of Thumb

### DO:
- Copy first, delete last
- Verify at each phase before proceeding
- Preserve original package numbers within chains
- Add `migrated_from:` for traceability
- Create chain-specific READMEs
- Test builds after code moves

### DO NOT:
- Renumber packages to flat 00-60 (loses narrative)
- Edit original packages (append-only principle)
- Delete originals before verification
- Skip the backup branch creation
- Mix package chains in single directory

### When Uncertain:
- Preserve more context, not less
- Keep both versions temporarily
- Ask user before destructive operations
- Document the uncertainty in the package

---

## Verification Commands (Consolidated)

```bash
# Package counts by chain
ls apps/tastematter/specs/context_packages/01_query_engine/*.md | wc -l  # 11
ls apps/tastematter/specs/context_packages/02_ui_foundation/*.md | wc -l  # 22
ls apps/tastematter/specs/context_packages/03_current/*.md | wc -l       # 28
ls apps/tastematter/specs/context_packages/04_daemon/*.md | wc -l        # 1

# Total packages
find apps/tastematter/specs/context_packages -name "*.md" | wc -l        # 62+

# Build verification
cd apps/tastematter/core && cargo build --release
cd apps/tastematter/frontend && pnpm install && pnpm dev

# No old references
grep -r "context-os" apps/tastematter/ --include="*.ts" --include="*.rs" | wc -l
# Should be 0 (or only in migrated_from fields)
```

---

## For Next Agent

**Context Chain:**
- Previous: [[26_2026-01-12_REPOSITORY_CONSOLIDATION_PLAN]] (high-level plan)
- This package: Detailed execution guide with rollback procedures
- Next action: Execute Phase 0-9

**Start here:**
1. Read this package completely (you're doing it now)
2. Create backup branch (Phase 0)
3. Execute phases sequentially with verification at each step
4. If any phase fails, use rollback procedure before continuing

**Key Decision Made:**
- Use Option A (preserve chain identity with subdirectories)
- 4 chains: query_engine, ui_foundation, current, daemon
- NO flat renumbering to 00-60

**Do NOT:**
- Skip the backup branch
- Delete originals before verifying copies
- Flatten the package hierarchy
- Edit existing packages (only add `migrated_from:`)

**Success Criteria:**
- [ ] All 62 packages in new location
- [ ] 4 chain subdirectories with READMEs
- [ ] `cargo build` works
- [ ] `pnpm dev` works
- [ ] CLAUDE.md created
- [ ] No references to `context-os` in code

---

## Evidence & Attribution

[VERIFIED: Explore agent audit found 94 total packages, 61 Tastematter-relevant]
[VERIFIED: User chose "Yes, merge all 61 packages"]
[VERIFIED: User guidance "First subdivide by app area..."]
[INFERRED: Option A (subdirectories) best preserves narrative]
[PLAN: Located at ~/.claude/plans/synchronous-coalescing-harbor.md]
