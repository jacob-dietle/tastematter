"""JSONL parser for Claude Code session files.

Parses Claude session JSONL files to extract:
- File access patterns (game trails)
- Tool usage patterns
- Grep patterns (automation candidates)
"""

import json
import logging
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from sqlite3 import Connection
from typing import Any, Dict, List, Optional, TypedDict

logger = logging.getLogger(__name__)


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class ToolUse:
    """Extracted tool use from assistant message."""
    id: str
    name: str                    # Read, Edit, Write, Grep, etc.
    input: Dict[str, Any]        # Tool-specific inputs
    timestamp: datetime

    # Extracted fields (computed)
    file_path: Optional[str]     # Primary file being accessed
    is_read: bool                # True for Read, Grep, Glob
    is_write: bool               # True for Edit, Write


@dataclass
class ParsedMessage:
    """Single parsed message from JSONL."""
    type: str                    # user, assistant, tool_result
    role: Optional[str]          # user, assistant
    content: Any                 # str or List[content_block]
    timestamp: datetime
    tool_uses: List[ToolUse]     # Extracted from content if assistant


@dataclass
class SessionSummary:
    """Aggregated session data for database."""
    session_id: str              # UUID from filename
    project_path: str            # Decoded project path

    # Timing
    started_at: datetime
    ended_at: datetime
    duration_seconds: int

    # Message counts
    user_message_count: int
    assistant_message_count: int
    total_messages: int

    # File interactions
    files_read: List[str]        # Unique files read
    files_written: List[str]     # Unique files written/edited
    files_created: List[str]     # Files created (Write to new path)

    # Tool usage
    tools_used: Dict[str, int]   # {"Read": 15, "Edit": 8, ...}

    # Patterns (automation candidates)
    grep_patterns: List[str]     # Patterns used in Grep calls

    # Size metrics
    file_size_bytes: int         # JSONL file size


class ParseOptions(TypedDict, total=False):
    """Options for parsing sessions."""
    project_path: str            # Path to project (will be encoded)
    incremental: bool            # Only parse new/modified sessions


class ParseResult(TypedDict):
    """Result of parse operation."""
    sessions_parsed: int
    sessions_skipped: int        # Already in DB, unchanged
    total_tool_uses: int
    errors: List[str]


# ============================================================================
# Path Encoding
# ============================================================================

def encode_project_path(path: str) -> str:
    """Encode filesystem path to Claude project directory name.

    Windows: C:\\Users\\dietl\\Project → C--Users-dietl-Project
    Unix: /home/user/project → -home-user-project

    Args:
        path: Filesystem path to project

    Returns:
        Encoded directory name
    """
    # Normalize path
    path = str(Path(path).resolve())

    # Windows: Replace :\ with --, then \ with -
    if ":" in path:
        # C:\foo → C--foo
        path = path.replace(":\\", "--")
        path = path.replace("\\", "-")
    else:
        # Unix: /foo/bar → -foo-bar
        path = path.replace("/", "-")

    # Replace spaces and underscores with dashes
    path = path.replace(" ", "-")
    path = path.replace("_", "-")

    return path


def decode_project_path(encoded: str) -> str:
    """Decode Claude project directory name to filesystem path.

    Inverse of encode_project_path.

    Args:
        encoded: Encoded directory name

    Returns:
        Filesystem path (best effort - may not be exact)
    """
    import re

    # Windows detection: starts with X--
    if re.match(r'^[A-Za-z]--', encoded):
        # C--Users-foo → C:\Users\foo
        drive = encoded[0]
        rest = encoded[3:]  # Skip X--
        # Note: Can't distinguish - from original spaces/slashes
        # This is best-effort
        path = f"{drive}:\\" + rest.replace("-", "\\")
    else:
        # Unix: -home-user → /home/user
        path = encoded.replace("-", "/")

    return path


def get_claude_projects_dir() -> Path:
    """Get the Claude projects directory.

    Returns:
        Path to ~/.claude/projects/
    """
    return Path.home() / ".claude" / "projects"


# ============================================================================
# File Discovery
# ============================================================================

def find_session_files(
    project_path: str,
    claude_dir: Optional[Path] = None
) -> List[Path]:
    """Find all JSONL session files for a project.

    Args:
        project_path: Filesystem path to project
        claude_dir: Override Claude directory (for testing)

    Returns:
        List of JSONL file paths, sorted by modification time
    """
    claude_dir = claude_dir or get_claude_projects_dir()
    encoded = encode_project_path(project_path)

    project_dir = claude_dir / encoded

    if not project_dir.exists():
        return []

    # Find all JSONL files
    files = list(project_dir.glob("*.jsonl"))

    # Sort by modification time (newest first)
    files.sort(key=lambda f: f.stat().st_mtime, reverse=True)

    return files


def extract_session_id(filepath: Path) -> str:
    """Extract session UUID from JSONL filename.

    Args:
        filepath: Path to JSONL file

    Returns:
        Session ID (UUID string)
    """
    return filepath.stem  # filename without extension


# ============================================================================
# Tool Use Extraction
# ============================================================================

def extract_tool_uses(content: List[Dict], timestamp: datetime) -> List[ToolUse]:
    """Extract tool_use blocks from assistant message content.

    Args:
        content: List of content blocks from assistant message
        timestamp: Message timestamp

    Returns:
        List of ToolUse objects
    """
    tool_uses = []

    for block in content:
        if block.get("type") != "tool_use":
            continue

        name = block.get("name", "")
        input_data = block.get("input", {})

        # Extract file path based on tool type
        file_path = extract_file_path(name, input_data)

        # Determine read vs write
        is_read = name in ("Read", "Grep", "Glob", "WebFetch", "WebSearch")
        is_write = name in ("Edit", "Write", "NotebookEdit")

        tool_uses.append(ToolUse(
            id=block.get("id", ""),
            name=name,
            input=input_data,
            timestamp=timestamp,
            file_path=file_path,
            is_read=is_read,
            is_write=is_write
        ))

    return tool_uses


def extract_file_path(tool_name: str, input_data: Dict) -> Optional[str]:
    """Extract primary file path from tool input.

    Args:
        tool_name: Name of the tool
        input_data: Tool input dictionary

    Returns:
        File path string or None
    """
    # Grep pattern (for automation detection) - check first for Grep
    # We want to track the search pattern, not the path being searched
    if tool_name == "Grep" and "pattern" in input_data:
        return f"GREP:{input_data['pattern']}"

    # Glob pattern - check first for Glob
    if tool_name == "Glob" and "pattern" in input_data:
        return f"GLOB:{input_data['pattern']}"

    # Direct file_path parameter
    if "file_path" in input_data:
        return input_data["file_path"]

    # Notebook path
    if "notebook_path" in input_data:
        return input_data["notebook_path"]

    # Generic path (for other tools)
    if "path" in input_data:
        return input_data["path"]

    return None


# ============================================================================
# Message Parsing
# ============================================================================

def parse_jsonl_line(line: str) -> Optional[ParsedMessage]:
    """Parse a single line from JSONL file.

    Args:
        line: Raw line from file

    Returns:
        ParsedMessage or None if line is empty/invalid
    """
    line = line.strip()
    if not line:
        return None

    try:
        data = json.loads(line)
    except json.JSONDecodeError:
        return None

    msg_type = data.get("type")
    if msg_type not in ("user", "assistant", "tool_result"):
        return None

    # Parse timestamp
    timestamp_str = data.get("timestamp", "")
    try:
        # Handle Z suffix
        timestamp_str = timestamp_str.replace("Z", "+00:00")
        timestamp = datetime.fromisoformat(timestamp_str)
    except ValueError:
        timestamp = datetime.now()

    # Extract message content
    message = data.get("message", {})
    role = message.get("role")
    content = message.get("content", data.get("content"))

    # Extract tool uses if assistant message
    tool_uses = []
    if msg_type == "assistant" and isinstance(content, list):
        tool_uses = extract_tool_uses(content, timestamp)

    return ParsedMessage(
        type=msg_type,
        role=role,
        content=content,
        timestamp=timestamp,
        tool_uses=tool_uses
    )


# ============================================================================
# Session Aggregation
# ============================================================================

def aggregate_session(
    session_id: str,
    project_path: str,
    messages: List[ParsedMessage],
    file_size: int
) -> SessionSummary:
    """Aggregate parsed messages into session summary.

    Args:
        session_id: Session UUID
        project_path: Decoded project path
        messages: List of parsed messages
        file_size: JSONL file size in bytes

    Returns:
        SessionSummary for database insertion
    """
    # Timing
    timestamps = [m.timestamp for m in messages if m.timestamp]
    started_at = min(timestamps) if timestamps else datetime.now()
    ended_at = max(timestamps) if timestamps else datetime.now()
    duration = int((ended_at - started_at).total_seconds())

    # Message counts
    user_count = sum(1 for m in messages if m.type == "user")
    assistant_count = sum(1 for m in messages if m.type == "assistant")

    # Collect all tool uses
    all_tool_uses = []
    for msg in messages:
        all_tool_uses.extend(msg.tool_uses)

    # Files read (deduplicated)
    files_read: set = set()
    files_written: set = set()
    grep_patterns: List[str] = []

    for tool_use in all_tool_uses:
        if tool_use.file_path:
            path = tool_use.file_path

            # Handle pseudo-paths
            if path.startswith("GREP:"):
                grep_patterns.append(path[5:])  # Remove prefix
            elif path.startswith("GLOB:"):
                pass  # Track separately if needed
            elif tool_use.is_read:
                files_read.add(path)
            elif tool_use.is_write:
                files_written.add(path)

    # Tool usage counts
    tools_used: Dict[str, int] = {}
    for tool_use in all_tool_uses:
        tools_used[tool_use.name] = tools_used.get(tool_use.name, 0) + 1

    # Files created vs modified (heuristic: Write = create, Edit = modify)
    files_created: set = set()
    for tool_use in all_tool_uses:
        if tool_use.name == "Write" and tool_use.file_path:
            if not tool_use.file_path.startswith(("GREP:", "GLOB:")):
                files_created.add(tool_use.file_path)

    return SessionSummary(
        session_id=session_id,
        project_path=project_path,
        started_at=started_at,
        ended_at=ended_at,
        duration_seconds=duration,
        user_message_count=user_count,
        assistant_message_count=assistant_count,
        total_messages=len(messages),
        files_read=list(files_read),
        files_written=list(files_written),
        files_created=list(files_created),
        tools_used=tools_used,
        grep_patterns=grep_patterns,
        file_size_bytes=file_size
    )


# ============================================================================
# Session File Parsing
# ============================================================================

def parse_session_file(filepath: Path, project_path: str) -> SessionSummary:
    """Parse a complete JSONL session file.

    Args:
        filepath: Path to JSONL file
        project_path: Decoded project path

    Returns:
        SessionSummary for database insertion
    """
    session_id = extract_session_id(filepath)
    file_size = filepath.stat().st_size

    messages = []
    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        for line in f:
            msg = parse_jsonl_line(line)
            if msg:
                messages.append(msg)

    return aggregate_session(session_id, project_path, messages, file_size)


# ============================================================================
# Database Operations
# ============================================================================

def session_needs_update(
    db: Connection,
    session_id: str,
    file_size: int
) -> bool:
    """Check if session needs to be parsed/updated.

    Uses file size as change detection heuristic.

    Args:
        db: Database connection
        session_id: Session UUID
        file_size: Current file size

    Returns:
        True if session should be (re)parsed
    """
    cursor = db.execute("""
        SELECT file_size_bytes FROM claude_sessions
        WHERE session_id = ?
    """, (session_id,))

    row = cursor.fetchone()
    if not row:
        return True  # New session

    # If file grew, re-parse
    return file_size > row[0]


def upsert_session(db: Connection, summary: SessionSummary) -> None:
    """Insert or update session in database.

    Args:
        db: Database connection
        summary: Session summary to store
    """
    db.execute("""
        INSERT OR REPLACE INTO claude_sessions (
            session_id, project_path,
            started_at, ended_at, duration_seconds,
            user_message_count, assistant_message_count, total_messages,
            files_read, files_written, files_created,
            tools_used, grep_patterns,
            file_size_bytes
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, (
        summary.session_id,
        summary.project_path,
        summary.started_at.isoformat(),
        summary.ended_at.isoformat(),
        summary.duration_seconds,
        summary.user_message_count,
        summary.assistant_message_count,
        summary.total_messages,
        json.dumps(summary.files_read),
        json.dumps(summary.files_written),
        json.dumps(summary.files_created),
        json.dumps(summary.tools_used),
        json.dumps(summary.grep_patterns),
        summary.file_size_bytes
    ))


# ============================================================================
# Main Sync Function
# ============================================================================

def sync_sessions(
    db: Connection,
    options: ParseOptions
) -> ParseResult:
    """Sync Claude sessions to database.

    Args:
        db: Database connection
        options: Parse configuration

    Returns:
        ParseResult with counts
    """
    project_path = options["project_path"]
    incremental = options.get("incremental", True)

    # Find session files
    files = find_session_files(project_path)

    parsed = 0
    skipped = 0
    total_tools = 0
    errors: List[str] = []

    for filepath in files:
        session_id = extract_session_id(filepath)
        file_size = filepath.stat().st_size

        try:
            # Check if needs update
            if incremental and not session_needs_update(db, session_id, file_size):
                skipped += 1
                continue

            # Parse and insert/update
            summary = parse_session_file(filepath, project_path)
            upsert_session(db, summary)

            parsed += 1
            total_tools += sum(summary.tools_used.values())

        except Exception as e:
            errors.append(f"{filepath.name}: {str(e)}")
            logger.warning(f"Failed to parse {filepath}: {e}")
            continue

    db.commit()

    return ParseResult(
        sessions_parsed=parsed,
        sessions_skipped=skipped,
        total_tool_uses=total_tools,
        errors=errors
    )
