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

# VPS Agent Bootstrap — Trail Round-Trip Verification

## Who You Are

You are a Claude Code agent running on a Linux VPS (Tailscale IP: 100.109.204.117, user: jacob). Your job is to set up the development environment, build tastematter with trail sync, and verify the full round-trip: laptop pushes context data → Cloudflare D1 → VPS pulls it down.

## What You're Working On

**Taste Systems** is a solo GTM consultancy. The owner (Jacob) runs two primary repos:

1. **gtm_operating_system** — A knowledge graph and operational system for GTM strategy, client engagements, and context engineering. Contains skills, state files, specs, transcripts, and a two-layer knowledge architecture.

2. **tastematter** — A Rust CLI that indexes Claude Code sessions into a local SQLite database, making work patterns queryable. Lives inside `apps/tastematter/` in the GTM OS repo (gitignored — separate git repo).

The **global trail** feature syncs tastematter's SQLite data across machines via a Cloudflare Worker + D1 database. Jacob's Windows laptop has been pushing 24K+ rows of session data to D1. Your VPS needs to pull that data down so both machines share the same context.

## Step 1: Clone Both Repos

```bash
# GTM Operating System (the knowledge graph + operational system)
cd ~
git clone https://github.com/jacob-dietle/gtm_operating_system.git

# Tastematter (the Rust CLI — lives inside apps/ but is a separate repo)
cd ~/gtm_operating_system/apps
git clone https://github.com/jacob-dietle/tastematter.git
```

After cloning, the structure should be:
```
~/gtm_operating_system/
├── CLAUDE.md                    # System navigation guide — read this
├── 00_foundation/               # Positioning, messaging, brand
├── 03_gtm_engagements/          # Client work
├── 04_knowledge_base/           # Atomic knowledge graph
├── _system/                     # State files, agents, workflows
└── apps/
    └── tastematter/              # Separate git repo
        ├── CLAUDE.md             # Tastematter navigation guide
        ├── core/                 # Rust source (this is what you build)
        ├── specs/                # Canonical specs + context packages
        └── trail-worker/         # Gitignored (CF infrastructure)
```

## Step 2: Read the CLAUDE.md Files

```bash
# Understand the overall system
cat ~/gtm_operating_system/CLAUDE.md

# Understand tastematter specifically
cat ~/gtm_operating_system/apps/tastematter/CLAUDE.md
```

Key things to know:
- Tastematter uses **Cargo feature flags** — `--features trail` enables the sync module
- The trail module is private infrastructure (not in public releases)
- Tests MUST use `--test-threads=2` (memory safety — see CLAUDE.md Known Issues)

## Step 3: Install Rust (if needed)

```bash
which cargo && cargo --version || {
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source ~/.cargo/env
}
```

## Step 4: Build Tastematter with Trail

```bash
cd ~/gtm_operating_system/apps/tastematter/core
cargo build --release --features trail
```

Build takes ~2-5 min on first compile. Expected: `Finished release profile [optimized]`

Verify the trail subcommand exists:
```bash
./target/release/tastematter trail --help
# Should show: status, push, pull
```

## Step 5: Write Trail Config

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

## Step 6: Verify Connection + Pull

```bash
# Check D1 is reachable (should show 24K+ rows across 7 tables)
./target/release/tastematter trail status

# Pull all laptop data into local SQLite
./target/release/tastematter trail pull
# Expected: "24000+ rows pulled"

# Verify incremental works (second pull = 0 new rows)
./target/release/tastematter trail pull
# Expected: "No new rows to pull"
```

## Step 7: Test Daemon Auto-Sync

```bash
# Run a single sync cycle — parses local sessions + trail push + trail pull
./target/release/tastematter daemon once
```

The daemon output should show trail rows pushed/pulled with 0 errors.

## Step 8: Verify Round-Trip

The round-trip is complete when:
1. Laptop pushed to D1 (already done — 24K+ rows)
2. VPS pulled from D1 (Step 6 above)
3. VPS pushes its own sessions to D1 (Step 7 daemon)
4. Laptop pulls VPS data (Jacob will verify from laptop)

After Step 7, report the daemon output and trail status so Jacob can verify from the laptop side.

## What Success Looks Like

- `trail status` shows row counts matching D1
- `trail pull` pulled 24K+ rows on first run, 0 on second
- `daemon once` completed with 0 errors
- `tastematter query flex --time 30d` returns laptop's work sessions
- VPS-generated sessions appear in D1 (visible via `trail status` after daemon)

## Key Architecture Details

### Trail Sync Flow
```
Local SQLite ──push──► CF Worker ──► D1 ──► CF Worker ──pull──► Local SQLite
                       (Phase 5)              (Phase 6)
```

### Feature Flags
| Build | Command | Trail? |
|-------|---------|--------|
| Public | `cargo build --release` | No |
| Personal | `cargo build --release --features trail` | Yes |

### Auth
CF Access service tokens: `CF-Access-Client-Id` / `CF-Access-Client-Secret` headers. Same creds on both machines.

### Database
- Local: `~/.context-os/context_os_events.db` (auto-created on first daemon run)
- Remote: Cloudflare D1 (tastematter-trail worker)

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `trail status` connection error | Check endpoint URL and CF Access creds in config.yaml |
| First pull timeout | D1 has 24K+ rows. Run `trail pull` again — incremental picks up |
| `daemon once` shows errors | Read error messages. Common: git sync skip (optional, not a real error) |
| Empty query results after pull | Data uses normalized Unix paths. Queries should work. Check `~/.context-os/context_os_events.db` exists |
