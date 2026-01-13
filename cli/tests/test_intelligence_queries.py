"""Tests for intelligence query module.

TDD Phase: RED - These tests should FAIL until queries.py is implemented.
"""

import sqlite3
from datetime import datetime, timedelta
from pathlib import Path

import pytest


class TestIntelligenceQueries:
    """Test suite for intelligence query functions."""

    @pytest.fixture
    def test_db(self, tmp_path):
        """Create test database with sample data."""
        db_path = tmp_path / "test.db"
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row

        # Create schema
        conn.executescript("""
            CREATE TABLE claude_sessions (
                session_id TEXT PRIMARY KEY,
                project_path TEXT,
                started_at TEXT,
                ended_at TEXT,
                duration_seconds INTEGER,
                user_message_count INTEGER,
                assistant_message_count INTEGER,
                total_messages INTEGER,
                files_read TEXT,
                files_written TEXT,
                files_created TEXT,
                tools_used TEXT,
                grep_patterns TEXT,
                file_size_bytes INTEGER,
                parsed_at TEXT
            );

            CREATE TABLE git_commits (
                hash TEXT PRIMARY KEY,
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
                is_agent_commit BOOLEAN,
                is_merge_commit BOOLEAN,
                synced_at TEXT
            );

            CREATE VIEW game_trails AS
            SELECT
                json_each.value as path,
                COUNT(*) as total_accesses
            FROM claude_sessions, json_each(files_read)
            GROUP BY path
            ORDER BY total_accesses DESC;

            CREATE VIEW tool_patterns AS
            SELECT
                json_each.key as tool,
                SUM(json_each.value) as total_uses,
                COUNT(DISTINCT session_id) as sessions_used_in
            FROM claude_sessions, json_each(tools_used)
            GROUP BY json_each.key
            ORDER BY total_uses DESC;
        """)

        # Insert test data
        now = datetime.now()
        yesterday = now - timedelta(days=1)
        last_week = now - timedelta(days=7)

        # Recent session (yesterday)
        conn.execute("""
            INSERT INTO claude_sessions VALUES (
                'session-001', '/test/project',
                ?, ?, 3600,
                10, 15, 25,
                '["file1.py", "file2.py", "config.yaml"]',
                '["file1.py"]',
                '[]',
                '{"Read": 5, "Edit": 3, "Bash": 2}',
                '["pattern1", "pattern2"]',
                1000000,
                ?
            )
        """, (yesterday.isoformat(), yesterday.isoformat(), now.isoformat()))

        # Older session (last week)
        conn.execute("""
            INSERT INTO claude_sessions VALUES (
                'session-002', '/test/project',
                ?, ?, 1800,
                5, 8, 13,
                '["file1.py", "file3.py"]',
                '["file3.py"]',
                '["file3.py"]',
                '{"Read": 3, "Write": 1}',
                '[]',
                500000,
                ?
            )
        """, (last_week.isoformat(), last_week.isoformat(), now.isoformat()))

        # Recent commit
        conn.execute("""
            INSERT INTO git_commits VALUES (
                'abc123def456', 'abc123',
                ?, 'feat: add feature',
                'Test User', 'test@example.com',
                '["file1.py", "file2.py"]',
                '["file2.py"]', '[]', '["file1.py"]',
                50, 10, 2,
                1, 0, ?
            )
        """, (yesterday.isoformat(), now.isoformat()))

        conn.commit()
        yield conn
        conn.close()

    def test_get_recent_sessions_returns_sessions_within_days(self, test_db):
        """get_recent_sessions should return sessions within specified days."""
        from context_os_events.intelligence.queries import get_recent_sessions

        # Get sessions from last 3 days (should include yesterday's session)
        sessions = get_recent_sessions(test_db, days=3)

        assert len(sessions) == 1
        assert sessions[0]["session_id"] == "session-001"

    def test_get_recent_sessions_respects_days_filter(self, test_db):
        """get_recent_sessions should filter by days correctly."""
        from context_os_events.intelligence.queries import get_recent_sessions

        # Get sessions from last 14 days (should include both)
        sessions = get_recent_sessions(test_db, days=14)

        assert len(sessions) == 2

    def test_get_game_trails_returns_most_accessed(self, test_db):
        """get_game_trails should return files ordered by access count."""
        from context_os_events.intelligence.queries import get_game_trails

        trails = get_game_trails(test_db, limit=10)

        # file1.py appears in both sessions, should be first (most accessed)
        assert len(trails) > 0
        assert trails[0]["path"] == "file1.py"
        # file1.py should have more accesses than other files
        assert trails[0]["total_accesses"] >= 2

    def test_get_game_trails_respects_limit(self, test_db):
        """get_game_trails should respect limit parameter."""
        from context_os_events.intelligence.queries import get_game_trails

        trails = get_game_trails(test_db, limit=2)

        assert len(trails) <= 2

    def test_get_tool_patterns_returns_usage(self, test_db):
        """get_tool_patterns should return tool usage breakdown."""
        from context_os_events.intelligence.queries import get_tool_patterns

        patterns = get_tool_patterns(test_db)

        # Should have Read, Edit, Bash, Write
        tool_names = [p["tool"] for p in patterns]
        assert "Read" in tool_names

        # Read should have highest count (5 + 3 = 8)
        read_pattern = next(p for p in patterns if p["tool"] == "Read")
        assert read_pattern["total_uses"] == 8

    def test_get_recent_commits_returns_commits_within_days(self, test_db):
        """get_recent_commits should return commits within specified days."""
        from context_os_events.intelligence.queries import get_recent_commits

        commits = get_recent_commits(test_db, days=3)

        assert len(commits) == 1
        assert commits[0]["short_hash"] == "abc123"
        assert commits[0]["is_agent_commit"] == 1

    def test_get_session_jsonl_path_returns_correct_path(self, test_db):
        """get_session_jsonl_path should construct correct path."""
        from context_os_events.intelligence.queries import get_session_jsonl_path

        path = get_session_jsonl_path("session-001", "/test/project")

        # Should return a Path object pointing to JSONL file
        assert isinstance(path, Path)
        assert path.name == "session-001.jsonl"
        # Check for .claude in path (works on both Windows and Unix)
        assert ".claude" in str(path)
