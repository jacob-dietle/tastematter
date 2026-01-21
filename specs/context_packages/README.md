# Tastematter Context Packages

Unified context package archive organized by development phase.

## Structure

| Directory | Packages | Date Range | Focus |
|-----------|----------|------------|-------|
| [[01_query_engine/]] | 11 | 2025-12-16 to 2025-12-24 | Python query engine, indexes |
| [[02_ui_foundation/]] | 22 | 2025-12-28 to 2026-01-04 | Svelte/Tauri UI, TDD |
| [[03_current/]] | 27 | 2026-01-05 to 2026-01-12 | Rust port, bug fixes |
| [[04_daemon/]] | 1 | 2026-01-12 | Chain linking investigation |
| [[05_mcp_publishing/]] | 1 | 2026-01-17 | Phase 5: Context-as-a-service |

**Total:** 62 packages

## How to Navigate

1. **New work:** Add to `03_current/` (increment package number)
2. **Understand component evolution:** Read chain README, then packages in order
3. **Find specific work:** Check chain themes above, then search within
4. **Load context:** Run `/context-foundation` (reads latest from 03_current/)

## Migration History

Consolidated on 2026-01-12 from:
- `apps/context-os/specs/context_os_intelligence/context_packages/` → 01_query_engine/
- `apps/context-os/specs/tastematter/context_packages/` → 02_ui_foundation/
- `apps/tastematter/specs/context_packages/` → 03_current/
- `apps/context-os/specs/event_capture/context_packages/` → 04_daemon/

Each package has `migrated_from:` field in frontmatter for traceability.

## Philosophy

- **Append-only:** Never edit existing packages
- **Wiki-linked:** Use [[node-name]] for traceable chains
- **Evidence-based:** Every claim has [VERIFIED/INFERRED/UNVERIFIABLE] attribution
- **Chain-organized:** Group by app area, not flat chronological
