"""Tests for daemon state serialization.

Following TDD Red-Green-Refactor cycle.
These tests verify chain fields are properly serialized.
"""

from datetime import datetime

import pytest


class TestStateChainFieldSerialization:
    """Tests for chain field serialization in DaemonState."""

    def test_state_serializes_chain_fields_to_dict(self):
        """DaemonState.to_dict() should include chain fields."""
        from context_os_events.daemon.state import DaemonState

        state = DaemonState(
            chains_built=42,
            last_chain_build=datetime(2026, 1, 13, 12, 0, 0)
        )

        data = state.to_dict()

        assert data["chains_built"] == 42
        assert data["last_chain_build"] == "2026-01-13T12:00:00"

    def test_state_deserializes_chain_fields_from_dict(self):
        """DaemonState.from_dict() should restore chain fields."""
        from context_os_events.daemon.state import DaemonState

        data = {
            "started_at": "2026-01-13T10:00:00",
            "chains_built": 42,
            "last_chain_build": "2026-01-13T12:00:00",
            "file_events_captured": 0,
            "git_commits_synced": 0,
            "sessions_parsed": 0,
        }

        restored = DaemonState.from_dict(data)

        assert restored.chains_built == 42
        assert restored.last_chain_build == datetime(2026, 1, 13, 12, 0, 0)

    def test_state_handles_none_last_chain_build(self):
        """DaemonState should handle None last_chain_build."""
        from context_os_events.daemon.state import DaemonState

        state = DaemonState(
            chains_built=0,
            last_chain_build=None
        )

        data = state.to_dict()
        assert data["last_chain_build"] is None

        restored = DaemonState.from_dict(data)
        assert restored.last_chain_build is None
        assert restored.chains_built == 0

    def test_state_roundtrip_preserves_chain_fields(self):
        """Round-trip serialization should preserve chain fields."""
        from context_os_events.daemon.state import DaemonState

        original = DaemonState(
            chains_built=42,
            last_chain_build=datetime(2026, 1, 13, 12, 0, 0)
        )

        data = original.to_dict()
        restored = DaemonState.from_dict(data)

        assert restored.chains_built == original.chains_built
        assert restored.last_chain_build == original.last_chain_build
