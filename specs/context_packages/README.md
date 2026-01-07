# Tastematter Context Packages

Append-only context packages for preserving state across Claude sessions.

## Philosophy

- **Append-only:** Never edit existing packages. New state = new file.
- **Wiki-linked:** Use [[node-name]] for traceable chains.
- **Evidence-based:** Every claim has [VERIFIED/INFERRED/UNVERIFIABLE] attribution.

## Canonical Reference Documents

For permanent vision, principles, and roadmap, see:
- [[canonical/00_VISION]] - What Tastematter IS
- [[canonical/01_PRINCIPLES]] - Non-negotiable design principles (Bret Victor-informed)
- [[canonical/02_ROADMAP]] - Phased development plan

Context packages track session-by-session progress toward the roadmap.

## Timeline

| # | Date | Description |
|---|------|-------------|
| 00 | 2026-01-05 | Unified data architecture implementation (TDD complete, 246 tests) |
| 01 | 2026-01-05 | Logging service (Spec 09), observability-engineering skill, bug fixes |
| 02 | 2026-01-05 | Performance optimization handoff - Phase 1 complete (3 fixes) |
| 03 | 2026-01-06 | Phase 2 in progress - Fix 4 partial (files.svelte.ts) |
| 04 | 2026-01-06 | Performance optimization complete - all 6 fixes implemented |
| 05 | 2026-01-07 | Vision foundation - synthesized 6 foundational specs, created canonical docs |

## Current State

Latest package: [[05_2026-01-07_VISION_FOUNDATION]]

**Status:** Canonical vision/principles/roadmap docs created in [[canonical/]]. 5 principles derived from foundational specs (Bret Victor-informed). 6-phase roadmap established with implementation-aware current state assessment.

## How to Use

1. **To continue work:** Read latest package, follow "Start here" section
2. **To understand history:** Read packages in order (00 → latest)
3. **To add new package:** Increment number, never edit existing
4. **To load context:** Run `/context-foundation` in new session
