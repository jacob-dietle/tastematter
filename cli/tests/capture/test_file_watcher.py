"""Tests for file_watcher module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).
"""

import tempfile
import time
from datetime import datetime
from pathlib import Path

import pytest


class TestEventFilter:
    """Test event filtering for noise reduction."""

    def test_filter_ignores_git_directory(self):
        """Paths inside .git directory should be filtered.

        RED: Run before implementation - should fail
        GREEN: Implement EventFilter with .git pattern
        """
        from context_os_events.capture.file_watcher import EventFilter

        filter = EventFilter(watch_path="/repo")

        # These should be ignored
        assert filter.should_ignore("/repo/.git/objects/abc123") == True
        assert filter.should_ignore("/repo/.git/HEAD") == True
        assert filter.should_ignore("/repo/.git/config") == True

        # These should NOT be ignored
        assert filter.should_ignore("/repo/src/main.py") == False
        assert filter.should_ignore("/repo/README.md") == False

    def test_filter_ignores_pycache(self):
        """Paths containing __pycache__ should be filtered.

        RED: Run before implementation
        GREEN: Implement __pycache__ pattern
        """
        from context_os_events.capture.file_watcher import EventFilter

        filter = EventFilter(watch_path="/repo")

        # These should be ignored
        assert filter.should_ignore("/repo/__pycache__/module.cpython-311.pyc") == True
        assert filter.should_ignore("/repo/src/__pycache__/utils.cpython-311.pyc") == True
        assert filter.should_ignore("/repo/tests/__pycache__/test.pyc") == True

        # Regular .py files should NOT be ignored
        assert filter.should_ignore("/repo/src/module.py") == False

    def test_filter_ignores_node_modules(self):
        """node_modules directories should be filtered."""
        from context_os_events.capture.file_watcher import EventFilter

        filter = EventFilter(watch_path="/repo")

        assert filter.should_ignore("/repo/node_modules/lodash/index.js") == True
        assert filter.should_ignore("/repo/frontend/node_modules/react/index.js") == True

        # Regular JS files should not be ignored
        assert filter.should_ignore("/repo/src/app.js") == False

    def test_filter_ignores_venv(self):
        """.venv directories should be filtered."""
        from context_os_events.capture.file_watcher import EventFilter

        filter = EventFilter(watch_path="/repo")

        assert filter.should_ignore("/repo/.venv/lib/python3.11/site-packages/click.py") == True
        assert filter.should_ignore("/repo/venv/bin/python") == True

    def test_filter_calculates_relative_paths(self):
        """Filter should convert absolute paths to relative.

        RED: Run before implementation
        GREEN: Implement path normalization
        """
        import os
        from context_os_events.capture.file_watcher import EventFilter

        filter = EventFilter(watch_path="/repo/project")

        # Should return relative path (normalized for current OS)
        relative = filter.get_relative_path("/repo/project/src/main.py")
        # Normalize expected path for cross-platform
        assert relative == os.path.join("src", "main.py")

        # Nested paths
        relative = filter.get_relative_path("/repo/project/a/b/c/file.txt")
        assert relative == os.path.join("a", "b", "c", "file.txt")

    def test_filter_ignores_sqlite_files(self):
        """SQLite database files should be filtered."""
        from context_os_events.capture.file_watcher import EventFilter

        filter = EventFilter(watch_path="/repo")

        # Database files and journals
        assert filter.should_ignore("/repo/data/events.db") == True
        assert filter.should_ignore("/repo/data/events.db-journal") == True
        assert filter.should_ignore("/repo/data/events.db-wal") == True
        assert filter.should_ignore("/repo/data/events.db-shm") == True


class TestEventDebouncer:
    """Test debouncing of rapid events."""

    def test_debouncer_consolidates_rapid_events(self):
        """Multiple rapid events on same file should consolidate.

        RED: Run before implementation
        GREEN: Implement EventDebouncer with time window
        """
        from context_os_events.capture.file_watcher import EventDebouncer, FileEvent

        debouncer = EventDebouncer(debounce_ms=100)

        now = datetime.now()

        # Simulate rapid saves (IDE behavior)
        event1 = FileEvent(
            timestamp=now,
            path="src/main.py",
            event_type="write",
            size_bytes=1000,
            old_path=None,
            is_directory=False,
            extension=".py"
        )
        event2 = FileEvent(
            timestamp=now,
            path="src/main.py",
            event_type="write",
            size_bytes=1001,  # Slightly different
            old_path=None,
            is_directory=False,
            extension=".py"
        )
        event3 = FileEvent(
            timestamp=now,
            path="src/main.py",
            event_type="write",
            size_bytes=1002,
            old_path=None,
            is_directory=False,
            extension=".py"
        )

        # Add all three rapidly
        debouncer.add(event1)
        debouncer.add(event2)
        debouncer.add(event3)

        # Should be buffered, not flushed yet
        assert debouncer.pending_count() == 1  # One unique path

        # Wait for debounce window to pass
        time.sleep(0.15)

        # Flush and get consolidated events
        flushed = debouncer.flush()

        # Should only get ONE event (the latest)
        assert len(flushed) == 1
        assert flushed[0].path == "src/main.py"
        assert flushed[0].size_bytes == 1002  # Latest value

    def test_debouncer_separates_different_files(self):
        """Events on different files should not be consolidated."""
        from context_os_events.capture.file_watcher import EventDebouncer, FileEvent

        debouncer = EventDebouncer(debounce_ms=100)
        now = datetime.now()

        event1 = FileEvent(
            timestamp=now, path="file1.py", event_type="write",
            size_bytes=100, old_path=None, is_directory=False, extension=".py"
        )
        event2 = FileEvent(
            timestamp=now, path="file2.py", event_type="write",
            size_bytes=200, old_path=None, is_directory=False, extension=".py"
        )

        debouncer.add(event1)
        debouncer.add(event2)

        assert debouncer.pending_count() == 2  # Two unique paths

        time.sleep(0.15)
        flushed = debouncer.flush()

        assert len(flushed) == 2


class TestFileEvent:
    """Test FileEvent data structure."""

    def test_handler_creates_event_with_metadata(self):
        """Events should capture extension and size.

        RED: Run before implementation
        GREEN: Implement FileEvent with metadata
        """
        from context_os_events.capture.file_watcher import FileEvent

        event = FileEvent(
            timestamp=datetime.now(),
            path="src/utils/helper.py",
            event_type="write",
            size_bytes=2048,
            old_path=None,
            is_directory=False,
            extension=".py"
        )

        assert event.path == "src/utils/helper.py"
        assert event.extension == ".py"
        assert event.size_bytes == 2048
        assert event.event_type == "write"
        assert event.is_directory == False

    def test_event_handles_no_extension(self):
        """Files without extension should have None extension."""
        from context_os_events.capture.file_watcher import FileEvent

        event = FileEvent(
            timestamp=datetime.now(),
            path="Makefile",
            event_type="create",
            size_bytes=512,
            old_path=None,
            is_directory=False,
            extension=None
        )

        assert event.extension is None

    def test_event_handles_directory(self):
        """Directory events should set is_directory True."""
        from context_os_events.capture.file_watcher import FileEvent

        event = FileEvent(
            timestamp=datetime.now(),
            path="src/new_module",
            event_type="create",
            size_bytes=None,
            old_path=None,
            is_directory=True,
            extension=None
        )

        assert event.is_directory == True
        assert event.size_bytes is None


class TestDatabaseInsert:
    """Test inserting events into database."""

    def test_insert_event_stores_all_fields(self):
        """FileEvent should be stored in database with all fields."""
        from context_os_events.capture.file_watcher import FileEvent, insert_event
        from context_os_events.db.connection import init_database

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = Path(f.name)

        try:
            conn = init_database(db_path)

            event = FileEvent(
                timestamp=datetime(2025, 1, 15, 10, 30, 0),
                path="src/main.py",
                event_type="write",
                size_bytes=1024,
                old_path=None,
                is_directory=False,
                extension=".py"
            )

            insert_event(conn, event)
            conn.commit()

            # Verify stored
            cursor = conn.execute(
                "SELECT path, event_type, size_bytes, extension FROM file_events"
            )
            row = cursor.fetchone()

            assert row["path"] == "src/main.py"
            assert row["event_type"] == "write"
            assert row["size_bytes"] == 1024
            assert row["extension"] == ".py"

            conn.close()
        finally:
            db_path.unlink(missing_ok=True)


class TestIntegration:
    """Integration tests with real filesystem."""

    def test_watcher_captures_file_creation(self):
        """Watcher should capture file creation events.

        This is an integration test with the real filesystem.
        """
        from context_os_events.capture.file_watcher import (
            EventFilter, FileEvent, create_event_from_path
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            filter = EventFilter(watch_path=tmpdir)

            # Create a file
            test_file = Path(tmpdir) / "test.py"
            test_file.write_text("print('hello')")

            # Create event from path
            event = create_event_from_path(
                str(test_file),
                event_type="create",
                watch_path=tmpdir
            )

            assert event is not None
            assert event.path == "test.py"
            assert event.event_type == "create"
            assert event.extension == ".py"
            assert event.size_bytes > 0
            assert event.is_directory == False

    def test_watcher_filters_during_creation(self):
        """Filtered files should not create events."""
        from context_os_events.capture.file_watcher import (
            EventFilter, create_event_from_path
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            # Create .git directory
            git_dir = Path(tmpdir) / ".git"
            git_dir.mkdir()
            git_file = git_dir / "HEAD"
            git_file.write_text("ref: refs/heads/main")

            filter = EventFilter(watch_path=tmpdir)

            # This should be filtered
            assert filter.should_ignore(str(git_file)) == True

            # Normal file should not be filtered
            normal_file = Path(tmpdir) / "main.py"
            normal_file.write_text("print('hello')")
            assert filter.should_ignore(str(normal_file)) == False
