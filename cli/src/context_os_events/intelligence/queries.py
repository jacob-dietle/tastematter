"""Intelligence query module.

Thin wrappers around database queries for the meta-intelligence agent.
The agent interprets the results - these functions just fetch data.
"""

from datetime import datetime, timedelta
from pathlib import Path
from sqlite3 import Connection
from typing import Any, Dict, List

from ..capture.jsonl_parser import encode_project_path


def get_recent_sessions(
    db: Connection,
    days: int = 7
) -> List[Dict[str, Any]]:
    """Get recent Claude sessions within specified days.

    Args:
        db: Database connection
        days: Number of days to look back (default: 7)

    Returns:
        List of session dictionaries with all columns
    """
    cutoff = (datetime.now() - timedelta(days=days)).isoformat()

    cursor = db.execute("""
        SELECT *
        FROM claude_sessions
        WHERE started_at >= ?
        ORDER BY started_at DESC
    """, (cutoff,))

    return [dict(row) for row in cursor.fetchall()]


def get_game_trails(
    db: Connection,
    limit: int = 20
) -> List[Dict[str, Any]]:
    """Get most accessed files (game trails).

    Args:
        db: Database connection
        limit: Maximum number of results (default: 20)

    Returns:
        List of file paths with access counts, ordered by frequency
    """
    cursor = db.execute("""
        SELECT path, total_accesses
        FROM game_trails
        LIMIT ?
    """, (limit,))

    return [dict(row) for row in cursor.fetchall()]


def get_tool_patterns(db: Connection) -> List[Dict[str, Any]]:
    """Get tool usage patterns across all sessions.

    Args:
        db: Database connection

    Returns:
        List of tools with usage counts, ordered by frequency
    """
    cursor = db.execute("""
        SELECT tool, total_uses, sessions_used_in
        FROM tool_patterns
    """)

    return [dict(row) for row in cursor.fetchall()]


def get_recent_commits(
    db: Connection,
    days: int = 7
) -> List[Dict[str, Any]]:
    """Get recent git commits within specified days.

    Args:
        db: Database connection
        days: Number of days to look back (default: 7)

    Returns:
        List of commit dictionaries
    """
    cutoff = (datetime.now() - timedelta(days=days)).isoformat()

    cursor = db.execute("""
        SELECT *
        FROM git_commits
        WHERE timestamp >= ?
        ORDER BY timestamp DESC
    """, (cutoff,))

    return [dict(row) for row in cursor.fetchall()]


def get_session_jsonl_path(
    session_id: str,
    project_path: str
) -> Path:
    """Get the path to a session's raw JSONL file.

    Args:
        session_id: Session UUID
        project_path: Project filesystem path

    Returns:
        Path to the JSONL file in ~/.claude/projects/
    """
    encoded = encode_project_path(project_path)
    claude_dir = Path.home() / ".claude" / "projects" / encoded

    return claude_dir / f"{session_id}.jsonl"
