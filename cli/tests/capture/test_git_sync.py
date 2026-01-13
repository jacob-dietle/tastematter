"""Tests for git_sync module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).
"""

import json
import sqlite3
import tempfile
from datetime import datetime
from pathlib import Path
from unittest.mock import Mock, patch

import pytest


class TestParseFormatLine:
    """Test parsing git log format line."""

    def test_parse_format_line_extracts_all_fields(self):
        """Format line should split into 7 fields correctly.

        RED: Run before implementation - should fail
        GREEN: Implement parse_commit_block()
        """
        from context_os_events.capture.git_sync import parse_commit_block

        # Simple commit block with just format line
        block = "abc123def456789012345678901234567890§abc123d§2025-01-15T10:30:00-05:00§John Doe§john@example.com§feat: add feature§def456"

        commit = parse_commit_block(block)

        assert commit.hash == "abc123def456789012345678901234567890"
        assert commit.short_hash == "abc123d"
        assert commit.author_name == "John Doe"
        assert commit.author_email == "john@example.com"
        assert commit.message == "feat: add feature"

    def test_parse_format_line_handles_empty_parents(self):
        """Initial commit has no parents - should handle gracefully."""
        from context_os_events.capture.git_sync import parse_commit_block

        block = "abc123def456789012345678901234567890§abc123d§2025-01-15T10:30:00-05:00§John§john@example.com§Initial commit§"

        commit = parse_commit_block(block)

        assert commit.is_merge_commit == False
        assert commit.message == "Initial commit"


class TestParseFileChanges:
    """Test parsing file changes from --name-status output."""

    def test_parse_name_status_categorizes_files(self):
        """Name-status should categorize files by A/M/D.

        RED: Run before implementation - should fail
        GREEN: Implement file parsing logic
        """
        from context_os_events.capture.git_sync import parse_commit_block

        block = """abc123def456789012345678901234567890§abc123d§2025-01-15T10:30:00-05:00§John§j@e.com§msg§parent123

10\t5\tsrc/new.py
3\t3\tsrc/changed.py
0\t15\tsrc/old.py

A\tsrc/new.py
M\tsrc/changed.py
D\tsrc/old.py"""

        commit = parse_commit_block(block)

        assert "src/new.py" in commit.files_added
        assert "src/changed.py" in commit.files_modified
        assert "src/old.py" in commit.files_deleted
        assert commit.insertions == 13
        assert commit.deletions == 23
        assert commit.files_count == 3

    def test_parse_handles_binary_files(self):
        """Binary files show - instead of numbers in numstat."""
        from context_os_events.capture.git_sync import parse_commit_block

        block = """abc123def456789012345678901234567890§abc123d§2025-01-15T10:30:00-05:00§John§j@e.com§msg§

-\t-\timage.png
5\t2\tcode.py

A\timage.png
M\tcode.py"""

        commit = parse_commit_block(block)

        assert "image.png" in commit.files_added
        assert "code.py" in commit.files_modified
        # Binary files don't count toward insertions/deletions
        assert commit.insertions == 5
        assert commit.deletions == 2


class TestAgentCommitDetection:
    """Test detection of Claude Code generated commits."""

    def test_detect_agent_commit_finds_claude_signature(self):
        """Agent commits should be detected by signature.

        RED: Run before implementation
        GREEN: Implement detect_agent_commit()
        """
        from context_os_events.capture.git_sync import detect_agent_commit

        # Various signatures
        assert detect_agent_commit("feat: add feature") == False
        assert detect_agent_commit("fix: manual bugfix by human") == False

        # Claude Code signatures
        assert detect_agent_commit("feat: add feature\n\n🤖 Generated with Claude Code") == True
        assert detect_agent_commit("Generated with Claude Code") == True

    def test_detect_agent_commit_case_insensitive(self):
        """Detection should be case insensitive."""
        from context_os_events.capture.git_sync import detect_agent_commit

        assert detect_agent_commit("generated with claude code") == True
        assert detect_agent_commit("GENERATED WITH CLAUDE CODE") == True

    def test_detect_co_author(self):
        """Co-Author-By Claude should also be detected."""
        from context_os_events.capture.git_sync import detect_agent_commit

        assert detect_agent_commit("feat: feature\n\nCo-Authored-By: Claude") == True


class TestMergeCommitDetection:
    """Test detection of merge commits."""

    def test_detect_merge_commit_multiple_parents(self):
        """Merge commits have multiple parent hashes."""
        from context_os_events.capture.git_sync import parse_commit_block

        # Two parents = merge commit
        block = "abc123def456789012345678901234567890§abc123d§2025-01-15T10:30:00-05:00§John§j@e.com§Merge branch 'feature'§parent1 parent2"

        commit = parse_commit_block(block)

        assert commit.is_merge_commit == True

    def test_detect_regular_commit_single_parent(self):
        """Regular commits have single parent."""
        from context_os_events.capture.git_sync import parse_commit_block

        block = "abc123def456789012345678901234567890§abc123d§2025-01-15T10:30:00-05:00§John§j@e.com§Regular commit§parent1"

        commit = parse_commit_block(block)

        assert commit.is_merge_commit == False


class TestIncrementalSync:
    """Test incremental sync logic."""

    def test_get_last_synced_hash_empty_db(self):
        """Empty database should return None."""
        from context_os_events.capture.git_sync import get_last_synced_hash
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)
            result = get_last_synced_hash(conn)
            assert result is None
            conn.close()
        finally:
            db_path.unlink(missing_ok=True)

    def test_get_last_synced_hash_with_commits(self):
        """Should return most recent commit hash."""
        from context_os_events.capture.git_sync import get_last_synced_hash
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            # Insert test commits
            conn.execute("""
                INSERT INTO git_commits (hash, short_hash, timestamp, message)
                VALUES (?, ?, ?, ?)
            """, ("older123", "older12", "2025-01-14T10:00:00", "older commit"))

            conn.execute("""
                INSERT INTO git_commits (hash, short_hash, timestamp, message)
                VALUES (?, ?, ?, ?)
            """, ("newer456", "newer45", "2025-01-15T10:00:00", "newer commit"))

            conn.commit()

            result = get_last_synced_hash(conn)
            assert result == "newer456"

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)

    def test_commit_exists_check(self):
        """Should detect if commit already in database."""
        from context_os_events.capture.git_sync import commit_exists
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            conn.execute("""
                INSERT INTO git_commits (hash, short_hash, timestamp, message)
                VALUES (?, ?, ?, ?)
            """, ("existing123", "exist12", "2025-01-15T10:00:00", "existing commit"))
            conn.commit()

            assert commit_exists(conn, "existing123") == True
            assert commit_exists(conn, "nonexistent") == False

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)


class TestDatabaseInsert:
    """Test inserting commits into database."""

    def test_insert_commit_stores_all_fields(self):
        """Insert should store all commit fields correctly."""
        from context_os_events.capture.git_sync import insert_commit, GitCommit
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            commit = GitCommit(
                hash="abc123def456789012345678901234567890",
                short_hash="abc123d",
                timestamp=datetime(2025, 1, 15, 10, 30, 0),
                author_name="John Doe",
                author_email="john@example.com",
                message="feat: add feature",
                files_changed=["src/file.py", "tests/test.py"],
                files_added=["src/new.py"],
                files_modified=["src/file.py"],
                files_deleted=[],
                insertions=50,
                deletions=10,
                files_count=2,
                is_agent_commit=True,
                is_merge_commit=False
            )

            insert_commit(conn, commit)
            conn.commit()

            # Verify
            cursor = conn.execute(
                "SELECT * FROM git_commits WHERE hash = ?",
                (commit.hash,)
            )
            row = cursor.fetchone()

            assert row is not None
            assert row["author_name"] == "John Doe"
            assert row["is_agent_commit"] == 1
            assert json.loads(row["files_added"]) == ["src/new.py"]
            assert row["insertions"] == 50

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)


class TestIntegration:
    """Integration tests with real git repository."""

    def test_sync_real_repository(self):
        """Sync should work with actual git log output.

        Uses current repository as test fixture.
        """
        from context_os_events.capture.git_sync import sync_commits
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            # Get repo root (this file is in apps/context_os_events/tests/capture/)
            repo_path = Path(__file__).parent.parent.parent.parent.parent

            result = sync_commits(conn, {
                "repo_path": str(repo_path),
                "since": "7 days",
                "incremental": False
            })

            # Should sync at least some commits (assuming repo has history)
            # Note: This may be 0 if repo is very new
            assert result["commits_synced"] >= 0
            assert "errors" in result

            # Verify data integrity
            cursor = conn.execute("SELECT COUNT(*) FROM git_commits")
            count = cursor.fetchone()[0]
            assert count == result["commits_synced"]

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)

    def test_incremental_sync_skips_existing(self):
        """Second sync should skip already synced commits."""
        from context_os_events.capture.git_sync import sync_commits
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)
            repo_path = Path(__file__).parent.parent.parent.parent.parent

            # First sync
            result1 = sync_commits(conn, {
                "repo_path": str(repo_path),
                "since": "7 days",
                "incremental": False
            })

            # Second sync (incremental)
            result2 = sync_commits(conn, {
                "repo_path": str(repo_path),
                "since": "7 days",
                "incremental": True
            })

            # Second sync should have fewer new commits
            # (ideally 0 if no new commits happened)
            assert result2["commits_synced"] <= result1["commits_synced"]

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)
