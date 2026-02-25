---
title: "Tastematter Context Package 48"
package_number: 48
date: 2026-02-25
status: current
previous_package: "[[47_2026-02-25_TRAIL_PULL_AND_AUTO_SYNC]]"
related:
  - "[[canonical/08_GLOBAL_TRAIL_SPEC]]"
  - "[[core/src/trail/pull.rs]]"
  - "[[core/src/trail/push.rs]]"
  - "[[core/Cargo.toml]]"
tags:
  - context-package
  - tastematter
  - global-trail
  - vps-setup
---

# Tastematter - Context Package 48: VPS Trail Setup

## Purpose

This package is for the Claude Code agent running on the VPS. It contains everything needed to build tastematter with trail sync, verify the round-trip, and start generating VPS-local context data.

## What Is This?

Tastematter is a context visualization CLI that indexes Claude Code sessions into a local SQLite database and makes them queryable. The **global trail** feature syncs this data across machines via a Cloudflare D1 database.

**Architecture:**
```
Laptop (Windows)                    VPS (Linux)
┌──────────────┐                    ┌──────────────┐
│ Local SQLite  │──push──►          │              │
│ 24K+ rows     │         ┌──────┐  │ Local SQLite  │
│               │         │ CF   │  │ (empty)       │
│               │         │ D1   │  │               │
│               │◄──pull──│      │──pull──►         │
└──────────────┘         └──────┘  └──────────────┘
                    Worker: tastematter-trail.jacob-4c8.workers.dev
```

The laptop has been pushing data to D1 for days. This VPS needs to pull it down.

## Step 1: Install Rust (if needed)

```bash
# Check if Rust is installed
which cargo && cargo --version

# If not installed:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
```

## Step 2: Build with Trail Feature

```bash
cd ~/tastematter/core  # or wherever you cloned to
cargo build --release --features trail
```

**CRITICAL:** The `--features trail` flag enables the trail sync module. Without it, there is no `trail` subcommand and no auto-sync in the daemon.

Build takes ~2-5 min on first compile. Subsequent builds are fast (~10s).

Expected output: `Finished release profile [optimized] target(s)`

## Step 3: Write Trail Config

```bash
mkdir -p ~/.context-os
cat > ~/.context-os/config.yaml << 'YAML'
session_dirs:
  - ~/.claude/projects

trail:
  endpoint: https://tastematter-trail.jacob-4c8.workers.dev
  machine_id: vps
  client_id: d738fb9fdad210ab9c200b971ea294fc.access
  client_secret: d9a091be21c9a81d29492a94d251c572dee80232465a739b5cb404e50e030cf7
YAML
```

**Note:** Change `machine_id: vps` to the actual Tailscale hostname if desired. This identifies which machine pushed which rows to D1.

## Step 4: Verify Connection

```bash
./target/release/tastematter trail status
```

**Expected output:**
```json
{
  "counts": {
    "chain_graph": 1634,
    "chain_metadata": 558,
    "chain_summaries": 35,
    "chains": 813,
    "claude_sessions": 1888,
    "file_access_events": 17731,
    "file_edges": 1633,
    "git_commits": 0
  },
  "last_sync": "2026-02-25T..."
}
```

If you get connection errors, verify:
- The endpoint URL is correct
- The CF Access client_id/client_secret are correct
- The VPS can reach Cloudflare (curl the endpoint directly)

## Step 5: Pull Laptop Data

```bash
./target/release/tastematter trail pull
```

**Expected output:** `24000+ rows pulled` (first pull gets everything from D1)

**Verify locally:**
```bash
./target/release/tastematter trail status
# Should show same counts as D1

./target/release/tastematter query flex --time 30d
# Should show laptop's work sessions
```

## Step 6: Verify Incremental Sync

```bash
# Second pull should show nothing new
./target/release/tastematter trail pull
# Expected: "No new rows to pull"
```

## Step 7: Test Daemon Auto-Sync

```bash
# Run a single sync cycle — this parses local sessions AND does trail push+pull
./target/release/tastematter daemon once
```

The daemon will:
1. Parse any local Claude Code sessions (from `~/.claude/projects/`)
2. Push local data to D1 (Phase 5)
3. Pull remote data from D1 (Phase 6)

## Step 8: Verify Round-Trip (from laptop)

After the VPS pushes its own sessions to D1, go back to the laptop and run:
```bash
tastematter trail pull
# Should pull VPS sessions
```

This confirms the full round-trip: laptop → D1 → VPS → D1 → laptop.

## How It Works

### Feature Flags

The trail module is behind `#[cfg(feature = "trail")]` in Cargo.toml. Building without `--features trail` produces a public binary with no trail code at all.

| Build | Command | Trail? |
|-------|---------|--------|
| Public | `cargo build --release` | No |
| Personal | `cargo build --release --features trail` | Yes |

### Daemon Sync Phases

```
Phase 1: Git sync (commit history)
Phase 2: Session parsing (JSONL → SQLite)
Phase 3: Chain building (session → chain graph)
Phase 3.5: Intelligence enrichment (optional)
Phase 3.7: Temporal edge extraction
Phase 4: Inverted index
Phase 5: Trail PUSH (local → D1)    ← auto if trail configured
Phase 6: Trail PULL (D1 → local)    ← auto if trail configured
```

### Key Files

| File | Purpose |
|------|---------|
| `core/src/trail/push.rs` | Push local rows to D1 |
| `core/src/trail/pull.rs` | Pull D1 rows to local SQLite |
| `core/src/trail/config.rs` | TrailConfig (endpoint, machine_id, creds) |
| `core/src/trail/paths.rs` | Path normalization (Windows→Unix) |
| `core/src/daemon/sync.rs` | Orchestrates all sync phases |
| `~/.context-os/config.yaml` | Runtime config (trail section) |
| `~/.context-os/context_os_events.db` | Local SQLite database |

### Upsert Strategy

- Natural PK tables (sessions, chains, metadata): `INSERT OR REPLACE`
- Auto-increment tables (file_access_events, file_edges): `INSERT OR IGNORE` with UNIQUE constraints

### Incremental Sync

Pull tracks `last_trail_pull` in `_metadata` table. Only rows synced after this timestamp are fetched. Push currently sends all rows (incremental push is a future optimization).

## Troubleshooting

### "Trail not configured"
Config file missing or incomplete. Check `~/.context-os/config.yaml` has all 4 trail fields.

### Connection timeout
D1 has 24K+ rows. First pull takes ~2 min. Timeout is 60s. If it fails, run `trail pull` again — incremental sync will pick up where it left off.

### "attempt to write a readonly database"
The database was opened read-only. The `trail pull` CLI command opens a read-write connection. The `daemon once` command also uses read-write. This error should not occur with the current code.

### Empty query results after pull
Data is pulled with normalized Unix paths (e.g., `/Users/dietl/project`). Queries should work cross-platform. If paths look wrong, check `core/src/trail/paths.rs` for the normalization logic.

## For Next Agent

**You are on the VPS.** The laptop has already pushed 24K+ rows to D1. Your job:
1. Build with `--features trail`
2. Write config.yaml
3. Pull laptop data
4. Verify queries work
5. Run `daemon once` to generate VPS-local sessions + push them
6. Confirm laptop can pull VPS data (round-trip complete)

**Previous context:** [[47_2026-02-25_TRAIL_PULL_AND_AUTO_SYNC]] (pull module built, tested on laptop)
