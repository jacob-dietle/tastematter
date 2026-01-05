# Tastematter Context Packages

Append-only context packages for preserving state across Claude sessions.

## Philosophy

- **Append-only:** Never edit existing packages. New state = new file.
- **Wiki-linked:** Use [[node-name]] for traceable chains.
- **Evidence-based:** Every claim has [VERIFIED/INFERRED/UNVERIFIABLE] attribution.

## Timeline

| # | Date | Description |
|---|------|-------------|
| 00 | 2026-01-05 | Unified data architecture implementation (TDD complete, 246 tests) |
| 01 | 2026-01-05 | Logging service (Spec 09), observability-engineering skill, bug fixes |

## Current State

Latest package: [[01_2026-01-05_LOGGING_SERVICE]]

**Status:** Logging service implemented per Spec 09. Two-layer logging: JSONL for IPC events with correlation IDs (`~/.tastematter/logs/`), rust.log for Rust console output (`%LOCALAPPDATA%/com.tastematter.app/logs/`). Created `observability-engineering` skill. Fixed TimelineView store bug and infinite fetch loop. All 246 tests passing. Changes uncommitted.

## How to Use

1. **To continue work:** Read latest package, follow "Start here" section
2. **To understand history:** Read packages in order (00 → latest)
3. **To add new package:** Increment number, never edit existing
4. **To load context:** Run `/context-foundation` in new session
