"""EventLogger for agent-context logging.

Appends events to JSONL files for agent consumption.
Designed for minimal overhead with maximum agent utility.
"""

import json
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Optional

from .events import Event

# Retention period in days
RETENTION_DAYS = 7


class EventLogger:
    """Logs events to JSONL files.

    Events are appended to daily-rotated files:
    - events.jsonl (current day, symlink)
    - events.2026-01-02.jsonl (dated files)

    Attributes:
        log_dir: Directory for event files (typically ~/.context-os/)
    """

    def __init__(self, log_dir: Path):
        """Initialize the event logger.

        Args:
            log_dir: Directory to store event files
        """
        self.log_dir = Path(log_dir)

    def _get_current_file(self) -> Path:
        """Get the current day's event file path.

        Returns:
            Path to today's events file (events.YYYY-MM-DD.jsonl)
        """
        today = datetime.utcnow().strftime("%Y-%m-%d")
        return self.log_dir / f"events.{today}.jsonl"

    def log(self, event: Event) -> None:
        """Append event to the current day's JSONL file.

        Creates the file and directory if they don't exist.

        Args:
            event: Event to log
        """
        # Ensure directory exists
        self.log_dir.mkdir(parents=True, exist_ok=True)

        # Get current file path
        file_path = self._get_current_file()

        # Append event as JSON line
        with open(file_path, "a", encoding="utf-8") as f:
            json_line = json.dumps(event.to_dict())
            f.write(json_line + "\n")

        # Cleanup old files
        self.cleanup()

    def cleanup(self) -> None:
        """Remove event files older than retention period.

        Deletes files with dates older than RETENTION_DAYS.
        """
        if not self.log_dir.exists():
            return

        cutoff = datetime.utcnow() - timedelta(days=RETENTION_DAYS)
        cutoff_str = cutoff.strftime("%Y-%m-%d")

        for file_path in self.log_dir.glob("events.*.jsonl"):
            # Extract date from filename (events.YYYY-MM-DD.jsonl)
            try:
                date_str = file_path.name[7:-6]  # Extract YYYY-MM-DD
                if date_str < cutoff_str:
                    file_path.unlink()
            except (ValueError, IndexError):
                # Skip files with unexpected naming
                continue

    def get_recent(self, limit: int = 20) -> List[Event]:
        """Read recent events from log files.

        Reads from current and recent daily files, returns most recent first.

        Args:
            limit: Maximum number of events to return (default 20)

        Returns:
            List of Event objects, most recent first
        """
        events: List[Event] = []

        # Get all event files, sorted by date (newest first)
        if not self.log_dir.exists():
            return []

        files = sorted(
            self.log_dir.glob("events.*.jsonl"),
            key=lambda p: p.name,
            reverse=True
        )

        for file_path in files:
            if len(events) >= limit:
                break

            try:
                with open(file_path, "r", encoding="utf-8") as f:
                    lines = f.readlines()

                # Process lines in reverse (newest first within file)
                for line in reversed(lines):
                    if len(events) >= limit:
                        break

                    line = line.strip()
                    if not line:
                        continue

                    try:
                        data = json.loads(line)
                        event = Event.from_dict(data)
                        events.append(event)
                    except json.JSONDecodeError:
                        # Skip malformed lines
                        continue

            except IOError:
                # Skip unreadable files
                continue

        return events
