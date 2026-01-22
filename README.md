# Tastematter

Context intelligence CLI for Claude Code sessions. Query your work patterns, find related files, and understand what you've been working on.

## Install

**Windows (PowerShell):**
```powershell
irm https://install.tastematter.dev/install.ps1 | iex
```

**macOS/Linux:**
```bash
curl -fsSL https://install.tastematter.dev/install.sh | bash
```

## Quick Start

```bash
# Check installation
tastematter --version

# Initialize (first time only) - index your Claude Code sessions
tastematter parse-sessions --project ~/.claude/projects
tastematter build-chains
tastematter index-files

# Query your most accessed files (last 7 days)
tastematter query flex --time 7d --limit 10

# Find files related to a specific file
tastematter query co-access path/to/file.ts --limit 10

# List your conversation chains
tastematter query chains --limit 10

# Start HTTP server for browser tools
tastematter serve --port 3001
```

## Commands

| Command | Purpose |
|---------|---------|
| `query flex` | Flexible file queries with time/pattern filters |
| `query co-access` | Find files accessed together |
| `query chains` | List conversation chains |
| `query session` | Get all files from a session |
| `query search` | Search file paths by keyword |
| `parse-sessions` | Index Claude Code JSONL files |
| `build-chains` | Build conversation chain graph |
| `index-files` | Build file access index |
| `serve` | Start HTTP API server |
| `daemon` | Background sync commands |
| `watch` | Watch directory for changes |

## Query Examples

### Find your hot files
```bash
# Most accessed files in the last 30 days
tastematter query flex --time 30d --limit 20

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

## Data Location

- **Database:** `~/.context-os/context_os_events.db`
- **Config:** `~/.context-os/config.yaml`
- **Query receipts:** `~/.context-os/query_ledger/`

## How It Works

Tastematter indexes your Claude Code session files (JSONL) and builds:

1. **File access history** - Which files were read/written, when, how often
2. **Session context** - What files were accessed together in each session
3. **Conversation chains** - How sessions link together via `leafUuid`
4. **Co-access graph** - Which files appear together (implicit relationships)

This lets you query patterns like:
- "What am I working on?" (most accessed files)
- "What's related to X?" (co-access relationships)
- "What did I do last week?" (temporal queries)
- "What did I abandon?" (old files with no recent activity)

## Integration with Claude Code

Add to your `CLAUDE.md` for Claude to use tastematter:

```markdown
## Context Queries

Use tastematter to understand work context:
- `tastematter query flex --time 7d` - Recent activity
- `tastematter query co-access <file>` - Related files
- `tastematter query chains --limit 5` - Conversation threads
```

See the [context-query skill](.claude/skills/context-query/SKILL.md) for advanced query strategies.

## Troubleshooting

### "No such table" or empty results
```bash
# Check database exists
ls ~/.context-os/context_os_events.db

# Re-initialize if missing
tastematter parse-sessions --project ~/.claude/projects
tastematter build-chains
tastematter index-files
```

### Windows path issues
Use backslashes for Windows paths:
```powershell
tastematter parse-sessions --project "C:\Users\YourName\.claude\projects"
```

### Update to latest version
Just re-run the install command - it will overwrite the existing binary.

## Development

```bash
# Build from source
cd core
cargo build --release

# Run tests
cargo test

# The binary will be at:
./target/release/tastematter
```

## Architecture

```
~/.context-os/
├── context_os_events.db    # SQLite database (all indexed data)
├── config.yaml             # Daemon configuration
├── daemon.state.json       # Daemon state
└── query_ledger/           # Query receipts for verification
```

The CLI is read-only against the database. Use `parse-sessions`, `build-chains`, and `index-files` to populate it from your Claude Code session data.

## License

Private - Not for distribution.
