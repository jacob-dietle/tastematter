"""Tests for daemon runner module.

Phase 2 of daemon implementation - Main daemon loop orchestration.
Following TDD Red-Green-Refactor cycle.
"""

import tempfile
import threading
import time
from datetime import datetime
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest


class TestDaemonInitialization:
    """Tests for daemon initialization."""

    def test_daemon_initializes_with_config(self):
        """Daemon should store config and initialize state."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        daemon = ContextOSDaemon(config)

        assert daemon.config == config
        assert daemon._running is False
        assert daemon.state is not None

    def test_daemon_state_tracks_start_time(self):
        """State should track when daemon started."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        daemon = ContextOSDaemon(config)

        before = datetime.now()
        daemon.start()
        after = datetime.now()

        # Stop immediately for test
        daemon.stop()

        assert daemon.state.started_at >= before
        assert daemon.state.started_at <= after


class TestDaemonEventSystem:
    """Tests for daemon event emission and handling."""

    def test_daemon_event_handler_registration(self):
        """Should be able to register event handlers with on()."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        daemon = ContextOSDaemon(config)

        handler_called = []

        def handler(event: str, data: dict):
            handler_called.append((event, data))

        daemon.on("test_event", handler)
        daemon.emit("test_event", {"key": "value"})

        assert len(handler_called) == 1
        assert handler_called[0] == ("test_event", {"key": "value"})

    def test_daemon_emits_sync_complete_event(self):
        """run_sync should emit sync_complete event."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        daemon = ContextOSDaemon(config)

        events_received = []

        def handler(event: str, data: dict):
            events_received.append((event, data))

        daemon.on("sync_complete", handler)

        # Mock the sync operations to avoid actual file/db operations
        with patch.object(daemon, "_sync_git", return_value=5):
            with patch.object(daemon, "_sync_sessions", return_value=3):
                daemon.run_sync()

        assert len(events_received) == 1
        event_name, event_data = events_received[0]
        assert event_name == "sync_complete"
        assert "git_commits" in event_data
        assert "sessions" in event_data


class TestDaemonLifecycle:
    """Tests for daemon start/stop lifecycle."""

    def test_daemon_stop_sets_running_false(self):
        """stop() should set _running to False."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        daemon = ContextOSDaemon(config)

        daemon.start()
        assert daemon._running is True

        daemon.stop()
        assert daemon._running is False

    def test_daemon_start_is_idempotent(self):
        """Starting twice should not cause issues."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        daemon = ContextOSDaemon(config)

        daemon.start()
        daemon.start()  # Should not raise

        daemon.stop()
        assert daemon._running is False


class TestDaemonScheduler:
    """Tests for scheduler integration."""

    def test_scheduler_triggers_sync_at_interval(self):
        """Scheduler should call run_sync at configured interval.

        This is an integration test that verifies scheduler is set up correctly.
        Uses a very short interval (1 second) to make test fast.
        """
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        # Use very short interval for testing
        config["sync"]["interval_minutes"] = 1 / 60  # 1 second

        daemon = ContextOSDaemon(config)

        sync_count = []

        # Track sync calls
        original_run_sync = daemon.run_sync

        def tracked_run_sync():
            sync_count.append(1)
            # Don't actually run sync operations in test

        daemon.run_sync = tracked_run_sync

        daemon.start()

        # Wait for scheduler to trigger at least once
        time.sleep(1.5)

        daemon.stop()

        # Should have been called at least once by scheduler
        assert len(sync_count) >= 1


class TestDaemonStatePersistence:
    """Tests for state persistence."""

    def test_daemon_persists_state_to_file(self, tmp_path: Path):
        """Daemon should persist state to file on sync."""
        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        config = get_default_config()
        state_file = tmp_path / "daemon.state.json"

        daemon = ContextOSDaemon(config, state_file=state_file)

        daemon.start()

        # Run sync to trigger state save
        with patch.object(daemon, "_sync_git", return_value=0):
            with patch.object(daemon, "_sync_sessions", return_value=0):
                daemon.run_sync()

        daemon.stop()

        # State file should exist and contain valid JSON
        assert state_file.exists()

        import json

        with open(state_file) as f:
            saved_state = json.load(f)

        assert "started_at" in saved_state
        assert "file_events_captured" in saved_state

    def test_daemon_loads_existing_state(self, tmp_path: Path):
        """Daemon should load state from existing file."""
        import json

        from context_os_events.daemon.config import get_default_config
        from context_os_events.daemon.runner import ContextOSDaemon

        # Create existing state file
        state_file = tmp_path / "daemon.state.json"
        existing_state = {
            "started_at": "2025-12-15T10:00:00",
            "last_git_sync": "2025-12-15T10:30:00",
            "last_session_parse": "2025-12-15T10:30:00",
            "file_events_captured": 100,
            "git_commits_synced": 50,
            "sessions_parsed": 25,
        }
        state_file.write_text(json.dumps(existing_state))

        config = get_default_config()
        daemon = ContextOSDaemon(config, state_file=state_file)

        # Should load existing counts
        assert daemon.state.file_events_captured == 100
        assert daemon.state.git_commits_synced == 50
        assert daemon.state.sessions_parsed == 25
