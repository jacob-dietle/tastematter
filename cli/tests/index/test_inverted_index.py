"""Tests for inverted_index module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).

The inverted file index maps: file_path -> List[FileAccess]
This enables queries like "Which sessions touched this file?"

Algorithm:
1. Parse tool_use blocks from JSONL (Read, Edit, Write, etc.)
2. Extract file paths from tool inputs
3. Filter out pseudo-paths (GREP:, GLOB:)
4. Classify access type (read/write/create)
5. Build file -> sessions mapping
"""

import json
import sqlite3
import tempfile
from datetime import datetime
from pathlib import Path

import pytest


class TestFileAccessDataclass:
    """Test FileAccess dataclass properties."""

    def test_file_access_has_required_fields(self):
        """FileAccess should track file access with context.

        RED: Run before implementation - should fail
        GREEN: Implement FileAccess dataclass
        """
        from context_os_events.index.inverted_index import FileAccess

        access = FileAccess(
            session_id="session-a",
            chain_id="chain-1",
            file_path="/path/to/file.py",
            access_type="read",
            tool_name="Read",
            timestamp=datetime.now(),
            access_count=1,
        )

        assert access.session_id == "session-a"
        assert access.chain_id == "chain-1"
        assert access.file_path == "/path/to/file.py"
        assert access.access_type == "read"
        assert access.tool_name == "Read"
        assert access.access_count == 1

    def test_file_access_chain_id_is_optional(self):
        """Chain ID should be optional (None for orphan sessions)."""
        from context_os_events.index.inverted_index import FileAccess

        access = FileAccess(
            session_id="session-a",
            chain_id=None,
            file_path="/path/to/file.py",
            access_type="read",
            tool_name="Read",
            timestamp=datetime.now(),
        )

        assert access.chain_id is None


class TestExtractFileAccesses:
    """Test extracting file accesses from JSONL files."""

    def test_extracts_read_operations_from_jsonl(self):
        """Should extract file paths from Read tool uses.

        RED: Run before implementation - should fail
        GREEN: Implement extract_file_accesses()

        JSONL structure from real Claude Code files:
        {"type":"assistant","uuid":"...","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/path"}}]}}
        """
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"user","uuid":"u1","message":{"role":"user","content":"Read the file"}}',
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            assert len(accesses) == 1
            assert accesses[0].file_path == "/src/main.py"
            assert accesses[0].access_type == "read"
            assert accesses[0].tool_name == "Read"
        finally:
            filepath.unlink()

    def test_extracts_write_operations_from_jsonl(self):
        """Should extract file paths from Edit/Write tool uses."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Edit","input":{"file_path":"/src/utils.py","old_string":"foo","new_string":"bar"}}]}}',
            '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Write","input":{"file_path":"/src/new_file.py","content":"# New file"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            assert len(accesses) == 2

            # Edit operation
            edit_access = next(a for a in accesses if a.tool_name == "Edit")
            assert edit_access.file_path == "/src/utils.py"
            assert edit_access.access_type == "write"

            # Write operation (creates new file)
            write_access = next(a for a in accesses if a.tool_name == "Write")
            assert write_access.file_path == "/src/new_file.py"
            assert write_access.access_type == "create"
        finally:
            filepath.unlink()

    def test_handles_pseudo_paths_separately(self):
        """GREP: and GLOB: patterns should not appear as file paths.

        These are search patterns, not actual file accesses.
        """
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Grep","input":{"pattern":"TODO","path":"."}}]}}',
            '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Glob","input":{"pattern":"**/*.py"}}]}}',
            '{"type":"assistant","uuid":"a3","message":{"role":"assistant","content":[{"type":"tool_use","id":"t3","name":"Read","input":{"file_path":"/src/real_file.py"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            # Should only have the real file, not grep/glob patterns
            assert len(accesses) == 1
            assert accesses[0].file_path == "/src/real_file.py"
            assert not any(a.file_path.startswith("GREP:") for a in accesses)
            assert not any(a.file_path.startswith("GLOB:") for a in accesses)
        finally:
            filepath.unlink()

    def test_deduplicates_within_session(self):
        """Same file accessed multiple times = 1 entry with access_count."""
        from context_os_events.index.inverted_index import extract_file_accesses

        # Same file read 3 times in one session
        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
            '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
            '{"type":"assistant","uuid":"a3","message":{"role":"assistant","content":[{"type":"tool_use","id":"t3","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            # Should have 1 entry with access_count=3
            assert len(accesses) == 1
            assert accesses[0].file_path == "/src/main.py"
            assert accesses[0].access_count == 3
        finally:
            filepath.unlink()

    def test_handles_mixed_read_write_same_file(self):
        """Same file read then written should have separate entries."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
            '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"/src/main.py","old_string":"a","new_string":"b"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            # Same file with different access types = separate entries
            assert len(accesses) == 2
            access_types = {a.access_type for a in accesses}
            assert "read" in access_types
            assert "write" in access_types
        finally:
            filepath.unlink()

    def test_handles_empty_file(self):
        """Empty JSONL file should return empty list."""
        from context_os_events.index.inverted_index import extract_file_accesses

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('')
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)
            assert len(accesses) == 0
        finally:
            filepath.unlink()

    def test_handles_malformed_json(self):
        """Malformed JSON lines should be skipped, not crash."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/valid.py"}}]}}',
            'not valid json at all',
            '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"/also_valid.py"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)
            assert len(accesses) == 2
        finally:
            filepath.unlink()


class TestBuildInvertedIndex:
    """Test building complete inverted index from JSONL directory."""

    def test_build_inverted_index_from_directory(self):
        """Should build file -> sessions mapping from all JSONL files.

        RED: Run before implementation - should fail
        GREEN: Implement build_inverted_index()
        """
        from context_os_events.index.inverted_index import build_inverted_index

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Session A reads file1
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text(
                '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}'
            )

            # Session B reads file1 and file2
            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text('\n'.join([
                '{"type":"assistant","uuid":"b1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
                '{"type":"assistant","uuid":"b2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"/src/utils.py"}}]}}',
            ]))

            index = build_inverted_index(tmpdir)

            # file1 touched by both sessions
            assert "/src/main.py" in index
            sessions_for_main = {a.session_id for a in index["/src/main.py"]}
            assert sessions_for_main == {"session-a", "session-b"}

            # file2 only touched by session B
            assert "/src/utils.py" in index
            sessions_for_utils = {a.session_id for a in index["/src/utils.py"]}
            assert sessions_for_utils == {"session-b"}

    def test_build_inverted_index_with_chain_context(self):
        """Should include chain_id when chains dict is provided."""
        from context_os_events.index.inverted_index import build_inverted_index
        from context_os_events.index.chain_graph import Chain

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Session in a chain
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text(
                '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}'
            )

            # Provide chain context
            chains = {
                "chain-1": Chain(
                    chain_id="chain-1",
                    root_session="session-a",
                    sessions=["session-a"],
                    branches={},
                    time_range=None,
                    total_duration_seconds=0,
                    files_bloom=None,
                    files_list=[],
                )
            }

            index = build_inverted_index(tmpdir, chains=chains)

            # Access should have chain context
            assert "/src/main.py" in index
            access = index["/src/main.py"][0]
            assert access.chain_id == "chain-1"

    def test_build_inverted_index_empty_directory(self):
        """Empty directory should return empty index."""
        from context_os_events.index.inverted_index import build_inverted_index

        with tempfile.TemporaryDirectory() as tmpdir:
            index = build_inverted_index(Path(tmpdir))
            assert len(index) == 0


class TestInvertedIndexQueries:
    """Test querying the inverted index."""

    def test_get_sessions_for_file(self):
        """Should return all sessions that touched a file.

        RED: Run before implementation - should fail
        GREEN: Implement get_sessions_for_file()
        """
        from context_os_events.index.inverted_index import (
            FileAccess,
            get_sessions_for_file,
        )

        # Build test index
        index = {
            "/src/main.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id="chain-1",
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id="chain-1",
                    file_path="/src/main.py",
                    access_type="write",
                    tool_name="Edit",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/utils.py": [
                FileAccess(
                    session_id="session-b",
                    chain_id="chain-1",
                    file_path="/src/utils.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
        }

        accesses = get_sessions_for_file(index, "/src/main.py")

        assert len(accesses) == 2
        session_ids = {a.session_id for a in accesses}
        assert session_ids == {"session-a", "session-b"}

    def test_get_sessions_for_file_not_found(self):
        """Non-existent file should return empty list."""
        from context_os_events.index.inverted_index import get_sessions_for_file

        index = {}
        accesses = get_sessions_for_file(index, "/nonexistent.py")

        assert accesses == []

    def test_get_files_for_session(self):
        """Should return all files touched in a session.

        RED: Run before implementation - should fail
        GREEN: Implement get_files_for_session()
        """
        from context_os_events.index.inverted_index import (
            FileAccess,
            get_files_for_session,
        )

        # Build test index
        index = {
            "/src/main.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/utils.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/utils.py",
                    access_type="write",
                    tool_name="Edit",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/other.py": [
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/other.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
        }

        files = get_files_for_session(index, "session-a")

        assert len(files) == 2
        assert set(files) == {"/src/main.py", "/src/utils.py"}

    def test_get_files_for_session_not_found(self):
        """Non-existent session should return empty list."""
        from context_os_events.index.inverted_index import get_files_for_session

        index = {}
        files = get_files_for_session(index, "nonexistent-session")

        assert files == []


class TestInvertedIndexPersistence:
    """Test persisting inverted index to database."""

    def test_persist_inverted_index(self):
        """Should write to file_conversation_index table.

        RED: Run before implementation - should fail
        GREEN: Implement persist_inverted_index()
        """
        from context_os_events.index.inverted_index import (
            FileAccess,
            persist_inverted_index,
        )

        # Create in-memory database with schema
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE file_conversation_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                session_id TEXT NOT NULL,
                access_type TEXT,
                access_count INTEGER DEFAULT 1,
                first_accessed_at TEXT,
                chain_id TEXT,
                UNIQUE(file_path, session_id, access_type)
            )
        """)

        # Build test index
        now = datetime.now()
        index = {
            "/src/main.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id="chain-1",
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=now,
                    access_count=3,
                ),
            ],
        }

        stats = persist_inverted_index(db, index)

        assert stats["files_stored"] == 1
        assert stats["accesses_stored"] == 1

        # Verify in database
        cursor = db.execute("SELECT * FROM file_conversation_index")
        row = cursor.fetchone()
        assert row[1] == "/src/main.py"  # file_path
        assert row[2] == "session-a"     # session_id
        assert row[3] == "read"          # access_type
        assert row[4] == 3               # access_count

    def test_load_inverted_index(self):
        """Should load inverted index from database.

        RED: Run before implementation - should fail
        GREEN: Implement load_inverted_index()
        """
        from context_os_events.index.inverted_index import load_inverted_index

        # Create in-memory database with test data
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE file_conversation_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                session_id TEXT NOT NULL,
                access_type TEXT,
                access_count INTEGER DEFAULT 1,
                first_accessed_at TEXT,
                chain_id TEXT,
                UNIQUE(file_path, session_id, access_type)
            )
        """)
        db.execute("""
            INSERT INTO file_conversation_index
            (file_path, session_id, access_type, access_count, chain_id)
            VALUES ('/src/main.py', 'session-a', 'read', 2, 'chain-1')
        """)
        db.execute("""
            INSERT INTO file_conversation_index
            (file_path, session_id, access_type, access_count, chain_id)
            VALUES ('/src/main.py', 'session-b', 'write', 1, 'chain-1')
        """)
        db.commit()

        index = load_inverted_index(db)

        assert "/src/main.py" in index
        assert len(index["/src/main.py"]) == 2

        # Check loaded data
        session_ids = {a.session_id for a in index["/src/main.py"]}
        assert session_ids == {"session-a", "session-b"}


class TestRealWorldScenarios:
    """Test with realistic data patterns from actual JSONL files."""

    def test_handles_notebook_edit_tool(self):
        """NotebookEdit tool uses notebook_path instead of file_path."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"NotebookEdit","input":{"notebook_path":"/notebooks/analysis.ipynb","cell_id":"cell1","new_source":"print(1)"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            assert len(accesses) == 1
            assert accesses[0].file_path == "/notebooks/analysis.ipynb"
            assert accesses[0].access_type == "write"
        finally:
            filepath.unlink()

    def test_handles_multiple_tools_in_one_message(self):
        """Assistant message can have multiple tool_use blocks."""
        from context_os_events.index.inverted_index import extract_file_accesses

        # Single message with multiple tool uses (common pattern)
        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/a.py"}},{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"/src/b.py"}},{"type":"tool_use","id":"t3","name":"Read","input":{"file_path":"/src/c.py"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            assert len(accesses) == 3
            paths = {a.file_path for a in accesses}
            assert paths == {"/src/a.py", "/src/b.py", "/src/c.py"}
        finally:
            filepath.unlink()

    def test_handles_tool_with_no_file_path(self):
        """Some tools don't have file paths (WebSearch, etc.)."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"WebSearch","input":{"query":"python async"}}]}}',
            '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"/src/main.py"}}]}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath)

            # Only the Read should be captured
            assert len(accesses) == 1
            assert accesses[0].file_path == "/src/main.py"
        finally:
            filepath.unlink()


class TestSubdirectoryIndexing:
    """Test inverted index includes subdirectory sessions.

    Bug: inverted_index.py:233 uses *.jsonl which misses {session}/subagents/agent-*.jsonl
    Fix: Use **/*.jsonl for recursive discovery

    Reference: specs/implementation/phase_00_glob_bug_fix/SPEC.md
    """

    def test_build_index_includes_subdirectory_sessions(self):
        """Should index file accesses from agent sessions in subdirectories.

        RED: Run before fix - index misses agent sessions in subdirectories
        GREEN: Fix glob pattern to **/*.jsonl - index includes all sessions
        """
        from context_os_events.index.inverted_index import build_inverted_index

        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_dir = Path(tmpdir)

            # Top-level session with file access
            session_content = '{"type":"assistant","uuid":"a1","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/src/main.py"}}]}}'
            (jsonl_dir / "session-1.jsonl").write_text(session_content + '\n')

            # Agent session in subdirectory with different file access
            subdir = jsonl_dir / "session-1" / "subagents"
            subdir.mkdir(parents=True)
            agent_content = '{"type":"assistant","uuid":"a2","message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"/src/agent.py"}}]}}'
            (subdir / "agent-x.jsonl").write_text(agent_content + '\n')

            # Build index
            index = build_inverted_index(jsonl_dir, chains=None)

            # Should include files from BOTH sessions
            indexed_files = set(index.keys())
            assert "/src/main.py" in indexed_files, \
                f"Should index top-level session. Got: {indexed_files}"
            assert "/src/agent.py" in indexed_files, \
                f"Should index agent in subdirectory. Got: {indexed_files}"

    def test_glob_pattern_for_inverted_index(self):
        """Verify the glob pattern difference for inverted index discovery."""
        with tempfile.TemporaryDirectory() as tmpdir:
            jsonl_dir = Path(tmpdir)

            # Create structure with file accesses at different levels
            (jsonl_dir / "session.jsonl").write_text('{"type":"user"}\n')
            subdir = jsonl_dir / "session" / "subagents"
            subdir.mkdir(parents=True)
            (subdir / "agent.jsonl").write_text('{"type":"user"}\n')

            # BUG: *.jsonl only finds top-level
            buggy = list(jsonl_dir.glob("*.jsonl"))
            assert len(buggy) == 1

            # FIX: **/*.jsonl finds all levels
            fixed = list(jsonl_dir.glob("**/*.jsonl"))
            assert len(fixed) == 2


class TestToolUseResultExtraction:
    """Test extraction of file paths from user toolUseResult records.

    Gap 1 from Package 17: Parser misses toolUseResult.filePath in user messages.
    Reference: Canonical spec lines 256-267
    """

    def test_extract_tool_use_result_file_path(self):
        """Should extract filePath from user record's toolUseResult.

        RED: Run before implementation - should fail (no extraction)
        GREEN: Add toolUseResult handling to extract_file_accesses()
        """
        from context_os_events.index.inverted_index import extract_file_accesses

        # User message with toolUseResult (file creation confirmed)
        jsonl_content = [
            json.dumps({
                "type": "user",
                "uuid": "user-uuid-1",
                "timestamp": "2026-01-16T10:00:00.000Z",
                "sessionId": "test-session",
                "toolUseResult": {
                    "type": "create",
                    "filePath": "/path/to/created/file.md",
                    "content": "# New File"
                }
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")

            # Should find the file from toolUseResult
            assert len(accesses) == 1, f"Expected 1 access, got {len(accesses)}"
            assert accesses[0].file_path == "/path/to/created/file.md"
            assert accesses[0].access_type == "create"
            assert accesses[0].tool_name == "toolUseResult"
        finally:
            filepath.unlink()

    def test_classify_tool_use_result_types(self):
        """Should classify access_type based on toolUseResult.type.

        Mapping:
        - "create" → access_type = "create"
        - "update" → access_type = "write"
        - "text"   → access_type = "read"
        """
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            # Create
            json.dumps({
                "type": "user",
                "timestamp": "2026-01-16T10:00:00.000Z",
                "toolUseResult": {"type": "create", "filePath": "/file1.md"}
            }),
            # Update
            json.dumps({
                "type": "user",
                "timestamp": "2026-01-16T10:01:00.000Z",
                "toolUseResult": {"type": "update", "filePath": "/file2.py"}
            }),
            # Text (read)
            json.dumps({
                "type": "user",
                "timestamp": "2026-01-16T10:02:00.000Z",
                "toolUseResult": {"type": "text", "filePath": "/file3.rs"}
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")

            # Sort by file path for predictable order
            accesses.sort(key=lambda a: a.file_path)

            assert len(accesses) == 3, f"Expected 3 accesses, got {len(accesses)}"
            assert accesses[0].access_type == "create"  # file1.md
            assert accesses[1].access_type == "write"   # file2.py
            assert accesses[2].access_type == "read"    # file3.rs
        finally:
            filepath.unlink()

    def test_extract_nested_file_path(self):
        """Should extract filePath from nested file object.

        Some toolUseResult records have filePath in nested file object:
        toolUseResult.file.filePath instead of toolUseResult.filePath
        """
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            json.dumps({
                "type": "user",
                "timestamp": "2026-01-16T10:00:00.000Z",
                "toolUseResult": {
                    "type": "update",
                    "file": {
                        "filePath": "/nested/path/file.py",
                        "content": "# Content"
                    }
                }
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")

            assert len(accesses) == 1
            assert accesses[0].file_path == "/nested/path/file.py"
            assert accesses[0].access_type == "write"
        finally:
            filepath.unlink()


class TestFileHistorySnapshotExtraction:
    """Test extraction of file paths from file-history-snapshot records.

    Gap 2 from Package 17: Parser rejects file-history-snapshot record type.
    Reference: Canonical spec lines 353-372
    """

    def test_extract_tracked_file_paths(self):
        """Should extract file paths from trackedFileBackups keys.

        RED: Run before implementation - should fail (record type rejected)
        GREEN: Add file-history-snapshot handling
        """
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            json.dumps({
                "type": "file-history-snapshot",
                "messageId": "test-msg-id",
                "timestamp": "2026-01-16T09:00:00.000Z",
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
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")

            # Should find both tracked files
            assert len(accesses) == 2, f"Expected 2 accesses, got {len(accesses)}"

            file_paths = {a.file_path for a in accesses}
            assert "/path/to/tracked/file1.py" in file_paths
            assert "/path/to/tracked/file2.md" in file_paths

            # All should be classified as reads (tracking = reading)
            for access in accesses:
                assert access.access_type == "read"
                assert access.tool_name == "file-history-snapshot"
        finally:
            filepath.unlink()

    def test_handles_empty_tracked_files(self):
        """Should handle file-history-snapshot with no tracked files."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            json.dumps({
                "type": "file-history-snapshot",
                "timestamp": "2026-01-16T09:00:00.000Z",
                "snapshot": {
                    "trackedFileBackups": {}
                }
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")
            assert len(accesses) == 0
        finally:
            filepath.unlink()

    def test_handles_windows_paths_in_tracked_files(self):
        """Should handle Windows-style paths in trackedFileBackups keys."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            json.dumps({
                "type": "file-history-snapshot",
                "timestamp": "2026-01-16T09:00:00.000Z",
                "snapshot": {
                    "trackedFileBackups": {
                        "C:\\Users\\test\\file.py": {"version": 1},
                        "_system\\specs\\file.md": {"version": 2}
                    }
                }
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")

            assert len(accesses) == 2
            file_paths = {a.file_path for a in accesses}
            assert "C:\\Users\\test\\file.py" in file_paths
            assert "_system\\specs\\file.md" in file_paths
        finally:
            filepath.unlink()


class TestParserGapFixIntegration:
    """Integration test verifying both gaps are fixed together.

    This tests that all 3 sources are extracted from a single session:
    1. assistant.tool_use (existing)
    2. user.toolUseResult (Gap 1)
    3. file-history-snapshot (Gap 2)
    """

    def test_extracts_all_sources_in_single_session(self):
        """Should extract file paths from all 3 sources in one session."""
        from context_os_events.index.inverted_index import extract_file_accesses

        jsonl_content = [
            # Source 1: Assistant tool_use (existing)
            json.dumps({
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
            }),
            # Source 2: User toolUseResult (Gap 1)
            json.dumps({
                "type": "user",
                "timestamp": "2026-01-16T10:01:00.000Z",
                "toolUseResult": {
                    "type": "create",
                    "filePath": "/user/created.md"
                }
            }),
            # Source 3: file-history-snapshot (Gap 2)
            json.dumps({
                "type": "file-history-snapshot",
                "timestamp": "2026-01-16T10:02:00.000Z",
                "snapshot": {
                    "trackedFileBackups": {
                        "/tracked/file.ts": {"version": 1}
                    }
                }
            })
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            accesses = extract_file_accesses(filepath, "test-session")

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
        finally:
            filepath.unlink()
