---
title: "CLI Installation Fix and Rename to Tastematter"
package_number: 04
date: 2026-01-13
status: current
previous_package: "[[03_2026-01-13_INTEL_LAYER_PRIORITY_DECISION]]"
related:
  - "[[apps/tastematter/cli/pyproject.toml]]"
  - "[[apps/tastematter/cli/src/context_os_events/db/connection.py]]"
  - "[[.claude/skills/context-query/SKILL.md]]"
tags:
  - context-package
  - tastematter
  - cli
  - debugging
---

# CLI Installation Fix and Rename to Tastematter

## Executive Summary

Fixed broken CLI installation caused by editable install pointing to deleted `apps/context-os/cli/` directory. Renamed CLI from `context-os` to `tastematter`. Fixed database path from relative to canonical `~/.context-os/context_os_events.db`. Updated context-query skill with troubleshooting documentation.

---

## Session Timeline

1. Continued from Package 03 (Intel Layer Priority Decision)
2. User tried to run CLI - got `ModuleNotFoundError: No module named 'context_os_events'`
3. Applied debugging skill for RCA
4. Discovered root cause: editable install pointed to deleted directory
5. Fixed database path constant
6. Renamed CLI to "tastematter"
7. Updated context-query skill documentation

---

## Root Cause Analysis

### The Bug

```bash
$ context-os --help
ModuleNotFoundError: No module named 'context_os_events'
```

### Investigation (Per Debugging Skill)

**Checkpoint 1: Measure Current State**
```bash
pip show context-os-events
# Output:
# Editable project location: C:\Users\dietl\...\apps\context-os\cli
#                                                    ^^^^^^^^^^
#                                                    DELETED DIRECTORY
```

[VERIFIED: pip show output 2026-01-13]

### Root Cause

The package was installed as an **editable install** (`pip install -e .`) pointing to `apps/context-os/cli/`. The repository consolidation deleted `apps/context-os/` and moved everything to `apps/tastematter/cli/`. The editable install became a broken symlink.

### Secondary Bug

```python
# connection.py - WRONG
DEFAULT_DB_PATH = Path(__file__).parent.parent.parent.parent / "data" / "context_os_events.db"
# This points to cli/data/ - a LOCAL database with no data

# connection.py - CORRECT
DEFAULT_DB_PATH = Path.home() / ".context-os" / "context_os_events.db"
# This is the CANONICAL location with 782 sessions
```

[VERIFIED: [[connection.py]]:7-8]

---

## Fixes Applied

### Fix 1: Reinstall from Correct Location

```bash
pip uninstall context-os-events -y
pip install -e "C:/Users/dietl/.../apps/tastematter/cli/"
```

[VERIFIED: pip install output 2026-01-13]

### Fix 2: Rename CLI to "tastematter"

**File:** `[[pyproject.toml]]`

```toml
# Before
[project]
name = "context-os-events"
[project.scripts]
context-os = "context_os_events.cli:cli"

# After
[project]
name = "tastematter"
[project.scripts]
tastematter = "context_os_events.cli:cli"
```

[VERIFIED: [[pyproject.toml]]:1-22]

### Fix 3: Fix Database Path

**File:** `[[connection.py]]`

```python
# Before (line 8)
DEFAULT_DB_PATH = Path(__file__).parent.parent.parent.parent / "data" / "context_os_events.db"

# After (line 8)
DEFAULT_DB_PATH = Path.home() / ".context-os" / "context_os_events.db"
```

[VERIFIED: [[connection.py]]:7-8]

### Fix 4: Update context-query Skill

**File:** `[[.claude/skills/context-query/SKILL.md]]`

Added:
- Changed all `context-os query` references to `tastematter query`
- Added troubleshooting section for database issues
- Documented canonical database path

[VERIFIED: [[SKILL.md]]:23-47, version 2.2]

---

## Verification

```bash
# CLI now works globally
$ tastematter --version
tastematter, version 0.1.0

# Correct database being used
$ tastematter status
| Claude Sessions |   782 | Messages: 55228 |
Session range: 2025-12-07 to 2026-01-12
```

[VERIFIED: command output 2026-01-13]

---

## Known Issue Discovered

### Foreign Key Constraint in build-chains

```bash
$ tastematter build-chains --project "/path/to/gtm_operating_system"
sqlite3.IntegrityError: FOREIGN KEY constraint failed
```

**Root Cause:** The `chain_graph` table has a foreign key to `claude_sessions`, but some sessions in the JSONL files haven't been indexed yet.

**Status:** NOT FIXED this session - documented for next session

[VERIFIED: command error output 2026-01-13]

---

## Files Modified This Session

| File | Change | Status |
|------|--------|--------|
| [[pyproject.toml]] | Renamed CLI to "tastematter" | In tastematter repo |
| [[connection.py]] | Fixed DEFAULT_DB_PATH | In tastematter repo |
| [[context-query/SKILL.md]] | Updated CLI name, added troubleshooting | Tracked in main repo |

**Note:** `apps/` is gitignored in main repo. CLI changes are in the separate tastematter git repo.

---

## Jobs To Be Done (Next Session)

1. [ ] **Fix foreign key constraint in build-chains** - Sessions need indexing first
   - Success criteria: `tastematter build-chains` completes without error

2. [ ] **Commit CLI changes in tastematter repo** - Push to remote
   - Success criteria: Changes visible on GitHub

3. [ ] **Continue Priority 1: Core Stability** - From Package 03
   - ISSUE-003: Timeline shows files not sessions
   - ISSUE-007: File paths truncated
   - ISSUE-008,009: May already work (verify)

---

## For Next Agent

### Context Chain

- Previous: [[03_2026-01-13_INTEL_LAYER_PRIORITY_DECISION]] (priority order established)
- This package: CLI installation fixed, renamed to "tastematter"
- Next action: Fix build-chains foreign key constraint

### Start Here

1. Read this package (done)
2. Verify CLI works: `tastematter status`
3. Investigate foreign key issue in `[[chain_graph.py]]`
4. Continue with Priority 1 from Package 03

### Do NOT

- Use `context-os` command (renamed to `tastematter`)
- Assume database is at `cli/data/` (canonical path is `~/.context-os/`)
- Reinstall CLI without specifying correct path

### Key Insight

> Editable installs (`pip install -e .`) create symlinks to source directories. When directories are moved/deleted during repository consolidation, the symlinks break. Always reinstall editable packages after moving their source directories.

[INFERRED: From debugging session RCA]

---

## Test Commands

```bash
# Verify CLI installed correctly
tastematter --version
# Expected: tastematter, version 0.1.0

# Verify database connection
tastematter status
# Expected: Shows 782 sessions, 55228 messages

# Test build-chains (known to fail currently)
tastematter build-chains --project "."
# Expected: FOREIGN KEY constraint error (known issue)
```

---

**Document Status:** CURRENT
**Session Duration:** ~1 hour
**Primary Work:** CLI debugging and installation fix
**Key Achievement:** CLI renamed to "tastematter" and working from any terminal
