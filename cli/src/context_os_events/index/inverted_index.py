"""Inverted file index for Context OS Intelligence.

Maps: file_path -> List[FileAccess]
Enables: "Which sessions touched this file?"

Algorithm:
1. Parse tool_use blocks from JSONL (Read, Edit, Write, etc.)
2. Extract file paths from tool inputs
3. Filter out pseudo-paths (GREP:, GLOB:)
4. Classify access type (read/write/create)
5. Build file -> sessions mapping
6. Deduplicate within session (increment count)
"""

import json
import logging
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

logger = logging.getLogger(__name__)


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class FileAccess:
    """Single file access record with context."""
    session_id: str
    chain_id: Optional[str]
    file_path: str
    access_type: str  # 'read', 'write', 'create', 'mention'
    tool_name: str
    timestamp: datetime
    access_count: int = 1


# ============================================================================
# Tool Classification
# ============================================================================

# Tools that read files
READ_TOOLS = {"Read", "Grep", "Glob", "WebFetch", "WebSearch"}

# Tools that write/modify files
WRITE_TOOLS = {"Edit", "NotebookEdit"}

# Tools that create new files
CREATE_TOOLS = {"Write"}


def _classify_access_type(tool_name: str) -> Optional[str]:
    """Classify tool into access type.

    Args:
        tool_name: Name of the tool

    Returns:
        'read', 'write', 'create', or None if not a file tool
    """
    if tool_name in READ_TOOLS:
        return "read"
    elif tool_name in WRITE_TOOLS:
        return "write"
    elif tool_name in CREATE_TOOLS:
        return "create"
    return None


def _extract_file_path_from_tool(tool_name: str, input_data: Dict[str, Any]) -> Optional[str]:
    """Extract file path from tool input.

    Handles different parameter names for different tools.
    Filters out pseudo-paths (GREP:, GLOB:).

    Args:
        tool_name: Name of the tool
        input_data: Tool input parameters

    Returns:
        File path or None if not a file-based tool
    """
    # Skip Grep/Glob - these are patterns, not file accesses
    if tool_name in ("Grep", "Glob"):
        return None

    # NotebookEdit uses notebook_path
    if tool_name == "NotebookEdit":
        return input_data.get("notebook_path")

    # Most tools use file_path
    if "file_path" in input_data:
        return input_data["file_path"]

    # Fallback to generic path
    if "path" in input_data:
        return input_data["path"]

    return None


# ============================================================================
# Extraction Functions
# ============================================================================

def extract_file_accesses(filepath: Path, session_id: Optional[str] = None) -> List[FileAccess]:
    """Extract file accesses from a JSONL session file.

    Parses tool_use blocks and extracts file paths with access metadata.

    Args:
        filepath: Path to JSONL file
        session_id: Optional session ID (defaults to filename stem)

    Returns:
        List of FileAccess records (deduplicated within session)
    """
    if session_id is None:
        session_id = filepath.stem

    # Track accesses by (file_path, access_type) for deduplication
    access_tracker: Dict[tuple, FileAccess] = {}

    try:
        with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue

                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue

                # Only process assistant messages with tool_use
                if record.get("type") != "assistant":
                    continue

                message = record.get("message", {})
                content = message.get("content", [])

                # Content can be string or list
                if not isinstance(content, list):
                    continue

                # Extract timestamp (use current time as fallback)
                timestamp_str = record.get("timestamp")
                if timestamp_str:
                    try:
                        timestamp = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
                    except (ValueError, AttributeError):
                        timestamp = datetime.now()
                else:
                    timestamp = datetime.now()

                # Process each content block
                for block in content:
                    if not isinstance(block, dict):
                        continue

                    if block.get("type") != "tool_use":
                        continue

                    tool_name = block.get("name", "")
                    input_data = block.get("input", {})

                    # Skip non-file tools
                    access_type = _classify_access_type(tool_name)
                    if access_type is None:
                        continue

                    # Extract file path
                    file_path = _extract_file_path_from_tool(tool_name, input_data)
                    if file_path is None:
                        continue

                    # Deduplication key
                    key = (file_path, access_type)

                    if key in access_tracker:
                        # Increment count for existing entry
                        access_tracker[key].access_count += 1
                    else:
                        # Create new entry
                        access_tracker[key] = FileAccess(
                            session_id=session_id,
                            chain_id=None,  # Populated later by build_inverted_index
                            file_path=file_path,
                            access_type=access_type,
                            tool_name=tool_name,
                            timestamp=timestamp,
                            access_count=1,
                        )

    except Exception as e:
        logger.warning(f"Failed to extract file accesses from {filepath}: {e}")

    return list(access_tracker.values())


# ============================================================================
# Index Building
# ============================================================================

def build_inverted_index(
    jsonl_dir: Path,
    chains: Optional[Dict[str, Any]] = None
) -> Dict[str, List[FileAccess]]:
    """Build inverted file index from JSONL directory.

    Maps each file path to the list of sessions that accessed it.

    Args:
        jsonl_dir: Directory containing JSONL session files
        chains: Optional chain graph for adding chain context

    Returns:
        Dict mapping file_path -> List[FileAccess]
    """
    # Build session -> chain lookup
    session_to_chain: Dict[str, str] = {}
    if chains:
        for chain_id, chain in chains.items():
            for session_id in chain.sessions:
                session_to_chain[session_id] = chain_id

    # Find all JSONL files
    jsonl_files = list(jsonl_dir.glob("*.jsonl"))

    if not jsonl_files:
        return {}

    # Build inverted index
    index: Dict[str, List[FileAccess]] = defaultdict(list)

    for jsonl_file in jsonl_files:
        session_id = jsonl_file.stem
        accesses = extract_file_accesses(jsonl_file, session_id)

        # Add chain context
        chain_id = session_to_chain.get(session_id)
        for access in accesses:
            access.chain_id = chain_id
            index[access.file_path].append(access)

    return dict(index)


# ============================================================================
# Query Functions
# ============================================================================

def get_sessions_for_file(
    index: Dict[str, List[FileAccess]],
    file_path: str
) -> List[FileAccess]:
    """Get all sessions that touched a file.

    Args:
        index: Inverted file index
        file_path: File path to look up

    Returns:
        List of FileAccess records for this file
    """
    return index.get(file_path, [])


def get_files_for_session(
    index: Dict[str, List[FileAccess]],
    session_id: str
) -> List[str]:
    """Get all files touched in a session.

    Args:
        index: Inverted file index
        session_id: Session to look up

    Returns:
        List of file paths touched in this session
    """
    files = []
    for file_path, accesses in index.items():
        for access in accesses:
            if access.session_id == session_id:
                files.append(file_path)
                break  # Only add once per file
    return files


# ============================================================================
# Database Persistence
# ============================================================================

def persist_inverted_index(db, index: Dict[str, List[FileAccess]]) -> Dict[str, int]:
    """Persist inverted index to database.

    Writes to file_conversation_index table.

    Args:
        db: SQLite connection
        index: Inverted file index

    Returns:
        Stats dict: {"files_stored": N, "accesses_stored": M}
    """
    files_stored = 0
    accesses_stored = 0

    files_seen = set()

    for file_path, accesses in index.items():
        for access in accesses:
            db.execute("""
                INSERT OR REPLACE INTO file_conversation_index (
                    file_path, session_id, access_type, access_count,
                    first_accessed_at, chain_id
                ) VALUES (?, ?, ?, ?, ?, ?)
            """, (
                access.file_path,
                access.session_id,
                access.access_type,
                access.access_count,
                access.timestamp.isoformat() if access.timestamp else None,
                access.chain_id,
            ))
            accesses_stored += 1

            if file_path not in files_seen:
                files_seen.add(file_path)
                files_stored += 1

    db.commit()

    return {
        "files_stored": files_stored,
        "accesses_stored": accesses_stored,
    }


def load_inverted_index(db) -> Dict[str, List[FileAccess]]:
    """Load inverted index from database.

    Args:
        db: SQLite connection

    Returns:
        Dict mapping file_path -> List[FileAccess]
    """
    index: Dict[str, List[FileAccess]] = defaultdict(list)

    cursor = db.execute("""
        SELECT file_path, session_id, access_type, access_count,
               first_accessed_at, chain_id
        FROM file_conversation_index
    """)

    for row in cursor.fetchall():
        file_path = row[0]
        session_id = row[1]
        access_type = row[2]
        access_count = row[3] or 1
        timestamp_str = row[4]
        chain_id = row[5]

        # Parse timestamp
        if timestamp_str:
            try:
                timestamp = datetime.fromisoformat(timestamp_str)
            except ValueError:
                timestamp = datetime.now()
        else:
            timestamp = datetime.now()

        access = FileAccess(
            session_id=session_id,
            chain_id=chain_id,
            file_path=file_path,
            access_type=access_type or "read",
            tool_name="",  # Not stored in DB
            timestamp=timestamp,
            access_count=access_count,
        )

        index[file_path].append(access)

    return dict(index)
