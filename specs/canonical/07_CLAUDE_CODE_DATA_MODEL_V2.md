---
title: "Claude Code Data Model Specification v2"
type: architecture-spec
created: 2026-02-05
last_updated: 2026-02-05
status: approved
supersedes: "[[07_CLAUDE_CODE_DATA_MODEL]]"
foundation:
  - "[[canonical/00_VISION]]"
  - "[[canonical/03_CORE_ARCHITECTURE]]"
evidence:
  - "[[_system/temp/fs_audit_report.md]]"
  - "[[_system/temp/schema_taxonomy_report.md]]"
  - "[[_system/temp/linking_verification_report.md]]"
  - "[[_system/temp/code_audit_report.md]]"
  - "[[_system/temp/cross_verify_types_report.md]]"
  - "[[_system/reports/audit_session_schema.md]]"
  - "[[_system/reports/audit_metadata_layer.md]]"
  - "[[_system/reports/audit_session_identity.md]]"
tags:
  - tastematter
  - claude-code
  - data-model
  - canonical
  - reference
---

# Claude Code Data Model Specification v2

## How to Use This Spec

Every claim in this document has an attribution tag:

- **[VERIFIED: source]** — Confirmed against real data with specific evidence
- **[INFERRED: from X + Y]** — Deduced from multiple evidence sources
- **[UNVERIFIABLE]** — Cannot confirm from local data alone
- **[MEASURED: YYYY-MM-DD]** — Point-in-time count, not fixed architecture

If a claim doesn't verify when you test it, **the spec is wrong, not your data.** File an issue or update the spec. Section 8 provides exact commands to re-verify every major claim.

**What changed from v1:** The original spec (07_CLAUDE_CODE_DATA_MODEL.md, 2026-01-15) was built by a single agent and conflated top-level record types with nested content blocks (listing "16+ types" when there are 7). It also missed `progress` records, `stats-cache.json`, 7 linking mechanisms, the dual purpose of `leafUuid`, and the tree structure of `parentUuid`. This v2 was built by a 4-agent team with cross-verification on every claim.

---

## 1. Filesystem Structure

### 1.1 Complete Directory Map

[VERIFIED: fs-auditor PowerShell measurements, 2026-02-05]

```
~/.claude/                                  # 2.10 GB total [MEASURED: 2026-02-05]
  .anthropic/claude-code/                   # Empty placeholder
  .credentials.json                 5.8 KB  # OAuth tokens (claudeAiOauth, mcpOAuth)
  cache/
    changelog.md                   82 KB    # Cached release changelog
  chrome/
    chrome-native-host.bat                  # Chrome MCP bridge launcher
  debug/                          483 MB    # 669 files [MEASURED: 2026-02-05]
    <agent-uuid>.txt                        # Per-agent debug logs (plaintext)
  downloads/                      126 MB    # Old binary (claude-1.0.86-win32-x64.exe)
  file-history/                   175 MB    # 424 session dirs, 17,845 file snapshots
    <session-uuid>/
      <content-hash>@v<N>                  # Versioned file backups
  history.jsonl                   4.40 MB   # 9,323 lines, global user message log
  ide/
    <pid>.lock                              # IDE integration lock
  paste-cache/                    0.44 MB   # 111 files, deduplicated paste content
    <content-hash>.txt
  plans/                          0.21 MB   # 29 files, plan-mode documents
    <adjective-verb-noun>.md
  plugins/                        112 MB    # 1,445 files, MCP plugins & marketplace
    cache/  marketplaces/  repos/
  projects/                       ~1.19 GB  # PRIMARY DATA STORE
    <project-path-slug>/                    # Per-project session data (6 dirs)
      <session-uuid>.jsonl                  # Session conversation logs
      agent-<7-char-hex>.jsonl              # Top-level agent sessions
      <session-uuid>/                       # Session artifact directory
        subagents/
          agent-<7-char-hex>.jsonl          # Nested agent sessions
        tool-results/
          toolu_<id>.txt                    # Overflow tool outputs (1,714 files)
      sessions-index.json                   # Session catalog (NEW since v1)
      tmpclaude-<hex>-cwd                   # Temp working directory markers
  session-env/                    0 B       # 33 empty dirs (unused on Windows)
  settings.json                   611 B     # User settings
  settings.json.backup-*                    # Settings backup
  shell-snapshots/                2.42 MB   # 1,095 files (all ~identical)
    snapshot-bash-<epoch>-<rand>.sh
  stats-cache.json                9.8 KB    # Aggregated usage statistics
  statsig/                        0.18 MB   # 14 files, feature flags
  tasks/                          0.03 MB   # 73 files, 25 dirs, team task coordination
    <session-uuid or team-name>/
      <N>.json  .lock  .highwatermark
  teams/                          0.05 MB   # 11 files, agent team config
    <team-name>/
      config.json
      inboxes/<agent-name>.json
  telemetry/                      0.01 MB   # 4 files, FAILED events only (retry queue)
    1p_failed_events.<session>.<batch>.json
  todos/                          0.41 MB   # 2,872 files
    <sessionId>-agent-<agentId>.json
  usage-data/                     0.06 MB
    report.html                             # Pre-generated /insights report
    facets/<session-uuid>.json              # 4 session analyses
```

### 1.2 Size Distribution

[MEASURED: 2026-02-05]

| Store | Size | % of Total | Files |
|-------|------|-----------|-------|
| projects/ | ~1,193 MB | 55.5% | 2,894 |
| debug/ | 483 MB | 22.5% | 669 |
| file-history/ | 175 MB | 8.1% | 17,845 |
| downloads/ | 126 MB | 5.9% | 1 |
| plugins/ | 112 MB | 5.2% | 1,445 |
| All other | ~59 MB | 2.8% | ~1,500 |
| **Total** | **~2,148 MB** | **100%** | **~24,400** |

### 1.3 Project Directories

[VERIFIED: fs-auditor recursive JSONL count, 2026-02-05]

Project directory slugs encode the working directory path with separators replaced by `--`:

| Project Directory | JSONL Files | Size (MB) |
|-------------------|-------------|-----------|
| GTM Operating System (main) | 1,024 | 1,080 |
| LinkedIn Augmentation | 5 | 111 |
| Tastematter | 54 | 0.1 |
| Pixee AI GTM | 36 | 2.1 |
| Jurassic Park education | 7 | 0.2 |
| Claude Scripts | 1 | 0.0 |
| **Total** | **1,127** | **~1,193** |

### 1.4 New Since v1 (Discovered 2026-02-05)

[VERIFIED: fs-auditor, not present in 2026-01-15 spec]

1. **sessions-index.json** — Session catalog file in project directories. Schema:
   ```json
   {
     "version": 1,
     "originalPath": "absolute/project/path",
     "entries": [
       {
         "sessionId": "uuid", "fullPath": "path", "fileMtime": epoch_ms,
         "firstPrompt": "string", "summary": "string", "messageCount": number,
         "created": "ISO-8601", "modified": "ISO-8601", "gitBranch": "string",
         "projectPath": "path", "isSidechain": boolean
       }
     ]
   }
   ```
   GTM OS index has 170 entries (subset of 1,024 JSONL files).

2. **tool-results/ overflow** — 36 session subdirs contain 1,714 `.txt` files (48.28 MB). Tool outputs exceeding inline size limit are persisted as `toolu_<id>.txt` files.

3. **.highwatermark files** in tasks/ directories — Task ID cursor for concurrency control.

4. **claude-opus-4-6 model** — New model appearing in stats-cache.json.

---

## 2. Top-Level JSONL Record Types

[VERIFIED: schema-analyst parsing 12 files + broad scan of 1,117 files, cross-verified by linking-verifier independently scanning 1,030 files. Zero mismatches. 2026-02-05]

There are exactly **7 top-level record types**. Each line in a `.jsonl` file has a `type` field with one of these values:

| # | Type | Description | Participates in parentUuid tree? |
|---|------|-------------|----------------------------------|
| 1 | `assistant` | Model API responses | Yes |
| 2 | `user` | Human input + tool results | Yes |
| 3 | `progress` | Real-time streaming updates | Yes |
| 4 | `system` | System-level events | Yes (except some subtypes) |
| 5 | `summary` | Context compaction summaries | **No** — minimal record |
| 6 | `file-history-snapshot` | File backup tracking | **No** — minimal record |
| 7 | `queue-operation` | User input queue management | **No** — own field set |

**The v1 spec listed "16+ record types" by conflating these 7 top-level types with nested content block types (tool_use, thinking, text, etc.). That was wrong.**
[VERIFIED: schema-analyst Table 1 + cross-verify report]

### 2.1 Base Fields (shared by types 1-4)

All `assistant`, `user`, `progress`, and `system` records share these fields:

```
uuid              # This record's UUID
timestamp         # ISO 8601
type              # Discriminator (one of 7 values)
sessionId         # Parent session UUID
parentUuid        # Previous message in conversation tree (null = root or non-participant)
cwd               # Working directory at record time
version           # Claude Code version (e.g. "2.1.17")
gitBranch         # Active git branch
userType          # "external" in all observed data
isSidechain       # Boolean — true for agent/subagent threads

# Optional agent context:
agentId           # 7-char hex hash (e.g. "a1e1f0c")
slug              # Human-readable name (e.g. "dazzling-nibbling-thacker")
teamName          # Team name for multi-agent sessions
```

[VERIFIED: schema-analyst deep sample + cross-verify of progress base fields]

**Types 5-7 are minimal records** with NO base fields:
- `summary`: only `type`, `summary`, `leafUuid` [VERIFIED: cross-verifier checked 6 records]
- `file-history-snapshot`: only `type`, `messageId`, `snapshot`, `isSnapshotUpdate` [VERIFIED: cross-verifier]
- `queue-operation`: only `type`, `operation`, `content`, `sessionId`, `timestamp` [VERIFIED: cross-verifier]

### 2.2 `type: "assistant"` — Model Responses

[VERIFIED: schema-analyst, 65,853 records in full corpus audit]

Each API response produces **multiple JSONL records** — one per content block. A single API call with `[thinking, text, tool_use, tool_use]` = 4 separate records sharing the same `requestId` but with different `uuid` values chained via `parentUuid`.

**Specific fields:**
- `message` — Raw Anthropic API response object:
  ```json
  {
    "model": "claude-opus-4-5-20251101",
    "id": "msg_...",
    "type": "message",
    "role": "assistant",
    "content": [ContentBlock, ...],
    "stop_reason": "tool_use" | "end_turn" | "stop_sequence" | null,
    "usage": {
      "input_tokens": int, "output_tokens": int,
      "cache_creation_input_tokens": int, "cache_read_input_tokens": int,
      "cache_creation": {"ephemeral_5m_input_tokens": int, "ephemeral_1h_input_tokens": int},
      "service_tier": "standard"
    }
  }
  ```
- `requestId` — API request ID (e.g. `"req_011CWvfH..."`)
- `error` — Error object (when `isApiErrorMessage: true`, `message.model` = `"<synthetic>"`)

**`stop_reason` semantics:**
| Value | Meaning |
|-------|---------|
| `null` | Intermediate block (not the final block of the API response) |
| `"tool_use"` | Response ended with tool call(s) |
| `"end_turn"` | Natural end of response |
| `"stop_sequence"` | Hit stop sequence (usually `<synthetic>` error messages) |

[VERIFIED: schema-analyst examples at agent-aad98a2.jsonl:2]

### 2.3 `type: "user"` — Human Input + Tool Results

[VERIFIED: schema-analyst, 35,260 records in full corpus audit]

Two variants based on `message.content`:

**Variant A: Human input** — `message.content` is a **string**
- Direct human text. No `sourceToolUseID` or `toolUseResult` fields.
- [VERIFIED: schema-analyst, 6,461 records are plain text in full corpus]

**Variant B: Tool result** — `message.content` is an **array** of content blocks
- Returns tool execution output to the model
- `sourceToolAssistantUUID` — UUID of assistant record that made the tool call
- `sourceToolUseID` — The `tool_use` ID being responded to
- `toolUseResult` — Structured metadata about tool execution:
  ```
  type            # "text" | "create" | "update"
  file            # {filePath, content, numLines, startLine, totalLines}
  stdout/stderr   # For Bash results
  durationMs/code # For Bash results
  filenames/numFiles/numMatches  # For Glob/Grep
  structuredPatch/oldString/newString  # For Edit
  agentId/agent_type/isAgent/task     # For Task/Agent
  team_name/teammate_id/recipients    # For team operations
  ```
- [VERIFIED: schema-analyst, 28,801 records are tool results in full corpus]

**Special flags on user records:**
| Flag | Purpose |
|------|---------|
| `isCompactSummary: true` | Compaction summary replacing older messages |
| `isMeta: true` | System-injected metadata |
| `isVisibleInTranscriptOnly: true` | Not sent to model |
| `permissionMode` | Permission mode in effect |
| `planContent` | Plan mode content |
| `todos` | Todo list state snapshot |

### 2.4 `type: "progress"` — Streaming Updates

[VERIFIED: schema-analyst, 27,483 records in full corpus audit. **Missing from v1 spec.**]

Real-time progress updates for long-running operations. Discriminated by `data.type`:

| `data.type` | Description | Key `data` fields |
|-------------|-------------|-------------------|
| `bash_progress` | Bash command streaming | `elapsedTimeSeconds`, `fullOutput`, `output`, `totalLines` |
| `agent_progress` | Subagent activity | `agentId`, `message`, `normalizedMessages`, `prompt` |
| `hook_progress` | Hook execution | `hookEvent`, `hookName`, `command` |
| `mcp_progress` | MCP tool calls | `serverName`, `status`, `toolName` |
| `query_update` | Web search in progress | `query` |
| `search_results_received` | Search results arrived | `query`, `resultCount` |
| `waiting_for_task` | Waiting for background task | `taskDescription`, `taskType` |

All progress records include `toolUseID` and `parentToolUseID` linking them to the tool call they report on.
[VERIFIED: schema-analyst examples at agent-a0248be.jsonl:7, 0b08e794.jsonl, etc.]

### 2.5 `type: "system"` — System Events

[VERIFIED: schema-analyst, 1,838 records in full corpus audit]

Discriminated by `subtype` field:

| `subtype` | Description | Key fields |
|-----------|-------------|------------|
| `turn_duration` | Duration of assistant turn | `durationMs`, `isMeta: true` |
| `local_command` | Slash commands (/clear, /compact) | `content` (command XML), `level: "info"` |
| `compact_boundary` | Context compaction marker | `compactMetadata`, `logicalParentUuid`, `parentUuid: null` |
| `microcompact_boundary` | Selective tool result compaction | `microcompactMetadata` (includes `compactedToolIds`) |
| `api_error` | API errors with retry | `error`, `retryInMs`, `retryAttempt`, `maxRetries` |
| `stop_hook_summary` | Post-turn hook summary | `hookCount`, `hookInfos[]`, `hookErrors[]` |

**Critical distinction:** `compact_boundary` resets `parentUuid` to null and uses `logicalParentUuid` to bridge. `microcompact_boundary` does NOT reset — it preserves `parentUuid` continuity.
[VERIFIED: linking-verifier, 271 compact_boundary + 40 microcompact_boundary records tested]

### 2.6 `type: "summary"` — Compaction Summaries

[VERIFIED: schema-analyst + cross-verifier, 11,241 records in full corpus]

Minimal records with exactly 3 fields:
```json
{"type": "summary", "summary": "Compressed narrative...", "leafUuid": "uuid-of-last-message"}
```

These are prepended to session files during context compaction. A session file can have many summaries (one per compaction event). The `leafUuid` has a **dual purpose**:
1. **Cross-file continuation** — Points to the last message in a parent session (for session resumption)
2. **Same-file compaction bookmark** — Points to the last message before compaction within the same file

[VERIFIED: linking-verifier, 9,799 cross-file + 1,361 same-file leafUuid references out of 11,177 total]

### 2.7 `type: "file-history-snapshot"` — File Backup Tracking

[VERIFIED: schema-analyst + cross-verifier, 8,580 records in full corpus]

Minimal records with 4 fields:
```json
{
  "type": "file-history-snapshot",
  "messageId": "uuid-of-triggering-message",
  "isSnapshotUpdate": false,
  "snapshot": {
    "messageId": "same-uuid",
    "timestamp": "ISO-8601",
    "trackedFileBackups": {
      "relative/file/path": {
        "backupFileName": "hash@vN",
        "version": 3,
        "backupTime": "ISO-8601"
      }
    }
  }
}
```

- `isSnapshotUpdate: false` = initial snapshot (7,162 records)
- `isSnapshotUpdate: true` = incremental update (1,418 records)
- Backup files stored at `~/.claude/file-history/<session-uuid>/<hash>@v<N>`

### 2.8 `type: "queue-operation"` — Input Queue Management

[VERIFIED: schema-analyst, 2,158 records in full corpus]

```json
{"type": "queue-operation", "operation": "enqueue", "content": "message text", "sessionId": "uuid", "timestamp": "ISO-8601"}
```

| Operation | Description |
|-----------|-------------|
| `enqueue` | User typed while assistant was busy |
| `dequeue` | Message dequeued for processing |
| `remove` | Message removed without processing |
| `popAll` | Queue cleared (e.g. after /clear) |

---

## 3. Content Block Types

[VERIFIED: schema-analyst deep sample of 12 files + cross-verified by linking-verifier on 100 files. Zero mismatches.]

These are **NOT** record types. They appear nested inside `message.content[]` arrays within `assistant` and `user` records.

### 3.1 Assistant Content Blocks (`assistant.message.content[]`)

| Type | Fields | Description |
|------|--------|-------------|
| `tool_use` | `id`, `name`, `input` | Tool invocation |
| `thinking` | `thinking`, `signature` | Extended thinking block |
| `text` | `text` | Text response to user |

No other assistant content block types were observed across 1,117 files.
[VERIFIED: schema-analyst + cross-verifier. The v1 spec's `image` and `base64` types were NOT FOUND.]

### 3.2 User Content Blocks (`user.message.content[]`)

| Type | Fields | Description |
|------|--------|-------------|
| `tool_result` | `tool_use_id`, `content`, `is_error` | Tool execution result |
| `text` | `text` | System-injected text (e.g. reminders) |

When `user.message.content` is a **string** (not array), it is direct human input — no content blocks.

### 3.3 `toolUseResult.type` Values

These are metadata annotations on user records, NOT content block types:

| Value | Description |
|-------|-------------|
| `text` | Text-based tool output (Read, Bash, Grep, etc.) |
| `create` | File creation (Write tool) |
| `update` | File modification (Edit tool) |

### 3.4 Complete Type Hierarchy

```
JSONL Line (Top-Level Record Types) ─── 7 types
├── assistant
│   └── message.content[] ─── 3 content block types
│       ├── thinking    {thinking, signature}
│       ├── text        {text}
│       └── tool_use    {id, name, input}
│
├── user
│   ├── message.content (string) ─── human input, no blocks
│   └── message.content[] ─── 2 content block types
│       ├── tool_result {tool_use_id, content, is_error}
│       └── text        {text}  (system-injected)
│   └── toolUseResult.type ─── 3 values
│       ├── text
│       ├── create
│       └── update
│
├── progress
│   └── data.type ─── 7 subtypes
│       ├── bash_progress
│       ├── agent_progress
│       ├── hook_progress
│       ├── mcp_progress
│       ├── query_update
│       ├── search_results_received
│       └── waiting_for_task
│
├── system
│   └── subtype ─── 6 subtypes
│       ├── turn_duration
│       ├── local_command
│       ├── compact_boundary
│       ├── microcompact_boundary
│       ├── api_error
│       └── stop_hook_summary
│
├── summary           {summary, leafUuid}           ← minimal, no base fields
├── file-history-snapshot  {messageId, snapshot, isSnapshotUpdate}  ← minimal
└── queue-operation   {operation, content, sessionId, timestamp}   ← own field set
```

**Total distinct type identifiers:** 7 top-level + 5 content block + 3 toolUseResult values = **15**, cleanly separated into 3 namespaces.

---

## 4. Session Types

### 4.1 Regular Sessions

[VERIFIED: fs-auditor + linking-verifier]

**Format:** UUID (e.g., `846b76ee-3534-49ac-8555-cff4745c4a41`)
**Location:** `projects/<project>/<uuid>.jsonl`
**Characteristics:**
- Started by human user
- May spawn agent sessions
- May have associated directory with `subagents/` and `tool-results/`
- Some UUID files contain ONLY `summary` + `file-history-snapshot` records (the conversation content may live in agent files)

### 4.2 Agent Sessions

[VERIFIED: linking-verifier, 495 agent files tested with 100% match rate]

**Format:** `agent-<7-char-hex>` (e.g., `agent-a179da3`)
**Locations:**
- Top-level: `projects/<project>/agent-<hash>.jsonl` (62 found in GTM OS)
- Subdirectory: `projects/<project>/<parent-uuid>/subagents/agent-<hash>.jsonl` (433 found)

**Characteristics:**
- Spawned by Task tool during parent session
- `isSidechain: true` on all records (100% verified)
- `sessionId` points to parent session UUID (100% verified)
- `agentId` matches the filename hash
- Independent message chain (`parentUuid` starts from null)

**File count by location** [MEASURED: 2026-02-05, GTM OS project]:

| Location | Count |
|----------|-------|
| Top-level regular sessions | 530 UUID files |
| Top-level agent sessions | 62 `agent-*` files |
| Subdirectory agent sessions | 433 `agent-*` files in subagents/ |
| **Total JSONL** | **1,024** (GTM OS only) |

### 4.3 Session Identity Model

[VERIFIED: audit_session_identity.md]

```
SESSION (user starts conversation)
  UUID: e.g. 846b76ee-...
  Tracked in: history.jsonl, projects/*.jsonl, file-history/, tasks/
    │
    ├── MAIN AGENT (agentId = sessionId, same UUID)
    │   Tracked in: todos/{sid}-agent-{sid}.json, debug/{sid}.txt
    │
    ├── SUB-AGENT 1 (agentId = new UUID)
    │   Tracked in: todos/{sid}-agent-{aid}.json, debug/{aid}.txt
    │
    └── SUB-AGENT N (agentId = new UUID)
```

**Why counts diverge across stores:**

| Store | Count | What it counts |
|-------|-------|---------------|
| stats-cache totalSessions | 212 | Sessions at last cache computation (lazy, incomplete) |
| file-history/ dirs | 424 | Sessions that edited files |
| projects/*.jsonl (GTM OS) | 1,024 | All session files including agents |
| history.jsonl unique sessionIds | 902 | User-message sessions across 18 projects |
| todos/ files | 2,872 | Every agent process ever (main + sub-agents, including deleted projects) |

[VERIFIED: audit_session_identity cross-reference matrix, 3,998 unique UUIDs across all stores]

---

## 5. Linking Mechanisms

[VERIFIED: linking-verifier testing against 1,024 session files with Python scripts]

### 5.1 parentUuid — Conversation Tree

**Scope:** Within a single session file
**Direction:** Child → Parent (backward-looking)
**Participants:** `assistant`, `user`, `system`, `progress` records. NOT `summary`, `file-history-snapshot`, `queue-operation`.

**CORRECTION from v1: This is a TREE, not a linear chain.**

```
user (parentUuid: null)                    ← Root
  ├── assistant:thinking (parentUuid: user)     ← API response block 1
  │     └── assistant:tool_use (parentUuid: thinking)
  │           └── user:tool_result (parentUuid: tool_use)
  │                 └── assistant:text (parentUuid: tool_result)
  │
  └── user:tool_result (parentUuid: user)  ← BRANCHING: same parent, different child
```

Evidence: 253 branching points found in one 7,913-record session. Max children per parent: 4.
[VERIFIED: linking-verifier, f3a66b46.jsonl, 6,694 records with parentUuid, 0 broken references]

**Null parentUuid breakdown:**

| Cause | Description |
|-------|-------------|
| First message in session | Root of conversation tree |
| `compact_boundary` system record | Chain reset after compaction |
| `summary`, `file-history-snapshot`, `queue-operation` | Never participate in tree |

### 5.2 Agent Spawn — sessionId + Directory

**Scope:** Cross-file (agent → parent session)
**Direction:** Backward-looking

All agent session files use `sessionId` to identify their parent:
- **Subagent files:** `sessionId` matches parent directory UUID (433/433 = 100%)
- **Top-level agents:** `sessionId` points to a valid parent session file (62/62 = 100%)
[VERIFIED: linking-verifier, 495 total agents, 0 mismatches]

### 5.3 leafUuid — Session Continuation + Compaction Bookmarks

**Scope:** Cross-file (continuation) or within-file (compaction)
**Direction:** Backward-looking
**Location:** `summary` records only

**CORRECTION from v1:** leafUuid has a **dual purpose**:

| Purpose | Count | Description |
|---------|-------|-------------|
| Cross-file continuation | 9,799 | Links resumed session to parent session's last message |
| Same-file compaction bookmark | 1,361 | Records last message before in-session compaction |
| Unresolved | 17 (0.8%) | Likely in other project directories |

[VERIFIED: linking-verifier, 2,028 unique leafUuids, 99.2% resolve]

**Last-summary-not-first rule** (from v1, re-verified):

When session C continues B which continued A, C's file contains summaries stacked oldest-first: `[summary from A, summary from B]`. The **LAST** summary points to the immediate parent (B). The **FIRST** summary points to the root ancestor (A).

[VERIFIED: linking-verifier, 81 multi-target sessions confirm stacking order]

### 5.4 logicalParentUuid — Compaction Bridge

**Scope:** Within session, across compaction boundaries
**Direction:** Post-compaction → Pre-compaction

Only present on `compact_boundary` system records:
```
compact_boundary:
  parentUuid: null                        ← Chain reset
  logicalParentUuid: <pre-compaction-uuid> ← Bridge
  compactMetadata: {trigger: "auto", preTokens: ~155000}

Next record (isCompactSummary user):
  parentUuid: <compact_boundary.uuid>     ← Chains TO boundary
```

[VERIFIED: linking-verifier, 271 compact_boundary records across 3 sessions, 100% resolution]

**`microcompact_boundary` is DIFFERENT:** Does NOT reset parentUuid, does NOT use logicalParentUuid. It preserves chain continuity while compacting specific tool results (tracked via `microcompactMetadata.compactedToolIds`).
[VERIFIED: linking-verifier, 40 microcompact_boundary records, all have non-null parentUuid, no logicalParentUuid]

### 5.5 Additional Linking Mechanisms (Not in v1)

[VERIFIED: linking-verifier discovery from real data]

| Field | Location | Points To | Scope |
|-------|----------|-----------|-------|
| `sourceToolAssistantUUID` | User tool-result records | Assistant that called tool | Within session |
| `sourceToolUseID` | Some user tool-result records | Specific tool_use block ID | Within session |
| `requestId` | All assistant records | Groups blocks from same API call | Within session |
| `messageId` | file-history-snapshot records | Message that caused file changes | Within session |
| `toolUseID` | All progress records | Synthetic progress ID | Within session |
| `parentToolUseID` | All progress records | Actual tool_use block ID | Within session |
| `teamName` | Team session records | Team identity | Cross-session |

### 5.6 Complete Linking Field Reference

| Field | Location | Points To | Scope | In v1? |
|-------|----------|-----------|-------|--------|
| `uuid` | Every record | Self | Identity | Yes |
| `parentUuid` | Records in tree | Previous in conversation tree | Within session | Yes |
| `logicalParentUuid` | compact_boundary only | Pre-compaction message | Within session | Yes |
| `sessionId` | Every record | Parent session UUID | Cross-file | Yes |
| `leafUuid` | summary records | Final message of section | Within or cross-session | Yes |
| `agentId` | Agent records | Self (hash identifier) | Identity | Yes |
| `tool_use_id` | tool_result blocks | Corresponding tool_use | Within message | Yes |
| `sourceToolAssistantUUID` | User records | Assistant that called tool | Within session | **New** |
| `sourceToolUseID` | User records | Specific tool_use ID | Within session | **New** |
| `requestId` | Assistant records | Groups same API response | Within session | **New** |
| `messageId` | file-history-snapshot | Triggering message | Within session | **New** |
| `toolUseID` | Progress records | Progress tracker ID | Within session | **New** |
| `parentToolUseID` | Progress records | Actual tool call | Within session | **New** |
| `teamName` | Team records | Team identity | Cross-session | **New** |

---

## 6. Chain Building Algorithm

### 6.1 What Production Code Actually Does

[VERIFIED: code-checker audit of chain_graph.py, jsonl_parser.py, inverted_index.py]

The tastematter indexer uses only **2 of the 4 primary linking mechanisms**:

| Mechanism | Used? | How |
|-----------|-------|-----|
| leafUuid | **Yes** | chain_graph.py extracts from LAST summary record |
| sessionId | **Yes** | chain_graph.py links agent-* files to parent |
| parentUuid | **No** | Not used for chain building |
| logicalParentUuid | **No** | Not used for chain building |

**Algorithm (from chain_graph.py):**

```python
def build_chains(project_dir):
    # 1. CRITICAL: Recursive glob (line 217)
    all_sessions = list(project_dir.glob("**/*.jsonl"))  # NOT *.jsonl

    # 2. Build session index
    sessions = {}
    for path in all_sessions:
        session = parse_session(path)
        sessions[session.id] = session

    # 3. Pass 1: leafUuid linking (regular session continuation)
    #    Scans summary records at START of file, stops at first non-summary
    #    Uses LAST summary's leafUuid (not first)
    for session in sessions.values():
        last_summary = find_last_summary(session)
        if last_summary and last_summary.leafUuid:
            parent = find_session_containing_message(last_summary.leafUuid)
            if parent and parent.id != session.id:
                session.parent = parent

    # 4. Pass 2: sessionId linking (agent sessions)
    #    Only for files with agent-* prefix
    for session in sessions.values():
        if session.is_agent:
            parent_id = session.records[0].sessionId
            if parent_id in sessions:
                session.parent = sessions[parent_id]

    # 5. Build chains from roots
    roots = [s for s in sessions.values() if not s.parent]
    return build_chain_trees(roots, sessions)
```

### 6.2 What Code Parses vs What Exists

[VERIFIED: code-checker comparison table]

| Data | Parsed? | Details |
|------|---------|---------|
| `assistant` records | Partially | Only tool_use content blocks (name, id, input) |
| `user` records | Partially | Only toolUseResult.filePath and message.content tool_results |
| `file-history-snapshot` | Partially | Only file path keys from trackedFileBackups |
| `summary` records | leafUuid only | Used for chain linking |
| `system` records | **Not parsed** | Turn duration, compaction data lost |
| `progress` records | **Not parsed** | 27,483 records ignored |
| `queue-operation` records | **Not parsed** | Acceptable — low analytical value |
| Token usage (`message.usage`) | **Not extracted** | Cost/model tracking unavailable |
| `parentUuid` chain | **Not reconstructed** | Conversation order not rebuilt |
| `tool-results/` files | **Not indexed** | 1,714 overflow files invisible |

### 6.3 Schema Divergence: Python vs Rust

[VERIFIED: code-checker audit of connection.py vs storage.rs]

| Aspect | Python CLI | Rust Core |
|--------|-----------|-----------|
| `claude_sessions.files_created` | Has column | Missing |
| `claude_sessions.grep_patterns` | Has column | Missing |
| `chain_graph` columns | 7 columns (parent, position, etc.) | 2 columns (session_id, chain_id) |
| `chain_metadata` table | Missing | Has (generated_name, summary) |
| `conversation_intelligence` | Has (unused) | Missing |
| `work_chains` | Has (unused) | Missing |
| Chain rebuild | Incremental | **Destructive** (DROP + recreate) |

---

## 7. Operational Metadata

### 7.1 stats-cache.json

[VERIFIED: fs-auditor + audit_metadata_layer.md]

**Location:** `~/.claude/stats-cache.json`
**Scope:** GLOBAL (all projects combined, not project-scoped)
**Update:** Lazy — only computed when stats view is accessed

| Field | Value [MEASURED: 2026-02-05] | Meaning |
|-------|------|---------|
| `totalSessions` | 212 | Sessions counted at last cache computation (NOT total sessions) |
| `totalMessages` | 150,547 | User + assistant + all record types (NOT user-only) |
| `dailyActivity[]` | 39 entries | Per-day: messageCount, sessionCount, toolCallCount |
| `dailyModelTokens[]` | Per-day per-model token counts | Includes cache breakdown |
| `hourCounts{}` | 24 buckets | Session STARTS per hour-of-day |
| `modelUsage{}` | 3 models | Per-model token counts (input, output, cache) |
| `longestSession` | 20,867 messages | Session open ~98.6 days |
| `firstSessionDate` | 2025-10-06 | |

**Why totalSessions (212) ≠ JSONL file count (1,127):** Stats-cache is computed lazily and counts differently from filesystem discovery.

### 7.2 history.jsonl

[VERIFIED: fs-auditor]

**Location:** `~/.claude/history.jsonl`
**Size:** 4.40 MB, 9,323 lines [MEASURED: 2026-02-05]
**Scope:** Global across all projects

```json
{
  "display": "User input text",
  "pastedContents": "Clipboard data (optional)",
  "timestamp": 1736394754400,
  "project": "C:\\path\\to\\project",
  "sessionId": "uuid"
}
```

- 14.7% of entries lack `sessionId` (pre-sessionId era)
- Spans 18 project paths with 902 unique sessionIds

### 7.3 debug/

[VERIFIED: audit_metadata_layer.md + fs-auditor]

**Size:** 483 MB, 669 files [MEASURED: 2026-02-05]
**Format:** Per-agent plaintext debug logs (`<uuid>.txt`)
**Contains:** Timestamped entries with tool call names+args, plugin loading, permission grants, MCP bridge traffic
**Does NOT contain:** User message text, model response text

### 7.4 Feature Flags (statsig/)

[VERIFIED: audit_metadata_layer.md]

- 48 feature gates (14 true, 34 false)
- 55+ dynamic configs (spinner words, feedback timing, auto-compact threshold, etc.)
- `session_recording_rate: 1` (100% of sessions recorded)
- `auto_compact tokenThreshold: 0.92`

### 7.5 Telemetry

[VERIFIED: audit_metadata_layer.md]

**Only FAILED telemetry events stored locally** (retry queue). Successfully transmitted telemetry is NOT retained. 4 files from Dec 2025 auto-updater failures.

---

## 8. Verification Commands

Run these to re-verify this spec's claims. Expected values are approximate — they grow over time.

### Count JSONL files recursively

```powershell
# Expected: 1,100+ (grows with each session)
(Get-ChildItem -Path "$env:USERPROFILE\.claude\projects" -Recurse -Filter "*.jsonl" -File).Count
```

### Verify 7 top-level record types

```python
# Run: python verify_types.py
import json, sys, pathlib
sys.stdout.reconfigure(encoding='utf-8')
types = set()
p = pathlib.Path.home() / ".claude/projects"
for f in list(p.glob("**/*.jsonl"))[:20]:
    for line in open(f, encoding='utf-8'):
        try:
            r = json.loads(line)
            types.add(r.get("type"))
        except: pass
print(sorted(types))
# Expected: ['assistant', 'file-history-snapshot', 'progress', 'queue-operation', 'summary', 'system', 'user']
```

### Verify summary records are minimal

```python
import json, pathlib
f = next(pathlib.Path.home().glob(".claude/projects/**/*.jsonl"))
for line in open(f, encoding='utf-8'):
    r = json.loads(line)
    if r.get("type") == "summary":
        print(sorted(r.keys()))  # Expected: ['leafUuid', 'summary', 'type']
        break
```

### Verify agent sessionId linking

```python
import json, pathlib
p = pathlib.Path.home() / ".claude/projects"
for f in p.glob("**/subagents/agent-*.jsonl"):
    parent_dir = f.parent.parent.name  # UUID of parent session
    first = json.loads(open(f, encoding='utf-8').readline())
    assert first["sessionId"] == parent_dir, f"MISMATCH: {f}"
    print(f"OK: {f.name} -> {parent_dir}")
```

### Verify parentUuid tree structure (branching)

```python
import json, pathlib, collections
f = next(pathlib.Path.home().glob(".claude/projects/*/*.jsonl"))
parents = collections.Counter()
for line in open(f, encoding='utf-8'):
    r = json.loads(line)
    pu = r.get("parentUuid")
    if pu: parents[pu] += 1
branching = sum(1 for c in parents.values() if c > 1)
print(f"Branching points: {branching}")  # Expected: >0 (tree, not chain)
```

### Verify stats-cache.json exists and has expected fields

```powershell
$s = Get-Content "$env:USERPROFILE\.claude\stats-cache.json" | ConvertFrom-Json
$s | Select-Object version, totalSessions, totalMessages, lastComputedDate
# Expected: version=2, totalSessions>200, totalMessages>150000
```

### Count tool-results overflow files

```powershell
(Get-ChildItem -Path "$env:USERPROFILE\.claude\projects" -Recurse -Filter "toolu_*.txt" -File).Count
# Expected: 1,700+ (grows with tool usage)
```

---

## 9. Known Unknowns

### Cannot verify from local data alone [UNVERIFIABLE]

1. **How stats-cache.json is computed** — The lazy computation algorithm, session boundary definition, and what triggers a recompute are internal to Claude Code. We observe the output, not the process.

2. **Telemetry that sends successfully** — Only failed events are stored locally. What data Anthropic receives from successful telemetry transmissions is unknown.

3. **What the /insights pipeline does server-side** — The `usage-data/report.html` and `facets/` files suggest a pipeline that samples sessions and extrapolates, but the exact methodology is internal.

4. **Why some agents are top-level vs subdirectory** — 62 agents at top level, 433 in subagents/. Version-dependent? Spawn-method-dependent? [INFERRED: likely both, but unconfirmed]

5. **sessions-index.json generation** — New file found with 170 entries vs 1,024 JSONL files. What triggers index updates? Why is it a subset?

6. **The `<synthetic>` model marker** — 190 assistant records with `model: "<synthetic>"`. These appear to be system-generated error messages, not real API calls. [INFERRED: from `isApiErrorMessage: true` correlation]

### Questions for future investigation

7. **Do `image` and `base64` content blocks exist?** — The v1 spec listed them with counts (26 each), but our 2026-02-05 scan of 1,117 files found zero. Were they from a specific session that was deleted? Or were the v1 counts wrong?

8. **What happens to very old session files?** — Are they ever pruned? The todos/ store shrank from 5.2 MB to 0.41 MB within a day, suggesting some cleanup mechanism.

9. **Can the Python and Rust schemas be reconciled?** — The divergence (Python has extra columns, Rust has extra tables) needs resolution for the indexer Rust port.

10. **Are there content block types we haven't seen?** — The Anthropic API supports `image`, `document`, `server_tool_use`, `server_tool_result`, `mcp_tool_use`, `mcp_tool_result`, `redacted_thinking`, `thinking_delta`, `content_block_start`, `content_block_delta`, `content_block_stop`. None were observed in local data. They may appear in future sessions using image/document features.

---

## Appendix A: Data Flow Diagram

```
USER TYPES A MESSAGE
  │
  ├──→ history.jsonl         {display, timestamp, project, sessionId}
  ├──→ projects/.../session.jsonl
  │      type: "user"        {message.content: "string"}
  └──→ stats-cache.json      messageCount++ (on next lazy computation)

CLAUDE RESPONDS (single API call)
  │
  ├──→ projects/.../session.jsonl
  │      type: "assistant"   × N records (one per content block)
  │        ├── {message.content: [{type: "thinking", ...}]}
  │        ├── {message.content: [{type: "text", ...}]}
  │        └── {message.content: [{type: "tool_use", ...}]}
  │      All share same requestId, chained via parentUuid
  │
  ├──→ type: "progress"      {data: {type: "bash_progress", ...}}
  └──→ debug/<agent-uuid>.txt (tool call name + args logged)

TOOL EXECUTES AND RETURNS
  │
  ├──→ projects/.../session.jsonl
  │      type: "user"        {message.content: [{type: "tool_result", ...}]}
  │      + sourceToolAssistantUUID, sourceToolUseID, toolUseResult
  │
  └──→ projects/.../<session>/tool-results/toolu_<id>.txt  (if output too large)

FILE IS EDITED
  │
  ├──→ projects/.../session.jsonl
  │      type: "file-history-snapshot"  {messageId, snapshot.trackedFileBackups}
  │
  └──→ file-history/<session>/<hash>@v<N>  (actual file backup)

CONTEXT GETS COMPACTED
  │
  ├──→ type: "summary"                prepended to file {leafUuid}
  ├──→ type: "system" subtype: "compact_boundary"  {logicalParentUuid, parentUuid: null}
  └──→ type: "user" isCompactSummary: true         (compressed narrative)

SUB-AGENT SPAWNED
  │
  ├──→ projects/.../<parent>/subagents/agent-<hash>.jsonl  (or top-level)
  └──→ todos/<sessionId>-agent-<agentId>.json
```

---

## Appendix B: Corrections from v1

| v1 Claim | v2 Correction | Evidence |
|----------|---------------|----------|
| "16+ record types" | 7 top-level types + 5 content block types + 3 toolUseResult values | schema-analyst + cross-verifier |
| parentUuid forms "linear chain" | Forms a **tree** with branching (253+ branch points) | linking-verifier |
| leafUuid is cross-session only | Dual purpose: cross-session continuation + same-file compaction | linking-verifier (1,361 same-file) |
| 4 linking mechanisms | 4 primary + 7 additional (sourceToolAssistantUUID, requestId, etc.) | linking-verifier |
| microcompact_boundary works like compact_boundary | Different: preserves parentUuid, no logicalParentUuid | linking-verifier (40 records) |
| `image`/`base64` content blocks exist (26 each) | NOT FOUND in any of 1,117 files | schema-analyst + cross-verifier |
| summary/file-history-snapshot have base fields | Minimal records with NO uuid, sessionId, cwd, etc. | cross-verifier (6 records) |
| Missing: `progress` record type | 27,483 records with 7 data subtypes | schema-analyst |
| Missing: `stats-cache.json` | Documented with schema and semantics | fs-auditor + audit_metadata_layer |
| Missing: `sessions-index.json` | New session catalog file with 170 entries | fs-auditor |
| Missing: `tool-results/` overflow | 1,714 files, 48.28 MB | fs-auditor |
| Fixed counts as architecture | All counts marked [MEASURED: DATE] | This spec |

---

**Specification Status:** APPROVED
**Created:** 2026-02-05
**Supersedes:** 07_CLAUDE_CODE_DATA_MODEL.md (2026-01-15)
**Methodology:** 4-agent team (fs-auditor, schema-analyst, linking-verifier, code-checker) with cross-verification. Zero mismatches in cross-verification phase.
**Evidence chain:** 6 agent reports in `_system/temp/` + 4 existing audit reports in `_system/reports/`
