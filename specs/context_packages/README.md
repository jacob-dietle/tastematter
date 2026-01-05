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

## Current State

Latest package: [[00_2026-01-05_UNIFIED_DATA_ARCHITECTURE]]

**Status:** Unified data architecture implemented per Spec 08. ContextProvider manages global state (timeRange, selectedChain, chains). View-specific stores (FilesStore, TimelineStore, WorkstreamStore) subscribe to context. All 246 tests passing, build succeeds. Changes uncommitted.

## How to Use

1. **To continue work:** Read latest package, follow "Start here" section
2. **To understand history:** Read packages in order (00 → latest)
3. **To add new package:** Increment number, never edit existing
4. **To load context:** Run `/context-foundation` in new session
