# Tastematter - Agent Navigation Guide

Context visualization desktop app for understanding Claude Code work patterns.

## Quick Start

1. **Load context:** Run `/context-foundation`
2. **Latest package:** `specs/context_packages/03_current/` (read latest numbered file)
3. **Build core:** `cd core && cargo build --release`
4. **Run frontend:** `cd frontend && pnpm dev`

## Project Structure

```
apps/tastematter/
├── core/                    # Rust query engine (READ-ONLY database access)
│   ├── src/
│   │   ├── main.rs          # CLI entry point
│   │   ├── http.rs          # HTTP server for browser dev
│   │   ├── query.rs         # Query execution
│   │   ├── storage.rs       # SQLite storage layer
│   │   └── types.rs         # Data types
│   └── Cargo.toml
│
├── cli/                     # Python indexer/daemon (WRITES to database)
│   ├── src/context_os_events/
│   │   ├── daemon/          # Background service
│   │   ├── capture/         # JSONL parsing, file watching
│   │   ├── index/           # Index builders (chain_graph.py has bug)
│   │   └── db/              # Database writes
│   ├── tests/               # Python tests
│   └── pyproject.toml
│   ⚠️  TO BE REPLACED by Rust indexer - reference for port
│
├── frontend/                # Desktop app (Tauri + Svelte)
│   ├── src/                 # Svelte frontend
│   │   └── lib/
│   │       ├── components/  # UI components
│   │       ├── stores/      # State management
│   │       └── services/    # Data services
│   ├── src-tauri/           # Tauri Rust backend
│   │   └── src/lib.rs       # Tauri commands
│   ├── tests/               # Vitest unit tests
│   ├── package.json
│   └── vite.config.ts
│
├── specs/
│   ├── canonical/           # Blessed architecture docs
│   └── context_packages/    # Session context (UNIFIED)
│       ├── 01_query_engine/ # Python → Rust migration
│       ├── 02_ui_foundation/# Svelte/Tauri UI
│       ├── 03_current/      # Active development ← START HERE
│       └── 04_daemon/       # Indexer investigation
│
└── CLAUDE.md                # This file
```

## Key Commands

```bash
# Rust core
cd core && cargo build --release
cd core && cargo test

# Frontend (desktop app)
cd frontend && pnpm install
cd frontend && pnpm dev          # Development server
cd frontend && pnpm test:unit    # Run tests

# Query CLI
./core/target/release/context-os query flex --time 7d
./core/target/release/context-os serve  # HTTP server on :3001
```

## Database

**Canonical path:** `~/.context-os/context_os_events.db`

The Rust core is READ-ONLY. The Python daemon (to be replaced with Rust indexer) writes to the database.

## Context Package Chains

| Chain | Focus | Packages |
|-------|-------|----------|
| 01_query_engine | Python → Rust query engine | 11 |
| 02_ui_foundation | Svelte/Tauri UI, TDD | 22 |
| 03_current | Active development | 28+ |
| 04_daemon | Indexer/chain linking | 1 |

**Navigation:** Read chain README first, then packages in order.

## Known Issues

- **Chain linking broken:** Python indexer doesn't parse `leafUuid` → all sessions in one chain
- **Solution:** Port indexer to Rust (TODO)

## Migration History

Consolidated on 2026-01-12 from:
- `apps/context-os/` → merged into `apps/tastematter/`
- Context packages unified with `migrated_from:` traceability
