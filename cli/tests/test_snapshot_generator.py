"""Tests for SnapshotGenerator - Write FIRST, implement SECOND.

Following TDD: RED (tests fail) -> GREEN (implement) -> REFACTOR
"""

import tempfile
from datetime import datetime
from pathlib import Path

import pytest


class TestSnapshotDataQueries:
    """Test data extraction from database."""

    def test_generate_game_trails_returns_sorted_files(self):
        """Game trails should return files sorted by read count.

        RED: Run before implementation
        GREEN: Implement generate_game_trails()
        """
        from context_os_events.visibility.snapshot import SnapshotGenerator
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "test.db"
            output_dir = Path(tmpdir) / "_data"
            output_dir.mkdir()

            conn = init_database(db_path)

            # Insert test session with files_read
            conn.execute("""
                INSERT INTO claude_sessions
                (session_id, project_path, files_read, tools_used, file_size_bytes)
                VALUES
                ('sess1', '/project', '["file_a.py", "file_b.py", "file_a.py"]', '{}', 100)
            """)
            conn.commit()

            generator = SnapshotGenerator(conn, output_dir)
            trails = generator.generate_game_trails(limit=10)

            # file_a.py should be first (2 reads vs 1)
            assert len(trails) >= 1
            assert trails[0].path == "file_a.py"
            assert trails[0].read_count == 2

            conn.close()

    def test_generate_tool_patterns_calculates_percentages(self):
        """Tool patterns should include percentage of total.

        RED: Run before implementation
        GREEN: Implement generate_tool_patterns()
        """
        from context_os_events.visibility.snapshot import SnapshotGenerator
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "test.db"
            output_dir = Path(tmpdir) / "_data"
            output_dir.mkdir()

            conn = init_database(db_path)

            # Insert session with tool usage
            conn.execute("""
                INSERT INTO claude_sessions
                (session_id, project_path, tools_used, file_size_bytes)
                VALUES
                ('sess1', '/project', '{"Read": 80, "Edit": 20}', 100)
            """)
            conn.commit()

            generator = SnapshotGenerator(conn, output_dir)
            patterns = generator.generate_tool_patterns()

            # Should have 2 tools
            assert len(patterns) == 2

            # Read should be 80%
            read_pattern = next(p for p in patterns if p.tool_name == "Read")
            assert read_pattern.percentage == 80.0

            conn.close()

    def test_generate_automation_candidates_finds_repeated_patterns(self):
        """Should find grep patterns used more than once.

        RED: Run before implementation
        GREEN: Implement generate_automation_candidates()
        """
        from context_os_events.visibility.snapshot import SnapshotGenerator
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "test.db"
            output_dir = Path(tmpdir) / "_data"
            output_dir.mkdir()

            conn = init_database(db_path)

            # Insert sessions with grep patterns
            conn.execute("""
                INSERT INTO claude_sessions
                (session_id, project_path, grep_patterns, tools_used, file_size_bytes)
                VALUES
                ('sess1', '/project', '["TODO", "FIXME", "TODO"]', '{}', 100),
                ('sess2', '/project', '["TODO", "unique"]', '{}', 100)
            """)
            conn.commit()

            generator = SnapshotGenerator(conn, output_dir)
            candidates = generator.generate_automation_candidates()

            # TODO should be found (used 3 times across sessions)
            todo_candidate = next((c for c in candidates if c.pattern == "TODO"), None)
            assert todo_candidate is not None
            assert todo_candidate.use_count >= 3

            conn.close()


class TestSnapshotMarkdownGeneration:
    """Test markdown file generation."""

    def test_write_game_trails_creates_valid_markdown(self):
        """Should write properly formatted markdown file.

        RED: Run before implementation
        GREEN: Implement write_game_trails_md()
        """
        from context_os_events.visibility.snapshot import (
            SnapshotGenerator, GameTrailEntry
        )
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "test.db"
            output_dir = Path(tmpdir) / "_data"
            output_dir.mkdir()

            conn = init_database(db_path)
            generator = SnapshotGenerator(conn, output_dir)

            # Create test data
            data = [
                GameTrailEntry(
                    path="CLAUDE.md",
                    read_count=50,
                    last_accessed=datetime(2025, 1, 15),
                    category="config"
                ),
                GameTrailEntry(
                    path="src/main.py",
                    read_count=25,
                    last_accessed=datetime(2025, 1, 14),
                    category="code"
                )
            ]

            path = generator.write_game_trails_md(data)

            assert path.exists()
            content = path.read_text()

            # Check structure
            assert "# Game Trails" in content
            assert "CLAUDE.md" in content
            assert "50" in content
            assert "*Generated:" in content

            conn.close()

    def test_write_full_snapshot_creates_dated_file(self):
        """Should create both latest.md and dated snapshot.

        RED: Run before implementation
        GREEN: Implement write_full_snapshot_md()
        """
        from context_os_events.visibility.snapshot import SnapshotGenerator, Snapshot
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "test.db"
            output_dir = Path(tmpdir) / "_data"
            output_dir.mkdir()
            (output_dir / "snapshots").mkdir()

            conn = init_database(db_path)
            generator = SnapshotGenerator(conn, output_dir)

            snapshot = Snapshot(
                generated_at=datetime(2025, 1, 15, 16, 45),
                git_commits=99,
                agent_commits=29,
                sessions=406,
                total_messages=15503,
                tool_uses=4828,
                earliest_commit=datetime(2025, 1, 1),
                latest_commit=datetime(2025, 1, 15),
                earliest_session=datetime(2025, 1, 1),
                latest_session=datetime(2025, 1, 15),
                days_span=15,
                game_trails=[],
                tool_patterns=[],
                automation_candidates=[],
                commit_hotspots=[]
            )

            path = generator.write_full_snapshot_md(snapshot)

            # Should create latest.md
            latest = output_dir / "snapshots" / "latest.md"
            assert latest.exists()

            # Should create dated snapshot
            dated = output_dir / "snapshots" / "2025-01-15.md"
            assert dated.exists()

            conn.close()


class TestSnapshotIntegration:
    """Integration tests with real database queries."""

    def test_generate_all_creates_all_files(self):
        """generate_all() should create all snapshot files.

        RED: Run before implementation
        GREEN: Implement generate_all()
        """
        from context_os_events.visibility.snapshot import SnapshotGenerator
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "test.db"
            output_dir = Path(tmpdir) / "_data"
            output_dir.mkdir()
            (output_dir / "snapshots").mkdir()

            conn = init_database(db_path)

            # Insert minimal test data
            conn.execute("""
                INSERT INTO claude_sessions
                (session_id, project_path, files_read, files_written,
                 tools_used, grep_patterns, file_size_bytes)
                VALUES
                ('sess1', '/project', '["file.py"]', '[]',
                 '{"Read": 1}', '["TODO"]', 100)
            """)
            conn.execute("""
                INSERT INTO git_commits
                (hash, short_hash, timestamp, message, is_agent_commit, files_changed)
                VALUES
                ('abc123', 'abc', '2025-01-15', 'test', 0, '["file.py"]')
            """)
            conn.commit()

            generator = SnapshotGenerator(conn, output_dir)
            generator.generate_all()

            # Check all files created
            assert (output_dir / "game_trails.md").exists()
            assert (output_dir / "tool_patterns.md").exists()
            assert (output_dir / "automation_candidates.md").exists()
            assert (output_dir / "commit_hotspots.md").exists()
            assert (output_dir / "snapshots" / "latest.md").exists()

            conn.close()
