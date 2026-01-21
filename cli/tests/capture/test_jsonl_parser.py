"""Tests for jsonl_parser module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).
"""

import json
import tempfile
from datetime import datetime
from pathlib import Path

import pytest


class TestPathEncoding:
    """Test project path encoding/decoding."""

    def test_encode_project_path_handles_windows(self):
        """Windows paths should encode correctly.

        RED: Run before implementation - should fail
        GREEN: Implement encode_project_path()

        Note: Claude replaces underscores with dashes (discovered from actual .claude/projects/).
        """
        from context_os_events.capture.jsonl_parser import encode_project_path

        path = r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system"
        encoded = encode_project_path(path)

        # Underscores become dashes (Claude's actual behavior)
        assert encoded == "C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system"
        assert ":" not in encoded
        assert "\\" not in encoded
        assert " " not in encoded
        assert "_" not in encoded

    def test_encode_project_path_handles_underscores(self):
        """Underscores should be replaced with dashes (Claude's actual encoding)."""
        from context_os_events.capture.jsonl_parser import encode_project_path

        path = r"C:\Users\dietl\my_project"
        encoded = encode_project_path(path)

        # Underscores become dashes
        assert "_" not in encoded
        assert encoded == "C--Users-dietl-my-project"

    def test_get_claude_projects_dir(self):
        """Should return ~/.claude/projects path."""
        from context_os_events.capture.jsonl_parser import get_claude_projects_dir

        projects_dir = get_claude_projects_dir()

        assert projects_dir.name == "projects"
        assert projects_dir.parent.name == ".claude"


class TestToolUseExtraction:
    """Test extracting tool uses from assistant messages."""

    def test_extract_tool_uses_finds_read_and_edit(self):
        """Should extract file paths from Read and Edit tools.

        RED: Run before implementation
        GREEN: Implement extract_tool_uses()
        """
        from context_os_events.capture.jsonl_parser import extract_tool_uses

        content = [
            {"type": "text", "text": "I'll read the file."},
            {
                "type": "tool_use",
                "id": "toolu_01",
                "name": "Read",
                "input": {"file_path": "/path/to/file.py"}
            },
            {
                "type": "tool_use",
                "id": "toolu_02",
                "name": "Edit",
                "input": {"file_path": "/path/to/other.py", "old_string": "x", "new_string": "y"}
            }
        ]

        tool_uses = extract_tool_uses(content, datetime.now())

        assert len(tool_uses) == 2
        assert tool_uses[0].name == "Read"
        assert tool_uses[0].file_path == "/path/to/file.py"
        assert tool_uses[0].is_read == True
        assert tool_uses[1].name == "Edit"
        assert tool_uses[1].is_write == True

    def test_extract_tool_uses_handles_empty_content(self):
        """Empty content list should return empty tool uses."""
        from context_os_events.capture.jsonl_parser import extract_tool_uses

        tool_uses = extract_tool_uses([], datetime.now())

        assert len(tool_uses) == 0

    def test_extract_tool_uses_handles_text_only(self):
        """Content with only text blocks should return empty tool uses."""
        from context_os_events.capture.jsonl_parser import extract_tool_uses

        content = [
            {"type": "text", "text": "Just some text."},
            {"type": "text", "text": "More text."}
        ]

        tool_uses = extract_tool_uses(content, datetime.now())

        assert len(tool_uses) == 0


class TestGrepPatternExtraction:
    """Test extracting grep patterns for automation analysis."""

    def test_extract_grep_patterns_for_automation(self):
        """Grep patterns should be captured for automation analysis.

        RED: Run before implementation
        GREEN: Implement grep pattern handling
        """
        from context_os_events.capture.jsonl_parser import extract_tool_uses

        content = [
            {
                "type": "tool_use",
                "id": "toolu_01",
                "name": "Grep",
                "input": {"pattern": "TODO|FIXME", "path": "src/"}
            }
        ]

        tool_uses = extract_tool_uses(content, datetime.now())

        assert len(tool_uses) == 1
        assert tool_uses[0].name == "Grep"
        assert tool_uses[0].file_path == "GREP:TODO|FIXME"
        assert tool_uses[0].is_read == True

    def test_extract_glob_patterns(self):
        """Glob patterns should also be captured."""
        from context_os_events.capture.jsonl_parser import extract_tool_uses

        content = [
            {
                "type": "tool_use",
                "id": "toolu_01",
                "name": "Glob",
                "input": {"pattern": "**/*.py"}
            }
        ]

        tool_uses = extract_tool_uses(content, datetime.now())

        assert tool_uses[0].file_path == "GLOB:**/*.py"


class TestSessionAggregation:
    """Test aggregating messages into session summaries."""

    def test_aggregate_session_deduplicates_files(self):
        """Files read multiple times should appear once.

        RED: Run before implementation
        GREEN: Implement aggregate_session()
        """
        from context_os_events.capture.jsonl_parser import (
            aggregate_session, ParsedMessage, ToolUse
        )

        now = datetime(2025, 1, 15, 10, 0)
        messages = [
            ParsedMessage(
                type="assistant", role="assistant", content=[],
                timestamp=now,
                tool_uses=[
                    ToolUse(id="1", name="Read", input={},
                           timestamp=now,
                           file_path="/path/file.py", is_read=True, is_write=False),
                    ToolUse(id="2", name="Read", input={},
                           timestamp=now,
                           file_path="/path/file.py", is_read=True, is_write=False),  # Duplicate
                ]
            )
        ]

        summary = aggregate_session("test-id", "/project", messages, 1000)

        assert len(summary.files_read) == 1  # Deduplicated
        assert summary.tools_used["Read"] == 2  # Count both uses

    def test_aggregate_session_separates_reads_and_writes(self):
        """Should correctly categorize reads vs writes."""
        from context_os_events.capture.jsonl_parser import (
            aggregate_session, ParsedMessage, ToolUse
        )

        now = datetime(2025, 1, 15, 10, 0)
        messages = [
            ParsedMessage(
                type="assistant", role="assistant", content=[],
                timestamp=now,
                tool_uses=[
                    ToolUse(id="1", name="Read", input={},
                           timestamp=now, file_path="/read.py",
                           is_read=True, is_write=False),
                    ToolUse(id="2", name="Edit", input={},
                           timestamp=now, file_path="/edit.py",
                           is_read=False, is_write=True),
                    ToolUse(id="3", name="Write", input={},
                           timestamp=now, file_path="/new.py",
                           is_read=False, is_write=True),
                ]
            )
        ]

        summary = aggregate_session("test-id", "/project", messages, 1000)

        assert "/read.py" in summary.files_read
        assert "/edit.py" in summary.files_written
        assert "/new.py" in summary.files_written
        assert "/new.py" in summary.files_created

    def test_aggregate_session_extracts_grep_patterns(self):
        """Grep patterns should be collected in grep_patterns list."""
        from context_os_events.capture.jsonl_parser import (
            aggregate_session, ParsedMessage, ToolUse
        )

        now = datetime(2025, 1, 15, 10, 0)
        messages = [
            ParsedMessage(
                type="assistant", role="assistant", content=[],
                timestamp=now,
                tool_uses=[
                    ToolUse(id="1", name="Grep", input={"pattern": "TODO"},
                           timestamp=now, file_path="GREP:TODO",
                           is_read=True, is_write=False),
                    ToolUse(id="2", name="Grep", input={"pattern": "FIXME"},
                           timestamp=now, file_path="GREP:FIXME",
                           is_read=True, is_write=False),
                ]
            )
        ]

        summary = aggregate_session("test-id", "/project", messages, 1000)

        assert "TODO" in summary.grep_patterns
        assert "FIXME" in summary.grep_patterns


class TestIncrementalSync:
    """Test incremental sync detection."""

    def test_session_needs_update_new_session(self):
        """New sessions should always need update."""
        from context_os_events.capture.jsonl_parser import session_needs_update
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            # New session = needs update
            assert session_needs_update(conn, 'new-session-id', 500) == True

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)

    def test_session_needs_update_detects_growth(self):
        """Sessions should re-parse when file grows.

        RED: Run before implementation
        GREEN: Implement session_needs_update()
        """
        from context_os_events.capture.jsonl_parser import session_needs_update
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            # Insert session with size 1000
            conn.execute("""
                INSERT INTO claude_sessions (session_id, project_path, file_size_bytes)
                VALUES ('test-id', '/project', 1000)
            """)
            conn.commit()

            # Same size = no update
            assert session_needs_update(conn, 'test-id', 1000) == False

            # Larger size = needs update
            assert session_needs_update(conn, 'test-id', 2000) == True

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)


class TestParseJsonlLine:
    """Test parsing individual JSONL lines."""

    def test_parse_user_message(self):
        """Should parse user message correctly."""
        from context_os_events.capture.jsonl_parser import parse_jsonl_line

        line = json.dumps({
            "type": "user",
            "message": {"role": "user", "content": "Help me fix the bug"},
            "timestamp": "2025-01-15T10:30:00Z"
        })

        msg = parse_jsonl_line(line)

        assert msg is not None
        assert msg.type == "user"
        assert msg.role == "user"
        assert len(msg.tool_uses) == 0

    def test_parse_assistant_with_tool_use(self):
        """Should parse assistant message with tool uses."""
        from context_os_events.capture.jsonl_parser import parse_jsonl_line

        line = json.dumps({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Reading file..."},
                    {
                        "type": "tool_use",
                        "id": "toolu_01",
                        "name": "Read",
                        "input": {"file_path": "/test.py"}
                    }
                ]
            },
            "timestamp": "2025-01-15T10:30:01Z"
        })

        msg = parse_jsonl_line(line)

        assert msg is not None
        assert msg.type == "assistant"
        assert len(msg.tool_uses) == 1
        assert msg.tool_uses[0].name == "Read"

    def test_parse_empty_line_returns_none(self):
        """Empty lines should return None."""
        from context_os_events.capture.jsonl_parser import parse_jsonl_line

        assert parse_jsonl_line("") is None
        assert parse_jsonl_line("  \n") is None

    def test_parse_invalid_json_returns_none(self):
        """Invalid JSON should return None."""
        from context_os_events.capture.jsonl_parser import parse_jsonl_line

        assert parse_jsonl_line("{invalid json}") is None


class TestIntegration:
    """Integration tests with real or fixture data."""

    def test_parse_session_file_complete(self):
        """Should parse a complete session file."""
        from context_os_events.capture.jsonl_parser import parse_session_file

        # Create fixture
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False, encoding='utf-8'
        ) as f:
            # User message
            f.write(json.dumps({
                "type": "user",
                "message": {"role": "user", "content": "Read the file"},
                "timestamp": "2025-01-15T10:00:00Z"
            }) + "\n")

            # Assistant with tool use
            f.write(json.dumps({
                "type": "assistant",
                "message": {
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "Reading..."},
                        {
                            "type": "tool_use",
                            "id": "toolu_01",
                            "name": "Read",
                            "input": {"file_path": "/test.py"}
                        }
                    ]
                },
                "timestamp": "2025-01-15T10:00:01Z"
            }) + "\n")

            # Tool result
            f.write(json.dumps({
                "type": "tool_result",
                "tool_use_id": "toolu_01",
                "content": "file contents here",
                "timestamp": "2025-01-15T10:00:02Z"
            }) + "\n")

            # Another user message
            f.write(json.dumps({
                "type": "user",
                "message": {"role": "user", "content": "Now edit it"},
                "timestamp": "2025-01-15T10:00:03Z"
            }) + "\n")

            fixture_path = Path(f.name)

        try:
            summary = parse_session_file(fixture_path, "/project")

            assert summary.user_message_count == 2
            assert summary.assistant_message_count == 1
            assert "/test.py" in summary.files_read
            assert summary.tools_used.get("Read", 0) == 1
            assert summary.session_id == fixture_path.stem

        finally:
            fixture_path.unlink(missing_ok=True)

    def test_find_session_files_for_project(self):
        """Should find JSONL files in encoded project directory."""
        from context_os_events.capture.jsonl_parser import (
            find_session_files, encode_project_path
        )

        # Create mock claude directory structure
        with tempfile.TemporaryDirectory() as tmpdir:
            claude_dir = Path(tmpdir)

            # Create project folder with encoded name
            project_path = r"C:\Users\test\my_project"
            encoded = encode_project_path(project_path)
            project_dir = claude_dir / encoded
            project_dir.mkdir(parents=True)

            # Create some session files
            (project_dir / "session1.jsonl").write_text("{}")
            (project_dir / "session2.jsonl").write_text("{}")
            (project_dir / "other.txt").write_text("")  # Not JSONL

            files = find_session_files(project_path, claude_dir=claude_dir)

            assert len(files) == 2
            assert all(f.suffix == ".jsonl" for f in files)


class TestSubdirectoryDiscovery:
    """Test discovering JSONL files in subdirectories (glob bug fix).

    Bug: Uses *.jsonl which misses {session}/subagents/agent-*.jsonl
    Fix: Use **/*.jsonl for recursive discovery

    Reference: specs/implementation/phase_00_glob_bug_fix/SPEC.md
    """

    def test_find_session_files_includes_subdirectories(self):
        """Should find JSONL files in subagents/ subdirectories.

        RED: Run before fix - finds only top-level files (2 instead of 3)
        GREEN: Fix glob pattern to **/*.jsonl - finds all files (3)

        This tests the actual Claude Code directory structure where agent
        sessions are stored in {session-uuid}/subagents/agent-*.jsonl
        """
        from context_os_events.capture.jsonl_parser import (
            find_session_files,
            encode_project_path,
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            claude_dir = Path(tmpdir)

            # Create project folder with encoded name
            project_path = r"C:\Users\test\my_project"
            encoded = encode_project_path(project_path)
            project_dir = claude_dir / encoded
            project_dir.mkdir(parents=True)

            # Create Claude Code's actual directory structure:
            # 1. Top-level session
            (project_dir / "session-abc123.jsonl").write_text(
                '{"type":"user","message":"hello"}\n'
            )

            # 2. Another top-level file (agent at root - older format)
            (project_dir / "agent-xyz789.jsonl").write_text(
                '{"type":"user","message":"root agent"}\n'
            )

            # 3. Agent in subagents/ directory (MISSED by *.jsonl bug)
            subagents_dir = project_dir / "session-abc123" / "subagents"
            subagents_dir.mkdir(parents=True)
            (subagents_dir / "agent-def456.jsonl").write_text(
                '{"type":"user","message":"nested agent"}\n'
            )

            # Call function under test
            files = find_session_files(project_path, claude_dir=claude_dir)

            # Should find ALL 3 files (including subdirectory)
            filenames = {f.name for f in files}
            assert "session-abc123.jsonl" in filenames, "Should find top-level session"
            assert "agent-xyz789.jsonl" in filenames, "Should find top-level agent"
            assert "agent-def456.jsonl" in filenames, "Should find agent in subdirectory"
            assert len(files) == 3, f"Expected 3 files, found {len(files)}: {filenames}"

    def test_glob_pattern_behavior_documented(self):
        """Document the difference between *.jsonl and **/*.jsonl patterns.

        This test explicitly shows the bug behavior vs the fix.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create hierarchical structure
            (project_dir / "session-1.jsonl").write_text('{"type":"user"}\n')
            subdir = project_dir / "session-1" / "subagents"
            subdir.mkdir(parents=True)
            (subdir / "agent-1.jsonl").write_text('{"type":"user"}\n')

            # BUG pattern: *.jsonl only finds top-level
            buggy_files = list(project_dir.glob("*.jsonl"))
            assert len(buggy_files) == 1, "*.jsonl misses subdirectory files"

            # FIX pattern: **/*.jsonl finds all levels
            fixed_files = list(project_dir.glob("**/*.jsonl"))
            assert len(fixed_files) == 2, "**/*.jsonl finds all files"
