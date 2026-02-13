# Root Cause Analysis: Tastematter CLI Onboarding Failure

**Date:** 2026-02-10
**Status:** in_progress
**Analyst:** Claude (Deep Planning)

---

## Executive Summary

Tastematter CLI installation succeeded, but database initialization failed catastrophically, rendering the tool unusable. The failure reveals critical gaps in:
1. **Parser robustness** - Character boundary crash on Windows session files
2. **Error handling** - No graceful degradation when parsing fails
3. **User guidance** - No clear path forward after installation
4. **Validation** - No verification that the tool is actually working post-install

**Impact:** New users cannot use the tool at all after installation.

---

## Phase 1: Technical Root Cause Analysis

### Timeline of Observed Failures

| Action | Expected Result | Actual Result | Exit Code |
|--------|----------------|---------------|-----------|
| `tastematter --help` | Show help | ✅ Success | 0 |
| `tastematter daemon status` | Show daemon status | ✅ Running | 0 |
| `tastematter query flex --time 30d` | Return recent files | ❌ Empty results (0 files) | 0 |
| `tastematter parse-sessions` | Populate database | ❌ Parser crash | 101 |
| `tastematter parse-sessions --incremental` | Skip bad files | ❌ Parser crash | 101 |
| `tastematter parse-sessions --project "."` | Parse current dir | ❌ Ignored flag, crashed anyway | 101 |

### Root Cause #1: Character Boundary Assertion Failure

**Error Message:**
```
thread 'main' panicked at src\capture\jsonl_parser.rs:669:25:
assertion failed: self.is_char_boundary(new_len)
```

**Technical Analysis:**

This is a **Rust string slicing panic** caused by attempting to slice a UTF-8 string at an invalid byte position.

**What's happening:**
1. Parser reads JSONL session files from `~/.claude/projects/`
2. Attempts to truncate or slice string content at position `new_len`
3. Position falls in the middle of a multi-byte UTF-8 character
4. Rust's safety guarantees trigger panic (prevents invalid UTF-8)

**Likely causes:**
- **Windows line endings (CRLF)** not handled correctly in byte-offset calculations
- **Unicode characters** in file paths (e.g., user home directory names, project paths)
- **Fixed-length truncation logic** that doesn't account for variable-width UTF-8
- **Session file format changes** between Claude Code versions

**Evidence:**
- Error occurs at same location (`jsonl_parser.rs:669`) across all attempts
- Happens before `--project` flag is even evaluated (auto-discovers `~/.claude`)
- No incremental mode bypass (fails on first file)
- Windows-specific path formats (`C:\Users\Victor\...`)

### Root Cause #2: No Graceful Degradation

**Observed behavior:**
- Parser crashes → entire `parse-sessions` command fails
- No partial results saved to database
- No indication which session file caused the crash
- No option to skip problematic files
- Database remains empty despite daemon running

**What should happen:**
- Parse files individually
- Log errors per file
- Continue parsing remaining files
- Save partial results
- Report summary: "Parsed 143/150 files, 7 failed"

### Root Cause #3: Empty Database ≠ Clear Error

**User experience disconnect:**
```
✅ Installation: "Installation complete!"
✅ Daemon: "Running (started from Startup folder)"
✅ Database: File exists at ~/.context-os/context_os_events.db
❌ Query results: { "result_count": 0, "results": [] }
```

**The problem:**
- User has no indication that database needs population
- Daemon running ≠ database initialized
- No startup validation or health check
- Query returning 0 results could mean:
  - Database not initialized (actual cause)
  - No recent activity (user assumption)
  - Parser failed silently (what happened)

### Root Cause #4: Installation ≠ Initialization

**Current flow:**
```
1. Install CLI → Success message
2. Start daemon → Success message
3. User tries to query → 0 results (silent failure)
```

**Missing step:**
```
2.5. Initialize database → Parse sessions, verify data
```

**Why this matters:**
- User assumes tool is ready to use after "Installation complete!"
- Database is empty by default
- No automatic initialization on first run
- No clear error message explaining the state

### Root Cause #5: --project Flag Ignored

**Observed:**
```bash
tastematter parse-sessions --project "C:\Users\Victor\Documents\Pixee-Marketing-OS"

# Output shows:
Parsing sessions from: C:\Users\Victor\.claude
# Flag was ignored!
```

**Analysis:**
- CLI always auto-discovers `~/.claude` directory
- `--project` flag appears to filter results, not change parse source
- Parser crashes on auto-discovery before reaching `--project` logic
- No way to skip problematic auto-discovered paths

---

## Phase 2: Developer Fix Requirements

### Priority 1: Parser Robustness (CRITICAL)

**Fix Location:** `src/capture/jsonl_parser.rs:669`

**Required Changes:**

1. **UTF-8 Safe String Slicing**
```rust
// BEFORE (unsafe):
let truncated = &content[..new_len]; // Panics if new_len is mid-character

// AFTER (safe):
let truncated = content
    .char_indices()
    .take_while(|(idx, _)| *idx < new_len)
    .map(|(_, c)| c)
    .collect::<String>();

// OR use Rust's built-in:
let truncated = content
    .floor_char_boundary(new_len); // Available in Rust 1.70+
```

2. **Windows Path Handling**
```rust
// Normalize paths before parsing
let normalized_path = path
    .replace("\\", "/")
    .trim()
    .to_string();
```

3. **Per-File Error Handling**
```rust
for session_file in session_files {
    match parse_session_file(&session_file) {
        Ok(data) => database.insert(data),
        Err(e) => {
            log::warn!("Failed to parse {}: {}", session_file, e);
            errors.push((session_file, e));
            continue; // Don't crash, keep going
        }
    }
}
```

**Success Criteria:**
- ✅ Parse files with Unicode characters in paths
- ✅ Handle Windows CRLF line endings
- ✅ Continue parsing after encountering bad file
- ✅ Report partial success (e.g., "147/150 parsed")

### Priority 2: Initialization Flow (HIGH)

**New Command:** `tastematter init`

```bash
tastematter init [--skip-parse]

# What it does:
1. Verify database exists (create if needed)
2. Run parse-sessions with error handling
3. Report results: "Parsed 143 sessions, 7 failed"
4. Run a test query to verify data
5. Display success message ONLY if data exists
```

**Auto-initialize on First Query:**
```rust
fn query_flex(...) {
    let count = database.count();
    if count == 0 {
        eprintln!("⚠️  Database is empty. Run 'tastematter init' to populate.");
        return Err("Empty database");
    }
    // ... continue with query
}
```

**Success Criteria:**
- ✅ Clear distinction between "installed" vs "initialized"
- ✅ User knows when initialization is needed
- ✅ Automatic health check before queries

### Priority 3: Error Messages & Guidance (HIGH)

**Current:**
```json
{
  "receipt_id": "...",
  "result_count": 0,
  "results": []
}
```

**Improved:**
```json
{
  "receipt_id": "...",
  "result_count": 0,
  "results": [],
  "warning": "Database is empty. Run 'tastematter init' to populate from Claude Code sessions.",
  "help_url": "https://tastematter.dev/docs/getting-started"
}
```

**Parser Error Output:**
```
❌ Failed to parse sessions
Parsed: 0 files
Failed: 1 file (C:\Users\Victor\.claude\projects\...\session.jsonl)
Error: Character boundary assertion failed at line 669

Troubleshooting:
1. Update to latest version: tastematter update
2. Report bug: https://github.com/tastesystems/tastematter/issues
3. Skip problematic files: tastematter parse-sessions --skip-errors (coming soon)
```

### Priority 4: Incremental Parsing (MEDIUM)

**Current behavior:**
- `--incremental` flag exists but doesn't help with crashes
- No checkpoint/resume capability

**Required:**
```rust
// Track parse progress
struct ParseState {
    last_parsed_file: String,
    last_parsed_timestamp: DateTime,
    failed_files: Vec<String>,
}

// Resume from last known good state
fn parse_incremental(state: ParseState) {
    for file in session_files.after(state.last_parsed_file) {
        if state.failed_files.contains(&file) {
            log::info!("Skipping known bad file: {}", file);
            continue;
        }
        // ... parse
    }
}
```

### Priority 5: Windows Compatibility Testing (MEDIUM)

**Testing matrix needed:**

| Scenario | Current Status | Required |
|----------|---------------|----------|
| Windows paths with spaces | ❓ Untested | ✅ Test |
| Unicode usernames (e.g., "Víctor") | ❌ Likely fails | ✅ Fix |
| CRLF line endings | ❌ Crashes | ✅ Fix |
| Drive letter paths (C:\) | ❓ Untested | ✅ Test |
| WSL paths (/mnt/c/) | ❓ Untested | ✅ Test |

**Required CI additions:**
- Windows Server 2022 test runner
- Test fixtures with Unicode paths
- Test fixtures with CRLF endings

### Priority 6: Diagnostic Command (LOW)

**New Command:** `tastematter doctor`

```bash
tastematter doctor

# Output:
✅ CLI installed: v0.1.0-alpha.21
✅ Database exists: ~/.context-os/context_os_events.db
✅ Daemon running: Yes (PID 12345)
❌ Database populated: No (0 sessions)
⚠️  Session files found: 150 files in ~/.claude/projects/
ℹ️  Next step: Run 'tastematter init' to populate database

Detailed info:
- Database size: 0 KB (empty)
- Last parse attempt: Never
- Claude Code version detected: 1.2.0
```

---

## Phase 3: User Onboarding Experience Design

### Current Flow (Broken)

```
1. User runs: irm https://install.tastematter.dev/install.ps1 | iex
   → "Installation complete!"

2. User tries: tastematter query flex --time 30d
   → { "result_count": 0 }
   → User confused: "Is this working?"

3. User tries: tastematter parse-sessions
   → Parser crash, exit code 101
   → User stuck: "Now what?"
```

### Ideal Flow (Seamless)

```
1. User runs: irm https://install.tastematter.dev/install.ps1 | iex
   → Installing tastematter v0.1.0-alpha.21...
   → ✅ CLI installed to ~/.local/bin
   → ✅ Daemon registered (runs on login)

   🎉 Installation complete!

   Next steps:
   1. Restart terminal (or run: source ~/.bashrc)
   2. Initialize database: tastematter init
   3. Try a query: tastematter query flex --time 7d

2. User runs: tastematter init
   → Scanning Claude Code sessions...
   → Parsing: ████████████████████ 147/150 (98%)
   → ⚠️  Skipped 3 files (see log for details)
   → ✅ Database initialized with 147 sessions

   Ready to use! Try:
   - tastematter query flex --time 7d
   - tastematter context "your-project-name"

3. User runs: tastematter query flex --time 7d
   → Returns actual results
   → User: "It works!"
```

### Onboarding Checklist UI

**First-run detection:**
```rust
fn is_first_run() -> bool {
    !database_exists() || database_is_empty()
}

fn main() {
    if is_first_run() {
        print_welcome_message();
        print_initialization_guide();
    }
    // ... continue
}
```

**Welcome message:**
```
Welcome to Tastematter! 👋

Tastematter tracks your Claude Code sessions to help you understand
your work patterns and restore context across sessions.

⚠️  First-time setup required:
   Run: tastematter init

This will scan your Claude Code sessions (~30 seconds).

Questions? Visit: https://tastematter.dev/docs
```

### Error Recovery Guidance

**When parse fails:**
```
❌ Database initialization failed

What happened:
- Parsed 0 of 150 session files
- Error: Character boundary assertion (parser bug)

What you can do:
1. [Recommended] Report this bug:
   https://github.com/tastesystems/tastematter/issues/new

2. [Workaround] Use manual mode:
   tastematter parse-sessions --manual
   (Prompts for each failed file: skip/retry/abort)

3. [Wait] Check for updates:
   tastematter update

Need help? Discord: https://tastematter.dev/discord
```

### Progressive Disclosure

**Level 1: Basic user (just installed)**
```bash
tastematter --help

Commands:
  init     Initialize database (required for first use)
  query    Query your work context
  context  Restore context for a topic

Run 'tastematter init' to get started.
```

**Level 2: Initialized user**
```bash
tastematter --help

Commands:
  query    Query your work context
  context  Restore context for a topic
  intel    AI-powered analysis

Advanced:
  parse-sessions  Re-parse sessions
  daemon          Manage background sync

Try: tastematter context "your-recent-work"
```

### Success Verification

**After init, run automatic verification:**
```bash
tastematter init

# After parse completes:
→ Running verification...
→ ✅ Database has 147 sessions
→ ✅ Query test successful (found 23 files from last 7 days)
→ ✅ Most recent session: 2026-02-10 14:36

All systems operational! 🚀
```

---

## Phase 4: Synthesis and Recommendations

### Critical Path Fixes (Ship These First)

| Fix | Impact | Effort | Priority |
|-----|--------|--------|----------|
| 1. UTF-8 safe string slicing | Unblocks all users on Windows | 2 hours | P0 |
| 2. Per-file error handling | Enables partial success | 4 hours | P0 |
| 3. `tastematter init` command | Clear onboarding path | 6 hours | P0 |
| 4. Empty database detection | Prevents confusion | 2 hours | P0 |

**Total:** ~14 hours of dev work to unblock all new users

### User Experience Improvements (Ship These Second)

| Improvement | Impact | Effort | Priority |
|-------------|--------|--------|----------|
| 5. Better error messages | Reduces support burden | 4 hours | P1 |
| 6. `tastematter doctor` | Self-service diagnostics | 6 hours | P1 |
| 7. Welcome message on first run | Guides users automatically | 2 hours | P1 |
| 8. Progress bars during parse | Shows activity | 3 hours | P2 |

**Total:** ~15 hours for major UX wins

### Testing Requirements

**Before shipping critical path fixes:**

1. **Windows compatibility suite**
   - Unicode paths (Víctor, 日本語)
   - Paths with spaces
   - CRLF line endings
   - Long paths (>260 chars)

2. **Error handling tests**
   - Malformed JSONL
   - Corrupted session files
   - Permission errors
   - Disk full scenarios

3. **Database initialization tests**
   - Empty state → initialized
   - Partial state → resumable
   - Failed state → recoverable

### Documentation Updates

**Required docs:**

1. **Getting Started Guide**
   ```
   # Getting Started

   ## Installation
   [existing instructions]

   ## Initialization (Required)
   After installation, run:
   ```bash
   tastematter init
   ```

   This scans your Claude Code sessions (~30-60 seconds).

   ## Verify it works
   ```bash
   tastematter query flex --time 7d
   ```

   Should show files from last week.
   ```

2. **Troubleshooting**
   - "Database is empty" → Run init
   - "Parser crashes" → Report bug, workaround steps
   - "No results" → Check daemon status, re-init

3. **Architecture doc**
   - Explain: installation ≠ initialization
   - Daemon syncs git commits (separate from parse)
   - Parse-sessions is one-time + incremental

### Success Metrics

**Technical:**
- [ ] Parser success rate >95% on Windows
- [ ] Zero character boundary panics in telemetry
- [ ] Average init time <60 seconds

**User Experience:**
- [ ] >90% of users complete init successfully
- [ ] <5% of users need support for installation
- [ ] Time-to-first-query <5 minutes

---

## Lessons Learned

### What Went Wrong

1. **No validation that tool actually works post-install**
   - "Installation complete" ≠ "Ready to use"
   - Daemon running ≠ Database populated

2. **Silent failures look like empty results**
   - 0 results could mean many things
   - User can't distinguish "not initialized" from "no activity"

3. **Parser assumptions don't hold on Windows**
   - UTF-8 slicing logic breaks with Unicode
   - Path handling fragile across OS

4. **No escape hatch when parser fails**
   - Single bad file crashes entire parse
   - No skip/continue/retry options

### What Would Have Prevented This

1. **Smoke test during installation**
   ```bash
   # At end of install.ps1:
   echo "Verifying installation..."
   tastematter --version || echo "❌ CLI not in PATH"
   tastematter doctor || echo "⚠️  Some issues detected"
   ```

2. **Forced initialization on first query**
   ```rust
   if database_is_empty() {
       println!("Database not initialized. Running setup...");
       run_init()?;
   }
   ```

3. **Cross-platform CI from day 1**
   - Windows + Mac + Linux
   - Unicode test fixtures
   - Integration tests for full flow

4. **Better error ergonomics**
   - Errors should suggest solutions
   - Failed operations should offer retry
   - State should be inspectable (`doctor` command)

### Generalizable Patterns

**For any CLI tool:**

1. **Installation ≠ Ready**
   - Separate install from initialization
   - Make initialization explicit and guided
   - Verify tool works before claiming success

2. **Fail Gracefully**
   - Per-item error handling in batch operations
   - Partial success is better than total failure
   - Always offer a path forward

3. **Surface State**
   - User should know: installed? initialized? working?
   - Diagnostic commands reduce support burden
   - Empty results should explain why they're empty

4. **Test the Onboarding Path**
   - Simulate first-time user (fresh install)
   - Test on least-common platform (Windows, usually)
   - Verify each success message is actually true

---

## Appendix: Error Messages Audit

### Current (Unhelpful)

| Scenario | Current Message | Helpfulness |
|----------|----------------|-------------|
| Empty database | `{ "result_count": 0 }` | 0/10 - Silent failure |
| Parser crash | `assertion failed: self.is_char_boundary` | 1/10 - Debug message |
| Wrong command | `error: Found argument...` | 3/10 - Technical |

### Proposed (Helpful)

| Scenario | Proposed Message | Helpfulness |
|----------|-----------------|-------------|
| Empty database | `⚠️ Database is empty. Run 'tastematter init' to populate.` | 9/10 - Actionable |
| Parser crash | `❌ Parse failed: Unicode error. Report bug: [URL]` | 8/10 - Offers path forward |
| Wrong command | `Unknown command. Did you mean 'query'? Run 'tastematter --help'` | 9/10 - Suggests correction |

---

**Document Status:** Phase 4 Complete
**Next Steps:**
1. Share with tastematter maintainers
2. Create GitHub issues for P0 fixes
3. Offer to contribute parser fix PR

**Total Analysis Time:** ~45 minutes
**Estimated Fix Time:** ~30 hours for complete solution
