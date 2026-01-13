"""Tests for QueryLogger - Write FIRST, implement SECOND.

Following TDD: RED (tests fail) -> GREEN (implement) -> REFACTOR
"""

import tempfile
from datetime import datetime
from pathlib import Path

import pytest


class TestQueryLogger:
    """Test query logging to markdown file."""

    def test_log_creates_file_if_not_exists(self):
        """Logger should create query_log.md if it doesn't exist.

        RED: Run before implementation - should fail
        GREEN: Implement QueryLogger.__init__ and log()
        """
        from context_os_events.visibility.query_logger import QueryLogger, QueryLogEntry

        with tempfile.TemporaryDirectory() as tmpdir:
            log_path = Path(tmpdir) / "query_log.md"
            logger = QueryLogger(log_path)

            entry = QueryLogEntry(
                timestamp=datetime.now(),
                command="context-os status",
                duration_seconds=0.12,
                results_summary="Git: 99, Sessions: 406",
                row_count=None
            )

            logger.log(entry)

            assert log_path.exists()
            content = log_path.read_text()
            assert "context-os status" in content

    def test_log_prepends_new_entries(self):
        """New entries should appear at TOP of file (most recent first).

        RED: Run before implementation
        GREEN: Implement prepend logic
        """
        from context_os_events.visibility.query_logger import QueryLogger, QueryLogEntry

        with tempfile.TemporaryDirectory() as tmpdir:
            log_path = Path(tmpdir) / "query_log.md"
            logger = QueryLogger(log_path)

            # Log first entry
            entry1 = QueryLogEntry(
                timestamp=datetime(2025, 1, 15, 10, 0, 0),
                command="context-os status",
                duration_seconds=0.1,
                results_summary="First",
                row_count=None
            )
            logger.log(entry1)

            # Log second entry
            entry2 = QueryLogEntry(
                timestamp=datetime(2025, 1, 15, 11, 0, 0),
                command="context-os game-trails",
                duration_seconds=0.2,
                results_summary="Second",
                row_count=20
            )
            logger.log(entry2)

            content = log_path.read_text()

            # Second entry should appear BEFORE first
            pos_second = content.find("game-trails")
            pos_first = content.find("status")
            assert pos_second < pos_first, "Newer entries should be at top"

    def test_log_includes_timestamp_and_duration(self):
        """Log entry should include formatted timestamp and duration.

        RED: Run before implementation
        GREEN: Implement formatting
        """
        from context_os_events.visibility.query_logger import QueryLogger, QueryLogEntry

        with tempfile.TemporaryDirectory() as tmpdir:
            log_path = Path(tmpdir) / "query_log.md"
            logger = QueryLogger(log_path)

            entry = QueryLogEntry(
                timestamp=datetime(2025, 1, 15, 16, 45, 23),
                command="context-os game-trails --limit 20",
                duration_seconds=0.23,
                results_summary="20 files returned",
                row_count=20
            )
            logger.log(entry)

            content = log_path.read_text()

            assert "2025-01-15" in content
            assert "16:45:23" in content
            assert "0.23s" in content or "0.23" in content
            assert "game-trails" in content

    def test_get_recent_returns_entries(self):
        """Should be able to read recent entries back.

        RED: Run before implementation
        GREEN: Implement get_recent()
        """
        from context_os_events.visibility.query_logger import QueryLogger, QueryLogEntry

        with tempfile.TemporaryDirectory() as tmpdir:
            log_path = Path(tmpdir) / "query_log.md"
            logger = QueryLogger(log_path)

            # Log 3 entries
            for i in range(3):
                entry = QueryLogEntry(
                    timestamp=datetime(2025, 1, 15, 10 + i, 0, 0),
                    command=f"command-{i}",
                    duration_seconds=0.1,
                    results_summary=f"Result {i}",
                    row_count=i
                )
                logger.log(entry)

            recent = logger.get_recent(limit=2)

            assert len(recent) == 2
            # Most recent first
            assert "command-2" in recent[0].command
