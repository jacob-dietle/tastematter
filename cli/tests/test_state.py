"""TDD tests for state snapshot generation.

Agent 2 of AGENT_CONTEXT_LOGGING_SPEC.md implementation.
Tests follow Kent Beck RED→GREEN→REFACTOR pattern.
"""

import json
import tempfile
from datetime import datetime
from pathlib import Path

import pytest


# =============================================================================
# Cycle 1: HealthSnapshot Dataclass
# =============================================================================


class TestHealthSnapshotDataclass:
    """Tests for HealthSnapshot dataclass structure."""

    def test_health_snapshot_has_required_fields(self):
        """HealthSnapshot has generated_at, database, recent_errors, warnings."""
        from context_os_events.observability.state import HealthSnapshot

        snapshot = HealthSnapshot(
            generated_at="2026-01-02T19:00:00Z",
            database={"path": "/path/to/db", "size_mb": 10.5, "tables": {}},
            recent_errors=[],
            warnings=[],
        )

        assert snapshot.generated_at == "2026-01-02T19:00:00Z"
        assert snapshot.database == {"path": "/path/to/db", "size_mb": 10.5, "tables": {}}
        assert snapshot.recent_errors == []
        assert snapshot.warnings == []

    def test_health_snapshot_to_dict(self):
        """HealthSnapshot.to_dict() returns serializable dict."""
        from context_os_events.observability.state import HealthSnapshot

        snapshot = HealthSnapshot(
            generated_at="2026-01-02T19:00:00Z",
            database={"path": "/path/to/db", "size_mb": 10.5, "tables": {}},
            recent_errors=[{"ts": "2026-01-02T18:00:00Z", "message": "Test error"}],
            warnings=[],
        )

        result = snapshot.to_dict()

        assert isinstance(result, dict)
        assert result["generated_at"] == "2026-01-02T19:00:00Z"
        assert result["database"]["size_mb"] == 10.5
        assert len(result["recent_errors"]) == 1
        # Verify it's JSON-serializable
        json_str = json.dumps(result)
        assert "generated_at" in json_str

    def test_health_snapshot_database_structure(self):
        """database field contains path, size_mb, tables dict."""
        from context_os_events.observability.state import HealthSnapshot

        tables = {
            "claude_sessions": {"rows": 460, "last_updated": "2026-01-02T16:30:00Z"},
            "chains": {"rows": 614, "last_updated": "2026-01-02T16:45:00Z"},
        }

        snapshot = HealthSnapshot(
            generated_at="2026-01-02T19:00:00Z",
            database={"path": "/path/to/db.sqlite", "size_mb": 12.5, "tables": tables},
            recent_errors=[],
            warnings=[],
        )

        assert "path" in snapshot.database
        assert "size_mb" in snapshot.database
        assert "tables" in snapshot.database
        assert snapshot.database["tables"]["claude_sessions"]["rows"] == 460


# =============================================================================
# Cycle 2: ActivitySnapshot Dataclass
# =============================================================================


class TestActivitySnapshotDataclass:
    """Tests for ActivitySnapshot and RecentCommand dataclasses."""

    def test_recent_command_structure(self):
        """RecentCommand has ts, command, status, duration_ms."""
        from context_os_events.observability.state import RecentCommand

        cmd = RecentCommand(
            ts="2026-01-02T19:00:00Z",
            command="build-chains",
            status="success",
            duration_ms=3200,
        )

        assert cmd.ts == "2026-01-02T19:00:00Z"
        assert cmd.command == "build-chains"
        assert cmd.status == "success"
        assert cmd.duration_ms == 3200

    def test_activity_snapshot_has_required_fields(self):
        """ActivitySnapshot has generated_at, last_24h, recent_commands."""
        from context_os_events.observability.state import ActivitySnapshot, RecentCommand

        recent = [
            RecentCommand(
                ts="2026-01-02T19:00:00Z",
                command="build-chains",
                status="success",
                duration_ms=3200,
            )
        ]

        snapshot = ActivitySnapshot(
            generated_at="2026-01-02T19:00:00Z",
            last_24h={"commands_run": 12, "errors": 0},
            recent_commands=recent,
        )

        assert snapshot.generated_at == "2026-01-02T19:00:00Z"
        assert snapshot.last_24h["commands_run"] == 12
        assert len(snapshot.recent_commands) == 1
        assert snapshot.recent_commands[0].command == "build-chains"

    def test_activity_snapshot_to_dict(self):
        """ActivitySnapshot.to_dict() returns serializable dict."""
        from context_os_events.observability.state import ActivitySnapshot, RecentCommand

        recent = [
            RecentCommand(
                ts="2026-01-02T19:00:00Z",
                command="build-chains",
                status="success",
                duration_ms=3200,
            )
        ]

        snapshot = ActivitySnapshot(
            generated_at="2026-01-02T19:00:00Z",
            last_24h={"commands_run": 12, "errors": 0},
            recent_commands=recent,
        )

        result = snapshot.to_dict()

        assert isinstance(result, dict)
        assert result["generated_at"] == "2026-01-02T19:00:00Z"
        assert result["last_24h"]["commands_run"] == 12
        assert len(result["recent_commands"]) == 1
        # Verify it's JSON-serializable
        json_str = json.dumps(result)
        assert "build-chains" in json_str


# =============================================================================
# Cycle 3: generate_health_snapshot()
# =============================================================================


class TestGenerateHealthSnapshot:
    """Tests for generate_health_snapshot() function."""

    def test_returns_health_snapshot(self, tmp_path):
        """generate_health_snapshot() returns HealthSnapshot instance."""
        from context_os_events.observability.state import (
            generate_health_snapshot,
            HealthSnapshot,
        )
        from context_os_events.db.connection import init_database

        # Create test database
        db_path = tmp_path / "test.db"
        init_database(db_path)

        result = generate_health_snapshot(db_path)

        assert isinstance(result, HealthSnapshot)
        assert result.generated_at is not None

    def test_includes_database_path(self, tmp_path):
        """Result includes database path."""
        from context_os_events.observability.state import generate_health_snapshot
        from context_os_events.db.connection import init_database

        db_path = tmp_path / "test.db"
        init_database(db_path)

        result = generate_health_snapshot(db_path)

        assert result.database["path"] == str(db_path)

    def test_includes_table_row_counts(self, tmp_path):
        """Result includes row counts for known tables."""
        from context_os_events.observability.state import generate_health_snapshot
        from context_os_events.db.connection import init_database

        db_path = tmp_path / "test.db"
        conn = init_database(db_path)

        # Insert test data
        conn.execute(
            "INSERT INTO claude_sessions (session_id, started_at, project_path) VALUES (?, ?, ?)",
            ("test-session-uuid", "2026-01-02T19:00:00Z", "/test/project"),
        )
        conn.commit()
        conn.close()

        result = generate_health_snapshot(db_path)

        assert "tables" in result.database
        assert "claude_sessions" in result.database["tables"]
        assert result.database["tables"]["claude_sessions"]["rows"] == 1

    def test_handles_empty_database(self, tmp_path):
        """Returns valid snapshot when database is empty."""
        from context_os_events.observability.state import generate_health_snapshot
        from context_os_events.db.connection import init_database

        db_path = tmp_path / "test.db"
        init_database(db_path)

        result = generate_health_snapshot(db_path)

        # Should not raise, should return valid snapshot
        assert result.database["tables"]["claude_sessions"]["rows"] == 0


# =============================================================================
# Cycle 4: generate_activity_snapshot()
# =============================================================================


class TestGenerateActivitySnapshot:
    """Tests for generate_activity_snapshot() function."""

    def test_returns_activity_snapshot(self, tmp_path):
        """generate_activity_snapshot() returns ActivitySnapshot instance."""
        from context_os_events.observability.state import (
            generate_activity_snapshot,
            ActivitySnapshot,
        )
        from context_os_events.observability.event_logger import EventLogger

        # Use temp directory for events
        logger = EventLogger(tmp_path)

        result = generate_activity_snapshot(logger)

        assert isinstance(result, ActivitySnapshot)
        assert result.generated_at is not None

    def test_aggregates_last_24h_commands(self, tmp_path):
        """Counts commands in last 24 hours."""
        from context_os_events.observability.state import generate_activity_snapshot
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event
        from datetime import datetime

        logger = EventLogger(tmp_path)

        # Log some command events
        for i in range(3):
            event = Event(
                ts=datetime.utcnow().isoformat() + "Z",
                level="info",
                source="cli",
                event="command_complete",
                command=f"test-command-{i}",
                duration_ms=100 * i,
                context={},
            )
            logger.log(event)

        result = generate_activity_snapshot(logger)

        assert result.last_24h["commands_run"] == 3

    def test_extracts_recent_commands(self, tmp_path):
        """Returns last N commands with metadata."""
        from context_os_events.observability.state import generate_activity_snapshot
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event
        from datetime import datetime

        logger = EventLogger(tmp_path)

        # Log a command event
        event = Event(
            ts="2026-01-02T19:00:00Z",
            level="info",
            source="cli",
            event="command_complete",
            command="build-chains",
            duration_ms=3200,
            context={"chains_built": 614},
        )
        logger.log(event)

        result = generate_activity_snapshot(logger)

        assert len(result.recent_commands) >= 1
        assert result.recent_commands[0].command == "build-chains"
        assert result.recent_commands[0].status == "success"
        assert result.recent_commands[0].duration_ms == 3200

    def test_handles_no_events(self, tmp_path):
        """Returns valid snapshot when no events exist."""
        from context_os_events.observability.state import generate_activity_snapshot
        from context_os_events.observability.event_logger import EventLogger

        logger = EventLogger(tmp_path)

        result = generate_activity_snapshot(logger)

        # Should not raise, should return valid snapshot
        assert result.last_24h["commands_run"] == 0
        assert result.recent_commands == []


# =============================================================================
# Cycle 5: update_state() - File Writing
# =============================================================================


class TestUpdateState:
    """Tests for update_state() function."""

    def test_creates_state_directory(self, tmp_path, monkeypatch):
        """update_state() creates state directory if missing."""
        from context_os_events.observability.state import update_state
        from context_os_events.db.connection import init_database

        # Create test database
        db_path = tmp_path / "test.db"
        init_database(db_path)

        # Set up state directory in temp
        state_dir = tmp_path / "state"
        assert not state_dir.exists()

        update_state(db_path=db_path, state_dir=state_dir, event_log_dir=tmp_path)

        assert state_dir.exists()

    def test_writes_health_json(self, tmp_path):
        """Writes valid JSON to health.json."""
        from context_os_events.observability.state import update_state
        from context_os_events.db.connection import init_database

        db_path = tmp_path / "test.db"
        init_database(db_path)

        state_dir = tmp_path / "state"

        update_state(db_path=db_path, state_dir=state_dir, event_log_dir=tmp_path)

        health_path = state_dir / "health.json"
        assert health_path.exists()

        # Verify valid JSON
        with open(health_path, "r") as f:
            data = json.load(f)

        assert "generated_at" in data
        assert "database" in data

    def test_writes_activity_json(self, tmp_path):
        """Writes valid JSON to activity.json."""
        from context_os_events.observability.state import update_state
        from context_os_events.db.connection import init_database

        db_path = tmp_path / "test.db"
        init_database(db_path)

        state_dir = tmp_path / "state"

        update_state(db_path=db_path, state_dir=state_dir, event_log_dir=tmp_path)

        activity_path = state_dir / "activity.json"
        assert activity_path.exists()

        # Verify valid JSON
        with open(activity_path, "r") as f:
            data = json.load(f)

        assert "generated_at" in data
        assert "last_24h" in data

    def test_json_is_pretty_printed(self, tmp_path):
        """Output JSON is indented for readability."""
        from context_os_events.observability.state import update_state
        from context_os_events.db.connection import init_database

        db_path = tmp_path / "test.db"
        init_database(db_path)

        state_dir = tmp_path / "state"

        update_state(db_path=db_path, state_dir=state_dir, event_log_dir=tmp_path)

        health_path = state_dir / "health.json"
        with open(health_path, "r") as f:
            content = f.read()

        # Pretty-printed JSON has newlines and indentation
        assert "\n" in content
        assert "  " in content  # Indentation


# =============================================================================
# Cycle 6: Module Exports
# =============================================================================


class TestStateExports:
    """Tests for state module exports from observability package."""

    def test_generate_health_snapshot_importable(self):
        """generate_health_snapshot importable from observability module."""
        from context_os_events.observability import generate_health_snapshot

        assert callable(generate_health_snapshot)

    def test_generate_activity_snapshot_importable(self):
        """generate_activity_snapshot importable from observability module."""
        from context_os_events.observability import generate_activity_snapshot

        assert callable(generate_activity_snapshot)

    def test_update_state_importable(self):
        """update_state importable from observability module."""
        from context_os_events.observability import update_state

        assert callable(update_state)
