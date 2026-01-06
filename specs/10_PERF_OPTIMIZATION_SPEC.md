# Tastematter Performance Optimization Spec

**Date:** 2026-01-05
**Status:** Ready for Implementation
**Prerequisite:** Context Package 02 (02_2026-01-05_PERF_OPTIMIZATION_HANDOFF.md)
**Test Baseline:** 236 tests passing

---

## Executive Summary

Six verified performance optimizations across two phases. Phase 1 contains trivial fixes (~10 min). Phase 2 contains medium complexity fixes (~30 min).

**Major win already completed:** WorkstreamView N→1 API call refactor (previous session).

---

## Phase 1: Quick Wins (3 fixes, ~10 min total)

### Fix 1: App.svelte Error Refetch Loop

**File:** `src/App.svelte`
**Line:** 47
**Severity:** HIGH (causes infinite API calls on error)

**Current Code:**
```javascript
const hasInitialData = untrack(() => filesStore.data !== null || filesStore.error !== null);
```

**Problem:**
- If `filesStore.error` is set, `hasInitialData` becomes true
- This triggers `filesStore.fetch()` on line 49
- If fetch keeps failing, error stays set → infinite refetch loop

**Fix:**
```javascript
const hasInitialData = untrack(() => filesStore.data !== null);
```

**Rationale:** The intent is "only refetch if we've loaded data once" - error state shouldn't trigger refetch.

**Test Strategy:** Existing tests cover normal flow. No new tests needed (behavior-preserving fix).

---

### Fix 2: WorkstreamView colorScale Memoization

**File:** `src/lib/components/WorkstreamView.svelte`
**Lines:** 60-64
**Severity:** MEDIUM (O(n²) → O(n) complexity)

**Current Code:**
```javascript
function colorScale(count: number): string {
  const maxCount = Math.max(...sessions.map(s => s.total_accesses), 1);
  const intensity = Math.round((count / maxCount) * 100);
  return `rgb(${100 - intensity}, ${100 + intensity}, ${150})`;
}
```

**Problem:**
- `colorScale` is called once per SessionCard (N times)
- Each call recalculates `maxCount` by iterating all sessions (O(n) each)
- Total complexity: O(n²)

**Fix:**
```javascript
// Add after line 17 (expandedSessions declaration):
const maxAccessCount = $derived(Math.max(...sessions.map(s => s.total_accesses), 1));

// Replace colorScale function:
function colorScale(count: number): string {
  const intensity = Math.round((count / maxAccessCount) * 100);
  return `rgb(${100 - intensity}, ${100 + intensity}, ${150})`;
}
```

**Rationale:** Pre-compute maxAccessCount once using Svelte 5's `$derived` rune. Recalculates only when `sessions` changes.

**Test Strategy:** Existing SessionCard rendering tests cover colorScale usage. No new tests needed.

---

### Fix 3: Git Store Background Status Refresh

**File:** `src/lib/stores/git.svelte.ts`
**Lines:** 35-36 (in `pull()`) and 57 (in `push()`)
**Severity:** MEDIUM (0.5-2s UX improvement)

**Current Code (pull function, line 35-36):**
```javascript
if (lastOperation.success) {
  await fetchStatus();  // Blocks UI for 0.5-2s
}
```

**Current Code (push function, line 57):**
```javascript
if (lastOperation.success) {
  await fetchStatus();  // Blocks UI for 0.5-2s
}
```

**Problem:**
- After successful pull/push, user sees "Pulling..." / "Pushing..." for extra 0.5-2s
- The status refresh is not essential to the operation completing

**Fix (pull function):**
```javascript
if (lastOperation.success) {
  fetchStatus();  // Fire-and-forget - updates in background
}
```

**Fix (push function):**
```javascript
if (lastOperation.success) {
  fetchStatus();  // Fire-and-forget - updates in background
}
```

**Rationale:** User sees success immediately. Status updates in background. UI feels faster.

**Test Strategy:** Existing pull/push tests verify operation success. Status refresh is an implementation detail.

---

## Phase 2: Medium Wins (3 fixes, ~30 min total)

### Fix 4: Request Deduplication in Stores

**Files:** `src/lib/stores/files.svelte.ts`, `src/lib/stores/timeline.svelte.ts`, `src/lib/stores/context.svelte.ts`
**Severity:** MEDIUM (prevents 2-5 duplicate requests)

**Problem:**
- No request deduplication exists
- Rapid clicks on refresh → multiple in-flight requests
- Last response wins, but wastes bandwidth/compute

**Pattern to Implement:**
```typescript
// In each store:
let currentRequestId = $state(0);

async function fetch() {
  const requestId = ++currentRequestId;
  loading = true;
  error = null;

  try {
    const result = await apiCall();

    // Only update state if this is still the current request
    if (requestId === currentRequestId) {
      data = result;
    }
  } catch (e) {
    if (requestId === currentRequestId) {
      error = e as CommandError;
    }
  } finally {
    if (requestId === currentRequestId) {
      loading = false;
    }
  }
}
```

**Files to Modify:**
1. `files.svelte.ts` - `fetch()` function
2. `timeline.svelte.ts` - `fetch()` function
3. `context.svelte.ts` - `refreshChains()` function

**Test Strategy:** Add unit test for each store verifying old requests are ignored.

---

### Fix 5: TimelineRow Date Pre-Computation

**File:** `src/lib/components/TimelineRow.svelte`
**Lines:** 29-38
**Severity:** LOW (reduces Date object creation)

**Current Code:**
```javascript
function isWeekend(date: string): boolean {
  const d = new Date(date);  // Created per-cell!
  const day = d.getDay();
  return day === 0 || day === 6;
}

function isToday(date: string): boolean {
  const today = new Date().toISOString().split('T')[0];  // Created per-cell!
  return date === today;
}
```

**Problem:**
- Called for every cell (files × dates)
- Creates Date objects per-cell instead of per-date
- Redundant computation

**Fix:**
```javascript
// Pre-compute classifications once per render
const todayStr = new Date().toISOString().split('T')[0];

const dateClassifications = $derived(
  dates.reduce((acc, date) => {
    const d = new Date(date);
    const day = d.getDay();
    acc[date] = {
      isWeekend: day === 0 || day === 6,
      isToday: date === todayStr
    };
    return acc;
  }, {} as Record<string, { isWeekend: boolean; isToday: boolean }>)
);

// In template, replace:
// class:weekend={isWeekend(date)}
// class:today={isToday(date)}
// With:
// class:weekend={dateClassifications[date]?.isWeekend}
// class:today={dateClassifications[date]?.isToday}
```

**Test Strategy:** Existing TimelineRow tests cover rendering. Optimization is internal.

---

### Fix 6: Rust Git Command Consolidation

**File:** `src-tauri/src/commands.rs`
**Lines:** 181-221 (`git_status` function) and 223-255 (`get_ahead_behind` function)
**Severity:** MEDIUM (4 spawns → 1 spawn, saves 150-250ms)

**Current Implementation:**
```rust
// 4 separate process spawns:
1. git rev-parse --abbrev-ref HEAD           // Get branch
2. git rev-list --count @{u}..HEAD           // Get ahead count
3. git rev-list --count HEAD..@{u}           // Get behind count
4. git status --porcelain                    // Get file status
```

**Fix - Use Single Command:**
```rust
// 1 process spawn:
git status -sb --porcelain

// Output format:
// ## main...origin/main [ahead 2, behind 1]
//  M file.txt
// ?? new.txt
```

**Implementation:**
```rust
#[command]
pub async fn git_status() -> Result<GitStatus, CommandError> {
    let output = Command::new("git")
        .args(["status", "-sb", "--porcelain"])
        .output()
        .map_err(|e| CommandError {
            code: "GIT_NOT_FOUND".to_string(),
            message: "Git not found".to_string(),
            details: Some(e.to_string()),
        })?;

    if !output.status.success() {
        return Err(CommandError {
            code: "GIT_ERROR".to_string(),
            message: "Failed to get git status".to_string(),
            details: Some(String::from_utf8_lossy(&output.stderr).to_string()),
        });
    }

    let status_str = String::from_utf8_lossy(&output.stdout);
    parse_status_sb(&status_str)
}

fn parse_status_sb(output: &str) -> Result<GitStatus, CommandError> {
    let mut lines = output.lines();

    // First line: ## branch...remote [ahead N, behind M]
    let header = lines.next().unwrap_or("");
    let (branch, ahead, behind) = parse_header(header);

    // Remaining lines: file status
    let (staged, modified, untracked, has_conflicts) = parse_file_lines(lines);

    Ok(GitStatus {
        branch,
        ahead,
        behind,
        staged,
        modified,
        untracked,
        has_conflicts,
    })
}

fn parse_header(header: &str) -> (String, u32, u32) {
    // ## main...origin/main [ahead 2, behind 1]
    // or ## main (no tracking)

    let branch = header
        .trim_start_matches("## ")
        .split("...")
        .next()
        .unwrap_or("unknown")
        .to_string();

    let mut ahead = 0u32;
    let mut behind = 0u32;

    if let Some(bracket_start) = header.find('[') {
        if let Some(bracket_end) = header.find(']') {
            let tracking = &header[bracket_start+1..bracket_end];
            for part in tracking.split(", ") {
                if part.starts_with("ahead ") {
                    ahead = part[6..].parse().unwrap_or(0);
                } else if part.starts_with("behind ") {
                    behind = part[7..].parse().unwrap_or(0);
                }
            }
        }
    }

    (branch, ahead, behind)
}
```

**Test Strategy:**
- Unit test for `parse_status_sb` with various input formats
- Integration test verifying GitStatus struct is populated correctly

---

## Implementation Order

### Phase 1 (Do First - No Dependencies)

1. **Fix 1** - App.svelte error refetch (1 line change)
2. **Fix 2** - WorkstreamView colorScale (add 1 line, modify 1 function)
3. **Fix 3** - git.svelte.ts await removal (2 line changes)

**Run tests after each fix:** `npm test`

### Phase 2 (Do After Phase 1 Complete)

4. **Fix 4** - Request deduplication (pattern in 3 files)
5. **Fix 5** - TimelineRow date pre-computation (1 file)
6. **Fix 6** - Rust git consolidation (1 file, new parser)

**Run tests after each fix:** `npm test` (frontend) + `cargo test` (backend)

---

## Success Criteria

**Phase 1 Complete When:**
- [ ] All 3 fixes implemented
- [ ] 236+ tests still passing
- [ ] No error refetch loops on API failure
- [ ] Pull/push operations feel faster (no 2s delay)

**Phase 2 Complete When:**
- [ ] All 3 additional fixes implemented
- [ ] Tests still passing (may add new tests)
- [ ] Rapid clicks don't cause duplicate requests
- [ ] Git panel loads faster (1 spawn vs 4)

---

## Verification Commands

```bash
# Verify tests pass
cd apps/tastematter && npm test

# Verify no TypeScript errors
cd apps/tastematter && npm run check

# Verify app runs
cd apps/tastematter && npm run dev
```

---

## References

- Context Package: `docs/specs/context_packages/02_2026-01-05_PERF_OPTIMIZATION_HANDOFF.md`
- Previous Session: Major WorkstreamView refactor (N→1 API calls)
- Architecture: Svelte 5 + Tauri (Rust backend)
