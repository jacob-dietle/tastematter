"""Observability module for agent-context-first logging.

Provides structured event logging designed for Claude Code agents
to understand system state quickly.
"""

from pathlib import Path

from .events import Event
from .event_logger import EventLogger
from .state import (
    HealthSnapshot,
    ActivitySnapshot,
    RecentCommand,
    generate_health_snapshot,
    generate_activity_snapshot,
    update_state,
)

# Default singleton instance using ~/.context-os/
_default_log_dir = Path.home() / ".context-os"
event_logger = EventLogger(_default_log_dir)

__all__ = [
    "Event",
    "EventLogger",
    "event_logger",
    "HealthSnapshot",
    "ActivitySnapshot",
    "RecentCommand",
    "generate_health_snapshot",
    "generate_activity_snapshot",
    "update_state",
]
