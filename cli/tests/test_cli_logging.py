"""Tests for CLI logging integration - Write FIRST, implement SECOND.

Following TDD: RED (tests fail) -> GREEN (implement) -> REFACTOR
"""

import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest


class TestCLISnapshot:
    """Test the snapshot command."""

    def test_snapshot_command_creates_files(self):
        """snapshot command should create all _data files.

        RED: Run before implementation
        GREEN: Add snapshot command to cli.py
        """
        from click.testing import CliRunner
        from context_os_events.cli import cli
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "context_os_events.db"
            data_dir = Path(tmpdir) / "_data"
            data_dir.mkdir()
            (data_dir / "snapshots").mkdir()

            # Initialize database with test data
            conn = init_database(db_path)
            conn.execute("""
                INSERT INTO claude_sessions
                (session_id, project_path, files_read, tools_used, file_size_bytes)
                VALUES
                ('sess1', '/project', '["test.py"]', '{"Read": 1}', 100)
            """)
            conn.commit()
            conn.close()

            runner = CliRunner()

            # Mock the database and data directory paths
            with patch('context_os_events.cli.get_database_path', return_value=db_path):
                with patch('context_os_events.cli.get_data_dir', return_value=data_dir):
                    result = runner.invoke(cli, ['snapshot'])

            # Check command succeeded
            assert result.exit_code == 0, f"Command failed: {result.output}"

            # Check files created
            assert (data_dir / "game_trails.md").exists()
            assert (data_dir / "tool_patterns.md").exists()
            assert (data_dir / "snapshots" / "latest.md").exists()

    def test_snapshot_output_includes_stats(self):
        """snapshot command should output summary stats.

        RED: Run before implementation
        GREEN: Add output to snapshot command
        """
        from click.testing import CliRunner
        from context_os_events.cli import cli
        from context_os_events.db.connection import init_database

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "context_os_events.db"
            data_dir = Path(tmpdir) / "_data"
            data_dir.mkdir()
            (data_dir / "snapshots").mkdir()

            conn = init_database(db_path)
            conn.close()

            runner = CliRunner()

            with patch('context_os_events.cli.get_database_path', return_value=db_path):
                with patch('context_os_events.cli.get_data_dir', return_value=data_dir):
                    result = runner.invoke(cli, ['snapshot'])

            # Should mention files generated
            assert "snapshot" in result.output.lower() or "generated" in result.output.lower()
