# Daemon Investigation (Chain 4)

Context packages documenting the indexer/daemon investigation and Claude Code data model.

## Overview

**Date Range:** 2026-01-12 to 2026-01-13
**Package Count:** 5
**Theme:** Chain linking bug, data model, Intelligence Layer priority, CLI installation fix

## Narrative

This chain documents investigating and fixing the chain linking bug:
- **Bug:** Python indexer extracted ALL leafUuids (including compaction summaries)
- **Root cause:** Only FIRST record's leafUuid indicates session resumption
- **Additional finding:** Agent sessions link via `sessionId` field, not `leafUuid`
- **Fix applied:** chain_graph.py now correctly links 313+ sessions (vs 90 before)

## Timeline

| # | Date | Title |
|---|------|-------|
| 00 | 2026-01-12 | CHAIN_LINKING_BUG_INVESTIGATION |
| 01 | 2026-01-13 | CLAUDE_CODE_JSONL_DATA_MODEL (Complete Reference) |
| 02 | 2026-01-13 | CHAIN_LINKING_FIX_COMPLETE (Handoff) |
| 03 | 2026-01-13 | INTEL_LAYER_PRIORITY_DECISION (Architectural Necessity) |
| 04 | 2026-01-13 | CLI_INSTALLATION_FIX (Renamed to tastematter) |

## Key Findings

### Data Model
- **Regular sessions:** Link via `leafUuid` in FIRST summary record
- **Agent sessions:** Link via `sessionId` field (filename of parent)
- **Compaction summaries:** Have `leafUuid` pointing to SAME session (ignore these)
- **logicalParentUuid:** Within-session continuity, NOT cross-session

### Chain Statistics (GTM Project)
- Total sessions: 779 (314 regular + 465 agents)
- Largest chain: 313 sessions (98% of expected ~356)
- Chain linking success: 98% for regular, 100% for agents

### Fix Applied
- `chain_graph.py`: Only use FIRST record's leafUuid
- `chain_graph.py`: Added agent session linking via sessionId
- Five-pass algorithm: leafUuid → sessionId → uuid → relationships → chains

## Current State

**Latest package:** [[04_2026-01-13_CLI_INSTALLATION_FIX]]
**Status:** CLI renamed to `tastematter`, installation fixed, build-chains has known FK issue

## Related

- [[../03_current/22_2026-01-11_CHAIN_LINKAGE_BUG_RCA.md]] - Initial RCA
- [[../03_current/26_2026-01-12_REPOSITORY_CONSOLIDATION_PLAN.md]] - Decision to port
- [[../../cli/src/context_os_events/index/chain_graph.py]] - Fixed implementation
