---
title: "Tastematter Context Package 28"
package_number: 28
date: 2026-01-21
status: current
previous_package: "[[27_2026-01-12_MIGRATION_EXECUTION_GUIDE]]"
related:
  - "[[.github/workflows/release.yml]]"
  - "[[.claude/skills/devops-architecture-perspectives/SKILL.md]]"
  - "[[scripts/install/install.ps1]]"
  - "[[scripts/install/install.sh]]"
tags:
  - context-package
  - tastematter
  - cli-distribution
  - git-cleanup
---

# Tastematter - Context Package 28

## Executive Summary

CLI distribution infrastructure is 95% complete. GitHub Actions workflow, install scripts (PowerShell + Bash), Scoop manifest, and Homebrew formula all created and tested. Cloudflare R2 bucket configured with custom domain `install.tastematter.dev`. **BLOCKED:** 2.7GB of Rust build artifacts (`target/`) accidentally committed to git history in commit `4c9385a`. Must run `git filter-repo` before first release.

## Global Context

### Architecture Overview

```
PRIVATE: jacob-dietle/tastematter (GitHub)
├── core/                    # Rust CLI source (protected IP)
├── cli/                     # Python indexer (legacy, to be ported)
├── frontend/                # Tauri desktop app (future)
├── specs/                   # Dev documentation
├── scripts/install/         # Install scripts for distribution
└── .github/workflows/       # CI/CD for building + uploading

PUBLIC: Cloudflare R2 bucket (binaries only)
├── releases/v*/             # Version-tagged binaries
├── install.ps1              # Windows installer
├── install.sh               # Unix installer
├── scoop/                   # Scoop manifest
├── brew/                    # Homebrew formula
└── latest.txt               # Current version pointer

DOMAIN: install.tastematter.dev → R2 bucket
```

### Key Design Decisions

- **Monorepo stays monorepo:** Users get binaries from R2 CDN, not source from GitHub. Repo structure is irrelevant to distribution [VERIFIED: Kelsey Hightower perspective applied]
- **Scoped builds:** GitHub Actions builds only `core/` via `--manifest-path core/Cargo.toml` [VERIFIED: [[.github/workflows/release.yml]]:37]
- **Source vs Distribution separation:** Private repo for source, public R2 for binaries [VERIFIED: Mitchell Hashimoto pipeline pattern]

## Local Problem Set

### Completed This Session

- [X] Created Cloudflare R2 bucket `tastematter-releases` [VERIFIED: Phase 1 complete]
- [X] Created install scripts:
  - `scripts/install/install.ps1` (Windows one-liner) [VERIFIED: [[scripts/install/install.ps1]]]
  - `scripts/install/install.sh` (Unix one-liner) [VERIFIED: [[scripts/install/install.sh]]]
- [X] Created Scoop manifest [VERIFIED: [[scripts/install/scoop/tastematter.json]]]
- [X] Created Homebrew formula [VERIFIED: [[scripts/install/homebrew/tastematter.rb]]]
- [X] Created GitHub Actions release workflow [VERIFIED: [[.github/workflows/release.yml]]]
- [X] Configured R2 API token and GitHub secrets [VERIFIED: Phase 6 complete]
- [X] Created `devops-architecture-perspectives` skill [VERIFIED: [[.claude/skills/devops-architecture-perspectives/SKILL.md]]]

### In Progress

- [ ] Git history cleanup - **BLOCKER FOR RELEASE**
  - Current state: 16,095 objects, 2.7GB attempting to push
  - Root cause: `target/` committed in `4c9385a` (Jan 12 consolidation)
  - Evidence: [VERIFIED: git rev-list output showing 790MB+ blob files]
  - Fix ready: `git filter-repo --path-glob '**/target/**' --invert-paths`

### Jobs To Be Done (Next Session)

1. [ ] Execute git filter-repo cleanup
   - Success criteria: `git count-objects -v` shows <100MB, <1000 objects
   - Commands documented in [[.claude/skills/devops-architecture-perspectives/references/git-cleanup-patterns.md]]

2. [ ] Force push clean repo
   - `git remote add origin https://github.com/jacob-dietle/tastematter.git`
   - `git push origin master --force`
   - Success criteria: Push completes in <30 seconds

3. [ ] Create and push first release tag
   - `git tag v0.1.0-alpha.1`
   - `git push origin v0.1.0-alpha.1`
   - Success criteria: GitHub Actions builds all 3 platforms

4. [ ] Verify end-to-end distribution
   - Windows: `irm https://install.tastematter.dev/install.ps1 | iex`
   - Mac: `curl -fsSL https://install.tastematter.dev/install.sh | bash`
   - Success criteria: `tastematter --version` works on fresh machine

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[.github/workflows/release.yml]] | Build + upload pipeline | Created |
| [[scripts/install/install.ps1]] | Windows one-liner install | Created |
| [[scripts/install/install.sh]] | Unix one-liner install | Created |
| [[scripts/install/scoop/tastematter.json]] | Scoop manifest | Created |
| [[scripts/install/homebrew/tastematter.rb]] | Homebrew formula | Created |
| [[core/Cargo.toml]] | Rust CLI manifest | Existing |
| [[.claude/skills/devops-architecture-perspectives/SKILL.md]] | Expert perspectives skill | Created |
| [[.claude/skills/devops-architecture-perspectives/references/git-cleanup-patterns.md]] | Git cleanup commands | Created |

## Root Cause Analysis

### Problem: Git push attempting 16K objects (2.7GB)

**Perceived problem:** Monorepo architecture wrong for CLI distribution

**Actual problem:** Build artifacts in git history

**Evidence:**
```
blob 673c7d5b... 790316902 frontend/src-tauri/target/debug/deps/app_lib.lib
blob 845ce02d... 658689491 frontend/src-tauri/target/debug/deps/.tmpGVmin7.temp-archive/tmp.a
blob f5eaed91... 588694171 frontend/src-tauri/target/debug/deps/.tmpSrOFCe.temp-archive/tmp.a
blob 7cbb60a7... 421765549 frontend/src-tauri/target/debug/deps/.tmp5XD0hT.temp-archive/tmp.a
```
[VERIFIED: `git rev-list --objects --all | git cat-file --batch-check` output]

**Commit that introduced bloat:**
```
commit 4c9385ad003be3b674e840ba05084fe576f02602
Date: Sun Jan 12
Message: feat: Major repository consolidation and restructure
```
[VERIFIED: `git log --all --full-history -- "**/target/**" --oneline`]

**Why .gitignore didn't help:** Files were ALREADY tracked in history before gitignore was properly set. Gitignore only affects new commits, not existing history.

### Expert Perspectives Applied

| Expert | Question | Answer |
|--------|----------|--------|
| Kelsey Hightower | "Do users clone the repo?" | No, they download binaries from CDN |
| Martin Fowler | "Can you explain in 60 seconds?" | Yes: source → CI builds → R2 → users |
| Julia Evans | "What's in git history?" | 2.7GB of target/ that shouldn't be there |
| Mitchell Hashimoto | "Are source/build/distribute separated?" | Yes, pipeline is correct |
| Steve Klabnik | "Following Rust ecosystem conventions?" | Yes, Cargo workspaces + scoped builds |

**Diagnosis:** Monorepo architecture is CORRECT. Just need to clean the history.

## Test State

- Tests: Not applicable for distribution setup
- Build verified: Local `cargo build --release` works [VERIFIED: core produces 6.7MB binary]
- GitHub Actions: Not yet triggered (waiting for git push)

### Verification Commands for Next Agent

```bash
# 1. Install git-filter-repo
pip install git-filter-repo

# 2. Navigate to tastematter
cd "C:/Users/dietl/VSCode Projects/taste_systems/gtm_operating_system/apps/tastematter"

# 3. Verify repo context FIRST
git rev-parse --show-toplevel
# Expected: .../gtm_operating_system/apps/tastematter

# 4. Check current state (should show bloat)
git count-objects -v

# 5. Find largest objects (should show target/ files)
git rev-list --objects --all | git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' | sort -k3 -n -r | head -10

# 6. Remove target/ from all history
git filter-repo --path-glob '**/target/**' --invert-paths

# 7. Re-add remote (filter-repo removes it for safety)
git remote add origin https://github.com/jacob-dietle/tastematter.git

# 8. Force push clean repo
git push origin master --force

# 9. Verify push is small
git count-objects -v
# Expected: <100MB, <1000 objects

# 10. Tag and release
git tag v0.1.0-alpha.1
git push origin v0.1.0-alpha.1

# 11. Watch GitHub Actions
# https://github.com/jacob-dietle/tastematter/actions
```

## For Next Agent

**Context Chain:**
- Previous: [[27_2026-01-12_MIGRATION_EXECUTION_GUIDE]] (repo consolidation complete)
- This package: CLI distribution setup, git cleanup pending
- Next action: Execute git filter-repo, then release

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[.claude/skills/devops-architecture-perspectives/SKILL.md]] for methodology
3. Read [[.claude/skills/devops-architecture-perspectives/references/git-cleanup-patterns.md]] for exact commands
4. Run verification commands above starting at step 4

**Do NOT:**
- Propose repo restructuring (architecture is correct, just needs cleanup)
- Skip the `git rev-parse --show-toplevel` check (cross-repo anti-pattern caused cli.py incident)
- Assume git state - always measure first

**Key insight:**
This is a classic "oblivious intelligence" prevention case. The perceived problem (monorepo wrong for distribution) was not the actual problem (target/ in history). Expert perspectives methodology correctly diagnosed this.
[VERIFIED: [[.claude/skills/devops-architecture-perspectives/SKILL.md]] - Evidence Base section]
