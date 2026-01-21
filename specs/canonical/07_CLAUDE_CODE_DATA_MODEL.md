---
title: "Claude Code Data Model Specification"
type: architecture-spec
created: 2026-01-15
last_updated: 2026-01-15
status: approved
foundation:
  - "[[canonical/00_VISION]]"
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[context_packages/04_daemon/12_2026-01-15_GLOB_BUG_DISCOVERY]]"
related:
  - "[[context_packages/04_daemon/01_2026-01-13_CLAUDE_CODE_JSONL_DATA_MODEL]]"
  - "[[cli/src/context_os_events/index/chain_graph.py]]"
tags:
  - tastematter
  - claude-code
  - data-model
  - canonical
  - reference
---

# Claude Code Data Model Specification

## Executive Summary

This specification provides the authoritative reference for understanding Claude Code's filesystem structure and data model. It documents the hierarchical session storage, 16+ JSONL record types, and four distinct linking mechanisms that connect sessions into conversation chains.

**Key insight:** Claude Code's data architecture is more sophisticated than a flat file store. It uses a **hierarchical filesystem** where directory structure encodes parent-child relationships, combined with **UUID-based linking** within JSONL records for message chains and session continuations.

**Primary finding (2026-01-15):** Agent sessions spawned during a conversation are stored in `{session-uuid}/subagents/` subdirectories, not at the top level. This hierarchical structure requires recursive file discovery (`**/*.jsonl`) rather than flat globbing (`*.jsonl`).

---

## The Three Abstraction Layers

When working with Claude Code data, it's essential to understand which layer you're operating at:

```
┌─────────────────────────────────────────────────────────────────────┐
│  LAYER 3: Meta-Context (Tastematter)                                │
│  ─────────────────────────────────────                              │
│  What we build: work streams, patterns, chains-of-chains            │
│  Questions answered: "What am I working on?", "Show me trends"      │
│  Data source: SQLite index built from Layer 2                       │
└─────────────────────────────────────────────────────────────────────┘
                                   ↑
                           Indexing/Parsing
                                   ↑
┌─────────────────────────────────────────────────────────────────────┐
│  LAYER 2: Context (Claude Code Session Management)                  │
│  ───────────────────────────────────────────────                    │
│  What Claude Code manages: sessions, chains, continuations          │
│  Questions answered: "Which session?", "Continue conversation?"     │
│  Data format: UUID relationships via leafUuid, sessionId, parentUuid│
└─────────────────────────────────────────────────────────────────────┘
                                   ↑
                              Structured
                                   ↑
┌─────────────────────────────────────────────────────────────────────┐
│  LAYER 1: JSONL Substrate (Raw Files)                               │
│  ─────────────────────────────────────                              │
│  What exists on disk: .jsonl files, directories, tool-results/      │
│  Questions answered: "What records exist?", "What fields?"          │
│  Data format: JSONL lines, filesystem hierarchy                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Bugs often occur when mixing abstractions. The glob bug (missing 218 sessions) was a Layer 1 problem (filesystem discovery) that manifested as a Layer 3 symptom (broken chain filtering).

---

## Layer 1: Filesystem Structure

### Complete Directory Map

```
~/.claude/                           # 1.7GB total
├── .credentials.json                # OAuth tokens (sensitive)
├── settings.json                    # Config: model, plugins, thinking
├── history.jsonl                    # 3.6MB - Global user input log
│
├── projects/                        # 883MB (52% of total) - THE MAIN DATA
│   └── {project-path}/              # URL-encoded project directory
│       │
│       │   ┌─────────────────────────────────────────────────────┐
│       │   │  REGULAR SESSIONS (UUID format)                     │
│       │   │  Example: 846b76ee-3534-49ac-8555-cff4745c4a41      │
│       │   └─────────────────────────────────────────────────────┘
│       │
│       ├── {session-uuid}.jsonl     # Session conversation log
│       │
│       ├── {session-uuid}/          # Session directory (if has children)
│       │   │
│       │   ├── subagents/           # ⚠️  AGENT SESSIONS LIVE HERE
│       │   │   ├── agent-{hash}.jsonl
│       │   │   └── agent-{hash}.jsonl
│       │   │
│       │   └── tool-results/        # Large tool outputs (384+ files)
│       │       └── toolu_{id}.txt
│       │
│       │   ┌─────────────────────────────────────────────────────┐
│       │   │  AGENT SESSIONS (agent-{7-char-hash} format)        │
│       │   │  Example: agent-a179da3                             │
│       │   │  Can be at top level OR in subagents/ directory     │
│       │   └─────────────────────────────────────────────────────┘
│       │
│       └── agent-{hash}.jsonl       # Top-level agents (441 found)
│
├── debug/                           # 409MB - Session debug logs (1,452 files)
├── plugins/                         # 112MB - MCP plugins & marketplaces
├── file-history/                    # 33MB - Git-like file versioning
│   └── {uuid}/                      # 19,414 versioned file snapshots
│       └── {hash}@v{N}              # Content-addressed with version
│
├── plans/                           # 616KB - Plan mode markdown (76 files)
│   └── {adjective}-{verb}-{noun}.md # Generated names
│
├── todos/                           # 4.3MB - Per-conversation todos
│   └── {uuid}-agent-{uuid}.json     # 2,352 files (mostly empty)
│
├── shell-snapshots/                 # 3.8MB - Bash env snapshots (887 files)
├── cache/                           # Changelog cache
├── chrome/                          # Chrome extension integration
├── downloads/                       # Installer binaries
├── ide/                             # IDE process locks
├── paste-cache/                     # Clipboard history
├── statsig/                         # Feature flags
└── telemetry/                       # Failed telemetry logs
```

### Critical Discovery: Hierarchical Session Storage

**What was assumed:**
```
projects/{project}/
├── session1.jsonl
├── session2.jsonl
├── agent-xxx.jsonl    # All agents at top level
└── ...
```

**What actually exists:**
```
projects/{project}/
├── session1.jsonl                    # Regular session
├── session1/                         # Directory FOR session1
│   ├── subagents/                    # Agent children of session1
│   │   ├── agent-xxx.jsonl           # ← MISSED by *.jsonl glob
│   │   └── agent-yyy.jsonl           # ← MISSED by *.jsonl glob
│   └── tool-results/
│       └── toolu_xxx.txt
└── agent-aaa.jsonl                   # Some agents at top level
```

**The numbers (GTM Operating System project):**

| Location | Count | Discovery Method |
|----------|-------|------------------|
| Top-level regular sessions | 324 | `glob("*.jsonl")` |
| Top-level agent sessions | 441 | `glob("*.jsonl")` |
| **Subdirectory agent sessions** | **218** | `glob("**/*.jsonl")` only |
| **Total** | **983** | After recursive glob |

**Fix applied:** `chain_graph.py` line 217 changed from `*.jsonl` to `**/*.jsonl`

[VERIFIED: Glob comparison 2026-01-15, chain_graph.py:217]

### File Type Inventory

| Type | Count | Purpose |
|------|-------|---------|
| `.jsonl` | 983 | Session conversation logs |
| `.txt` (tool-results) | 384 | Large tool outputs stored separately |
| `.md` (plans) | 76 | Plan mode documents |
| `.json` (todos) | 2,352 | Per-conversation todo state |
| `{hash}@v{N}` | 19,414 | Versioned file snapshots |
| `.lock` (ide) | 4 | Process locks |

---

## Layer 2: JSONL Record Structure

### Base Record Schema

Every record in a `.jsonl` file contains these fields:

```typescript
interface BaseRecord {
  // Identity
  uuid: string;                    // Unique identifier for this record
  timestamp: string;               // ISO-8601 format
  type: RecordType;                // Discriminator field

  // Session context
  sessionId: string;               // Parent session UUID
  cwd: string;                     // Working directory at record time
  version: string;                 // Claude Code version (e.g., "2.1.2")
  gitBranch?: string;              // Active git branch

  // Message chain
  parentUuid: string | null;       // Previous message in chain
  logicalParentUuid?: string;      // Pre-compaction parent (if compacted)

  // Agent context (only on agent sessions)
  agentId?: string;                // Agent hash (e.g., "a179da3")
  isSidechain?: boolean;           // true = agent session

  // Metadata
  userType?: "external" | "internal";
  slug?: string;                   // Human-friendly session name
  isMeta?: boolean;                // Metadata-only record
}
```

### Record Type Taxonomy

The `type` field discriminates between 16+ record types:

| Type | Count* | Purpose | Key Fields |
|------|--------|---------|------------|
| `assistant` | 15,979 | AI response messages | `message.content[]`, `usage`, `requestId` |
| `user` | 8,368 | User input messages | `message.content`, `toolUseResult` |
| `tool_use` | 6,842 | Tool invocations (nested in assistant) | `id`, `name`, `input` |
| `tool_result` | 6,812 | Tool outputs (nested in user) | `tool_use_id`, `content` |
| `thinking` | 6,168 | Extended thinking (nested in assistant) | `thinking`, `signature` |
| `text` | 5,140 | Text content blocks (nested) | `text` |
| `file-history-snapshot` | 1,689 | File backup tracking | `snapshot.trackedFileBackups` |
| `summary` | 1,043 | Session boundary markers | `summary`, `leafUuid` |
| `queue-operation` | 875 | Todo queue operations | `operation`, `content` |
| `system` | 273 | System events | `subtype`, varies by subtype |
| `create` | 399 | File creation (in toolUseResult) | `filePath`, `content` |
| `update` | 50 | File update (in toolUseResult) | `filePath`, `content` |
| `image` | 26 | Image content | `source.data` |
| `base64` | 26 | Base64 encoded content | `data` |
| `error` | 6 | API errors | `error.type`, `error.message` |

*Counts from GTM Operating System session analysis

### Record Type Details

#### User Record

```jsonl
{
  "type": "user",
  "uuid": "3e46e012-9476-4155-bbcf-f3e10d2610db",
  "timestamp": "2026-01-09T02:32:34.400Z",
  "sessionId": "0c5d2026-66dd-46cb-a8a4-71fd62d64b11",
  "parentUuid": "previous-message-uuid",

  "message": {
    "role": "user",
    "content": "string | ToolResultContent[]"
  },

  "toolUseResult": {                    // Present when returning tool output
    "type": "create | text | update",
    "filePath": "string",
    "content": "string",
    "file": {
      "filePath": "string",
      "content": "string",
      "numLines": 100,
      "startLine": 1,
      "totalLines": 100
    }
  }
}
```

#### Assistant Record

```jsonl
{
  "type": "assistant",
  "uuid": "msg-uuid",
  "timestamp": "2026-01-09T02:32:35.000Z",
  "sessionId": "session-uuid",
  "parentUuid": "user-message-uuid",

  "message": {
    "model": "claude-opus-4-5-20251101",
    "id": "msg_01ABC...",
    "type": "message",
    "role": "assistant",
    "content": [
      { "type": "thinking", "thinking": "...", "signature": "..." },
      { "type": "text", "text": "..." },
      { "type": "tool_use", "id": "toolu_01...", "name": "Read", "input": {...} }
    ],
    "stop_reason": "end_turn | tool_use | null",
    "usage": {
      "input_tokens": 1000,
      "output_tokens": 500,
      "cache_creation_input_tokens": 100,
      "cache_read_input_tokens": 200
    }
  },
  "requestId": "req_01XYZ..."
}
```

#### Summary Record (Chain Linking)

```jsonl
{
  "type": "summary",
  "uuid": "summary-uuid",
  "timestamp": "2026-01-09T03:00:00.000Z",
  "sessionId": "session-uuid",

  "summary": "Human-readable summary of this conversation section",
  "leafUuid": "c775f26e-dae9-407f-b695-8fde7882d33f"  // Final message UUID
}
```

The `leafUuid` field is critical for chain linking - it points to the last message UUID before this summary was created. When a session is resumed, the new session's first summary contains a `leafUuid` pointing to the parent session's final message.

#### System Record (Multiple Subtypes)

```jsonl
// Compaction boundary
{
  "type": "system",
  "subtype": "compact_boundary",
  "compactMetadata": {
    "trigger": "auto | manual",
    "preTokens": 50000
  }
}

// API error with retry
{
  "type": "system",
  "subtype": "api_error",
  "error": {
    "status": 529,
    "error": { "type": "overloaded_error", "message": "Overloaded" }
  },
  "retryInMs": 5000,
  "retryAttempt": 1,
  "maxRetries": 3
}

// Local command
{
  "type": "system",
  "subtype": "local_command",
  "content": "/clear"
}
```

#### File History Snapshot Record

```jsonl
{
  "type": "file-history-snapshot",
  "messageId": "8e72e868-62a9-44dc-9eff-e9ff96b05991",
  "snapshot": {
    "messageId": "8e72e868-62a9-44dc-9eff-e9ff96b05991",
    "trackedFileBackups": {
      "/path/to/file.py": {
        "backupFileName": "062145421cb57553@v3",
        "version": 3,
        "backupTime": "2026-01-15T15:18:45.931Z"
      }
    },
    "timestamp": "2026-01-15T15:18:45.931Z"
  },
  "isSnapshotUpdate": false
}
```

---

## Linking Mechanisms

Claude Code uses four distinct mechanisms to connect sessions and messages:

### 1. Message Chain (`parentUuid`)

**Scope:** Within a single session file
**Direction:** Child → Parent (backward-looking)

```
┌──────────────────────────────────────────────────────┐
│  session.jsonl                                        │
│                                                       │
│  msg1 (parentUuid: null)        ← First message      │
│    ↓                                                  │
│  msg2 (parentUuid: msg1.uuid)                        │
│    ↓                                                  │
│  msg3 (parentUuid: msg2.uuid)                        │
│    ↓                                                  │
│  msg4 (parentUuid: msg3.uuid)   ← Latest message     │
│                                                       │
└──────────────────────────────────────────────────────┘
```

**Purpose:** Reconstructs conversation order within a session.

### 2. Agent Spawn (`sessionId` + directory)

**Scope:** Cross-file (parent → agent sessions)
**Direction:** Agent → Parent (backward-looking)

```
┌──────────────────────────────────────────────────────┐
│  846b76ee.../                                         │
│  ├── 846b76ee....jsonl         (parent session)      │
│  │     sessionId: "846b76ee..."                       │
│  │                                                    │
│  └── subagents/                                       │
│      ├── agent-a179da3.jsonl                         │
│      │     sessionId: "846b76ee..."  ← Links to parent│
│      │     agentId: "a179da3"                         │
│      │     isSidechain: true                          │
│      │                                                │
│      └── agent-a2cadda.jsonl                         │
│            sessionId: "846b76ee..."  ← Links to parent│
│            agentId: "a2cadda"                         │
│            isSidechain: true                          │
└──────────────────────────────────────────────────────┘
```

**Purpose:** Connects spawned agent work back to parent conversation.

**Discovery method:**
1. Check `sessionId` field in agent's first record
2. OR extract parent from directory path: `{parent}/subagents/agent-*.jsonl`

### 3. Session Continuation (`leafUuid` in summary)

**Scope:** Cross-file (session → previous session)
**Direction:** Continuation → Previous (backward-looking)

```
┌─────────────────────────────┐     ┌─────────────────────────────┐
│  Session A (ended)          │     │  Session B (continuation)   │
│                             │     │                             │
│  msg1 → msg2 → msg3         │     │  msg1 (parentUuid: null)    │
│                    ↓        │     │    ↑                        │
│  summary {                  │     │  summary {                  │
│    leafUuid: msg3.uuid ─────┼─────┼──▶ leafUuid: msg3.uuid      │
│  }                          │     │  }                          │
│                             │     │                             │
└─────────────────────────────┘     └─────────────────────────────┘
```

**Purpose:** Links continued conversations across session boundaries.

**Important nuance:** Multiple summaries can exist in a session. Claude Code stacks summaries oldest-first when continuing sessions. The `leafUuid` in the **last** summary record indicates the immediate parent for chain linking. Earlier summaries point to ancestors in the chain (Package 11 investigation, 2026-01-15).

### 4. Compaction Skip (`logicalParentUuid`)

**Scope:** Within a session, across compaction boundaries
**Direction:** Post-compaction → Pre-compaction (backward-looking)

```
┌──────────────────────────────────────────────────────┐
│  session.jsonl                                        │
│                                                       │
│  msg1 → msg2 → ... → msg50                           │
│                         ↓                             │
│  [COMPACTION BOUNDARY]  │                            │
│                         ↓                             │
│  msg51 (parentUuid: null,                            │
│         logicalParentUuid: msg50.uuid) ← Skip link   │
│    ↓                                                  │
│  msg52 (parentUuid: msg51.uuid)                      │
│                                                       │
└──────────────────────────────────────────────────────┘
```

**Purpose:** Maintains logical conversation continuity when compaction resets `parentUuid`.

### Linking Field Reference

| Field | Location | Points To | Scope |
|-------|----------|-----------|-------|
| `uuid` | Every record | Self | Identity |
| `parentUuid` | Every record | Previous message | Within session |
| `logicalParentUuid` | Post-compaction records | Pre-compaction message | Within session |
| `sessionId` | Every record | Parent session | Cross-file |
| `leafUuid` | Summary records | Final message of section | Cross-session |
| `agentId` | Agent records | Self (hash identifier) | Identity |
| `tool_use_id` | Tool result content | Corresponding tool_use | Within message |

---

## history.jsonl Structure

The global history file (`~/.claude/history.jsonl`) tracks user inputs across all projects:

```jsonl
{
  "display": "User input text or command",
  "pastedContents": "Content pasted from clipboard (if any)",
  "timestamp": 1736394754400,           // Unix milliseconds
  "project": "C:\\path\\to\\project",
  "sessionId": "846b76ee-3534-49ac-8555-cff4745c4a41"
}
```

**Fields:**
- `display`: The text shown in Claude Code's input area
- `pastedContents`: Clipboard data pasted during input (may be large)
- `timestamp`: Unix timestamp in milliseconds
- `project`: Full path to project directory
- `sessionId`: Which session this input was sent to

**Size:** ~3.6MB for ~20,000+ entries

**Use case:** Global audit log of user interactions, can be used to trace which sessions belong to which project.

---

## Tool Results Storage

When tool outputs exceed a size threshold, they're stored externally:

```
{session-uuid}/tool-results/toolu_01F2eEpmefTxTyAhCVYxBYi1.txt
```

**Reference in JSONL:**
```jsonl
{
  "type": "tool_result",
  "tool_use_id": "toolu_01F2eEpmefTxTyAhCVYxBYi1",
  "content": "Output too large (30.7KB). Full output saved to: C:\\Users\\...\\toolu_01F2eEpmefTxTyAhCVYxBYi1.txt"
}
```

**Current state:** 384 `.txt` files found in tool-results directories. Not currently indexed by Tastematter.

---

## Session Types

### Regular Sessions

**Format:** UUID (e.g., `846b76ee-3534-49ac-8555-cff4745c4a41`)
**Location:** `projects/{project}/{uuid}.jsonl`
**Characteristics:**
- Started by human user
- May spawn agent sessions
- May have associated directory with subagents/ and tool-results/

### Agent Sessions

**Format:** `agent-{7-char-hash}` (e.g., `agent-a179da3`)
**Location:**
- Top-level: `projects/{project}/agent-{hash}.jsonl` (441 found)
- Subdirectory: `projects/{project}/{parent}/subagents/agent-{hash}.jsonl` (218 found)

**Characteristics:**
- Spawned by Task tool during parent session
- `isSidechain: true` in all records
- `sessionId` points to parent session UUID
- Independent message chain (`parentUuid` starts from null)

**Why some agents are at top level vs subdirectory:**
- Possibly version-dependent (older Claude Code versions used flat structure)
- Possibly depends on how agent was spawned
- Both patterns are valid and must be discovered

---

## Chain Building Algorithm

The correct algorithm for building session chains:

```python
def build_chains(project_dir):
    # 1. CRITICAL: Use recursive glob
    all_sessions = list(project_dir.glob("**/*.jsonl"))  # NOT *.jsonl

    # 2. Build session index
    sessions = {}
    for path in all_sessions:
        session = parse_session(path)
        sessions[session.id] = session

    # 3. Pass 1: Link via leafUuid (regular session continuation)
    for session in sessions.values():
        first_summary = find_first_summary(session)
        if first_summary and first_summary.leafUuid:
            parent = find_session_containing_message(first_summary.leafUuid)
            if parent and parent.id != session.id:
                session.parent = parent

    # 4. Pass 2: Link via sessionId (agent sessions)
    for session in sessions.values():
        if session.is_agent:
            parent_id = session.records[0].sessionId
            if parent_id in sessions:
                session.parent = sessions[parent_id]

    # 5. Pass 3: Link via directory structure (fallback for agents)
    for session in sessions.values():
        if session.is_agent and not session.parent:
            parent_id = extract_parent_from_path(session.path)
            if parent_id in sessions:
                session.parent = sessions[parent_id]

    # 6. Build chains from roots
    roots = [s for s in sessions.values() if not s.parent]
    return build_chain_trees(roots, sessions)
```

---

## Common Pitfalls

### Pitfall 1: Flat Glob Pattern

**Wrong:**
```python
jsonl_files = list(project_dir.glob("*.jsonl"))
```

**Correct:**
```python
jsonl_files = list(project_dir.glob("**/*.jsonl"))
```

**Impact:** Missing 218 agent sessions (28% of agents in GTM project)

### Pitfall 2: Using Wrong leafUuid

**Wrong:** Use FIRST leafUuid found in session (points to root ancestor)
**Wrong:** Use ALL leafUuids found in session
**Correct:** Use leafUuid from LAST summary record only (immediate parent)

**Why:** Claude Code stacks summaries oldest-first when continuing sessions. When session C continues B which continued A, C gets `[summary from A, summary from B]`. The FIRST summary points to the root ancestor; the LAST summary points to the immediate parent. For proper chain linking, use LAST. (Package 11 investigation, 2026-01-15)

### Pitfall 3: Assuming All Agents in Subdirectories

**Reality:**
- 441 agents at top level (`projects/{project}/agent-*.jsonl`)
- 218 agents in subdirectories (`projects/{project}/{parent}/subagents/agent-*.jsonl`)

Both patterns must be handled.

### Pitfall 4: Ignoring tool-results/

**Reality:** 384 tool output files exist and may contain important context. Consider indexing for complete visibility.

---

## Statistics (GTM Operating System)

| Metric | Value |
|--------|-------|
| Total session files | 983 |
| Regular sessions | 324 |
| Agent sessions (top-level) | 441 |
| Agent sessions (subdirectory) | 218 |
| Largest chain | 356 sessions |
| Tool result files | 384 |
| Total ~/.claude size | 1.7GB |
| projects/ size | 883MB (52%) |
| debug/ size | 409MB (24%) |

---

## For Future Agents

### Quick Reference

1. **Finding all sessions:**
   ```bash
   find ~/.claude/projects -name "*.jsonl" -type f
   # OR in Python: Path(project_dir).glob("**/*.jsonl")
   ```

2. **Identifying session type:**
   - UUID format → Regular session
   - `agent-*` format → Agent session
   - Check `isSidechain` field for confirmation

3. **Finding parent of agent:**
   - Read first record's `sessionId` field
   - OR extract from path: `{parent}/subagents/agent-*.jsonl`

4. **Finding session continuation:**
   - Read first summary record's `leafUuid`
   - Search other sessions for message with that UUID

### Do NOT

- Use `*.jsonl` glob (misses subdirectory agents)
- Assume flat file structure
- Use leafUuid from compaction summaries for chain linking
- Ignore the projects/ directory hierarchy

### Do

- Use `**/*.jsonl` for recursive discovery
- Check both `sessionId` field AND directory path for agent parents
- Verify chain counts match Claude Code UI after indexing
- Consider tool-results/ for complete context

---

## References

**Investigation sources:**
- [[12_2026-01-15_GLOB_BUG_DISCOVERY]] - Original bug discovery
- [[01_2026-01-13_CLAUDE_CODE_JSONL_DATA_MODEL]] - Initial data model analysis
- [[11_2026-01-15_CHAIN_TOPOLOGY_INVESTIGATION]] - Topology analysis

**Implementation:**
- [[chain_graph.py]] - Chain building implementation (with fix at line 217)

**Related specs:**
- [[03_CORE_ARCHITECTURE]] - Tastematter architecture
- [[05_INTELLIGENCE_LAYER_ARCHITECTURE]] - Higher-level intelligence patterns

---

**Specification Status:** APPROVED
**Created:** 2026-01-15
**Author:** Deep forensic investigation session
**Evidence:** 3 parallel exploration agents analyzing ~/.claude structure
**Next Action:** Rebuild database with recursive glob, verify chain counts

