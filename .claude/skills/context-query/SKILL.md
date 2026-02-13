---
name: context-query
description: >
  This skill should be used when users ask about their work context,
  what they're working on, recent activity, or file relationships.
  Uses tastematter CLI combined with grep/glob for context restoration.
  Every claim MUST cite a receipt ID for user verification.
---

# Context Query Skill

Use `tastematter` CLI + grep + glob together for context restoration.

**Core insight:** The CLI finds WHICH files matter. Grep/Read find WHAT's in them.

---

## Quick Start (90% of queries)

```bash
# 1. Full context for a topic (start here — one command does it all)
tastematter context "pixee" --format json

# 2. If you need to drill deeper, use targeted queries
tastematter query flex --files "*pixee*" --agg count,recency --format json

# 3. Read hot files to understand what they are
Read: <top_file_from_step_1>

# 4. Search content IN those files
Grep: pattern="hubspot|integration" path=<directory_from_step_1>

# 5. Check git for recent changes
git log --oneline -10 -- <hot_files>
```

**Start with `context` for broad questions, drop to `query` subcommands for targeted drilldowns.**

---

## CLI Commands

### Top-Level Commands

| Command | Purpose |
|---------|---------|
| `context <query>` | **Start here.** Composed query across flex, heat, chains, sessions, timeline, co-access |
| `query <subcommand>` | Targeted queries (flex, heat, chains, etc.) |
| `intel health` | Check intel service health |
| `intel name-chain <id>` | Name a chain using AI |
| `daemon status` | Show daemon sync state + registration |
| `daemon once` | Run a single sync cycle |

### Query Subcommands

| Command | Purpose |
|---------|---------|
| `query flex --files "*pattern*"` | Find files by path pattern |
| `query flex --time 7d` | Find files touched in time window |
| `query heat --time 30d` | File heat metrics (specificity, velocity, score, level) |
| `query co-access <file>` | Find files accessed together |
| `query session <id>` | All files in a session |
| `query sessions --time 7d` | Session-grouped data |
| `query chains` | List conversation chains |
| `query timeline --time 7d` | Timeline data for visualization |
| `query verify <receipt>` | Verify a previous query result |
| `query receipts` | List recent query receipts |

**Always use `--format json`** — table output truncates long paths.

---

## The `context` Command

**The highest-level query.** One command gives you executive summary, heat, work clusters, insights, continuity, timeline, and suggested reads.

```bash
tastematter context "tastematter" --format json
tastematter context "pixee" --time 14d --format json
```

**Response includes:**
- `executive_summary` — status, work_tempo, hot_file_count, focus_ratio
- `current_state` — key_metrics, evidence (file excerpts)
- `continuity` — left_off_at, pending_items, chain_context
- `work_clusters` — co-accessed file groups with PMI scores
- `suggested_reads` — prioritized files (with surprise flag for unexpected)
- `timeline` — weekly focus areas with top files and access counts
- `insights` — abandoned file detection, anomalies
- `verification` — receipt_id, files/sessions/pairs analyzed
- `quick_start` — commands and directory structures from matching files

**When to use `context` vs `query`:**
| Need | Use |
|------|-----|
| "What's happening with project X?" | `context "X"` |
| "What files are hot right now?" | `query heat` |
| "Find files matching a pattern" | `query flex --files` |
| "What did I work on this week?" | `query flex --time 7d` |
| "Show me session history" | `query sessions` |
| "What files go with this one?" | `query co-access <file>` |

---

## The `query heat` Command

Shows file heat metrics with percentile-based classification.

```bash
tastematter query heat --time 30d --format json
tastematter query heat --files "*tastematter*" --sort specificity --format csv
```

**Options:**
```
-t, --time <TIME>      Long window: 30d (default), 14d, 60d, 90d
-f, --files <FILES>    File path pattern filter (glob-style)
-l, --limit <LIMIT>    Max results (default: 50)
-s, --sort <SORT>      Sort by: heat (default), specificity, velocity, name
    --format <FORMAT>  Output: table (default), json, compact, csv
```

**Heat levels use percentile classification:**
- Top 10% = HOT, 10-30% = WARM, 30-60% = COOL, Bottom 40% = COLD

**Key fields:** `specificity` (IDF-like), `velocity`, `heat_score`, `heat_level`

See `references/heat-metrics-model.md` for formula details and interpretation tables.

---

## Common Mistakes

### Wrong: Expect semantic search
```bash
# DON'T - CLI searches file PATHS, not content
tastematter query flex --files "*notification design*"  # Won't find anything
```

### Right: Use CLI for paths, grep for content
```bash
# DO - Find files first, then search content
tastematter query flex --files "*alert*" --format json  # Find alert-related files
Grep: pattern="notification|digest|slack" path=<results>  # Search content
```

### Wrong: Only use CLI
```bash
# DON'T - You'll miss semantic understanding
tastematter query flex --files "*hubspot*"  # Returns files but not WHAT they do
```

### Right: Combine tools
```bash
# DO - CLI narrows, Read/Grep understands
tastematter query flex --files "*hubspot*" --format json
Read: <top_result>  # Understand what the file actually does
```

### Wrong: Skip `context` and go straight to `query`
```bash
# DON'T - You'll miss insights, clusters, and continuity
tastematter query flex --files "*pixee*" --format json
```

### Right: Start broad, drill down
```bash
# DO - Get the full picture first
tastematter context "pixee" --format json
# THEN drill into specific areas if needed
tastematter query heat --files "*pixee*" --format json
```

---

## Combining Tools

| Need | Tool |
|------|------|
| Broad project context | `tastematter context "<topic>"` |
| Find files by path pattern | `tastematter query flex --files` |
| File heat/activity metrics | `tastematter query heat` |
| Find files by content | `Grep` |
| Read file contents | `Read` |
| Find files by name glob | `Glob` |
| Check recent changes | `git log` |

**Workflow pattern:**
```
context → full picture (summary, heat, clusters, insights)
    ↓
query heat/flex → drill into specific files or patterns
    ↓
Read → understand what files contain
    ↓
Grep → find specific concepts in content
    ↓
git log → understand evolution
```

---

## Database Location

**Canonical:** `~/.context-os/context_os_events.db`

**If queries return empty results:**
```bash
# Check database exists
ls ~/.context-os/context_os_events.db

# Check daemon status
tastematter daemon status

# Run a single sync
tastematter daemon once

# Rebuild indexes if needed
tastematter build-chains
tastematter index-files
```

---

## Citation Requirements

Every claim from query results MUST include receipt ID:

```markdown
Found 147 Pixee files [q_7f3a2b]
To verify: tastematter query verify q_7f3a2b
```

---

## When This Skill Helps vs Doesn't

| Helps | Doesn't Help |
|-------|--------------|
| "What files did I touch for project X?" | "What was I thinking about?" |
| "When was this file last accessed?" | "Why did I make this change?" |
| "What files are related to this one?" | "What's the best architecture for X?" |
| "What's the status of this work?" | "How should I fix this bug?" |
| "What files are hot/cold right now?" | "What's in my calendar?" |

**For semantic understanding, READ the files.** The CLI tells you which ones matter.

---

## References

For advanced patterns:
- `references/heat-metrics-model.md` - Heat formula: specificity, exponential decay, percentile classification
- `references/search-strategies.md` - 9 multi-step search strategies (Pilot Drilling, Triangulation, etc.)
- `references/query-patterns.md` - Path substring patterns and result interpretation

---

## CLI Full Reference

### context (Composed Query — Start Here)

```bash
tastematter context <QUERY> [OPTIONS]

ARGUMENTS:
  <QUERY>            Search query (used as glob pattern *query*)

OPTIONS:
  -t, --time <TIME>      Time window (default: 30d)
  -l, --limit <LIMIT>    Max results per sub-query (default: 20)
      --format <FORMAT>  Output: json (default), compact, table
```

### query flex (Targeted File Query)

```bash
tastematter query flex [OPTIONS]

OPTIONS:
  -f, --files <FILES>      File pattern (glob): "*pixee*", "*.py"
  -t, --time <TIME>        Time window: 7d (default), 14d, 30d
  -c, --chain <CHAIN>      Filter by chain ID
  -s, --session <SESSION>  Filter by session ID
  -a, --agg <AGG>          Aggregations: count, recency
  -l, --limit <LIMIT>      Max results (default: 20)
      --sort <SORT>        Order: count (default), recency
      --format <FORMAT>    Output: json (default), compact
```

### query heat (File Heat Metrics)

```bash
tastematter query heat [OPTIONS]

OPTIONS:
  -t, --time <TIME>      Window: 30d (default), 14d, 60d, 90d
  -f, --files <FILES>    File pattern filter (glob-style)
  -l, --limit <LIMIT>    Max results (default: 50)
  -s, --sort <SORT>      Sort: heat (default), specificity, velocity, name
      --format <FORMAT>  Output: table (default), json, compact, csv
```

### query timeline (Visualization Data)

```bash
tastematter query timeline [OPTIONS]

OPTIONS:
  -t, --time <TIME>      Time range: 7d (default), 14d, 30d
  -p, --files <FILES>    File pattern filter
  -c, --chain <CHAIN>    Filter by chain ID
  -l, --limit <LIMIT>    Max files (default: 30)
      --format <FORMAT>  Output: json (default), compact
```

### query sessions (Session-Grouped)

```bash
tastematter query sessions [OPTIONS]

OPTIONS:
  -t, --time <TIME>      Time range: 7d (default), 14d, 30d
  -c, --chain <CHAIN>    Filter by chain ID
  -l, --limit <LIMIT>    Max sessions (default: 50)
      --format <FORMAT>  Output: json (default), compact
```

### Other Query Commands

| Command | Purpose |
|---------|---------|
| `query search <term>` | Keyword search in file paths |
| `query file <path>` | History for specific file |
| `query co-access <path>` | Files accessed with target |
| `query chains` | List conversation chains |
| `query verify <receipt>` | Verify a receipt |
| `query receipts` | List recent receipts |

All support `--format json`.

### Non-Query Commands

| Command | Purpose |
|---------|---------|
| `sync-git` | Sync git commits from repository |
| `parse-sessions` | Parse JSONL session files |
| `build-chains` | Build chain graph from sessions |
| `index-files` | Build inverted file index |
| `watch` | Watch directory for file changes |
| `daemon once` | Run single sync cycle |
| `daemon start` | Start daemon (foreground) |
| `daemon status` | Show sync state + registration |
| `daemon install` | Install daemon to run on login |
| `daemon uninstall` | Remove daemon from login |
| `intel health` | Check intel service health |
| `intel name-chain <id>` | Name a chain using AI |
| `serve` | Start HTTP API server |

---

**Last Updated:** 2026-02-13
**Version:** 4.0 (Added context command, query heat, timeline, sessions, receipts, intel, daemon. Fixed flex options. Updated workflow to start with context.)
