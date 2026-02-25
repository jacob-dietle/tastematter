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
│       ├── 03_current/      # General development
│       ├── 04_daemon/       # Indexer investigation
│       └── 05_mcp_publishing/ # Phase 5: Context-as-a-service ← NEW
│
└── CLAUDE.md                # This file
```

## Feature Flags

| Feature | Description | Public release | Personal build |
|---------|-------------|----------------|----------------|
| (none) | Core CLI, query, daemon, intel | Yes | Yes |
| `trail` | Global trail sync — push + pull to CF Worker + D1 | **No** | **Yes** |

```bash
# Public build (CI default — no trail)
cd core && cargo build --release

# Personal build (includes trail subcommands + daemon auto-push/pull)
cd core && cargo build --release --features trail
```

**Rule:** `trail-worker/` is gitignored. Trail code compiles out without the feature. CI builds/tests without `--features trail`.

## Key Commands

```bash
# Rust core
cd core && cargo build --release                  # public
cd core && cargo build --release --features trail  # personal (with trail sync)
cd core && cargo test -- --test-threads=2  # ALWAYS limit threads (see Known Issues)

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
| 03_current | General development | 27 |
| 04_daemon | Indexer/chain linking | 1 |
| 05_mcp_publishing | Phase 5: Context-as-a-service | 1 |

**Navigation:** Read chain README first, then packages in order.

## Known Issues

- **CRITICAL: `cargo test` MUST use `--test-threads=2`** — The daemon integration tests (`test_full_daemon_workflow`, `test_sync_result_aggregates_all_phases`, `test_run_sync_*`) each spin up full SQLite databases, parse real JSONL session files, and build chain graphs. Running all 311 tests at default parallelism (= CPU core count) causes memory spikes that crash VS Code and all Claude Code instances. Always run: `cargo test -- --test-threads=2`. The `test_batch_insert_commits_performance` test is a known flaky failure under resource contention (4600ms vs 1000ms threshold) — not a real regression.
- **Chain linking broken:** Python indexer doesn't parse `leafUuid` → all sessions in one chain
- **Solution:** Port indexer to Rust (TODO)

## Intelligence Service (`intel/`)

TypeScript + Elysia HTTP server on port 3002. Provides LLM-powered agents called by the Rust core via `IntelClient`.

```
intel/
├── src/
│   ├── index.ts              # Elysia routes (9 endpoints)
│   ├── types/shared.ts       # Zod schemas (must match Rust serde)
│   ├── agents/               # One file per agent
│   │   ├── chain-naming.ts
│   │   ├── chain-summary.ts
│   │   ├── context-synthesis.ts   # Phase 2 — fills 5 None fields
│   │   ├── commit-analysis.ts
│   │   ├── gitops-decision.ts
│   │   ├── insights.ts
│   │   └── session-summary.ts
│   ├── middleware/            # Correlation IDs, operation logging
│   └── services/             # Logger
├── tests/
│   ├── unit/agents/          # Agent unit tests (bun:test)
│   ├── unit/types/           # Schema validation tests
│   ├── integration/          # HTTP endpoint tests
│   └── contract/             # Cross-language contract tests
└── package.json              # bun runtime
```

**Run intel tests:** `cd intel && bun test` (fast, ~500ms)

### Context Restore Phase 2: LLM Synthesis (shipped 2026-02-10)

Fills 5 `Option<String>` fields that Phase 1 left as `None`:

| Field | Location | What it does |
|-------|----------|-------------|
| `one_liner` | `ExecutiveSummary` | <120 char project summary |
| `narrative` | `CurrentState` | 2-4 sentence state description |
| `name` | `WorkCluster` (per cluster) | 2-4 word cluster label |
| `interpretation` | `WorkCluster` (per cluster) | What the cluster means |
| `reason` | `SuggestedRead` (per file) | Why to read this file |

**Architecture:**
- 1 LLM call per `tastematter context` request (Haiku, <$0.0003)
- `build_synthesis_request()` extracts curated 2-4K token subset → sends to intel service
- `merge_synthesis()` fills None fields using index-matched arrays from response
- Graceful degradation: if intel service is down, fields stay None (Phase 1 output unchanged)
- `QueryEngine.with_intel(IntelClient::default())` wired in `main.rs`

**Key files:**

| File | Purpose |
|------|---------|
| `intel/src/agents/context-synthesis.ts` | Agent: system prompt + tool_choice structured output |
| `intel/src/types/shared.ts` | `ContextSynthesisRequestSchema` / `ResponseSchema` |
| `core/src/intelligence/types.rs` | Rust serde mirrors of TS schemas |
| `core/src/intelligence/client.rs` | `synthesize_context()` — 15s timeout, graceful degradation |
| `core/src/context_restore.rs` | `build_synthesis_request()` + `merge_synthesis()` |
| `core/src/query.rs` | `QueryEngine.intel_client` field + Phase 5 call in `query_context()` |

**Tests:** 16 TS + 19 Rust = 35 tests covering schemas, prompts, serialization, merge logic, and edge cases (mismatched arrays, missing current_state).

## Test Strategy

**CRITICAL: Do NOT run `cargo test` with default parallelism.** See Known Issues below.

**Recommended approach for development:**
- `cargo check` for compile verification (fast, low memory)
- `cargo test <module>::tests -- --test-threads=1` for only the changed module
- `cd intel && bun test tests/unit/<file>` for specific TS tests
- Full suite only in CI or with `--test-threads=1`

## Known Issues

- **CRITICAL: `cargo test` MUST use `--test-threads=2` max** — The daemon integration tests each spin up full SQLite databases, parse real JSONL session files, and build chain graphs. Running all 330+ tests at default parallelism causes memory spikes that crash VS Code and all Claude Code instances. Always run: `cargo test -- --test-threads=2`. The `test_batch_insert_commits_performance` test is a known flaky failure under resource contention.
- **Chain linking broken:** Python indexer doesn't parse `leafUuid` → all sessions in one chain
- **Solution:** Port indexer to Rust (TODO)

## Migration History

Consolidated on 2026-01-12 from:
- `apps/context-os/` → merged into `apps/tastematter/`
- Context packages unified with `migrated_from:` traceability
