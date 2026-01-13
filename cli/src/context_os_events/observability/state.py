"""State snapshot generation for agent-context logging.

Generates health and activity snapshots for agent consumption.
Designed for quick parsing - JSON output, not streaming.
"""

import json
import sqlite3
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from context_os_events.observability.event_logger import EventLogger


@dataclass
class RecentCommand:
    """A recent command execution record.

    Used in ActivitySnapshot to show what commands ran recently.

    Attributes:
        ts: ISO8601 UTC timestamp of command execution
        command: Command name (e.g., "build-chains")
        status: Execution status ("success" or "error")
        duration_ms: Execution duration in milliseconds
    """

    ts: str
    command: str
    status: str
    duration_ms: int


@dataclass
class HealthSnapshot:
    """System health snapshot for agent context.

    Contains database statistics, recent errors, and warnings.
    Written to ~/.context-os/state/health.json by update_state().

    Attributes:
        generated_at: ISO8601 UTC timestamp of snapshot generation
        database: Database info (path, size_mb, tables dict)
        recent_errors: List of recent error events
        warnings: List of current system warnings
    """

    generated_at: str
    database: Dict[str, Any]
    recent_errors: List[Dict[str, Any]]
    warnings: List[Dict[str, Any]]

    def to_dict(self) -> Dict[str, Any]:
        """Convert to JSON-serializable dictionary."""
        return asdict(self)


@dataclass
class ActivitySnapshot:
    """Recent activity snapshot for agent context.

    Contains command execution history and aggregated metrics.
    Written to ~/.context-os/state/activity.json by update_state().

    Attributes:
        generated_at: ISO8601 UTC timestamp of snapshot generation
        last_24h: Aggregated metrics for last 24 hours
        recent_commands: List of recent command executions
    """

    generated_at: str
    last_24h: Dict[str, int]
    recent_commands: List[RecentCommand]

    def to_dict(self) -> Dict[str, Any]:
        """Convert to JSON-serializable dictionary."""
        return asdict(self)


# Tables to include in health snapshot
TRACKED_TABLES = [
    "claude_sessions",
    "chain_graph",
    "chains",
    "file_conversation_index",
    "conversation_intelligence",
    "work_chains",
    "git_commits",
    "file_events",
]


def generate_health_snapshot(db_path: Optional[Path] = None) -> HealthSnapshot:
    """Generate health snapshot from database state.

    Queries database for table statistics and returns a HealthSnapshot
    with row counts and database info.

    Args:
        db_path: Path to database. Uses default if not specified.

    Returns:
        HealthSnapshot with database statistics.
    """
    from context_os_events.db.connection import get_connection, DEFAULT_DB_PATH

    db_path = db_path or DEFAULT_DB_PATH
    conn = get_connection(db_path)

    # Get database file size
    try:
        size_mb = db_path.stat().st_size / (1024 * 1024)
    except (OSError, IOError):
        size_mb = 0.0

    # Get row counts for each table
    tables: Dict[str, Dict[str, Any]] = {}
    for table_name in TRACKED_TABLES:
        try:
            cursor = conn.execute(f"SELECT COUNT(*) FROM {table_name}")
            row_count = cursor.fetchone()[0]
            tables[table_name] = {
                "rows": row_count,
                "last_updated": None,  # TODO: Track last update timestamp
            }
        except sqlite3.OperationalError:
            # Table doesn't exist
            tables[table_name] = {"rows": 0, "last_updated": None}

    conn.close()

    return HealthSnapshot(
        generated_at=datetime.utcnow().isoformat() + "Z",
        database={
            "path": str(db_path),
            "size_mb": round(size_mb, 2),
            "tables": tables,
        },
        recent_errors=[],  # TODO: Populate from event_logger
        warnings=[],
    )


def generate_activity_snapshot(
    event_logger: Optional["EventLogger"] = None,
) -> ActivitySnapshot:
    """Generate activity snapshot from event log.

    Reads recent events from the event logger and aggregates
    command execution statistics.

    Args:
        event_logger: EventLogger instance to read from.
                      Uses default singleton if not specified.

    Returns:
        ActivitySnapshot with recent command history.
    """
    from datetime import timedelta

    from context_os_events.observability.event_logger import EventLogger

    if event_logger is None:
        from context_os_events.observability import event_logger as default_logger
        event_logger = default_logger

    # Get recent events
    recent_events = event_logger.get_recent(limit=100)

    # Filter command events
    command_events = [
        e for e in recent_events
        if e.event in ("command_complete", "command_error")
        and e.command is not None
    ]

    # Build recent commands list
    recent_commands: List[RecentCommand] = []
    for event in command_events[:20]:  # Limit to 20 recent commands
        status = "error" if event.event == "command_error" else "success"
        recent_commands.append(
            RecentCommand(
                ts=event.ts,
                command=event.command,
                status=status,
                duration_ms=event.duration_ms or 0,
            )
        )

    # Aggregate last 24h metrics
    now = datetime.utcnow()
    cutoff = now - timedelta(hours=24)
    cutoff_str = cutoff.isoformat() + "Z"

    commands_24h = [e for e in command_events if e.ts >= cutoff_str]
    errors_24h = [e for e in commands_24h if e.event == "command_error"]

    return ActivitySnapshot(
        generated_at=now.isoformat() + "Z",
        last_24h={
            "commands_run": len(commands_24h),
            "errors": len(errors_24h),
        },
        recent_commands=recent_commands,
    )


# Default state directory
DEFAULT_STATE_DIR = Path.home() / ".context-os" / "state"


def update_state(
    db_path: Optional[Path] = None,
    state_dir: Optional[Path] = None,
    event_log_dir: Optional[Path] = None,
) -> None:
    """Update state snapshot files.

    Generates health and activity snapshots and writes them to JSON files
    in the state directory.

    Args:
        db_path: Path to database. Uses default if not specified.
        state_dir: Directory for state files. Uses ~/.context-os/state/ if not specified.
        event_log_dir: Directory for event logs. Uses ~/.context-os/ if not specified.
    """
    from context_os_events.observability.event_logger import EventLogger

    state_dir = state_dir or DEFAULT_STATE_DIR
    state_dir.mkdir(parents=True, exist_ok=True)

    # Generate health snapshot
    health = generate_health_snapshot(db_path)
    health_path = state_dir / "health.json"
    with open(health_path, "w", encoding="utf-8") as f:
        json.dump(health.to_dict(), f, indent=2)

    # Generate activity snapshot
    if event_log_dir:
        event_logger = EventLogger(event_log_dir)
    else:
        from context_os_events.observability import event_logger
    activity = generate_activity_snapshot(event_logger)
    activity_path = state_dir / "activity.json"
    with open(activity_path, "w", encoding="utf-8") as f:
        json.dump(activity.to_dict(), f, indent=2)
