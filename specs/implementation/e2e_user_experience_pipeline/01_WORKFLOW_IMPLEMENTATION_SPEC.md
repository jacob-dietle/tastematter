# E2E Workflow Implementation Spec

**Mission:** Implement the `e2e-test` job in `staging.yml` — all 8 stages, all 3 platforms.

**Reading Time:** 20 minutes
**Implementation Time:** 4-6 hours (Phase 1 + Phase 2)
**Prerequisites:** `ANTHROPIC_API_KEY` added as GitHub repository secret

---

## Read These Files First

1. `00_ARCHITECTURE_GUIDE.md` — Full architecture (you're here)
2. `.github/workflows/staging.yml` — Current staging pipeline (add E2E job here)
3. `scripts/install/install.ps1` — Windows install script
4. `scripts/install/install.sh` — Unix install script
5. `specs/user_bug_reports/2026-02-10_tastematter-cli-root-cause-analysis.md` — What we're catching

---

## Phase 1: Core E2E Job (Stages 1-6)

### Step 1: Add the e2e-test job to staging.yml

Add this job AFTER `upload-staging` in `.github/workflows/staging.yml`:

```yaml
  e2e-test:
    name: E2E Test ${{ matrix.os }}
    needs: upload-staging
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            shell: pwsh
          - os: ubuntu-latest
            shell: bash
          - os: macos-latest
            shell: bash
    steps:
      # === STAGE 1: Environment Setup ===
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      # === STAGE 2: Install Claude Code ===
      - name: Install Claude Code CLI
        shell: bash
        run: |
          npm install -g @anthropic-ai/claude-code
          claude --version

      # === STAGE 3: Generate Real Session Data ===
      - name: Create test project
        shell: bash
        run: |
          mkdir -p ~/test-project/src
          cat > ~/test-project/src/main.py << 'PYEOF'
          def hello(name: str = "world") -> str:
              """Greet someone by name."""
              return f"Hello, {name}!"

          def add(a: int, b: int) -> int:
              """Add two numbers."""
              return a + b

          if __name__ == "__main__":
              print(hello())
              print(f"2 + 3 = {add(2, 3)}")
          PYEOF

          cat > ~/test-project/README.md << 'MDEOF'
          # Test Project

          A simple Python project for E2E testing of tastematter.

          ## Usage
          ```bash
          python src/main.py
          ```
          MDEOF

          echo '{"name": "test-project", "version": "1.0.0"}' > ~/test-project/package.json

      - name: Generate Claude Code sessions
        shell: bash
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          cd ~/test-project

          echo "=== Session 1: File listing ==="
          claude -p "List all files in this project directory." \
            --output-format json --allowedTools "Bash,Read" --max-turns 3 \
            --model haiku \
            || echo "Session 1 completed (may have non-zero exit)"

          echo "=== Session 2: Code analysis ==="
          claude -p "Read src/main.py and explain each function." \
            --output-format json --allowedTools "Read" --max-turns 3 \
            --model haiku \
            || echo "Session 2 completed"

          echo "=== Session 3: Code generation ==="
          claude -p "Create a file src/test_main.py with pytest tests for the hello and add functions in main.py." \
            --output-format json --allowedTools "Read,Write" --max-turns 5 \
            --model haiku \
            || echo "Session 3 completed"

          echo "=== Session 4: Bash usage ==="
          claude -p "Show me the contents of README.md using cat." \
            --output-format json --allowedTools "Bash,Read" --max-turns 3 \
            --model haiku \
            || echo "Session 4 completed"

      - name: Verify session data exists
        shell: bash
        run: |
          echo "=== Checking for .claude session data ==="

          if [ ! -d "$HOME/.claude" ]; then
            echo "FATAL: ~/.claude directory does not exist"
            echo "Claude Code did not generate any session data"
            exit 1
          fi

          SESSION_COUNT=$(find "$HOME/.claude" -name "*.jsonl" 2>/dev/null | wc -l)
          echo "Found $SESSION_COUNT JSONL session files"

          if [ "$SESSION_COUNT" -lt 1 ]; then
            echo "FATAL: No session JSONL files found in ~/.claude"
            echo "Directory listing:"
            find "$HOME/.claude" -type f | head -20
            exit 1
          fi

          echo "Session data generation: PASSED"

      # === STAGE 4: Install Tastematter ===
      - name: Install tastematter from staging (Windows)
        if: matrix.os == 'windows-latest'
        shell: pwsh
        env:
          TASTEMATTER_CHANNEL: staging
        run: |
          $env:TASTEMATTER_CHANNEL = "staging"
          Invoke-RestMethod https://install.tastematter.dev/install.ps1 | Invoke-Expression
          2>&1 | Tee-Object -FilePath "$env:GITHUB_WORKSPACE\e2e-install-output.txt"

      - name: Install tastematter from staging (Unix)
        if: matrix.os != 'windows-latest'
        shell: bash
        env:
          TASTEMATTER_CHANNEL: staging
        run: |
          TASTEMATTER_CHANNEL=staging curl -fsSL https://install.tastematter.dev/install.sh | bash \
            2>&1 | tee "$GITHUB_WORKSPACE/e2e-install-output.txt"

      - name: Verify tastematter install
        shell: bash
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          echo "=== Verifying tastematter install ==="
          tastematter --version
          echo "Install verification: PASSED"

      # === STAGE 5: Tastematter First Run ===
      - name: Parse sessions (daemon once)
        shell: bash
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          echo "=== Running daemon once (parse sessions) ==="
          tastematter daemon once 2>&1 | tee "$GITHUB_WORKSPACE/e2e-daemon-output.txt"
          echo "Daemon once: COMPLETED (exit code: $?)"

      - name: Query flex
        shell: bash
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          tastematter query flex --time 30d --format json \
            | tee "$GITHUB_WORKSPACE/e2e-query-output.json"

      - name: Context restore
        shell: bash
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          tastematter context "test" --time 30d --format json \
            | tee "$GITHUB_WORKSPACE/e2e-context-output.json"

      - name: Heat command
        shell: bash
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          tastematter heat --format json \
            | tee "$GITHUB_WORKSPACE/e2e-heat-output.json" || true

      - name: Query chains
        shell: bash
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          tastematter query chains --format json \
            | tee "$GITHUB_WORKSPACE/e2e-chains-output.json"

      # === STAGE 6: Hard Assertions ===
      - name: Assert daemon didn't panic
        shell: bash
        run: |
          if grep -qi "panicked\|SIGSEGV\|SIGABRT\|assertion failed" "$GITHUB_WORKSPACE/e2e-daemon-output.txt"; then
            echo "::error::FATAL: daemon panicked during session parsing"
            echo "=== Daemon output ==="
            cat "$GITHUB_WORKSPACE/e2e-daemon-output.txt"
            exit 1
          fi
          echo "Assertion PASSED: daemon did not panic"

      - name: Assert query returned results
        shell: bash
        run: |
          RESULT_COUNT=$(jq '.result_count // 0' "$GITHUB_WORKSPACE/e2e-query-output.json")
          echo "Query returned $RESULT_COUNT results"
          if [ "$RESULT_COUNT" -lt 1 ]; then
            echo "::error::FATAL: query returned 0 results after parsing real sessions"
            echo "=== Query output ==="
            cat "$GITHUB_WORKSPACE/e2e-query-output.json"
            exit 1
          fi
          echo "Assertion PASSED: query returned $RESULT_COUNT results"

      - name: Assert context restore works
        shell: bash
        run: |
          RECEIPT=$(jq -r '.receipt_id // "null"' "$GITHUB_WORKSPACE/e2e-context-output.json")
          if [ "$RECEIPT" = "null" ] || [ -z "$RECEIPT" ]; then
            echo "::error::FATAL: context command returned no receipt_id"
            cat "$GITHUB_WORKSPACE/e2e-context-output.json"
            exit 1
          fi

          STATUS=$(jq -r '.executive_summary.status // "null"' "$GITHUB_WORKSPACE/e2e-context-output.json")
          if [ "$STATUS" = "null" ]; then
            echo "::error::FATAL: context command returned no status"
            exit 1
          fi

          FILES_FOUND=$(jq '.current_state.key_metrics.context_files_found // 0' "$GITHUB_WORKSPACE/e2e-context-output.json")
          echo "Context found $FILES_FOUND files"

          echo "Assertion PASSED: context restore works (receipt=$RECEIPT, status=$STATUS, files=$FILES_FOUND)"

      # === STAGE 8: Upload all artifacts ===
      - name: Upload E2E artifacts
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: e2e-results-${{ matrix.os }}
          path: |
            e2e-*.txt
            e2e-*.json
            e2e-*.md
          retention-days: 14
```

---

## Phase 2: Agent Quality Evaluation (Stage 7)

Add this step BETWEEN "Assert context restore works" and "Upload E2E artifacts":

```yaml
      # === STAGE 7: Agent Quality Evaluation ===
      - name: Agent quality evaluation
        if: always()
        shell: bash
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          cd "$GITHUB_WORKSPACE"

          # Collect all outputs into a single evaluation prompt
          EVAL_INPUT=""
          EVAL_INPUT+="== INSTALL OUTPUT ==\n"
          EVAL_INPUT+="$(cat e2e-install-output.txt 2>/dev/null || echo 'Not captured')\n\n"
          EVAL_INPUT+="== DAEMON ONCE OUTPUT ==\n"
          EVAL_INPUT+="$(cat e2e-daemon-output.txt 2>/dev/null || echo 'Not captured')\n\n"
          EVAL_INPUT+="== QUERY FLEX OUTPUT (first 200 lines) ==\n"
          EVAL_INPUT+="$(head -200 e2e-query-output.json 2>/dev/null || echo 'Not captured')\n\n"
          EVAL_INPUT+="== CONTEXT RESTORE OUTPUT (first 200 lines) ==\n"
          EVAL_INPUT+="$(head -200 e2e-context-output.json 2>/dev/null || echo 'Not captured')\n\n"
          EVAL_INPUT+="== HEAT OUTPUT (first 100 lines) ==\n"
          EVAL_INPUT+="$(head -100 e2e-heat-output.json 2>/dev/null || echo 'Not captured')\n\n"
          EVAL_INPUT+="== CHAINS OUTPUT (first 100 lines) ==\n"
          EVAL_INPUT+="$(head -100 e2e-chains-output.json 2>/dev/null || echo 'Not captured')\n\n"

          # Write prompt to file to avoid shell escaping issues
          cat > eval-prompt.txt << 'PROMPTEOF'
          You are a QA engineer evaluating the first-run experience of a CLI tool called "tastematter" on a fresh machine.

          A brand new user just:
          1. Installed Claude Code and generated some coding sessions
          2. Installed tastematter via the install script
          3. Ran tastematter to parse their sessions and query results

          Review ALL outputs below and produce a quality report.

          Evaluate:
          1. INSTALL: Did it succeed cleanly? Any confusing messages? Would user know what to do next?
          2. PARSING: Did daemon parse sessions without errors? Warnings? How many sessions found?
          3. QUERIES: Did they return meaningful results? Or empty/confusing output?
          4. ERRORS: Were there any error messages? Were they clear and actionable?
          5. UX: Would a first-time user understand what happened at each step?
          6. RED FLAGS: Any panics, crashes, unexpected behavior, or missing output?

          Produce:
          - Overall rating: X/10
          - Issues found (CRITICAL/HIGH/MEDIUM/LOW severity)
          - What worked well
          - Specific recommendations

          Be honest and specific. Reference exact output when noting issues.
          PROMPTEOF

          # Append collected outputs to prompt
          echo "" >> eval-prompt.txt
          echo -e "$EVAL_INPUT" >> eval-prompt.txt

          # Run evaluation
          claude -p "$(cat eval-prompt.txt)" \
            --output-format json --max-turns 1 --model haiku \
            | jq -r '.result // "Evaluation failed"' > e2e-quality-report.md \
            || echo "Agent evaluation failed (non-fatal)" > e2e-quality-report.md

          echo "=== Quality Report ==="
          cat e2e-quality-report.md

          # Extract rating and warn if low
          RATING=$(grep -oP '\d+/10' e2e-quality-report.md | head -1 | cut -d/ -f1)
          if [ -n "$RATING" ] && [ "$RATING" -lt 7 ]; then
            echo "::warning::E2E quality rating: $RATING/10 — review report in artifacts"
          fi
```

---

## Windows-Specific Considerations

The workflow above uses `shell: bash` for most steps, which works on Windows runners (Git Bash). However, some steps need platform-specific handling:

### Windows PATH handling

```yaml
# Windows needs different PATH setup
- name: Parse sessions (daemon once) (Windows)
  if: matrix.os == 'windows-latest'
  shell: pwsh
  run: |
    $env:Path = "$env:USERPROFILE\.local\bin;$env:Path"
    tastematter daemon once 2>&1 | Out-File -FilePath "$env:GITHUB_WORKSPACE\e2e-daemon-output.txt"
```

**Decision:** Start with `shell: bash` for all platforms (simpler). If Windows bash quirks cause issues, split into platform-specific steps. The current smoke tests in staging.yml already use this split pattern — follow that if needed.

### Windows jq

GitHub Windows runners have `jq` pre-installed. No additional setup needed.

### Windows find command

`find` on Windows runners points to Git Bash's `find`, not Windows' `find.exe`. The `find ~/.claude -name "*.jsonl"` command works correctly.

---

## Verification Checklist

Before merging the E2E workflow:

- [ ] `ANTHROPIC_API_KEY` added as GitHub repo secret
- [ ] Workflow triggers on push to master (same as staging)
- [ ] All 3 platforms in matrix (windows-latest, ubuntu-latest, macos-latest)
- [ ] Claude Code generates real session data (not fixtures)
- [ ] Tastematter installed via install script (not cargo build)
- [ ] All 5 commands tested: daemon once, query flex, context, heat, chains
- [ ] Hard assertions block on failure (exit 1)
- [ ] All outputs captured to files
- [ ] Artifacts uploaded (even on failure: `if: always()`)
- [ ] Agent evaluation is non-blocking (informational only)
- [ ] Cost per run < $0.20

---

## Expected RED Phase Results

When this workflow runs against the CURRENT codebase (with known bugs):

**Windows runner:**
- Stage 5 (daemon once): EXPECTED FAILURE — `is_char_boundary` panic at `jsonl_parser.rs:669`
- Stage 6: EXPECTED FAILURE — assertion catches panic text in output
- Stage 7: Agent rates experience 1-3/10

**Linux/macOS runners:**
- May pass or fail depending on whether session data triggers other parser edge cases
- If passes: validates baseline functionality
- If fails: discovers NEW bugs (which is the point)

**This is the RED phase of TDD.** The test SHOULD fail. It proves the test catches real bugs. Then we fix the parser (GREEN phase), and the test passes.

---

## Common Pitfalls

1. **Don't use `set -e` in session generation** — Claude Code `-p` may exit non-zero on some prompts (tool permission denials, etc). Use `|| true` or `|| echo` for resilience.

2. **Don't assert exact result counts** — Session count and query results depend on Claude's behavior, which varies. Assert `> 0`, not `== 5`.

3. **Don't use `--continue` on first session** — No previous session to continue from. Only use `--continue` after at least one `-p` call.

4. **Truncate outputs for agent eval** — Full JSON outputs can be huge. Use `head -200` to keep eval prompt under token limits.

5. **Always upload artifacts on failure** — Use `if: always()` so you can debug failures by downloading the captured outputs.

6. **Use Haiku for everything** — Sessions and evaluation. Cheapest model, and session data format is identical regardless of model used.
