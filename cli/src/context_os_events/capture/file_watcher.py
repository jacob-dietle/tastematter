"""File watcher for capturing file system events.

Captures file creation, modification, deletion, and rename events
using the watchdog library. Events are filtered to exclude noise
(.git, __pycache__, node_modules, etc.) and debounced to consolidate
rapid saves from IDEs.
"""

import fnmatch
import logging
import os
import threading
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from sqlite3 import Connection
from typing import Dict, List, Optional

logger = logging.getLogger(__name__)


# ============================================================================
# Data Structures
# ============================================================================

@dataclass
class FileEvent:
    """A file system event."""
    timestamp: datetime
    path: str                    # Relative to repo root
    event_type: str              # create, write, delete, rename
    size_bytes: Optional[int]    # File size (None for delete)
    old_path: Optional[str]      # Previous path for renames
    is_directory: bool
    extension: Optional[str]     # File extension


# ============================================================================
# Default Ignore Patterns
# ============================================================================

DEFAULT_IGNORE_PATTERNS = [
    # Version control
    ".git",
    ".git/*",
    "*/.git/*",
    ".svn",
    ".svn/*",
    "*/.svn/*",
    ".hg",
    ".hg/*",
    "*/.hg/*",

    # Python
    "__pycache__",
    "__pycache__/*",
    "*/__pycache__/*",
    "*.pyc",
    "*.pyo",
    "*.pyd",
    ".pytest_cache",
    ".pytest_cache/*",
    "*/.pytest_cache/*",
    ".venv",
    ".venv/*",
    "*/.venv/*",
    "venv",
    "venv/*",
    "*/venv/*",
    "*.egg-info",
    "*.egg-info/*",

    # Node.js
    "node_modules",
    "node_modules/*",
    "*/node_modules/*",
    "*.min.js",
    "*.min.css",

    # IDE
    ".idea",
    ".idea/*",
    "*/.idea/*",
    ".vscode",
    ".vscode/*",
    "*/.vscode/*",
    "*.swp",
    "*.swo",
    "*~",
    ".DS_Store",

    # Build artifacts
    "dist",
    "dist/*",
    "build",
    "build/*",
    "*.egg",

    # SQLite
    "*.db",
    "*.db-journal",
    "*.db-wal",
    "*.db-shm",
    "*.sqlite",
    "*.sqlite3",

    # Logs and temp
    "*.log",
    "*.tmp",
    "*.temp",
    "*.bak",
]


# ============================================================================
# Event Filter
# ============================================================================

class EventFilter:
    """Filters file events based on ignore patterns."""

    def __init__(
        self,
        watch_path: str,
        ignore_patterns: Optional[List[str]] = None
    ):
        """Initialize the filter.

        Args:
            watch_path: Root directory being watched
            ignore_patterns: Patterns to ignore (defaults to DEFAULT_IGNORE_PATTERNS)
        """
        self.watch_path = str(Path(watch_path).resolve())
        self.ignore_patterns = ignore_patterns or DEFAULT_IGNORE_PATTERNS

    def should_ignore(self, path: str) -> bool:
        """Check if a path should be ignored.

        Args:
            path: Absolute path to check

        Returns:
            True if path should be ignored
        """
        # Get relative path for matching
        relative = self.get_relative_path(path)

        # Normalize to forward slashes for matching
        relative = relative.replace("\\", "/")

        # Check each pattern
        for pattern in self.ignore_patterns:
            # Check if pattern matches the full relative path
            if fnmatch.fnmatch(relative, pattern):
                return True

            # Check if any path component matches
            # This catches __pycache__ anywhere in the path
            parts = relative.split("/")
            for part in parts:
                if fnmatch.fnmatch(part, pattern):
                    return True

        return False

    def get_relative_path(self, path: str) -> str:
        """Convert absolute path to relative path.

        Args:
            path: Absolute path

        Returns:
            Path relative to watch_path
        """
        path = str(Path(path).resolve())

        # Remove watch_path prefix
        if path.startswith(self.watch_path):
            relative = path[len(self.watch_path):]
            # Remove leading separator
            if relative.startswith(os.sep):
                relative = relative[1:]
            if relative.startswith("/"):
                relative = relative[1:]
            return relative

        return path


# ============================================================================
# Event Debouncer
# ============================================================================

class EventDebouncer:
    """Consolidates rapid events on the same file."""

    def __init__(self, debounce_ms: int = 100):
        """Initialize the debouncer.

        Args:
            debounce_ms: Debounce window in milliseconds
        """
        self.debounce_ms = debounce_ms
        self._pending: Dict[str, FileEvent] = {}
        self._timestamps: Dict[str, float] = {}
        self._lock = threading.Lock()

    def add(self, event: FileEvent) -> None:
        """Add an event to the buffer.

        If an event for the same path exists, it will be replaced.

        Args:
            event: The file event to add
        """
        with self._lock:
            self._pending[event.path] = event
            self._timestamps[event.path] = time.time()

    def pending_count(self) -> int:
        """Get the number of pending events.

        Returns:
            Number of unique paths with pending events
        """
        with self._lock:
            return len(self._pending)

    def flush(self) -> List[FileEvent]:
        """Flush events that have passed the debounce window.

        Returns:
            List of events ready to be processed
        """
        now = time.time()
        threshold = self.debounce_ms / 1000.0

        flushed = []

        with self._lock:
            paths_to_remove = []

            for path, timestamp in self._timestamps.items():
                if now - timestamp >= threshold:
                    if path in self._pending:
                        flushed.append(self._pending[path])
                    paths_to_remove.append(path)

            for path in paths_to_remove:
                self._pending.pop(path, None)
                self._timestamps.pop(path, None)

        return flushed

    def flush_all(self) -> List[FileEvent]:
        """Flush all pending events regardless of time.

        Returns:
            All pending events
        """
        with self._lock:
            flushed = list(self._pending.values())
            self._pending.clear()
            self._timestamps.clear()
            return flushed


# ============================================================================
# Event Creation
# ============================================================================

def create_event_from_path(
    path: str,
    event_type: str,
    watch_path: str,
    old_path: Optional[str] = None
) -> Optional[FileEvent]:
    """Create a FileEvent from a file path.

    Args:
        path: Absolute path to the file
        event_type: Type of event (create, write, delete, rename)
        watch_path: Root directory being watched
        old_path: Previous path for rename events

    Returns:
        FileEvent or None if path doesn't exist (for non-delete events)
    """
    filter = EventFilter(watch_path=watch_path)
    relative_path = filter.get_relative_path(path)

    # Get file info
    try:
        p = Path(path)
        is_directory = p.is_dir()
        if event_type == "delete":
            size_bytes = None
        else:
            size_bytes = p.stat().st_size if p.exists() and not is_directory else None
        extension = p.suffix if p.suffix else None
    except (OSError, PermissionError) as e:
        logger.debug(f"Could not stat {path}: {e}")
        is_directory = False
        size_bytes = None
        extension = Path(path).suffix if Path(path).suffix else None

    # Convert old_path to relative if provided
    relative_old_path = None
    if old_path:
        relative_old_path = filter.get_relative_path(old_path)

    return FileEvent(
        timestamp=datetime.now(),
        path=relative_path,
        event_type=event_type,
        size_bytes=size_bytes,
        old_path=relative_old_path,
        is_directory=is_directory,
        extension=extension
    )


# ============================================================================
# Database Operations
# ============================================================================

def insert_event(db: Connection, event: FileEvent) -> None:
    """Insert a file event into the database.

    Args:
        db: Database connection
        event: Event to insert
    """
    db.execute("""
        INSERT INTO file_events (
            timestamp, path, event_type, size_bytes,
            old_path, is_directory, extension
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
    """, (
        event.timestamp.isoformat(),
        event.path,
        event.event_type,
        event.size_bytes,
        event.old_path,
        event.is_directory,
        event.extension
    ))


def insert_events(db: Connection, events: List[FileEvent]) -> int:
    """Insert multiple file events into the database.

    Args:
        db: Database connection
        events: Events to insert

    Returns:
        Number of events inserted
    """
    for event in events:
        insert_event(db, event)
    db.commit()
    return len(events)


# ============================================================================
# Watchdog Handler
# ============================================================================

try:
    from watchdog.events import FileSystemEventHandler
    from watchdog.observers import Observer

    class FileEventHandler(FileSystemEventHandler):
        """Watchdog handler that captures file events."""

        def __init__(
            self,
            watch_path: str,
            filter: EventFilter,
            debouncer: EventDebouncer,
            db: Connection
        ):
            """Initialize the handler.

            Args:
                watch_path: Root directory being watched
                filter: Event filter for ignoring paths
                debouncer: Event debouncer
                db: Database connection
            """
            super().__init__()
            self.watch_path = watch_path
            self.filter = filter
            self.debouncer = debouncer
            self.db = db
            self._stats = {
                "events_captured": 0,
                "events_filtered": 0,
                "events_debounced": 0,
            }
            self._lock = threading.Lock()

        def _process_event(self, event, event_type: str, old_path: str = None):
            """Process a watchdog event.

            Args:
                event: Watchdog event
                event_type: Our event type (create, write, delete, rename)
                old_path: Old path for rename events
            """
            path = event.src_path

            # Check filter
            if self.filter.should_ignore(path):
                with self._lock:
                    self._stats["events_filtered"] += 1
                return

            # Create file event
            file_event = create_event_from_path(
                path=path,
                event_type=event_type,
                watch_path=self.watch_path,
                old_path=old_path
            )

            if file_event:
                self.debouncer.add(file_event)
                with self._lock:
                    self._stats["events_captured"] += 1

        def on_created(self, event):
            self._process_event(event, "create")

        def on_modified(self, event):
            self._process_event(event, "write")

        def on_deleted(self, event):
            self._process_event(event, "delete")

        def on_moved(self, event):
            self._process_event(event, "rename", old_path=event.src_path)

        def get_stats(self) -> dict:
            """Get handler statistics."""
            with self._lock:
                return dict(self._stats)

except ImportError:
    # Watchdog not installed - provide stub
    class FileEventHandler:
        """Stub handler when watchdog is not installed."""
        pass

    class Observer:
        """Stub observer when watchdog is not installed."""
        pass


# ============================================================================
# Watcher Control
# ============================================================================

class FileWatcher:
    """Main file watcher orchestrator."""

    def __init__(
        self,
        watch_path: str,
        db: Connection,
        ignore_patterns: Optional[List[str]] = None,
        debounce_ms: int = 100
    ):
        """Initialize the watcher.

        Args:
            watch_path: Directory to watch
            db: Database connection
            ignore_patterns: Custom ignore patterns
            debounce_ms: Debounce window in milliseconds
        """
        self.watch_path = str(Path(watch_path).resolve())
        self.db = db
        self.filter = EventFilter(watch_path, ignore_patterns)
        self.debouncer = EventDebouncer(debounce_ms)
        self.handler = FileEventHandler(
            watch_path=self.watch_path,
            filter=self.filter,
            debouncer=self.debouncer,
            db=db
        )
        self.observer = Observer()
        self._running = False
        self._flush_thread = None

    def start(self) -> None:
        """Start the file watcher."""
        self.observer.schedule(self.handler, self.watch_path, recursive=True)
        self.observer.start()
        self._running = True

        # Start flush thread
        self._flush_thread = threading.Thread(target=self._flush_loop, daemon=True)
        self._flush_thread.start()

    def stop(self) -> dict:
        """Stop the file watcher.

        Returns:
            Statistics dictionary
        """
        self._running = False

        # Stop observer
        self.observer.stop()
        self.observer.join(timeout=5)

        # Final flush
        remaining = self.debouncer.flush_all()
        if remaining:
            insert_events(self.db, remaining)

        return self.handler.get_stats()

    def _flush_loop(self) -> None:
        """Background loop to flush debounced events."""
        while self._running:
            time.sleep(0.05)  # 50ms polling
            flushed = self.debouncer.flush()
            if flushed:
                try:
                    insert_events(self.db, flushed)
                except Exception as e:
                    logger.error(f"Failed to insert events: {e}")


def start_watcher(
    watch_path: str,
    db: Connection,
    ignore_patterns: Optional[List[str]] = None
) -> FileWatcher:
    """Start a file watcher.

    Args:
        watch_path: Directory to watch
        db: Database connection
        ignore_patterns: Custom ignore patterns

    Returns:
        Running FileWatcher instance
    """
    watcher = FileWatcher(watch_path, db, ignore_patterns)
    watcher.start()
    return watcher


def stop_watcher(watcher: FileWatcher) -> dict:
    """Stop a file watcher.

    Args:
        watcher: Running watcher to stop

    Returns:
        Statistics dictionary
    """
    return watcher.stop()
