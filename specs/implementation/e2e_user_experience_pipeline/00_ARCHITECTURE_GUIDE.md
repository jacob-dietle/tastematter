# E2E User Experience Testing Pipeline

**Mission:** Simulate a real first-time user installing and using tastematter on every push to master, on all 3 platforms, with zero human involvement.

**Status:** SPEC
**Created:** 2026-02-10
**Skill Stack:** feature-planning, devops-architecture-perspectives, test-driven-execution, specification-driven-development

---

## Problem Statement

Current CI validates that tastematter compiles and starts. It does NOT validate that it works. Victor's experience proved this: install succeeded, daemon ran, but parser crashed on real session data (UTF-8 boundary panic). Every bug discovered so far required a human on a call walking through the failure.

**The gap:** No CI stage feeds real data through the full user workflow.

**Success metric:** Every push to master → within ~15 minutes → we know whether a first-time user on Windows/macOS/Linux can:
1. Install tastematter
2. Parse real Claude Code sessions
3. Get meaningful query results

**Secondary metric:** An AI agent evaluates the qualitative experience and produces a report.

---

## Architecture Overview (60-Second Diagram)

```
                    STAGING BUILD (existing)
                    ┌──────────────────────┐
  push to master →  │ Build 4 binaries     │
                    │ Upload to staging R2  │
                    └──────────┬───────────┘
                               │
                    E2E USER EXPERIENCE TEST (new)
                    ┌──────────▼───────────┐
                    │                      │
                    │  ┌─── Windows VM ──┐ │
                    │  │                 │ │
                    │  ├─── macOS VM ───┤ │  ← 3 real VMs in parallel
                    │  │                 │ │
                    │  ├─── Linux VM ───┤ │
                    │  │                 │ │
                    │  └─────────────────┘ │
                    │                      │
                    │  Per VM:             │
                    │  ┌─────────────────┐ │
                    │  │ 1. Setup env    │ │
                    │  │ 2. Install CC   │ │  CC = Claude Code
                    │  │ 3. Generate     │ │
                    │  │    sessions     │ │
                    │  │ 4. Install TM   │ │  TM = Tastematter
                    │  │ 5. Run TM       │ │
                    │  │ 6. Assert       │ │
                    │  │ 7. Agent eval   │ │
                    │  │ 8. Upload report│ │
                    │  └─────────────────┘ │
                    └──────────────────────┘
```

**Components:** 8 stages, 1 workflow file, 1 helper script per platform.
**Data flow is unidirectional.** Each stage feeds the next. No backflow.

---

## Stage Breakdown

### Stage 1: Environment Setup (~1 min)

**Purpose:** Fresh VM with Node.js (Claude Code dependency)

**Actions:**
- `actions/checkout@v4` (for install scripts in the repo)
- `actions/setup-node@v4` (Node.js 20+)
- Verify: `node --version`

**Why Node.js:** Claude Code CLI is an npm package. It requires Node.js 18+.

**No Rust toolchain needed** — we're testing the distributed binary, not building from source.

---

### Stage 2: Install Claude Code (~2 min)

**Purpose:** Real Claude Code CLI on the runner, configured with API key.

**Actions:**
```bash
npm install -g @anthropic-ai/claude-code
claude --version
```

**Configuration:**
- `ANTHROPIC_API_KEY` from GitHub secrets
- No other config needed — Claude Code auto-discovers environment

**Why install Claude Code (not Agent SDK library):**
- CLI generates `.claude/projects/{slug}/*.jsonl` session files — the exact data tastematter parses
- `-p` flag (headless mode) runs the full agent loop non-interactively
- Same binary that real users have — maximum fidelity

**Secret required:** `ANTHROPIC_API_KEY` (GitHub repository secret)

---

### Stage 3: Generate Real Session Data (~3-5 min)

**Purpose:** Create REAL `.claude/` session data by running Claude Code headlessly against a test project.

**Actions:**

1. Create a test project directory with sample files:
```bash
mkdir -p ~/test-project/src
echo 'def hello(): print("hello world")' > ~/test-project/src/main.py
echo '# Test Project\nA simple project for E2E testing.' > ~/test-project/README.md
echo '{"name": "test", "version": "1.0.0"}' > ~/test-project/package.json
```

2. Run multiple Claude Code sessions with varying complexity:

```bash
cd ~/test-project

# Session 1: Simple read (generates tool_use: Read)
claude -p "What files are in this project? List them." \
  --output-format json --allowedTools "Read,Bash" --max-turns 3

# Session 2: Code analysis (generates tool_use: Read, Grep)
claude -p "Read main.py and explain what it does." \
  --output-format json --allowedTools "Read,Grep" --max-turns 3

# Session 3: Code generation (generates tool_use: Write, Read)
claude -p "Add a test file for main.py using pytest." \
  --output-format json --allowedTools "Read,Write" --max-turns 5

# Session 4: Multi-tool (generates diverse tool_use patterns)
claude -p "Run 'python src/main.py' and show the output." \
  --output-format json --allowedTools "Bash,Read" --max-turns 3

# Session 5: Continue conversation (tests session linking)
claude -p "What did we just do? Summarize." \
  --output-format json --continue --max-turns 2
```

3. Verify session data was generated:
```bash
# Assert .claude directory exists and has session files
ls ~/.claude/projects/
find ~/.claude -name "*.jsonl" | wc -l
# Assert: count > 0
```

**Why 5 sessions:**
- Covers different tool use patterns (Read, Write, Bash, Grep)
- Generates session linking data (--continue)
- Creates enough data for meaningful queries
- Small enough to run in ~3 minutes

**Cost per run:** ~5 Haiku-level calls × ~$0.001 each = ~$0.005 per platform. Total ~$0.015/run. Negligible.

**Model choice:** Use `--model haiku` to minimize cost. Session data format is identical regardless of model.

---

### Stage 4: Install Tastematter (~1 min)

**Purpose:** Install tastematter EXACTLY as a real user would — via the install script from staging.

**Actions (Windows):**
```powershell
$env:TASTEMATTER_CHANNEL = "staging"
Invoke-RestMethod https://install.tastematter.dev/install.ps1 | Invoke-Expression
```

**Actions (Unix):**
```bash
TASTEMATTER_CHANNEL=staging curl -fsSL https://install.tastematter.dev/install.sh | bash
```

**Assertions:**
- Install script exits 0
- Binary exists in `~/.local/bin/tastematter`
- `tastematter --version` returns version string
- Daemon registered (install script output contains "Background sync registered")

**Captures:** Save all stdout/stderr to `$GITHUB_WORKSPACE/e2e-install-output.txt`

---

### Stage 5: Tastematter First Run (~2 min)

**Purpose:** Run tastematter through its full workflow — parse sessions, query data, restore context.

**Actions:**

```bash
export PATH="$HOME/.local/bin:$PATH"

# 5a. Parse sessions (the critical step — this is where Victor's bug was)
tastematter daemon once 2>&1 | tee $GITHUB_WORKSPACE/e2e-daemon-output.txt

# 5b. Query for recent files
tastematter query flex --time 30d --format json \
  | tee $GITHUB_WORKSPACE/e2e-query-output.json

# 5c. Context restore (the composed query)
tastematter context "test" --format json \
  | tee $GITHUB_WORKSPACE/e2e-context-output.json

# 5d. Heat command (file access frequency)
tastematter heat --format json \
  | tee $GITHUB_WORKSPACE/e2e-heat-output.json

# 5e. Chain listing
tastematter query chains --format json \
  | tee $GITHUB_WORKSPACE/e2e-chains-output.json
```

**Captures:** Every command's stdout/stderr saved to workspace files for Stage 7.

---

### Stage 6: Hard Assertions (~30 sec)

**Purpose:** Binary pass/fail gates that block release if broken.

**Assertions:**

| Command | Assertion | Rationale |
|---------|-----------|-----------|
| `daemon once` | Exit code 0, no "panicked" in stderr | Victor's exact bug |
| `query flex` | `result_count > 0` in JSON | Proves parser populated DB |
| `context` | `receipt_id` exists in JSON | Proves composed query works |
| `context` | `executive_summary.status` is valid | Proves structured output |
| `context` | `context_files_found > 0` | Proves real data flows through |
| `heat` | Returns non-empty array | Proves heat scoring works |
| `query chains` | At least 1 chain | Proves chain graph works |

**Implementation (bash):**
```bash
# Assert daemon didn't panic
if grep -qi "panicked" $GITHUB_WORKSPACE/e2e-daemon-output.txt; then
  echo "FATAL: daemon panicked during parse"
  cat $GITHUB_WORKSPACE/e2e-daemon-output.txt
  exit 1
fi

# Assert query returned results
RESULT_COUNT=$(jq '.result_count' $GITHUB_WORKSPACE/e2e-query-output.json)
if [ "$RESULT_COUNT" -lt 1 ]; then
  echo "FATAL: query returned 0 results after parsing sessions"
  exit 1
fi

# Assert context restore works
RECEIPT=$(jq -r '.receipt_id' $GITHUB_WORKSPACE/e2e-context-output.json)
if [ -z "$RECEIPT" ] || [ "$RECEIPT" = "null" ]; then
  echo "FATAL: context command returned no receipt_id"
  exit 1
fi

FILES_FOUND=$(jq '.current_state.key_metrics.context_files_found' $GITHUB_WORKSPACE/e2e-context-output.json)
if [ "$FILES_FOUND" -lt 1 ]; then
  echo "FATAL: context found 0 files"
  exit 1
fi
```

**These assertions are the release gate.** If any fail, staging is broken.

---

### Stage 7: Agent Quality Evaluation (~1-2 min)

**Purpose:** An AI agent evaluates the end-to-end experience qualitatively — catching issues that assertions can't.

**Actions:**

Feed all captured outputs to Claude Code headlessly and ask it to evaluate:

```bash
cd $GITHUB_WORKSPACE

claude -p "$(cat <<'EVAL_PROMPT'
You are evaluating the first-run experience of a CLI tool called 'tastematter'.
A fresh user just installed it on a clean machine and ran these commands.

Review ALL the outputs below and produce a quality report.

== INSTALL OUTPUT ==
$(cat e2e-install-output.txt)

== DAEMON ONCE OUTPUT ==
$(cat e2e-daemon-output.txt)

== QUERY FLEX OUTPUT ==
$(cat e2e-query-output.json)

== CONTEXT RESTORE OUTPUT ==
$(cat e2e-context-output.json)

== HEAT OUTPUT ==
$(cat e2e-heat-output.json)

== CHAINS OUTPUT ==
$(cat e2e-chains-output.json)

Evaluate:
1. Did the install succeed cleanly? Any confusing messages?
2. Did the daemon parse sessions without errors? Any warnings?
3. Did queries return meaningful results? Or empty/confusing output?
4. Were there any error messages? Were they clear and actionable?
5. Would a first-time user understand what happened at each step?
6. Any unexpected behavior, warnings, or red flags?

Rate the overall first-run experience 1-10 and explain why.
List specific issues found (if any) with severity: CRITICAL/HIGH/MEDIUM/LOW.

Output format: Markdown report.
EVAL_PROMPT
)" --output-format json --max-turns 1 \
  | jq -r '.result' > e2e-quality-report.md
```

**Upload as workflow artifact:**
```yaml
- name: Upload quality report
  uses: actions/upload-artifact@v4
  if: always()
  with:
    name: e2e-quality-report-${{ matrix.os }}
    path: |
      e2e-quality-report.md
      e2e-*.txt
      e2e-*.json
```

**This report is informational, not a release gate.** It's for you to review qualitatively. If rating drops below 7, add a warning annotation.

---

### Stage 8: Cleanup & Results

**Purpose:** Upload all artifacts, annotate workflow.

**Actions:**
- Upload quality report + all captured outputs as workflow artifacts
- If quality rating < 7: add `::warning` annotation
- Summary step: print pass/fail for each assertion

---

## Workflow Placement

**Option A (Recommended): New job in staging.yml**

Add E2E as a job that depends on `upload-staging`:

```
staging.yml:
  build → upload-staging → smoke-test (existing, fast, shallow)
                         → e2e-test (new, thorough, real data)
```

**Why same workflow:** E2E needs staging binaries available. Same secrets context. Natural dependency.

**Why separate job (not merged with smoke-test):** Different purpose, different runtime, different cost. smoke-test is fast/free. E2E costs API dollars and takes longer.

**Option B: Separate workflow (e2e.yml) triggered by workflow_run**

Only if E2E job slows down the staging feedback loop unacceptably.

---

## Secret Requirements

| Secret | Purpose | Where to Add |
|--------|---------|--------------|
| `ANTHROPIC_API_KEY` | Claude Code CLI for session generation + agent evaluation | GitHub repo secrets |
| `R2_ACCESS_KEY_ID` | Already exists (staging upload) | Already configured |
| `R2_SECRET_ACCESS_KEY` | Already exists (staging upload) | Already configured |
| `R2_ENDPOINT` | Already exists (staging upload) | Already configured |

**Only 1 new secret needed:** `ANTHROPIC_API_KEY`

---

## Cost Analysis

| Component | Cost per Run | Frequency | Monthly Estimate |
|-----------|-------------|-----------|-----------------|
| GitHub Actions (3 VMs × ~15 min) | ~$0.12 | ~30 pushes/mo | ~$3.60 |
| Claude API: 5 sessions × 3 platforms (Haiku) | ~$0.015 | ~30 pushes/mo | ~$0.45 |
| Claude API: 3 quality evals (Haiku) | ~$0.003 | ~30 pushes/mo | ~$0.09 |
| **Total** | ~$0.14/run | | **~$4.14/mo** |

**Negligible.** Less than a single debugging call with another person.

---

## Platform Matrix

| Platform | Runner | Claude Code | Install Script | Key Risk |
|----------|--------|-------------|----------------|----------|
| Windows | `windows-latest` | npm -g install | `install.ps1` (pwsh) | UTF-8 paths, CRLF, backslashes |
| macOS ARM | `macos-latest` | npm -g install | `install.sh` (bash) | aarch64 binary, launchd |
| Linux x86 | `ubuntu-latest` | npm -g install | `install.sh` (bash) | Baseline (most tested) |

**Note:** macOS Intel runner (`macos-15-intel`) used for builds but NOT for E2E — users are overwhelmingly on ARM Macs now.

---

## What This Catches That Current CI Doesn't

| Bug Class | Current CI | E2E Pipeline |
|-----------|-----------|--------------|
| UTF-8 boundary panics | NO | YES — real session data |
| Windows path handling | NO (CI tests on Linux only) | YES — Windows runner |
| Empty DB confusion | Partial (empty DB test) | YES — real data expected |
| Parser crashes on real JSONL | NO | YES — real Claude Code sessions |
| Install script failures | YES (shallow) | YES (deep — full workflow after) |
| Daemon registration | NO | YES — install script runs it |
| Query returns 0 on populated DB | NO | YES — asserts result_count > 0 |
| Context restore output quality | NO | YES — agent evaluates |
| Session chain linking | NO | YES — --continue generates linked sessions |
| Error message clarity | NO | YES — agent evaluates messages |

---

## Test-Driven Execution Plan (RED-GREEN-REFACTOR)

### RED: Create the E2E workflow, run it, expect failure

The current codebase has the UTF-8 boundary bug. Running this E2E pipeline against it WILL fail on Windows. This proves the test catches the bug.

**Expected failures:**
1. `daemon once` panics on Windows (Victor's bug)
2. `query flex` returns 0 results (because daemon crashed)
3. Agent report rates experience 1-3/10

### GREEN: Fix the parser, run again, expect pass

Fix `jsonl_parser.rs:669` to use UTF-8 safe slicing. Add per-file error handling. Run E2E again.

**Expected results:**
1. `daemon once` completes without panic
2. `query flex` returns results
3. Agent report rates experience 7+/10

### REFACTOR: Optimize the workflow

- Tune session count (maybe 3 is enough, not 5)
- Tune model choice (Haiku for sessions, Haiku for eval)
- Add caching for npm install
- Parallelize where possible

---

## Implementation Phases

| Phase | Description | Effort | Dependencies |
|-------|-------------|--------|--------------|
| **Phase 1** | Write `e2e-test` job in `staging.yml` (Stages 1-6) | 3-4 hours | `ANTHROPIC_API_KEY` secret |
| **Phase 2** | Add agent quality evaluation (Stage 7-8) | 2-3 hours | Phase 1 working |
| **Phase 3** | Fix parser bugs surfaced by E2E | 4-6 hours | Phase 1 reveals bugs |
| **Phase 4** | Iterate: tune sessions, assertions, eval prompt | 2 hours | Phase 1-3 complete |

**Total: ~12-15 hours**

**Phase 1 is the MVP.** Stages 1-6 alone (no agent eval) catch 90% of the bugs. Agent eval (Phase 2) catches the qualitative issues.

---

## Success Criteria

**Pipeline is complete when:**
- [ ] E2E runs on Windows, macOS, Linux on every push to master
- [ ] Real Claude Code sessions generated (not fixtures)
- [ ] Tastematter installed via install script (not cargo build)
- [ ] Full workflow tested: daemon → query → context → heat → chains
- [ ] Hard assertions block release on failure
- [ ] Agent quality report uploaded as artifact
- [ ] Cost < $5/month
- [ ] Total runtime < 15 minutes

**Pipeline is validated when:**
- [ ] Catches a real bug that current CI misses
- [ ] (Ideally: catches Victor's UTF-8 bug in RED phase)

---

## Known Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Claude Code CLI install fails on runner | Low | HIGH | Pin npm version, cache node_modules |
| API rate limiting | Low | MEDIUM | Use Haiku, limit to 5 sessions |
| Session data too small for meaningful testing | Medium | MEDIUM | Use tool-heavy prompts (Read, Write, Bash) |
| CI runner has different .claude/ layout | Low | HIGH | This IS the test — if it differs, we find out |
| Quality eval is noisy/inconsistent | Medium | LOW | It's informational, not a gate |
| npm -g install requires sudo on Linux | Medium | MEDIUM | Use `--prefix` or runner's default npm config |

---

## Related Documents

- `specs/user_bug_reports/2026-02-10_tastematter-cli-root-cause-analysis.md` — Victor's bug report that motivated this
- `.github/workflows/staging.yml` — Existing staging pipeline (to be extended)
- `scripts/install/install.ps1` — Windows install script tested by E2E
- `scripts/install/install.sh` — Unix install script tested by E2E
- `core/src/capture/jsonl_parser.rs:669` — The UTF-8 boundary bug to be caught

---

## References

- [Claude Code Headless Mode](https://code.claude.com/docs/en/headless) — `-p` flag, session management
- [Claude Code GitHub Actions](https://code.claude.com/docs/en/github-actions) — Official CI/CD integration
- [anthropics/claude-code-action@v1](https://github.com/anthropics/claude-code-action) — GitHub Action
