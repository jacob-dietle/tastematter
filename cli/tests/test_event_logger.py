"""Tests for Event Logger - Agent Context Logging Infrastructure.

Following TDD: RED (tests fail) -> GREEN (implement) -> REFACTOR -> COMMIT
Written BEFORE implementation per Kent Beck methodology.

Spec: apps/context_os_events/specs/AGENT_CONTEXT_LOGGING_SPEC.md
"""

import json
import tempfile
from datetime import datetime, timedelta
from pathlib import Path

import pytest


class TestEventDataclass:
    """Test Event dataclass structure and serialization."""

    def test_event_has_required_fields(self):
        """Event should have all required fields from spec.

        RED: Run before implementation - should fail (ImportError)
        GREEN: Create events.py with Event dataclass
        """
        from context_os_events.observability.events import Event

        event = Event(
            ts="2026-01-02T16:45:00Z",
            level="info",
            source="cli",
            event="command_complete",
            command="build-chains",
            duration_ms=3200,
            context={"chains_built": 614},
            suggestion=None
        )

        assert event.ts == "2026-01-02T16:45:00Z"
        assert event.level == "info"
        assert event.source == "cli"
        assert event.event == "command_complete"
        assert event.command == "build-chains"
        assert event.duration_ms == 3200
        assert event.context == {"chains_built": 614}
        assert event.suggestion is None

    def test_event_optional_fields_default_to_none(self):
        """Optional fields should default to None.

        RED: Run before implementation
        GREEN: Add default values to dataclass
        """
        from context_os_events.observability.events import Event

        event = Event(
            ts="2026-01-02T16:45:00Z",
            level="info",
            source="cli",
            event="command_start",
            context={}
        )

        assert event.command is None
        assert event.duration_ms is None
        assert event.suggestion is None

    def test_event_to_dict(self):
        """Event should be convertible to dict for JSON serialization.

        RED: Run before implementation
        GREEN: Add to_dict() method
        """
        from context_os_events.observability.events import Event

        event = Event(
            ts="2026-01-02T16:45:00Z",
            level="error",
            source="cli",
            event="command_error",
            command="query",
            duration_ms=None,
            context={"error": "No data found"},
            suggestion="Run 'context-os build-chains' first"
        )

        d = event.to_dict()

        assert isinstance(d, dict)
        assert d["ts"] == "2026-01-02T16:45:00Z"
        assert d["level"] == "error"
        assert d["source"] == "cli"
        assert d["event"] == "command_error"
        assert d["suggestion"] == "Run 'context-os build-chains' first"

    def test_event_from_dict(self):
        """Event should be constructible from dict (for reading JSONL).

        RED: Run before implementation
        GREEN: Add from_dict() classmethod
        """
        from context_os_events.observability.events import Event

        data = {
            "ts": "2026-01-02T16:45:00Z",
            "level": "info",
            "source": "cli",
            "event": "command_complete",
            "command": "build-chains",
            "duration_ms": 3200,
            "context": {"chains_built": 614},
            "suggestion": None
        }

        event = Event.from_dict(data)

        assert event.ts == "2026-01-02T16:45:00Z"
        assert event.level == "info"
        assert event.command == "build-chains"
        assert event.duration_ms == 3200

    def test_event_to_json_string(self):
        """Event should serialize to valid JSON string.

        RED: Run before implementation
        GREEN: Ensure to_dict() works with json.dumps()
        """
        from context_os_events.observability.events import Event

        event = Event(
            ts="2026-01-02T16:45:00Z",
            level="info",
            source="cli",
            event="command_complete",
            context={}
        )

        json_str = json.dumps(event.to_dict())
        parsed = json.loads(json_str)

        assert parsed["ts"] == "2026-01-02T16:45:00Z"
        assert parsed["level"] == "info"


class TestEventLoggerLog:
    """Test EventLogger.log() method - append events to JSONL."""

    def test_log_creates_file_if_not_exists(self):
        """Logger should create events.jsonl if it doesn't exist.

        RED: Run before implementation - should fail (ImportError)
        GREEN: Implement EventLogger with log() method
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            event = Event(
                ts="2026-01-02T16:45:00Z",
                level="info",
                source="cli",
                event="command_complete",
                context={}
            )

            logger.log(event)

            # Should create a file
            files = list(log_dir.glob("events*.jsonl"))
            assert len(files) >= 1

    def test_log_appends_to_existing_file(self):
        """Multiple log calls should append to same file.

        RED: Run before implementation
        GREEN: Implement append logic
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            # Log two events
            for i in range(2):
                event = Event(
                    ts=f"2026-01-02T16:4{i}:00Z",
                    level="info",
                    source="cli",
                    event=f"event_{i}",
                    context={}
                )
                logger.log(event)

            # Read file and count lines
            files = list(log_dir.glob("events*.jsonl"))
            assert len(files) >= 1
            content = files[0].read_text()
            lines = [l for l in content.strip().split("\n") if l]
            assert len(lines) == 2

    def test_log_writes_valid_jsonl(self):
        """Each line should be valid JSON.

        RED: Run before implementation
        GREEN: Use json.dumps() for each event
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            event = Event(
                ts="2026-01-02T16:45:00Z",
                level="info",
                source="cli",
                event="command_complete",
                command="build-chains",
                duration_ms=3200,
                context={"chains_built": 614}
            )

            logger.log(event)

            # Read and parse
            files = list(log_dir.glob("events*.jsonl"))
            content = files[0].read_text().strip()
            parsed = json.loads(content)

            assert parsed["ts"] == "2026-01-02T16:45:00Z"
            assert parsed["event"] == "command_complete"
            assert parsed["context"]["chains_built"] == 614

    def test_log_preserves_all_fields(self):
        """All event fields should be preserved in JSONL.

        RED: Run before implementation
        GREEN: Use event.to_dict() for serialization
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            event = Event(
                ts="2026-01-02T16:45:00Z",
                level="error",
                source="cli",
                event="command_error",
                command="query",
                duration_ms=None,
                context={"error": "No data"},
                suggestion="Run build-chains first"
            )

            logger.log(event)

            files = list(log_dir.glob("events*.jsonl"))
            content = files[0].read_text().strip()
            parsed = json.loads(content)

            assert parsed["level"] == "error"
            assert parsed["suggestion"] == "Run build-chains first"
            assert parsed["context"]["error"] == "No data"


class TestEventLoggerGetRecent:
    """Test EventLogger.get_recent() method - read recent events."""

    def test_get_recent_returns_events(self):
        """get_recent() should return list of Event objects.

        RED: Run before implementation - should fail (AttributeError)
        GREEN: Implement get_recent() method
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            # Log an event
            event = Event(
                ts="2026-01-02T16:45:00Z",
                level="info",
                source="cli",
                event="command_complete",
                context={}
            )
            logger.log(event)

            # Get recent
            recent = logger.get_recent()

            assert len(recent) == 1
            assert isinstance(recent[0], Event)
            assert recent[0].ts == "2026-01-02T16:45:00Z"

    def test_get_recent_respects_limit(self):
        """get_recent(limit=N) should return at most N events.

        RED: Run before implementation
        GREEN: Implement limit parameter
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            # Log 5 events
            for i in range(5):
                event = Event(
                    ts=f"2026-01-02T16:4{i}:00Z",
                    level="info",
                    source="cli",
                    event=f"event_{i}",
                    context={}
                )
                logger.log(event)

            # Get only 3
            recent = logger.get_recent(limit=3)

            assert len(recent) == 3

    def test_get_recent_most_recent_first(self):
        """get_recent() should return most recent events first.

        RED: Run before implementation
        GREEN: Implement reverse ordering
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            # Log events in order
            for i in range(3):
                event = Event(
                    ts=f"2026-01-02T16:4{i}:00Z",
                    level="info",
                    source="cli",
                    event=f"event_{i}",
                    context={}
                )
                logger.log(event)

            recent = logger.get_recent(limit=3)

            # Most recent (event_2) should be first
            assert recent[0].event == "event_2"
            assert recent[1].event == "event_1"
            assert recent[2].event == "event_0"

    def test_get_recent_handles_empty_log(self):
        """get_recent() should return empty list if no events.

        RED: Run before implementation
        GREEN: Handle missing file case
        """
        from context_os_events.observability.event_logger import EventLogger

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            recent = logger.get_recent()

            assert recent == []

    def test_get_recent_parses_all_fields(self):
        """get_recent() should parse all event fields correctly.

        RED: Run before implementation
        GREEN: Use Event.from_dict() for parsing
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            event = Event(
                ts="2026-01-02T16:45:00Z",
                level="error",
                source="cli",
                event="command_error",
                command="query",
                duration_ms=1500,
                context={"error": "No data"},
                suggestion="Run build-chains"
            )
            logger.log(event)

            recent = logger.get_recent()

            assert recent[0].level == "error"
            assert recent[0].command == "query"
            assert recent[0].duration_ms == 1500
            assert recent[0].suggestion == "Run build-chains"


class TestDailyRotation:
    """Test daily file rotation for event logs."""

    def test_events_go_to_dated_file(self):
        """Events should be written to date-stamped files.

        RED: Run before implementation
        GREEN: _get_current_file() returns dated filename
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            event = Event(
                ts="2026-01-02T16:45:00Z",
                level="info",
                source="cli",
                event="test",
                context={}
            )
            logger.log(event)

            # File should have date pattern
            files = list(log_dir.glob("events.*.jsonl"))
            assert len(files) == 1
            # Should match pattern events.YYYY-MM-DD.jsonl
            assert files[0].name.startswith("events.")
            assert files[0].name.endswith(".jsonl")
            # Date part should be valid
            date_part = files[0].name[7:-6]  # Extract YYYY-MM-DD
            assert len(date_part) == 10

    def test_get_recent_reads_across_multiple_days(self):
        """get_recent() should read from multiple daily files.

        RED: Run before implementation
        GREEN: get_recent() iterates through dated files
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)

            # Manually create files for different days
            day1 = log_dir / "events.2026-01-01.jsonl"
            day2 = log_dir / "events.2026-01-02.jsonl"
            log_dir.mkdir(parents=True, exist_ok=True)

            import json
            day1.write_text(json.dumps({
                "ts": "2026-01-01T10:00:00Z",
                "level": "info",
                "source": "cli",
                "event": "day1_event",
                "context": {}
            }) + "\n")

            day2.write_text(json.dumps({
                "ts": "2026-01-02T10:00:00Z",
                "level": "info",
                "source": "cli",
                "event": "day2_event",
                "context": {}
            }) + "\n")

            recent = logger.get_recent(limit=10)

            assert len(recent) == 2
            # Most recent day first (day2)
            assert recent[0].event == "day2_event"
            assert recent[1].event == "day1_event"


class TestRetention:
    """Test 7-day retention cleanup."""

    def test_cleanup_removes_old_files(self):
        """cleanup() should remove files older than 7 days.

        RED: Run before implementation - should fail (AttributeError)
        GREEN: Implement cleanup() method
        """
        from context_os_events.observability.event_logger import EventLogger

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)
            log_dir.mkdir(parents=True, exist_ok=True)

            # Create files for different ages
            today = datetime.utcnow()
            old_date = (today - timedelta(days=10)).strftime("%Y-%m-%d")
            recent_date = (today - timedelta(days=3)).strftime("%Y-%m-%d")

            old_file = log_dir / f"events.{old_date}.jsonl"
            recent_file = log_dir / f"events.{recent_date}.jsonl"

            old_file.write_text('{"ts":"old"}\n')
            recent_file.write_text('{"ts":"recent"}\n')

            # Run cleanup
            logger.cleanup()

            # Old file should be removed
            assert not old_file.exists()
            # Recent file should remain
            assert recent_file.exists()

    def test_cleanup_keeps_files_within_retention(self):
        """cleanup() should keep files from last 7 days.

        RED: Run before implementation
        GREEN: Implement date comparison in cleanup()
        """
        from context_os_events.observability.event_logger import EventLogger

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)
            log_dir.mkdir(parents=True, exist_ok=True)

            # Create files for each of last 7 days
            today = datetime.utcnow()
            files = []
            for i in range(7):
                date_str = (today - timedelta(days=i)).strftime("%Y-%m-%d")
                f = log_dir / f"events.{date_str}.jsonl"
                f.write_text(f'{{"day":{i}}}\n')
                files.append(f)

            logger.cleanup()

            # All files should remain
            for f in files:
                assert f.exists(), f"{f.name} should still exist"

    def test_cleanup_called_on_log(self):
        """log() should trigger cleanup periodically.

        RED: Run before implementation
        GREEN: Call cleanup() in log() (or similar trigger)
        """
        from context_os_events.observability.event_logger import EventLogger
        from context_os_events.observability.events import Event

        with tempfile.TemporaryDirectory() as tmpdir:
            log_dir = Path(tmpdir)
            logger = EventLogger(log_dir)
            log_dir.mkdir(parents=True, exist_ok=True)

            # Create an old file
            today = datetime.utcnow()
            old_date = (today - timedelta(days=10)).strftime("%Y-%m-%d")
            old_file = log_dir / f"events.{old_date}.jsonl"
            old_file.write_text('{"ts":"old"}\n')

            # Log a new event (should trigger cleanup)
            event = Event(
                ts=today.isoformat() + "Z",
                level="info",
                source="cli",
                event="test",
                context={}
            )
            logger.log(event)

            # Old file should be cleaned up
            assert not old_file.exists()


class TestSingleton:
    """Test singleton export for easy import."""

    def test_event_logger_importable_from_module(self):
        """event_logger should be importable from observability module.

        RED: Run before implementation - should fail (ImportError)
        GREEN: Export event_logger in __init__.py
        """
        from context_os_events.observability import event_logger

        assert event_logger is not None

    def test_event_logger_is_eventlogger_instance(self):
        """event_logger should be an EventLogger instance.

        RED: Run before implementation
        GREEN: Create singleton instance
        """
        from context_os_events.observability import event_logger
        from context_os_events.observability.event_logger import EventLogger

        assert isinstance(event_logger, EventLogger)

    def test_event_logger_uses_default_path(self):
        """event_logger should use ~/.context-os/ as default path.

        RED: Run before implementation
        GREEN: Configure default path in singleton
        """
        from context_os_events.observability import event_logger

        # Should use ~/.context-os/ directory
        expected_dir = Path.home() / ".context-os"
        assert event_logger.log_dir == expected_dir
