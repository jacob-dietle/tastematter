# Phase 2.5: TDD Implementation Guide

**Methodology:** Red-Green-Refactor
**Total Tests:** 6 new tests
**Total Code:** ~75 lines

---

## TDD Order of Operations

| Step | Phase | Test/Code | File | Est. Time |
|------|-------|-----------|------|-----------|
| 1 | RED | Write test for toolUseResult extraction | test_inverted_index.py | 10 min |
| 2 | GREEN | Implement toolUseResult extraction | inverted_index.py | 20 min |
| 3 | RED | Write test for nested filePath | test_inverted_index.py | 5 min |
| 4 | GREEN | Handle nested file.filePath | inverted_index.py | 10 min |
| 5 | RED | Write test for file-history-snapshot | test_inverted_index.py | 10 min |
| 6 | GREEN | Implement file-history-snapshot extraction | inverted_index.py | 20 min |
| 7 | RED | Write integration test (both sources) | test_inverted_index.py | 10 min |
| 8 | GREEN | Verify integration | - | 5 min |
| 9 | REFACTOR | Update jsonl_parser.py for consistency | jsonl_parser.py | 15 min |
| 10 | VERIFY | Run full test suite | - | 5 min |
| **Total** | | | | **~110 min** |

---

## Step 1: RED - Test toolUseResult Extraction

**File:** `cli/tests/index/test_inverted_index.py`

**Add to existing test file:**

```python
import tempfile
import json
from pathlib import Path


class TestToolUseResultExtraction:
    """Test extraction of file paths from user toolUseResult records."""

    def test_extract_tool_use_result_file_path(self):
        """Should extract filePath from user record's toolUseResult.

        RED: Run before implementation - should fail (no extraction)
        GREEN: Add toolUseResult handling to extract_file_accesses()

        Reference: Canonical spec lines 256-267
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create test JSONL with toolUseResult
            jsonl_file = Path(tmpdir) / "test-session.jsonl"
            records = [
                # User message with toolUseResult (file creation confirmed)
                {
                    "type": "user",
                    "uuid": "user-uuid-1",
                    "timestamp": "2026-01-16T10:00:00.000Z",
                    "sessionId": "test-session",
                    "toolUseResult": {
                        "type": "create",
                        "filePath": "/path/to/created/file.md",
                        "content": "# New File"
                    }
                }
            ]
            jsonl_file.write_text("\n".join(json.dumps(r) for r in records))

            # Extract file accesses
            from context_os_events.index.inverted_index import extract_file_accesses
            accesses = extract_file_accesses(jsonl_file, "test-session")

            # Should find the file from toolUseResult
            assert len(accesses) == 1, f"Expected 1 access, got {len(accesses)}"
            assert accesses[0].file_path == "/path/to/created/file.md"
            assert accesses[0].access_type == "create"
            assert accesses[0].tool_name == "toolUseResult"

    def test_classify_tool_use_result_types(self):
        """Should classify access_type based on toolUseResult.type.

        Mapping:
        - "create" → access_type = "create"
        - "update" → access_type = "write"
        - "text"   → access_type = "read"
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_file = Path(tmpdir) / "test-session.jsonl"
            records = [
                # Create
                {
                    "type": "user",
                    "timestamp": "2026-01-16T10:00:00.000Z",
                    "toolUseResult": {"type": "create", "filePath": "/file1.md"}
                },
                # Update
                {
                    "type": "user",
                    "timestamp": "2026-01-16T10:01:00.000Z",
                    "toolUseResult": {"type": "update", "filePath": "/file2.py"}
                },
                # Text (read)
                {
                    "type": "user",
                    "timestamp": "2026-01-16T10:02:00.000Z",
                    "toolUseResult": {"type": "text", "filePath": "/file3.rs"}
                }
            ]
            jsonl_file.write_text("\n".join(json.dumps(r) for r in records))

            from context_os_events.index.inverted_index import extract_file_accesses
            accesses = extract_file_accesses(jsonl_file, "test-session")

            # Sort by file path for predictable order
            accesses.sort(key=lambda a: a.file_path)

            assert len(accesses) == 3
            assert accesses[0].access_type == "create"  # file1.md
            assert accesses[1].access_type == "write"   # file2.py
            assert accesses[2].access_type == "read"    # file3.rs
```

**Run test (should FAIL):**
```bash
cd apps/tastematter/cli
pytest tests/index/test_inverted_index.py::TestToolUseResultExtraction -v
```

---

## Step 2: GREEN - Implement toolUseResult Extraction

**File:** `cli/src/context_os_events/index/inverted_index.py`

**Modify `extract_file_accesses()` function (around line 140):**

```python
# Add constant at top of file (after line 53)
TOOL_USE_RESULT_TYPE_TO_ACCESS = {
    "create": "create",
    "update": "write",
    "text": "read",
}


def extract_file_accesses(filepath: Path, session_id: Optional[str] = None) -> List[FileAccess]:
    """Extract file accesses from a JSONL session file.

    Parses:
    - tool_use blocks from assistant messages
    - toolUseResult from user messages (NEW)
    - file-history-snapshot records (NEW)
    """
    if session_id is None:
        session_id = filepath.stem

    access_tracker: Dict[tuple, FileAccess] = {}

    try:
        with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue

                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue

                record_type = record.get("type")

                # Extract timestamp
                timestamp_str = record.get("timestamp")
                if timestamp_str:
                    try:
                        timestamp = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
                    except (ValueError, AttributeError):
                        timestamp = datetime.now()
                else:
                    timestamp = datetime.now()

                # === NEW: Handle user records with toolUseResult ===
                if record_type == "user":
                    tool_use_result = record.get("toolUseResult")
                    if tool_use_result:
                        # Extract file path (try direct, then nested)
                        file_path = tool_use_result.get("filePath")
                        if not file_path:
                            file_obj = tool_use_result.get("file", {})
                            file_path = file_obj.get("filePath")

                        if file_path:
                            # Classify access type
                            result_type = tool_use_result.get("type", "text")
                            access_type = TOOL_USE_RESULT_TYPE_TO_ACCESS.get(result_type, "read")

                            key = (file_path, access_type)
                            if key in access_tracker:
                                access_tracker[key].access_count += 1
                            else:
                                access_tracker[key] = FileAccess(
                                    session_id=session_id,
                                    chain_id=None,
                                    file_path=file_path,
                                    access_type=access_type,
                                    tool_name="toolUseResult",
                                    timestamp=timestamp,
                                    access_count=1,
                                )
                    continue  # Done with user record

                # === Existing: Handle assistant records with tool_use ===
                if record_type == "assistant":
                    # ... existing code unchanged ...
```

**Run test (should PASS):**
```bash
pytest tests/index/test_inverted_index.py::TestToolUseResultExtraction::test_extract_tool_use_result_file_path -v
pytest tests/index/test_inverted_index.py::TestToolUseResultExtraction::test_classify_tool_use_result_types -v
```

---

## Step 3: RED - Test Nested filePath

**Add to test class:**

```python
    def test_extract_nested_file_path(self):
        """Should extract filePath from nested file object.

        Some toolUseResult records have filePath in nested file object:
        toolUseResult.file.filePath instead of toolUseResult.filePath
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_file = Path(tmpdir) / "test-session.jsonl"
            records = [
                {
                    "type": "user",
                    "timestamp": "2026-01-16T10:00:00.000Z",
                    "toolUseResult": {
                        "type": "update",
                        "file": {
                            "filePath": "/nested/path/file.py",
                            "content": "# Content"
                        }
                    }
                }
            ]
            jsonl_file.write_text("\n".join(json.dumps(r) for r in records))

            from context_os_events.index.inverted_index import extract_file_accesses
            accesses = extract_file_accesses(jsonl_file, "test-session")

            assert len(accesses) == 1
            assert accesses[0].file_path == "/nested/path/file.py"
            assert accesses[0].access_type == "write"
```

---

## Step 4: GREEN - Handle Nested filePath

Already implemented in Step 2 (the code checks both locations).

---

## Step 5: RED - Test file-history-snapshot

**Add new test class:**

```python
class TestFileHistorySnapshotExtraction:
    """Test extraction of file paths from file-history-snapshot records."""

    def test_extract_tracked_file_paths(self):
        """Should extract file paths from trackedFileBackups keys.

        RED: Run before implementation - should fail (record type rejected)
        GREEN: Add file-history-snapshot handling

        Reference: Canonical spec lines 353-372
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_file = Path(tmpdir) / "test-session.jsonl"
            records = [
                {
                    "type": "file-history-snapshot",
                    "messageId": "test-msg-id",
                    "snapshot": {
                        "trackedFileBackups": {
                            "/path/to/tracked/file1.py": {
                                "backupFileName": "abc123@v3",
                                "version": 3
                            },
                            "/path/to/tracked/file2.md": {
                                "backupFileName": None,
                                "version": 1
                            }
                        }
                    }
                }
            ]
            jsonl_file.write_text("\n".join(json.dumps(r) for r in records))

            from context_os_events.index.inverted_index import extract_file_accesses
            accesses = extract_file_accesses(jsonl_file, "test-session")

            # Should find both tracked files
            assert len(accesses) == 2, f"Expected 2 accesses, got {len(accesses)}"

            file_paths = {a.file_path for a in accesses}
            assert "/path/to/tracked/file1.py" in file_paths
            assert "/path/to/tracked/file2.md" in file_paths

            # All should be classified as reads (tracking = reading)
            for access in accesses:
                assert access.access_type == "read"
                assert access.tool_name == "file-history-snapshot"

    def test_handles_empty_tracked_files(self):
        """Should handle file-history-snapshot with no tracked files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_file = Path(tmpdir) / "test-session.jsonl"
            records = [
                {
                    "type": "file-history-snapshot",
                    "snapshot": {
                        "trackedFileBackups": {}
                    }
                }
            ]
            jsonl_file.write_text("\n".join(json.dumps(r) for r in records))

            from context_os_events.index.inverted_index import extract_file_accesses
            accesses = extract_file_accesses(jsonl_file, "test-session")

            assert len(accesses) == 0
```

**Run test (should FAIL):**
```bash
pytest tests/index/test_inverted_index.py::TestFileHistorySnapshotExtraction -v
```

---

## Step 6: GREEN - Implement file-history-snapshot Extraction

**Add to `extract_file_accesses()` after user record handling:**

```python
                # === NEW: Handle file-history-snapshot records ===
                if record_type == "file-history-snapshot":
                    snapshot = record.get("snapshot", {})
                    tracked_files = snapshot.get("trackedFileBackups", {})

                    for file_path in tracked_files.keys():
                        # All tracked files are classified as reads
                        key = (file_path, "read")
                        if key in access_tracker:
                            access_tracker[key].access_count += 1
                        else:
                            access_tracker[key] = FileAccess(
                                session_id=session_id,
                                chain_id=None,
                                file_path=file_path,
                                access_type="read",
                                tool_name="file-history-snapshot",
                                timestamp=timestamp,
                                access_count=1,
                            )
                    continue  # Done with file-history-snapshot
```

**Run test (should PASS):**
```bash
pytest tests/index/test_inverted_index.py::TestFileHistorySnapshotExtraction -v
```

---

## Step 7: RED - Integration Test (Both Sources)

**Add integration test:**

```python
class TestParserGapFixIntegration:
    """Integration test verifying both gaps are fixed together."""

    def test_extracts_all_sources_in_single_session(self):
        """Should extract file paths from all 3 sources in one session:
        1. assistant.tool_use (existing)
        2. user.toolUseResult (Gap 1)
        3. file-history-snapshot (Gap 2)
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_file = Path(tmpdir) / "test-session.jsonl"
            records = [
                # Source 1: Assistant tool_use (existing)
                {
                    "type": "assistant",
                    "timestamp": "2026-01-16T10:00:00.000Z",
                    "message": {
                        "content": [
                            {
                                "type": "tool_use",
                                "id": "toolu_1",
                                "name": "Read",
                                "input": {"file_path": "/assistant/read.py"}
                            }
                        ]
                    }
                },
                # Source 2: User toolUseResult (Gap 1)
                {
                    "type": "user",
                    "timestamp": "2026-01-16T10:01:00.000Z",
                    "toolUseResult": {
                        "type": "create",
                        "filePath": "/user/created.md"
                    }
                },
                # Source 3: file-history-snapshot (Gap 2)
                {
                    "type": "file-history-snapshot",
                    "timestamp": "2026-01-16T10:02:00.000Z",
                    "snapshot": {
                        "trackedFileBackups": {
                            "/tracked/file.ts": {"version": 1}
                        }
                    }
                }
            ]
            jsonl_file.write_text("\n".join(json.dumps(r) for r in records))

            from context_os_events.index.inverted_index import extract_file_accesses
            accesses = extract_file_accesses(jsonl_file, "test-session")

            # Should find all 3 files
            assert len(accesses) == 3, f"Expected 3 accesses, got {len(accesses)}"

            file_paths = {a.file_path for a in accesses}
            assert "/assistant/read.py" in file_paths, "Missing assistant tool_use"
            assert "/user/created.md" in file_paths, "Missing user toolUseResult"
            assert "/tracked/file.ts" in file_paths, "Missing file-history-snapshot"

            # Verify tool_names
            tool_names = {a.tool_name for a in accesses}
            assert "Read" in tool_names
            assert "toolUseResult" in tool_names
            assert "file-history-snapshot" in tool_names
```

---

## Step 8: GREEN - Verify Integration

**Run full test suite:**
```bash
pytest tests/index/test_inverted_index.py -v
```

---

## Step 9: REFACTOR - Update jsonl_parser.py

The `jsonl_parser.py` file also has the same limitation (line 311). Update for consistency, though `inverted_index.py` is the primary extraction path.

**File:** `cli/src/context_os_events/capture/jsonl_parser.py`

**Modify `parse_jsonl_line()` (line 311):**

```python
def parse_jsonl_line(line: str) -> Optional[ParsedMessage]:
    """Parse a single line from JSONL file."""
    # ... existing code ...

    msg_type = data.get("type")

    # BEFORE: Rejected everything except user/assistant/tool_result
    # if msg_type not in ("user", "assistant", "tool_result"):
    #     return None

    # AFTER: Also accept file-history-snapshot for file tracking
    if msg_type not in ("user", "assistant", "tool_result", "file-history-snapshot"):
        return None

    # ... rest of function ...
```

**Also update tool use extraction (line 330) to handle user toolUseResult:**

```python
    # Extract tool uses if assistant message
    tool_uses = []
    if msg_type == "assistant" and isinstance(content, list):
        tool_uses = extract_tool_uses(content, timestamp)

    # NEW: Extract from toolUseResult in user messages
    if msg_type == "user":
        tool_use_result = data.get("toolUseResult")
        if tool_use_result:
            file_path = tool_use_result.get("filePath")
            if not file_path:
                file_obj = tool_use_result.get("file", {})
                file_path = file_obj.get("filePath")

            if file_path:
                result_type = tool_use_result.get("type", "text")
                is_write = result_type in ("create", "update")
                is_read = result_type == "text"

                tool_uses.append(ToolUse(
                    id="toolUseResult",
                    name="toolUseResult",
                    input={"filePath": file_path, "type": result_type},
                    timestamp=timestamp,
                    file_path=file_path,
                    is_read=is_read,
                    is_write=is_write,
                ))
```

---

## Step 10: VERIFY - Full Test Suite

**Run all tests:**
```bash
cd apps/tastematter/cli
pytest tests/ -v --tb=short
```

**Expected:** 461+ tests passing (existing 461 + 6 new)

---

## Verification After Implementation

### Database Rebuild

```bash
# Rebuild with new extraction
tastematter parse-sessions
tastematter build-index

# OR if using daemon
tastematter daemon rebuild
```

### Verify New Sources Captured

```bash
# Check tool_name distribution
sqlite3 ~/.context-os/context_os_events.db \
  "SELECT tool_name, COUNT(*) as count
   FROM file_conversation_index
   GROUP BY tool_name
   ORDER BY count DESC"
```

**Expected output should include:**
```
Read|XXXX
Edit|XXX
Write|XX
toolUseResult|XXX      <-- NEW
file-history-snapshot|XXX  <-- NEW
```

### Compare File Counts

```bash
# Query with chain filter
./core/target/release/context-os query flex --time 7d --chain 93a22459 --agg count

# Should show MORE files than before fix
```

---

## Success Criteria Checklist

- [ ] `test_extract_tool_use_result_file_path` passes
- [ ] `test_classify_tool_use_result_types` passes
- [ ] `test_extract_nested_file_path` passes
- [ ] `test_extract_tracked_file_paths` passes
- [ ] `test_handles_empty_tracked_files` passes
- [ ] `test_extracts_all_sources_in_single_session` passes
- [ ] All 461 existing tests still pass
- [ ] Database rebuild completes successfully
- [ ] `tool_name` query shows new sources
- [ ] File counts increased after rebuild
