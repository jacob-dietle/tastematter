"""Daemon state persistence module.

Tracks daemon state across restarts:
- Start time
- Last sync times
- Event counts
"""

import json
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Optional


@dataclass
class DaemonState:
    """Persisted state across daemon restarts."""

    started_at: datetime = field(default_factory=datetime.now)
    last_git_sync: Optional[datetime] = None
    last_session_parse: Optional[datetime] = None
    file_events_captured: int = 0
    git_commits_synced: int = 0
    sessions_parsed: int = 0

    def to_dict(self) -> dict:
        """Convert state to JSON-serializable dict."""
        return {
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "last_git_sync": self.last_git_sync.isoformat() if self.last_git_sync else None,
            "last_session_parse": self.last_session_parse.isoformat() if self.last_session_parse else None,
            "file_events_captured": self.file_events_captured,
            "git_commits_synced": self.git_commits_synced,
            "sessions_parsed": self.sessions_parsed,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "DaemonState":
        """Create state from dict (loaded from JSON)."""
        def parse_datetime(value: Optional[str]) -> Optional[datetime]:
            if value is None:
                return None
            return datetime.fromisoformat(value)

        return cls(
            started_at=parse_datetime(data.get("started_at")) or datetime.now(),
            last_git_sync=parse_datetime(data.get("last_git_sync")),
            last_session_parse=parse_datetime(data.get("last_session_parse")),
            file_events_captured=data.get("file_events_captured", 0),
            git_commits_synced=data.get("git_commits_synced", 0),
            sessions_parsed=data.get("sessions_parsed", 0),
        )

    def save(self, path: Path) -> None:
        """Save state to JSON file."""
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w") as f:
            json.dump(self.to_dict(), f, indent=2)

    @classmethod
    def load(cls, path: Path) -> "DaemonState":
        """Load state from JSON file, or return fresh state if not found."""
        if not path.exists():
            return cls()

        try:
            with open(path) as f:
                data = json.load(f)
            return cls.from_dict(data)
        except (json.JSONDecodeError, KeyError):
            # Corrupted state file, start fresh
            return cls()
