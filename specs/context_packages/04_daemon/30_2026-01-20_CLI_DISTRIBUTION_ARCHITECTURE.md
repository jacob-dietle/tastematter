---
title: "Tastematter CLI Distribution Architecture"
package_number: 30
date: 2026-01-20
status: current
previous_package: "[[29_2026-01-19_PARITY_TEST_SUITE_COMPLETE]]"
related:
  - "[[~/.claude/plans/synchronous-coalescing-harbor.md]]"
  - "[[apps/tastematter/core/Cargo.toml]]"
  - "[[~/.context-os/bin/tastematter.cmd]]"
tags:
  - context-package
  - tastematter
  - cli-distribution
---

# Tastematter CLI Distribution Architecture

## Executive Summary

Rust CLI fully working (6.7M, 9 commands including 5 new query commands). This session added missing query commands (search, file, co-access, verify, receipts), created `tastematter.cmd` wrapper, and planned cross-platform distribution architecture. **Phase 1 PATH fix complete** - user's machine now has `~/.context-os/bin` in PATH.

## Global Context

### What Was Built This Session

1. **Missing Query Commands Implemented in Rust:**
   - `query search` - Substring search across file paths [VERIFIED: core/src/query.rs:623-689]
   - `query file` - Show sessions that touched a file [VERIFIED: core/src/query.rs:691-722]
   - `query co-access` - PMI-scored co-access analysis [VERIFIED: core/src/query.rs:724-801]
   - `query verify` - Verify receipt against ledger [VERIFIED: core/src/query.rs:803-852]
   - `query receipts` - List receipts from ledger [VERIFIED: core/src/query.rs:854-927]

2. **Wrapper Scripts:**
   - Created `~/.context-os/bin/tastematter.cmd` → points to Rust binary
   - User command: `tastematter` (not `context-os`)
   - Binary internally named `context-os.exe`, renamed at distribution

3. **PATH Configuration:**
   - Created `apps/tastematter/scripts/install/fix-path.ps1`
   - Executed successfully - `~/.context-os/bin` now in user PATH
   - **User must restart terminal** for PATH to take effect

### Architecture Decision

```
User types: tastematter query flex ...
     ↓
Wrapper: ~/.context-os/bin/tastematter.cmd
     ↓
Binary: apps/tastematter/core/target/release/context-os.exe
     ↓
Database: ~/.context-os/context_os_events.db
```

**Distribution tiers:**
| Tier | Source | Install |
|------|--------|---------|
| Dev | `cargo build --release` | Manual PATH/alias |
| Alpha | GitHub Release | Install script (one-liner) |
| Prod | Package managers | `brew`/`scoop` (future) |

## Local Problem Set

### Completed This Session

- [X] `query search` command [VERIFIED: tested, returns results]
- [X] `query file` command [VERIFIED: tested, returns session data]
- [X] `query co-access` command [VERIFIED: tested, returns PMI scores]
- [X] `query verify` command [VERIFIED: tested, reads from ledger]
- [X] `query receipts` command [VERIFIED: tested, lists 823 receipts]
- [X] Created `tastematter.cmd` wrapper [VERIFIED: ~/.context-os/bin/tastematter.cmd]
- [X] Fixed user PATH [VERIFIED: fix-path.ps1 executed successfully]
- [X] Distribution architecture planned [VERIFIED: plan file written]

### In Progress

- [ ] Install scripts (install.ps1 + install.sh)
  - Templates written in plan file
  - Need to create actual files
  - Location: `apps/tastematter/scripts/install/`

### Jobs To Be Done (Next Session)

1. **Create install.ps1** - Windows installer script
   - Download from GitHub releases
   - Install to `~/.local/bin`
   - Add to PATH
   - Location: `apps/tastematter/scripts/install/install.ps1`

2. **Create install.sh** - Unix installer script
   - Detect platform (darwin/linux, x86_64/aarch64)
   - Download correct binary
   - Location: `apps/tastematter/scripts/install/install.sh`

3. **Create GitHub Actions release workflow**
   - Trigger on tag push (v*)
   - Build matrix: Windows x86, macOS Intel, macOS ARM
   - Rename binary from `context-os` to `tastematter`
   - Upload to GitHub releases
   - Location: `apps/tastematter/.github/workflows/release.yml`

4. **Test first release**
   - Tag: `git tag v0.1.0-alpha.1 && git push origin v0.1.0-alpha.1`
   - Verify GitHub Actions builds
   - Test install scripts on fresh machine

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/query.rs]] | Query engine with new commands | Modified |
| [[core/src/types.rs]] | Type definitions for new commands | Modified |
| [[core/src/main.rs]] | CLI command definitions | Modified |
| [[~/.context-os/bin/tastematter.cmd]] | User-facing wrapper | Created |
| [[scripts/install/fix-path.ps1]] | PATH fix script | Created |
| [[~/.claude/plans/synchronous-coalescing-harbor.md]] | Full distribution plan | Created |

## Test State

All 5 new query commands verified working:

```bash
# Search
tastematter query search "SKILL" --limit 5
# Returns: 5 files with access counts

# File
tastematter query file "SKILL.md" --limit 5
# Returns: sessions that touched file

# Co-access
tastematter query co-access "App.svelte" --limit 5
# Returns: related files with PMI scores

# Verify
tastematter query verify q_807c97
# Returns: MATCH status with original timestamp

# Receipts
tastematter query receipts --limit 5
# Returns: 823 total receipts in ledger
```

## For Next Agent

**Context Chain:**
- Previous: [[29_2026-01-19_PARITY_TEST_SUITE_COMPLETE]] (Rust port complete, 691 tests)
- This package: CLI distribution architecture planned, Phase 1 PATH fix done
- Next: Create install scripts and GitHub Actions workflow

**Start here:**
1. Read the plan file: `~/.claude/plans/synchronous-coalescing-harbor.md`
2. The full install scripts and workflow YAML are in that plan
3. Create the files in `apps/tastematter/scripts/install/` and `.github/workflows/`

**Critical context:**
- Repo for releases: `jacob-dietle/tastematter` (confirmed by user)
- Binary naming: Build as `context-os`, distribute as `tastematter`
- User's PATH is now fixed (restart terminal to verify)

**Test after creating files:**
```bash
# Verify PATH works (after terminal restart)
tastematter --version
tastematter query flex --time 7d --limit 5
```

**Do NOT:**
- Change the context-query skill to use `context-os` - user wants `tastematter` name
- Worry about observability-engineering skill - not relevant to CLI distribution
- Touch the Cargo.toml [[bin]] name yet - handle at release packaging time

**Key files to create:**
1. `apps/tastematter/scripts/install/install.ps1` (~40 lines)
2. `apps/tastematter/scripts/install/install.sh` (~35 lines)
3. `apps/tastematter/.github/workflows/release.yml` (~50 lines)
4. Update `apps/tastematter/README.md` with install section
