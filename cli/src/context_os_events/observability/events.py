"""Event dataclass for agent-context logging.

Defines the schema for events logged to events.jsonl.
Designed for agent consumption - minimal but informative.
"""

from dataclasses import dataclass, field, asdict
from typing import Any, Dict, Literal, Optional


@dataclass
class Event:
    """Single event in the observability log.

    Attributes:
        ts: ISO8601 UTC timestamp
        level: Severity level (info, warn, error)
        source: Where event originated (cli, tastematter, daemon)
        event: Event type (command_start, command_complete, command_error, etc.)
        command: Which command triggered this (optional)
        duration_ms: Operation duration in milliseconds (optional)
        context: Event-specific data dictionary
        suggestion: For errors - what to do about it (optional)
    """

    ts: str
    level: Literal["info", "warn", "error"]
    source: Literal["cli", "tastematter", "daemon"]
    event: str
    context: Dict[str, Any] = field(default_factory=dict)
    command: Optional[str] = None
    duration_ms: Optional[int] = None
    suggestion: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert event to dictionary for JSON serialization."""
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Event":
        """Create Event from dictionary (for reading JSONL)."""
        return cls(
            ts=data["ts"],
            level=data["level"],
            source=data["source"],
            event=data["event"],
            context=data.get("context", {}),
            command=data.get("command"),
            duration_ms=data.get("duration_ms"),
            suggestion=data.get("suggestion"),
        )
