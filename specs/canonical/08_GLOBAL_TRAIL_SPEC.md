---
title: "Global Trail Specification"
type: architecture-spec
created: 2026-02-24
last_updated: 2026-02-24
status: draft
foundation:
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[canonical/07_CLAUDE_CODE_DATA_MODEL_V2]]"
  - "[[06_products/tastematter/strategy/context-ops-offering]]"
  - "[[04_knowledge_base/methodology/stigmergy]]"
tags:
  - tastematter
  - global-trail
  - sync
  - d1
  - cloudflare
---

# Global Trail Specification

## One-Liner

Sync your local work trail to a remote D1 database so any machine can query your full context history.

---

## Problem

You work on multiple machines (laptop, VPS, future Mac). Each machine runs tastematter locally and builds its own trail (SQLite at `~/.context-os/context_os_events.db`). Context intelligence is trapped on the machine that generated it. A new Claude Code session on the VPS starts cold — no chains, no heat, no co-access patterns.

## Solution

A **global trail** — a Cloudflare D1 database that aggregates trail data from all your machines. Push from any machine, pull to any machine, query from anywhere.

```
LOCAL TRAIL (per machine)              GLOBAL TRAIL (D1)
~/.context-os/context_os_events.db     tastematter-trail D1
├── claude_sessions                     ├── claude_sessions + source_machine
├── chain_graph                         ├── chain_graph + source_machine
├── chain_metadata                      ├── chain_metadata + source_machine
├── chain_summaries                     ├── chain_summaries + source_machine
├── file_access_events                  ├── file_access_events + source_machine
├── file_edges                          ├── file_edges + source_machine
├── git_commits                         ├── git_commits + source_machine
└── _metadata                           └── sync_log
```

---

## Architecture

```
PUSH (any machine):
  tastematter trail push
    → read local SQLite (rows since last sync)
    → normalize paths (Windows → Unix)
    → POST /trail/push → CF Worker → upsert into D1

PULL (any machine):
  tastematter trail pull
    → GET /trail/pull?since={last_sync} → CF Worker → query D1
    → write to local SQLite (merge, skip duplicates)

STATUS:
  tastematter trail status
    → query local counts + GET /trail/status → compare
```

### Components

| # | Component | Location | ~Lines |
|---|---|---|---|
| 1 | CF Worker + D1 | `apps/tastematter/trail-worker/` | ~200 |
| 2 | `trail push` command | `apps/tastematter/core/src/trail/push.rs` | ~300 |
| 3 | `trail pull` command | `apps/tastematter/core/src/trail/pull.rs` | ~200 |
| 4 | Path normalizer | `apps/tastematter/core/src/trail/paths.rs` | ~50 |
| 5 | CLI subcommand | `apps/tastematter/core/src/main.rs` (extend) | ~50 |

### Auth

CF Access service token. Same pattern as Nickel workers.

```
Headers:
  CF-Access-Client-Id: {client_id}
  CF-Access-Client-Secret: {client_secret}
```

Stored in `~/.context-os/trail.toml` (gitignored):

```toml
[global]
endpoint = "https://trail.tastematter.dev"
machine_id = "laptop-2phko1ph"  # Tailscale hostname

[auth]
client_id = "xxx.access"
client_secret = "yyy"
```

---

## D1 Schema

### Design Decisions

| Decision | Choice | Reasoning |
|---|---|---|
| Primary keys | Use existing natural keys (session_id, hash, chain_id) | Already UUIDs/hashes from Claude Code. No collisions across machines. [GROUNDED: 07_DATA_MODEL_V2 Section 4.3] |
| Auto-increment tables | Composite unique constraint instead | `file_access_events` and `file_edges` use auto-increment locally. For D1, use (source_machine + local fields) as unique key. Idempotent pushes. |
| Path storage | Unix-normalized always | `normalize_path()` on push. D1 never stores Windows paths. |
| Conflict strategy | Last-write-wins via INSERT OR REPLACE | 95%+ INSERTs, rare conflicts, acceptable for single-user prototype. |
| Timestamps | UTC ISO-8601 | Already the format in local SQLite. |

### Migration SQL

```sql
-- migrations/001_global_trail.sql

-- Session metadata (one row = one Claude Code session)
CREATE TABLE IF NOT EXISTS claude_sessions (
    session_id TEXT PRIMARY KEY,
    source_machine TEXT NOT NULL,
    project_path TEXT,
    started_at TEXT,
    ended_at TEXT,
    duration_seconds INTEGER,
    user_message_count INTEGER,
    assistant_message_count INTEGER,
    total_messages INTEGER,
    files_read TEXT,
    files_written TEXT,
    tools_used TEXT,
    file_size_bytes INTEGER,
    first_user_message TEXT,
    conversation_excerpt TEXT,
    parsed_at TEXT,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Chain graph (one row = one session's position in a chain)
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    source_machine TEXT NOT NULL,
    chain_id TEXT NOT NULL,
    parent_session_id TEXT,
    indexed_at TEXT,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Chain metadata (one row = one chain's AI-generated name/summary)
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    source_machine TEXT NOT NULL,
    generated_name TEXT,
    summary TEXT,
    key_topics TEXT,
    category TEXT,
    confidence REAL,
    generated_at TEXT,
    model_used TEXT,
    created_at TEXT,
    updated_at TEXT,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Chain summaries (one row = one chain's narrative summary)
CREATE TABLE IF NOT EXISTS chain_summaries (
    chain_id TEXT PRIMARY KEY,
    source_machine TEXT NOT NULL,
    summary TEXT,
    accomplishments TEXT,
    status TEXT,
    key_files TEXT,
    workstream_tags TEXT,
    model_used TEXT,
    created_at TEXT,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Chains (one row = one chain's aggregate stats)
CREATE TABLE IF NOT EXISTS chains (
    chain_id TEXT PRIMARY KEY,
    source_machine TEXT NOT NULL,
    root_session_id TEXT,
    session_count INTEGER,
    files_count INTEGER,
    updated_at TEXT,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- File access events (one row = one file touched in one session)
CREATE TABLE IF NOT EXISTS file_access_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_machine TEXT NOT NULL,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    file_path TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    access_type TEXT NOT NULL,
    sequence_position INTEGER NOT NULL,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fae_dedup
    ON file_access_events(source_machine, session_id, file_path, tool_name, sequence_position);

-- File edges (one row = one co-access relationship between files)
CREATE TABLE IF NOT EXISTS file_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_machine TEXT NOT NULL,
    source_file TEXT NOT NULL,
    target_file TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    session_count INTEGER NOT NULL DEFAULT 0,
    total_sessions_with_source INTEGER NOT NULL DEFAULT 0,
    avg_time_delta_seconds REAL,
    confidence REAL NOT NULL DEFAULT 0.0,
    lift REAL,
    first_seen TEXT,
    last_seen TEXT,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fe_dedup
    ON file_edges(source_machine, source_file, target_file, edge_type);

-- Git commits (one row = one git commit)
CREATE TABLE IF NOT EXISTS git_commits (
    hash TEXT PRIMARY KEY,
    source_machine TEXT NOT NULL,
    short_hash TEXT,
    timestamp TEXT NOT NULL,
    message TEXT,
    author_name TEXT,
    author_email TEXT,
    files_changed TEXT,
    files_added TEXT,
    files_deleted TEXT,
    files_modified TEXT,
    insertions INTEGER,
    deletions INTEGER,
    files_count INTEGER,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Sync log (tracks push/pull history per machine)
CREATE TABLE IF NOT EXISTS sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    machine_id TEXT NOT NULL,
    direction TEXT NOT NULL,  -- 'push' or 'pull'
    tables_synced TEXT,       -- JSON array of table names
    rows_synced INTEGER,
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### Upsert Strategy

For tables with natural PRIMARY KEY (session_id, hash, chain_id):
```sql
INSERT OR REPLACE INTO claude_sessions (session_id, source_machine, ...) VALUES (?, ?, ...)
```

For tables with auto-increment + unique index (file_access_events, file_edges):
```sql
INSERT OR IGNORE INTO file_access_events (source_machine, session_id, ...) VALUES (?, ?, ...)
```

`INSERT OR REPLACE` = last-write-wins for metadata that may update.
`INSERT OR IGNORE` = skip duplicates for append-only event data.

---

## CF Worker

### wrangler.toml

```toml
name = "tastematter-trail"
main = "src/index.ts"
compatibility_date = "2024-12-01"
compatibility_flags = ["nodejs_compat"]
account_id = "4c8353a21e0bfc69a1e036e223cba4d8"

[[d1_databases]]
binding = "TRAIL_DB"
database_name = "tastematter-trail"
database_id = ""  # Fill after wrangler d1 create

[vars]
ENVIRONMENT = "production"
```

### Endpoints

```
POST /trail/push
  Body: { machine_id, tables: { claude_sessions: [...], ... }, since: "ISO-8601" }
  Auth: CF Access service token
  Action: Upsert rows into D1 per table
  Response: { rows_synced: N, synced_at: "ISO-8601" }

GET /trail/pull?since=ISO-8601&tables=claude_sessions,chain_graph
  Auth: CF Access service token
  Action: SELECT * FROM each table WHERE synced_at > ?since
  Response: { tables: { claude_sessions: [...], ... }, synced_at: "ISO-8601" }

GET /trail/status
  Auth: CF Access service token
  Action: SELECT COUNT(*) FROM each table
  Response: { claude_sessions: N, chain_graph: N, ..., last_sync: "ISO-8601" }
```

### Worker Implementation (Minimal)

```typescript
interface Env {
  TRAIL_DB: D1Database;
}

const TABLES = [
  'claude_sessions', 'chain_graph', 'chain_metadata',
  'chain_summaries', 'chains', 'file_access_events',
  'file_edges', 'git_commits'
];

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // Auth: CF Access handles this at the edge — no code needed
    // If we reach here, request is authenticated

    const url = new URL(request.url);

    if (url.pathname === '/trail/push' && request.method === 'POST') {
      return handlePush(request, env);
    }
    if (url.pathname === '/trail/pull' && request.method === 'GET') {
      return handlePull(url, env);
    }
    if (url.pathname === '/trail/status' && request.method === 'GET') {
      return handleStatus(env);
    }
    if (url.pathname === '/health') {
      return new Response('ok');
    }

    return new Response('Not Found', { status: 404 });
  }
};

async function handlePush(request: Request, env: Env): Promise<Response> {
  const body = await request.json() as PushRequest;
  let totalRows = 0;

  for (const table of TABLES) {
    const rows = body.tables?.[table];
    if (!rows?.length) continue;

    for (const row of rows) {
      row.source_machine = body.machine_id;
      row.synced_at = new Date().toISOString();

      const columns = Object.keys(row);
      const placeholders = columns.map(() => '?').join(', ');
      const values = columns.map(c => row[c]);

      // Natural PK tables: INSERT OR REPLACE
      // Auto-increment tables: INSERT OR IGNORE
      const strategy = ['file_access_events', 'file_edges'].includes(table)
        ? 'INSERT OR IGNORE' : 'INSERT OR REPLACE';

      await env.TRAIL_DB.prepare(
        `${strategy} INTO ${table} (${columns.join(', ')}) VALUES (${placeholders})`
      ).bind(...values).run();
    }
    totalRows += rows.length;
  }

  // Log sync
  await env.TRAIL_DB.prepare(
    `INSERT INTO sync_log (machine_id, direction, tables_synced, rows_synced) VALUES (?, 'push', ?, ?)`
  ).bind(body.machine_id, JSON.stringify(Object.keys(body.tables)), totalRows).run();

  return Response.json({ rows_synced: totalRows, synced_at: new Date().toISOString() });
}

async function handlePull(url: URL, env: Env): Promise<Response> {
  const since = url.searchParams.get('since') || '1970-01-01T00:00:00Z';
  const requestedTables = url.searchParams.get('tables')?.split(',') || TABLES;
  const result: Record<string, any[]> = {};

  for (const table of requestedTables) {
    if (!TABLES.includes(table)) continue;
    const { results } = await env.TRAIL_DB.prepare(
      `SELECT * FROM ${table} WHERE synced_at > ?`
    ).bind(since).all();
    result[table] = results;
  }

  return Response.json({ tables: result, synced_at: new Date().toISOString() });
}

async function handleStatus(env: Env): Promise<Response> {
  const counts: Record<string, number> = {};
  for (const table of TABLES) {
    const row = await env.TRAIL_DB.prepare(`SELECT COUNT(*) as count FROM ${table}`).first();
    counts[table] = (row as any)?.count || 0;
  }

  const lastSync = await env.TRAIL_DB.prepare(
    `SELECT synced_at FROM sync_log ORDER BY synced_at DESC LIMIT 1`
  ).first();

  return Response.json({ counts, last_sync: (lastSync as any)?.synced_at || null });
}

interface PushRequest {
  machine_id: string;
  tables: Record<string, Record<string, any>[]>;
}
```

---

## CLI Commands

### `tastematter trail status`

```
$ tastematter trail status

LOCAL TRAIL (laptop-2phko1ph)
  sessions: 1,024  chains: 160  files: 6,143  commits: 569

GLOBAL TRAIL (trail.tastematter.dev)
  sessions: 1,024  chains: 160  files: 6,143  commits: 569

STATUS: synced (last push: 2 hours ago)
```

### `tastematter trail push`

```
$ tastematter trail push

Pushing local trail → global...
  claude_sessions: 47 new rows
  chain_graph: 12 new rows
  file_access_events: 893 new rows
  file_edges: 34 new rows
  git_commits: 8 new rows

Pushed 994 rows in 1.2s
```

### `tastematter trail pull`

```
$ tastematter trail pull

Pulling global trail → local...
  claude_sessions: 47 rows (from laptop-2phko1ph)
  chain_graph: 12 rows
  file_access_events: 893 rows
  file_edges: 34 rows

Pulled 986 rows in 0.8s
Local trail updated.
```

### `tastematter trail push --select`

Future: interactive selection of what to push. For prototype, push everything.

---

## Path Normalization

```rust
/// Normalize a file path for cross-platform storage.
/// D1 always stores Unix-style paths.
pub fn normalize_path(raw: &str) -> String {
    let mut path = raw.replace('\\', "/");

    // Remove Windows drive letter (C:, D:, etc.)
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        path = path[2..].to_string();
    }

    path
}

// Examples:
// "C:\\Users\\dietl\\VSCode Projects\\foo" → "/Users/dietl/VSCode Projects/foo"
// "/home/jacob/repos/foo"                  → "/home/jacob/repos/foo" (unchanged)
```

Applied during `trail push` to every path column:
- `claude_sessions.project_path`
- `claude_sessions.files_read` (JSON array of paths)
- `claude_sessions.files_written` (JSON array of paths)
- `file_access_events.file_path`
- `file_edges.source_file`
- `file_edges.target_file`
- `git_commits.files_changed` (JSON array)
- `git_commits.files_added` (JSON array)
- `git_commits.files_deleted` (JSON array)
- `git_commits.files_modified` (JSON array)

---

## Incremental Sync

Each machine tracks its last sync timestamp in local `_metadata`:

```sql
-- Local SQLite
INSERT OR REPLACE INTO _metadata (key, value) VALUES ('last_trail_push', '2026-02-24T04:00:00Z');
INSERT OR REPLACE INTO _metadata (key, value) VALUES ('last_trail_pull', '2026-02-24T03:55:00Z');
```

`trail push` queries: `SELECT * FROM claude_sessions WHERE parsed_at > ?last_push`
`trail pull` queries: `GET /trail/pull?since={last_pull}`

First sync: `since` = epoch (sync everything).

---

## What Syncs vs What Doesn't

| Syncs | Why |
|---|---|
| claude_sessions | Session metadata, chains, first message |
| chain_graph | Chain relationships |
| chain_metadata | AI-generated chain names |
| chain_summaries | Narrative summaries |
| chains | Aggregate chain stats |
| file_access_events | File heat, co-access patterns |
| file_edges | Co-access relationships |
| git_commits | Commit metadata |

| Doesn't Sync | Why |
|---|---|
| file_events | Low value (raw FS watcher events) |
| Raw JSONL session files | Too large (1.2 GB), machine-specific paths everywhere |
| debug/ logs | Machine-specific, 483 MB |
| file-history/ snapshots | Machine-specific backups, 175 MB |
| tool-results/ overflow | Machine-specific, 48 MB |

---

## Deployment Steps

```bash
# 1. Scaffold worker
cp -r 00_foundation/services/templates/cf-worker-scaffold/ apps/tastematter/trail-worker/
cd apps/tastematter/trail-worker/

# 2. Create D1
wrangler d1 create tastematter-trail
# Copy database_id to wrangler.toml

# 3. Apply migration
wrangler d1 execute tastematter-trail --remote --file=migrations/001_global_trail.sql

# 4. Set up CF Access (dashboard)
# one.dash.cloudflare.com > Access > Applications > Add self-hosted
# Hostname: trail.tastematter.dev
# Policy: Allow service token

# 5. Deploy worker
pnpm install && wrangler deploy

# 6. Verify
curl -s -H "CF-Access-Client-Id: xxx" -H "CF-Access-Client-Secret: yyy" \
  https://trail.tastematter.dev/health

# 7. Configure local trail.toml
# ~/.context-os/trail.toml (see Auth section above)
```

---

## Success Criteria

| Metric | Target |
|---|---|
| `tastematter trail push` completes | <5s for full initial sync (~10K rows) |
| `tastematter trail pull` completes | <3s for full pull |
| `tastematter context "X"` on VPS after pull | Returns same quality results as on Windows |
| Incremental sync (daily use) | <1s (only new rows) |
| Idempotent push (same data twice) | Zero duplicates |

---

## Sequencing

| Step | What | Time |
|---|---|---|
| 1 | Deploy CF Worker + D1 with migration | 1 hour |
| 2 | Build `trail push` in Rust CLI (with path normalization) | 4 hours |
| 3 | Build `trail pull` in Rust CLI | 2 hours |
| 4 | Build `trail status` in Rust CLI | 1 hour |
| 5 | Test: push from Windows, pull on VPS, run `tastematter context` | 1 hour |
| 6 | Add incremental sync (since timestamp) | 1 hour |
| **Total** | | **~10 hours** |

---

## Future (Not This Spec)

- `trail push --select` — interactive selection of what to publish (private→published scope)
- Team D1 — multiple users pushing to same D1, with user_id scoping
- `trail watch` — auto-push on daemon sync completion
- Selective table sync — push only sessions/chains, not file events
- Encryption at rest — E2E encrypt trail data before pushing to D1

---

## Related Documents

- [[canonical/03_CORE_ARCHITECTURE]] — Local SQLite schema, Rust core
- [[canonical/07_CLAUDE_CODE_DATA_MODEL_V2]] — Source data model (JSONL → SQLite)
- [[06_products/tastematter/strategy/context-ops-offering]] — Business context for this feature
- [[04_knowledge_base/methodology/stigmergy]] — Theoretical foundation (trails = pheromone trails)
- [[_system/specs/architecture/context_operating_system/04_GIT_STIGMERGY_FOUNDATION]] — Git as coordination layer
