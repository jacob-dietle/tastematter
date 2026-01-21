---
title: "Glob Bug Discovery - Missing 218 Agent Sessions"
package_number: 12
date: 2026-01-15
status: current
previous_package: "[[11_2026-01-15_CHAIN_TOPOLOGY_INVESTIGATION]]"
related:
  - "[[chain_graph.py]]"
  - "[[01_2026-01-13_CLAUDE_CODE_JSONL_DATA_MODEL]]"
tags:
  - context-package
  - tastematter
  - chain-linking
  - bug-fix
  - glob-pattern
---

# Glob Bug Discovery - Context Package 12

## Executive Summary

Deep forensic investigation of Claude Code's file structure revealed the ACTUAL bug: `glob("*.jsonl")` was missing 218 agent sessions stored in subdirectories. One-line fix applied: `glob("**/*.jsonl")`. This explains why previous "leafUuid fixes" didn't work - we were missing sessions entirely, not parsing them wrong.

## Key Discovery: File Structure Is More Complex

### What We Assumed

```
.claude/projects/{project}/
├── session1.jsonl
├── session2.jsonl
├── agent-xxx.jsonl    # Agents at top level
└── ...
```

### What Actually Exists

```
.claude/projects/{project}/
├── session1.jsonl                    # Regular session
├── session1/                         # Directory FOR session1
│   ├── subagents/                    # Agent sessions spawned by session1
│   │   ├── agent-xxx.jsonl           # ❌ MISSED by *.jsonl
│   │   └── agent-yyy.jsonl           # ❌ MISSED by *.jsonl
│   └── tool-results/                 # Large tool outputs stored separately
│       └── toolu_xxx.txt
├── agent-aaa.jsonl                   # Some agents at top level (older format?)
└── ...
```

[VERIFIED: `ls -laR .claude/projects/.../846b76ee*/` shows subagents/ and tool-results/ directories]

### The Numbers

| Location | Count | Status |
|----------|-------|--------|
| Top-level regular sessions | 324 | ✅ Found |
| Top-level agent sessions | 441 | ✅ Found |
| **Subdirectory agent sessions** | **218** | ❌ **MISSED** |
| **Total** | **983** | After fix |

[VERIFIED: Python glob comparison 2026-01-15]

```
OLD glob (*.jsonl): 765 files
NEW glob (**/*.jsonl): 983 files
DIFF: +218 sessions now found!
```

## The Bug: One Line

**File:** `chain_graph.py` line 215

```python
# OLD (misses subdirectories):
jsonl_files = list(jsonl_dir.glob("*.jsonl"))

# NEW (finds all JSONL files recursively):
jsonl_files = list(jsonl_dir.glob("**/*.jsonl"))
```

[VERIFIED: Edit applied to [[chain_graph.py]]:214-217]

## Why Previous Fixes Didn't Work

### Fix #1: "Use FIRST leafUuid only"
- **Premise:** Multiple leafUuids in first record were confusing linking
- **Reality:** We were parsing correctly, just missing sessions
- **Result:** Didn't help

### Fix #2: "Use LAST leafUuid" (Package 11)
- **Premise:** Summaries are stacked, last = immediate parent
- **Reality:** This IS correct, but still missing 218 sessions
- **Result:** Marginal improvement (91→76 root children)

### Fix #3: "Use recursive glob" (This package)
- **Premise:** Agent sessions in subdirectories aren't being found
- **Reality:** CORRECT - 218 sessions were invisible to the algorithm
- **Result:** All sessions now discoverable

## Complete Data Model (Updated)

### Directory Structure

```
~/.claude/
├── history.jsonl              # User message log with session IDs
├── projects/
│   └── {project-path}/
│       ├── {session-uuid}.jsonl           # Main session file
│       ├── {session-uuid}/                # Session directory (if has children)
│       │   ├── subagents/                 # Agent sessions
│       │   │   └── agent-{hash}.jsonl
│       │   └── tool-results/              # Large tool outputs
│       │       └── toolu_{id}.txt
│       └── agent-{hash}.jsonl             # Top-level agents (legacy?)
├── file-history/              # File edit history (unexplored)
├── todos/                     # Todo state (unexplored)
├── plans/                     # Plan mode data (unexplored)
├── shell-snapshots/           # Shell state (unexplored)
└── ...
```

### File Types Found

| Type | Count | Purpose |
|------|-------|---------|
| `.jsonl` | 983 | Session conversation logs |
| `.txt` | 384 | Tool result outputs (large responses) |
| `tmpclaude-*-cwd` | Many | Temporary working directory markers |

### Session Types

1. **Regular sessions** (UUID format: `846b76ee-3534-49ac-8555-cff4745c4a41`)
   - Main conversation files
   - May have associated directory with subagents/tool-results

2. **Agent sessions** (format: `agent-{7-char-hash}`)
   - Can be at top level OR in `{parent}/subagents/`
   - Link to parent via `sessionId` field in first record

### Linking Mechanisms

| Type | Location | Link Field | Points To |
|------|----------|------------|-----------|
| Resume | Regular session | `leafUuid` in summary | Message UUID in parent |
| Agent spawn | Agent session | `sessionId` in first record | Parent session ID |
| Directory | Subdirectory path | `{parent}/subagents/{agent}.jsonl` | Parent from path |

## Outstanding Questions

### Explored But Not Fully Documented

1. **`tool-results/` directory**
   - Contains `.txt` files named `toolu_{id}.txt`
   - Stores large tool outputs separately from JSONL
   - May need indexing for complete context

2. **Other `.claude/` directories**
   - `file-history/` - Unknown structure
   - `todos/` - Todo persistence
   - `plans/` - Plan mode state
   - `shell-snapshots/` - Shell state

3. **Why some agents at top level vs subdirectory?**
   - 441 agents at top level
   - 218 agents in subdirectories
   - Possibly version/date dependent?

## Jobs To Be Done (Next Session)

1. [X] **Apply glob fix** [VERIFIED: [[chain_graph.py]]:214-217]
2. [ ] **Rebuild chain graph database**
   ```bash
   cd apps/tastematter/cli
   tastematter daemon rebuild
   ```
3. [ ] **Verify chain counts match Claude Code UI**
   - Claude Code shows: 356 sessions in largest chain
   - Our algorithm should now find: ~350+ (accounting for agent links)

4. [ ] **Document remaining `.claude/` structure**
   - Explore `file-history/`, `todos/`, `plans/`
   - Determine if these contain indexable data

5. [ ] **Investigate tool-results linking**
   - Do JSONL files reference `toolu_` IDs?
   - Should we index tool outputs?

## For Next Agent

### Context Chain

| # | Date | Title | Key Content |
|---|------|-------|-------------|
| 11 | 2026-01-15 | CHAIN_TOPOLOGY_INVESTIGATION | Star topology, forking hypothesis |
| 12 | 2026-01-15 | GLOB_BUG_DISCOVERY | **This package** - actual bug found |

### Start Here

1. Read this package (you're doing it now)
2. Verify glob fix is in place:
   ```bash
   grep -n "glob.*jsonl" apps/tastematter/cli/src/context_os_events/index/chain_graph.py
   # Should show: **/*.jsonl (not *.jsonl)
   ```
3. Rebuild database:
   ```bash
   cd apps/tastematter/cli && tastematter daemon rebuild
   ```
4. Test chain filtering in UI

### Key Insight

**We weren't parsing wrong - we were MISSING FILES.**

The recursive glob `**/*.jsonl` finds 218 more sessions that were completely invisible before. This is why no amount of "leafUuid parsing fixes" helped - the sessions didn't exist in our dataset.

### Do NOT

- Assume flat file structure (it's hierarchical)
- Ignore subdirectories (they contain real data)
- Skip the rebuild step (fix means nothing without re-indexing)

## Evidence Sources

| Claim | Source |
|-------|--------|
| 765 files with old glob | Python `glob("*.jsonl")` count |
| 983 files with new glob | Python `glob("**/*.jsonl")` count |
| 218 agent sessions in subdirs | Diff of above counts |
| subagents/ structure | `ls -laR 846b76ee*/` output |
| tool-results/ exists | Same directory listing |
| sessionId links agents | `head -1 agent-*.jsonl` inspection |

---

**Document Status:** CURRENT
**Session Duration:** ~45 minutes
**Primary Work:** Deep forensic investigation of Claude Code file structure
**Bug Found:** glob pattern missing subdirectory agent sessions
**Fix Applied:** `*.jsonl` → `**/*.jsonl` (one line)
