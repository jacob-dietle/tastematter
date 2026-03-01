# Tastematter

**Your work leaves trails you can't see.**

People think re-explaining yourself to your AI is a memory problem. It's a visibility problem. You use AI as a force multiplier, ship and learn faster, but the cost of that leverage is visibility into your systems.

This is not a memory solution. This is a visibility solution.

Tastematter runs in the background and processes your Claude Code session data, file access patterns, and repo into a realtime index: which files you touch, where your attention goes, what drifts over time. Instead of starting every session blind, your agent knows what you did yesterday, what's hot right now, and what you abandoned two weeks ago.

**[tastematter.dev](https://tastematter.dev)**

## Install

**Windows (PowerShell):**
```powershell
irm https://install.tastematter.dev/install.ps1 | iex
```

**macOS / Linux:**
```bash
curl -fsSL https://install.tastematter.dev/install.sh | bash
```

## What It Reveals

- **What's actually hot** — Not what you think you're working on. What the data says. 47 touches across 12 sessions this week.
- **Where attention shifted** — Your plan said API redesign. Your trails say 89% went to auth. The pivot happened Tuesday.
- **What got abandoned** — routes.ts was hot two weeks ago. Now it's cold. 11 days silent. Nobody flagged it.
- **What moves together** — Every time you touch middleware.ts, you also touch jwt.ts. Your agent should know that.

## Quick Start

```bash
# Start the background daemon (indexes automatically)
tastematter daemon start

# What am I working on?
tastematter query flex --time 7d --limit 10

# What's hot, warm, cold?
tastematter query heat --time 30d

# What files are related to this one?
tastematter query co-access src/main.rs --limit 10

# Restore full context for a project
tastematter context my-project
```

## Commands

### Core
| Command | What it shows you |
|---------|-------------------|
| `context <query>` | Full context restoration — composes flex, heat, chains, sessions, timeline, co-access |
| `serve` | HTTP API server for browser tools and integrations |
| `sync-git` | Sync git commit data from a repository |

### Query
| Command | What it shows you |
|---------|-------------------|
| `query flex` | Your hottest files with time/pattern filters |
| `query heat` | File heat metrics — specificity, velocity, composite score |
| `query co-access` | Files that move together |
| `query chains` | Your conversation threads over time |
| `query sessions` | Session-grouped file data |
| `query timeline` | Timeline visualization data |
| `query file` | All sessions that touched a specific file |
| `query search` | Find files by keyword |
| `query verify` | Verify a query receipt against current data |
| `query receipts` | List recent query receipts from the ledger |

### Daemon
| Command | What it does |
|---------|-------------|
| `daemon start` | Start background indexing (foreground) |
| `daemon once` | Run a single sync cycle and exit |
| `daemon status` | Show sync state + platform registration |
| `daemon install` | Install daemon to run on login |
| `daemon uninstall` | Remove daemon from login |

### Indexing (manual, daemon handles these automatically)
| Command | What it does |
|---------|-------------|
| `parse-sessions` | Parse Claude Code JSONL session files |
| `build-chains` | Build chain graph from session linking |
| `index-files` | Build inverted file index |

## Integration with Claude Code

Add to your `CLAUDE.md`:

```markdown
## Context

Use tastematter to understand work context before starting:
- `tastematter context <project>` - Full context restoration
- `tastematter query flex --time 7d` - What's hot right now
- `tastematter query heat` - Hot/warm/cold file classification
- `tastematter query co-access <file>` - What moves with this file
```

## How It Works

Tastematter indexes your Claude Code session files (JSONL) and builds:

1. **File access history** — Which files were read/written, when, how often
2. **Session chains** — How sessions link together over time
3. **Co-access graph** — Which files appear together (implicit relationships)
4. **Drift detection** — Plan vs reality divergence across sessions

## Data Location

- **Database:** `~/.context-os/context_os_events.db`
- **Config:** `~/.context-os/config.yaml`
- **Query receipts:** `~/.context-os/query_ledger/`

## Privacy

Your context stays 100% private and local. Anonymous tool usage telemetry (which commands you run, not your files or content) can be disabled:

```bash
tastematter config set telemetry.enabled false
```

## Query Examples

### Find your hot files
```bash
# Most accessed files in the last 30 days
tastematter query flex --time 30d --limit 20

# File heat metrics (hot/warm/cold classification)
tastematter query heat --time 30d --limit 20

# Files matching a pattern
tastematter query flex --files "*pixee*" --time 14d

# With full aggregations (for detailed analysis)
tastematter query flex --time 7d --agg count,recency,sessions,chains --format json
```

### Understand relationships
```bash
# What files are accessed together with this one?
tastematter query co-access src/main.rs --limit 15

# What files were in this session?
tastematter query session abc123-def456 --format json
```

### Explore work history
```bash
# Recent conversation chains
tastematter query chains --limit 10

# Search for files by keyword
tastematter query search "authentication"
```

## Architecture

```
~/.context-os/
├── context_os_events.db    # SQLite database (all indexed data)
├── config.yaml             # Daemon configuration
├── daemon.state.json       # Daemon state
└── query_ledger/           # Query receipts for verification
```

The CLI is read-only against the database. The daemon indexes your Claude Code session data automatically in the background.

## Troubleshooting

### "No such table" or empty results
```bash
# Check database exists
ls ~/.context-os/context_os_events.db

# Re-initialize if needed
tastematter daemon start
```

### Windows path issues
Use backslashes for Windows paths:
```powershell
tastematter parse-sessions --project "C:\Users\YourName\.claude\projects"
```

### Update to latest version
Re-run the install command — it will overwrite the existing binary.

## Development

```bash
cd core && cargo build --release
cd core && cargo test -- --test-threads=2
```

## Status

Alpha software from a solo founder. v0.1.0-alpha.15. Install it if the cold start problem costs you time. Don't if it doesn't.

## License

Private - Not for distribution.
