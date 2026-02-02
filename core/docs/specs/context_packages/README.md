# Tastematter Core Context Packages

Append-only context packages for preserving state across Claude sessions.

## Philosophy

- **Append-only:** Never edit existing packages. New state = new file.
- **Wiki-linked:** Use [[node-name]] for traceable chains.
- **Evidence-based:** Every claim has [VERIFIED/INFERRED/UNVERIFIABLE] attribution.

## Timeline

| # | Date | Description |
|---|------|-------------|
| 00 | 2026-02-01 | Database write path fix - sessions and chains now persist |
| 01 | 2026-02-02 | Database auto-init for fresh installs (IN PROGRESS) |

## Current State

**Latest package:** [[01_2026-02-02_DATABASE_AUTO_INIT]]

**Status:** Auto-init implementation in progress:
- `ensure_schema()` method added and tested (3 new tests)
- main.rs refactoring has type errors - needs API alignment
- 257 tests passing

**Blocker:** main.rs daemon command handling uses wrong API names

**Next priorities:**
1. Fix main.rs type errors (use correct platform API)
2. Test fresh install scenario
3. Incremental sync from database (avoid re-parsing unchanged sessions)

## How to Use

1. **To continue work:** Read latest package, follow "Start here" section
2. **To understand history:** Read packages in order (00 → latest)
3. **To add new package:** Increment number, never edit existing

## Quick Reference

```bash
# Navigate to core
cd apps/tastematter/core

# Run tests
cargo test --lib

# Build release
cargo build --release

# Test daemon
./target/release/tastematter.exe daemon once
./target/release/tastematter.exe query flex --time 7d --limit 5
./target/release/tastematter.exe query chains --limit 3
```
