"""Tests for CLI build-chains command.

Test-Driven Development: Written BEFORE implementation (RED phase).
These tests should FAIL initially, then PASS after implementation.

Spec:
- build-chains: Build and persist chain graph from JSONL leafUuid linking
- Scans JSONL files, builds chain graph, persists to chains/chain_graph tables
- Outputs stats: chains built, sessions linked
"""

import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock
import sqlite3

import pytest
from click.testing import CliRunner

from context_os_events.cli import cli


@pytest.fixture
def cli_runner():
    """Click CLI test runner."""
    return CliRunner()


class NonClosingConnection:
    """Wrapper that prevents close() from actually closing the connection."""

    def __init__(self, conn):
        self._conn = conn

    def close(self):
        """No-op: Don't actually close."""
        pass

    def __getattr__(self, name):
        return getattr(self._conn, name)


@pytest.fixture
def mock_db():
    """Create an in-memory SQLite database with schema.

    Note: We wrap in NonClosingConnection to prevent the CLI from closing it.
    """
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row

    # Create required tables (from migrations 002)
    conn.execute("""
        CREATE TABLE chains (
            chain_id TEXT PRIMARY KEY,
            root_session_id TEXT NOT NULL,
            session_count INTEGER DEFAULT 0,
            branch_count INTEGER DEFAULT 0,
            max_depth INTEGER DEFAULT 0,
            files_bloom BLOB,
            files_json TEXT,
            files_count INTEGER DEFAULT 0,
            started_at TEXT,
            ended_at TEXT,
            updated_at TEXT
        )
    """)

    conn.execute("""
        CREATE TABLE chain_graph (
            session_id TEXT PRIMARY KEY,
            parent_session_id TEXT,
            parent_message_uuid TEXT,
            chain_id TEXT NOT NULL,
            position_in_chain INTEGER DEFAULT 0,
            is_root BOOLEAN DEFAULT FALSE,
            children_count INTEGER DEFAULT 0,
            indexed_at TEXT
        )
    """)

    conn.commit()

    # Wrap to prevent close
    return NonClosingConnection(conn)


class TestBuildChainsCommand:
    """Test the build-chains CLI command."""

    def test_build_chains_command_exists(self, cli_runner):
        """build-chains command should be registered in CLI.

        RED: Should fail - command doesn't exist yet.
        GREEN: Add @cli.command() for build-chains.
        """
        result = cli_runner.invoke(cli, ['build-chains', '--help'])

        # Should not fail with "No such command"
        assert result.exit_code == 0
        assert "Build and persist chain graph" in result.output

    def test_build_chains_populates_chains_table(self, cli_runner, mock_db):
        """build-chains should populate the chains table.

        RED: Should fail - chains table stays empty.
        GREEN: Implement persist_chains call in command.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create test JSONL files that form a chain
            # Session A: Root (has message uuid)
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text(
                '{"type":"user","uuid":"link-point","message":{"role":"user","content":"Start"}}\n'
            )

            # Session B: Child (references A via leafUuid)
            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text(
                '{"type":"summary","summary":"Continue","leafUuid":"link-point"}\n'
                '{"type":"user","uuid":"b-msg","message":{"role":"user","content":"Continue"}}\n'
            )

            # Mock the database connection and JSONL directory resolution
            with patch('context_os_events.cli.get_connection', return_value=mock_db):
                with patch('context_os_events.cli.get_jsonl_dir_for_project', return_value=tmpdir):
                    result = cli_runner.invoke(cli, ['build-chains'])

            # Verify chains were persisted
            chains = mock_db.execute("SELECT * FROM chains").fetchall()
            assert len(chains) >= 1, f"Expected chains, got none. Output: {result.output}"

    def test_build_chains_populates_chain_graph_table(self, cli_runner, mock_db):
        """build-chains should populate chain_graph with session links.

        RED: Should fail - chain_graph table stays empty.
        GREEN: Implement session linking in persist_chains.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Chain: A -> B -> C
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('{"type":"user","uuid":"a-end","message":{}}\n')

            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text(
                '{"type":"summary","leafUuid":"a-end"}\n'
                '{"type":"user","uuid":"b-end","message":{}}\n'
            )

            session_c = tmpdir / "session-c.jsonl"
            session_c.write_text(
                '{"type":"summary","leafUuid":"b-end"}\n'
                '{"type":"user","uuid":"c-end","message":{}}\n'
            )

            with patch('context_os_events.cli.get_connection', return_value=mock_db):
                with patch('context_os_events.cli.get_jsonl_dir_for_project', return_value=tmpdir):
                    result = cli_runner.invoke(cli, ['build-chains'])

            # Verify session links were persisted
            graph = mock_db.execute("SELECT * FROM chain_graph").fetchall()
            assert len(graph) == 3, f"Expected 3 sessions in graph, got {len(graph)}. Output: {result.output}"

    def test_build_chains_outputs_stats(self, cli_runner, mock_db):
        """build-chains should output statistics about what was built.

        RED: Should fail - no stats in output.
        GREEN: Print stats after persist.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Single standalone session
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('{"type":"user","uuid":"a-1","message":{}}\n')

            with patch('context_os_events.cli.get_connection', return_value=mock_db):
                with patch('context_os_events.cli.get_jsonl_dir_for_project', return_value=tmpdir):
                    result = cli_runner.invoke(cli, ['build-chains'])

            # Should output stats
            assert "chain" in result.output.lower()
            assert "session" in result.output.lower()

    def test_build_chains_handles_empty_directory(self, cli_runner, mock_db):
        """build-chains should handle directories with no JSONL files gracefully.

        RED: Should fail - crashes or errors.
        GREEN: Handle empty case, output message.
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            with patch('context_os_events.cli.get_connection', return_value=mock_db):
                with patch('context_os_events.cli.get_jsonl_dir_for_project', return_value=tmpdir):
                    result = cli_runner.invoke(cli, ['build-chains'])

            # Should not crash
            assert result.exit_code == 0
            # Should indicate no chains found
            assert "0" in result.output or "no" in result.output.lower()


class TestBuildChainsIntegration:
    """Integration tests with real database and file system."""

    def test_build_chains_with_real_jsonl_structure(self, cli_runner):
        """Test with realistic JSONL content matching Claude Code format."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Realistic session file
            session = tmpdir / "abc123-def456.jsonl"
            session.write_text('\n'.join([
                '{"type":"summary","summary":"Working on feature","leafUuid":"parent-msg-uuid"}',
                '{"type":"user","uuid":"usr-001","timestamp":"2025-01-02T10:00:00Z","message":{"role":"user","content":"Hello"}}',
                '{"type":"assistant","uuid":"ast-001","timestamp":"2025-01-02T10:00:05Z","message":{"role":"assistant","content":"Hi there"}}',
                '{"type":"tool_use","uuid":"tool-001","message":{"tool":"Read","input":{"file_path":"/src/app.ts"}}}',
            ]))

            # Create in-memory DB
            real_conn = sqlite3.connect(":memory:")
            real_conn.row_factory = sqlite3.Row
            real_conn.execute("""
                CREATE TABLE chains (
                    chain_id TEXT PRIMARY KEY, root_session_id TEXT,
                    session_count INTEGER, branch_count INTEGER, max_depth INTEGER,
                    files_bloom BLOB, files_json TEXT, files_count INTEGER,
                    updated_at TEXT
                )
            """)
            real_conn.execute("""
                CREATE TABLE chain_graph (
                    session_id TEXT PRIMARY KEY, parent_session_id TEXT,
                    parent_message_uuid TEXT, chain_id TEXT,
                    position_in_chain INTEGER, is_root BOOLEAN,
                    children_count INTEGER, indexed_at TEXT
                )
            """)
            real_conn.commit()
            # Wrap to prevent close
            conn = NonClosingConnection(real_conn)

            with patch('context_os_events.cli.get_connection', return_value=conn):
                with patch('context_os_events.cli.get_jsonl_dir_for_project', return_value=tmpdir):
                    result = cli_runner.invoke(cli, ['build-chains'])

            # Should succeed and create at least one chain
            assert result.exit_code == 0
            chains = conn.execute("SELECT COUNT(*) as c FROM chains").fetchone()
            assert chains['c'] >= 1
