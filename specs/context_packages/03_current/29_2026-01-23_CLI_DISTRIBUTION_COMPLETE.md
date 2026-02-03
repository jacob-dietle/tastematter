---
title: "Tastematter Context Package 29"
package_number: 29
date: 2026-01-23
status: current
previous_package: "[[28_2026-01-21_CLI_DISTRIBUTION_GIT_CLEANUP]]"
related:
  - "[[.github/workflows/release.yml]]"
  - "[[core/Cargo.toml]]"
  - "[[README.md]]"
  - "[[.claude/skills/context-query/SKILL.md]]"
tags:
  - context-package
  - tastematter
  - cli-distribution
  - release
---

# Tastematter - Context Package 29

## Executive Summary

CLI distribution pipeline **100% complete**. Binary renamed from `context-os` to `tastematter`, 4-platform builds (Windows, Linux, macOS Intel, macOS ARM), one-liner install working, database synced with 1,079 sessions and 534K tool uses. Python CLI removed from PATH. Ready for use.

## Global Context

### Architecture Overview

```
PRIVATE: jacob-dietle/tastematter (GitHub)
├── core/                    # Rust CLI source
├── .github/workflows/       # CI/CD for 4-platform builds
└── scripts/install/         # Install scripts

PUBLIC: Cloudflare R2 bucket
├── releases/v*/             # Version-tagged binaries
├── install.ps1              # Windows installer
├── install.sh               # Unix installer
└── latest.txt               # v0.1.0-alpha.8

DOMAIN: install.tastematter.dev → R2 bucket
```

### Key Decisions This Session

1. **Binary name:** `tastematter` (not `context-os` or `tm`) [VERIFIED: [[core/Cargo.toml]]:11]
2. **Python CLI removed:** Rust port is complete, no need for Python indexer [VERIFIED: pip uninstall + PATH cleanup]
3. **Linux added:** 4 platforms now (was 3) [VERIFIED: [[.github/workflows/release.yml]]:21-23]
4. **Database location:** `~/.context-os/` unchanged (data dir, not CLI name)

## Local Problem Set

### Completed This Session

- [x] Renamed binary from `context-os` to `tastematter` [VERIFIED: `tastematter --version` → `tastematter 0.1.0`]
- [x] Updated Cargo.toml package name and [[bin]] name [VERIFIED: [[core/Cargo.toml]]:2,11]
- [x] Updated all `use context_os_core::` imports to `use tastematter::` [VERIFIED: [[core/src/main.rs]]]
- [x] Fixed GitHub Actions workflow to copy correct binary name [VERIFIED: [[.github/workflows/release.yml]]:42,49]
- [x] Removed Python CLI from PATH [VERIFIED: `which tastematter` → `~/.local/bin/tastematter`]
- [x] Added Linux x86_64 build target (ubuntu-latest) [VERIFIED: [[.github/workflows/release.yml]]:21-23]
- [x] Released v0.1.0-alpha.7 and v0.1.0-alpha.8 [VERIFIED: gh release list]
- [x] Tested Linux install in WSL [VERIFIED: `wsl -e bash -c "~/.local/bin/tastematter --version"`]
- [x] Created full README with quickstart [VERIFIED: [[README.md]]]
- [x] Updated context-query skill troubleshooting [VERIFIED: [[.claude/skills/context-query/SKILL.md]]:30-45]
- [x] Synced database: 1,079 sessions, 534,583 tool uses [VERIFIED: `tastematter parse-sessions` output]

### In Progress

None - all distribution tasks complete.

### Jobs To Be Done (Future)

1. [ ] Test Homebrew formula on actual macOS - Currently untested
2. [ ] Test Scoop manifest on Windows - Currently untested
3. [ ] Add `tastematter update` self-update command - Nice to have
4. [ ] Add `tastematter init` for first-time setup - Would improve UX

## Release History

| Version | Date | Changes |
|---------|------|---------|
| v0.1.0-alpha.1 | 2026-01-22 | First release attempt (macOS Intel failed) |
| v0.1.0-alpha.2 | 2026-01-22 | Fix: macos-13 → macos-15-intel |
| v0.1.0-alpha.3 | 2026-01-22 | Fix: macos-13 retired, use macos-15-intel |
| v0.1.0-alpha.4-5 | 2026-01-22 | Fix: R2 credentials (wrong key length) |
| v0.1.0-alpha.6 | 2026-01-22 | R2 working, all builds pass |
| v0.1.0-alpha.7 | 2026-01-23 | Binary renamed to `tastematter` |
| v0.1.0-alpha.8 | 2026-01-23 | Added Linux build |

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/Cargo.toml]] | Rust package config | Modified (name change) |
| [[core/src/main.rs]] | CLI entry point | Modified (imports, help text) |
| [[core/src/daemon/mod.rs]] | Daemon module | Modified (test commands) |
| [[.github/workflows/release.yml]] | CI/CD pipeline | Modified (4 platforms) |
| [[README.md]] | User documentation | Replaced (full quickstart) |
| [[scripts/install/install.ps1]] | Windows installer | Unchanged |
| [[scripts/install/install.sh]] | Unix installer | Unchanged |

## Test State

- **Rust tests:** 169 passing [VERIFIED: previous session]
- **Install test (Windows):** Working [VERIFIED: `irm install.tastematter.dev/install.ps1 | iex`]
- **Install test (Linux/WSL):** Working [VERIFIED: `curl -fsSL install.tastematter.dev/install.sh | bash`]
- **Database sync:** 1,079 sessions parsed [VERIFIED: `tastematter parse-sessions` output]

### Verification Commands
```bash
# Check version
tastematter --version
# Expected: tastematter 0.1.0

# Check daemon status
tastematter daemon status

# Query recent files
tastematter query flex --time 7d --limit 10

# Sync database (if needed)
tastematter parse-sessions --claude-dir ~/.claude
tastematter build-chains
tastematter index-files
```

## For Next Agent

**Context Chain:**
- Previous: [[28_2026-01-21_CLI_DISTRIBUTION_GIT_CLEANUP]] (git cleanup, initial pipeline)
- This package: Distribution complete, binary renamed, 4 platforms
- Next action: Use tastematter for context queries

**Start here:**
1. Verify: `tastematter --version` shows `tastematter 0.1.0`
2. Query: `tastematter query flex --time 7d` to see recent activity
3. If stale: `tastematter daemon once` to sync

**Install on new machine:**
```powershell
# Windows
irm https://install.tastematter.dev/install.ps1 | iex
```
```bash
# macOS/Linux
curl -fsSL https://install.tastematter.dev/install.sh | bash
```

**Do NOT:**
- Use pip install - Python CLI is deprecated
- Look for `context-os` command - renamed to `tastematter`
- Expect sessions from today in queries - JSONL timestamps are from message creation, not file modification

**Key insight:**
Distribution is separate from development. Users get binaries from R2 CDN, never clone the repo. Monorepo structure (core/, frontend/, cli/) is irrelevant to end users.
[VERIFIED: Kelsey Hightower perspective from [[.claude/skills/devops-architecture-perspectives/SKILL.md]]]
