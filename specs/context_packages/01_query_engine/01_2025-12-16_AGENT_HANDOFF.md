---
title: "AGENT HANDOFF"
package_number: 1
date: 2025-12-16
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/01_2025-12-16_AGENT_HANDOFF.md"
tags:
  - context-package
  - query-engine
  - legacy
---

# Agent Handoff Context - Context OS Intelligence Layer (v2)

**Purpose:** Complete context package for the next agent to implement the intelligence layer.

---

## Quick Start

### What You're Building

A **two-layer intelligence system**:
1. **Deterministic Index Layer** - Fast primitives from parsed data (no LLM)
2. **Intelligent Agent Layer** - Reasoning and judgment on top of the index

The system should know the user's workstreams better than they know themselves.

### Key Discovery: leafUuid

Claude Code already tracks conversation chains explicitly via `leafUuid`:

```json
// At start of JSONL file
{"type":"summary","summary":"Context OS Daemon","leafUuid":"22288505-..."}
```

The `leafUuid` points to a `message.uuid` in the parent conversation. **This gives us 80% of chain detection for free.**

### Read These Files

1. `01_ARCHITECTURE_GUIDE.md` - Two-layer architecture, leafUuid chains
2. `02_INDEX_STRUCTURES.md` - Detailed index specifications
3. `00_CURRENT_STATE.md` - What's currently built

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────┐
│  AGENT LAYER (Judgment)                                     │
│  • Workstream classification                                │
│  • Cross-chain reasoning                                    │
│  • Natural language queries                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  INDEX LAYER (Primitives)                                   │
│  • Chain graph (leafUuid-based)                             │
│  • File tree index (annotated trie)                         │
│  • Inverted file index (file → sessions)                    │
│  • Co-access matrix (game trails)                           │
│  • Temporal buckets (weekly)                                │
│  • Bloom filters (fast checks)                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  RAW DATA                                                   │
│  • JSONL files (primary)                                    │
│  • Git commits                                              │
│  • File events (supplementary)                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Order

### Phase 1: Chain Graph from leafUuid

**File:** `index/chain_graph.py`

```python
def build_chain_graph(jsonl_dir: Path) -> Dict[str, Chain]:
    # Pass 1: Collect leafUuid → sessions
    # Pass 2: Find uuid → session ownership
    # Pass 3: Build parent-child links
    # Pass 4: Group into chains
```

**Test:** Verify chain detection matches Claude Code's UI chain display.

### Phase 2: Inverted File Index

**File:** `index/inverted_index.py`

```python
def build_inverted_index(sessions: List[Session]) -> Dict[str, List[FileAccess]]:
    # Extract file paths from tool_use blocks
    # Read, Edit, Write, Grep, Glob
```

**Test:** Query "which sessions touched jsonl_parser.py?" returns correct list.

### Phase 3: File Tree Annotation

**File:** `index/file_tree.py`

```python
def build_file_tree_index(project: Path, file_index, chains) -> FileTreeNode:
    # Create tree from directory structure
    # Add access records at file nodes
    # Bubble up chains/sessions to parents
```

**Test:** `get_directory_stats("apps/context_os_events/")` returns correct counts.

### Phase 4: Co-Access Matrix

**File:** `index/co_access.py`

```python
def compute_co_access_matrix(file_index) -> Dict[str, List[Tuple[str, float]]]:
    # Compute Jaccard similarity between file pairs
    # Keep pairs with >= 30% overlap
```

**Test:** `get_co_accessed("jsonl_parser.py")` returns test_parser.py with high score.

### Phase 5: Temporal Buckets

**File:** `index/temporal.py`

```python
def build_temporal_buckets(sessions, chains, commits) -> Dict[str, TemporalBucket]:
    # Group by week
    # Add bloom filters for fast "was file touched?" checks
```

**Test:** `get_week_summary("2025-W50")` returns sessions/chains/commits.

### Phase 6: Unified Interface

**File:** `index/context_index.py`

```python
class ContextIndex:
    def get_chain(chain_id) -> Chain
    def get_sessions_for_file(path) -> List[FileAccess]
    def get_directory_stats(path) -> DirectoryStats
    def get_co_accessed(path) -> List[Tuple[str, float]]
    def chain_touched_file(chain_id, path) -> bool  # Bloom filter
```

---

## Current Codebase State

### What EXISTS

```
apps/context_os_events/
├── src/context_os_events/
│   ├── capture/
│   │   ├── jsonl_parser.py     # Enhanced - extracts signals
│   │   ├── git_sync.py         # Working
│   │   └── file_watcher.py     # Working
│   ├── daemon/                 # Working (NSSM service)
│   ├── db/
│   │   ├── schema.sql          # Has Layer 4/5 tables
│   │   └── connection.py       # Working
│   ├── intelligence/
│   │   ├── queries.py          # Basic queries
│   │   └── jsonl_context.py    # Context fetching
│   └── cli.py                  # Working
├── data/
│   └── context_os_events.db    # 460 sessions
└── tests/                      # 47 passing
```

### What DOESN'T EXIST (Build This)

```
apps/context_os_events/
├── src/context_os_events/
│   └── index/                  # NEW DIRECTORY
│       ├── __init__.py
│       ├── chain_graph.py      # leafUuid chain detection
│       ├── file_tree.py        # Annotated trie
│       ├── inverted_index.py   # File → sessions
│       ├── co_access.py        # Game trails
│       ├── temporal.py         # Weekly buckets
│       ├── bloom.py            # Bloom filter impl
│       └── context_index.py    # Unified interface
```

---

## Key Data Locations

### JSONL Files (Primary Data Source)

```
~/.claude/projects/C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system/*.jsonl
```

Each file is one conversation session. Contains:
- `type: "summary"` records with `leafUuid` (chain links)
- `type: "user"` and `type: "assistant"` messages
- Tool uses in assistant message `content` arrays

### Database

```
apps/context_os_events/data/context_os_events.db
```

Schema version 2.0. Has tables for sessions, commits, intelligence (empty).

---

## Critical Implementation Details

### Parsing leafUuid

```python
# At start of each JSONL file, look for summary records
for line in jsonl_file:
    record = json.loads(line)
    if record.get("type") == "summary":
        leaf_uuid = record.get("leafUuid")
        summary_text = record.get("summary")
        # This leafUuid points to message.uuid in parent conversation
```

### Matching leafUuid to Parent

```python
# A leafUuid in file B points to a message.uuid in file A
# File A "owns" that uuid, File B "continues from" it

# Build mapping: uuid → session_id
for jsonl_file in all_files:
    session_id = jsonl_file.stem
    for line in jsonl_file:
        record = json.loads(line)
        if "uuid" in record:
            uuid_to_session[record["uuid"]] = session_id

# Then resolve: leafUuid → parent session
parent_session = uuid_to_session.get(leaf_uuid)
```

### Extracting File Access

```python
# Tool uses are in assistant message content
for message in session.messages:
    if message.type == "assistant":
        for block in message.content:
            if block.get("type") == "tool_use":
                tool_name = block["name"]
                input_data = block["input"]

                if tool_name == "Read":
                    file_path = input_data["file_path"]
                elif tool_name == "Edit":
                    file_path = input_data["file_path"]
                # etc.
```

---

## Testing Strategy

### Unit Tests

```python
def test_chain_graph_from_leafuuid():
    """Chains correctly built from leafUuid references."""
    # Create mock JSONL with known chain structure
    # Verify chain graph matches expected

def test_inverted_index():
    """File → sessions mapping correct."""
    # Parse sessions with known file access
    # Verify index returns correct sessions

def test_co_access_jaccard():
    """Jaccard scores computed correctly."""
    # Create sessions with known file overlap
    # Verify Jaccard scores match expected
```

### Integration Tests

```python
def test_full_index_build():
    """Complete index builds without error."""
    index = build_full_index(project_path, db_path)
    assert index.get_chain("chain_001") is not None

def test_chain_file_bloom():
    """Bloom filter correctly reports membership."""
    # Build index
    # Verify bloom filter matches actual file list
```

---

## Performance Targets

| Operation | Target |
|-----------|--------|
| Full index build (500 sessions) | < 30s |
| Get chain | < 10ms |
| Get sessions for file | < 10ms |
| Chain touched file? (bloom) | < 1ms |
| Get directory stats | < 50ms |

---

## User's Philosophy

> "The system should know the user's workstreams better than they know themselves and serve the right information at the right time."

> "Index gives efficient primitives. Agent provides judgment and reasoning."

> "Fog of war" - click deeper into file tree, more context is revealed.

---

## Common Pitfalls

1. **Don't use heuristics for chain detection** - Use leafUuid
2. **Don't rebuild co-access on every refresh** - It's expensive, do periodically
3. **Don't store full file lists in bloom filters** - Bloom filters are approximate
4. **Don't forget to bubble up stats** - Directory counts aggregate from children

---

## Success Criteria

1. Chain detection matches Claude Code UI (verify manually)
2. File tree visualization shows correct session counts
3. Co-access suggestions are useful (user validates)
4. "What was I working on?" returns accurate answer
5. Index builds in < 30 seconds

---

## Files in This Spec Package

```
_system/specs/context_os_intelligence/
├── 00_CURRENT_STATE.md           # What's built
├── 01_ARCHITECTURE_GUIDE.md      # Two-layer architecture
├── 02_INDEX_STRUCTURES.md        # Detailed index specs
├── 03_INTELLIGENCE_EXTRACTION_SPEC.md  # Agent layer (future)
├── 04_CHAIN_DETECTION_SPEC.md    # Deprecated (use leafUuid)
└── AGENT_HANDOFF_CONTEXT.md      # THIS FILE
```

---

## Next Steps

1. Create `index/` directory
2. Implement `chain_graph.py` with leafUuid parsing
3. Implement `inverted_index.py` with tool_use extraction
4. Build and test with real data (460 sessions)
5. Add CLI commands: `context-os index build`, `context-os index stats`
6. Integrate with existing daemon for incremental updates

Good luck. The foundation is solid.
