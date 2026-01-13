"""Chain graph builder from Claude Code's leafUuid mechanism.

Claude Code tracks conversation chains explicitly via leafUuid:
- Summary records at start of JSONL have {"type":"summary","leafUuid":"..."}
- The leafUuid points to a message.uuid in the parent conversation
- This gives us explicit chain linking without heuristics

Algorithm:
1. Pass 1: Extract leafUuid from all sessions (who references whom)
2. Pass 2: Extract message.uuid from all sessions (who owns what uuid)
3. Pass 3: Build parent-child links (leafUuid -> uuid matching)
4. Pass 4: Group into chains (connected components)
"""

import hashlib
import json
import logging
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

logger = logging.getLogger(__name__)


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class ChainNode:
    """Single session's position in the chain graph."""
    session_id: str
    parent_session_id: Optional[str]  # Session containing the leafUuid message
    parent_message_uuid: str          # The actual leafUuid value
    children: List[str] = field(default_factory=list)  # Sessions that continue from this


@dataclass
class Chain:
    """A connected chain of sessions."""
    chain_id: str                     # Generated hash of root session
    root_session: str                 # First session (no parent)
    sessions: List[str]               # All sessions in order
    branches: Dict[str, List[str]]    # If chain has branches: parent -> [children]
    time_range: Optional[Tuple[datetime, datetime]]  # Start/end timestamps
    total_duration_seconds: int
    files_bloom: Optional[bytes]      # Bloom filter of all files (serialized)
    files_list: List[str]             # All unique files touched


# ============================================================================
# Extraction Functions
# ============================================================================

def extract_leaf_uuids(filepath: Path) -> List[str]:
    """Extract leafUuid values from summary records in a JSONL file.

    Args:
        filepath: Path to JSONL file

    Returns:
        List of leafUuid values found in summary records
    """
    leaf_uuids = []

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

                # Only extract from summary records
                if record.get("type") == "summary" and record.get("leafUuid"):
                    leaf_uuids.append(record["leafUuid"])

    except Exception as e:
        logger.warning(f"Failed to extract leafUuids from {filepath}: {e}")

    return leaf_uuids


def extract_message_uuids(filepath: Path) -> List[str]:
    """Extract message uuid values from a JSONL file.

    Note: Only extracts uuid from message records (user/assistant),
    NOT leafUuid from summary records.

    Args:
        filepath: Path to JSONL file

    Returns:
        List of message uuid values found
    """
    uuids = []

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

                # Only extract uuid from message records (not summary records)
                # Summary records have leafUuid, not uuid
                if record.get("type") in ("user", "assistant", "tool_result"):
                    if record.get("uuid"):
                        uuids.append(record["uuid"])

    except Exception as e:
        logger.warning(f"Failed to extract message uuids from {filepath}: {e}")

    return uuids


# ============================================================================
# Chain Graph Building
# ============================================================================

def build_chain_graph(jsonl_dir: Path) -> Dict[str, Chain]:
    """Build chain graph from leafUuid references in JSONL files.

    Algorithm:
    1. Pass 1: Collect leafUuid -> [sessions that reference it]
    2. Pass 2: Collect uuid -> session that owns it
    3. Pass 3: Build parent-child relationships
    4. Pass 4: Group into chains (connected components)

    Args:
        jsonl_dir: Directory containing JSONL files

    Returns:
        Dict mapping chain_id to Chain objects
    """
    # Find all JSONL files
    jsonl_files = list(jsonl_dir.glob("*.jsonl"))

    if not jsonl_files:
        return {}

    # Pass 1: Collect leafUuid references
    # leafUuid -> [sessions that have this as leafUuid in summary]
    leaf_refs: Dict[str, List[str]] = {}

    for jsonl_file in jsonl_files:
        session_id = jsonl_file.stem
        leaf_uuids = extract_leaf_uuids(jsonl_file)

        for leaf_uuid in leaf_uuids:
            if leaf_uuid not in leaf_refs:
                leaf_refs[leaf_uuid] = []
            if session_id not in leaf_refs[leaf_uuid]:
                leaf_refs[leaf_uuid].append(session_id)

    # Pass 2: Collect uuid ownership
    # message.uuid -> session that owns it
    uuid_to_session: Dict[str, str] = {}

    for jsonl_file in jsonl_files:
        session_id = jsonl_file.stem
        message_uuids = extract_message_uuids(jsonl_file)

        for uuid in message_uuids:
            uuid_to_session[uuid] = session_id

    # Pass 3: Build parent-child relationships
    # child_session -> parent_session
    parent_map: Dict[str, str] = {}
    # parent_session -> [child_sessions]
    children_map: Dict[str, List[str]] = {}

    for leaf_uuid, child_sessions in leaf_refs.items():
        parent_session = uuid_to_session.get(leaf_uuid)

        if parent_session:
            for child in child_sessions:
                if child != parent_session:  # Don't self-link
                    parent_map[child] = parent_session

                    if parent_session not in children_map:
                        children_map[parent_session] = []
                    if child not in children_map[parent_session]:
                        children_map[parent_session].append(child)

    # Pass 4: Group into chains (connected components)
    # Find all unique sessions
    all_sessions: Set[str] = set(f.stem for f in jsonl_files)

    # Find roots (sessions with no parent)
    sessions_with_parents = set(parent_map.keys())
    roots = all_sessions - sessions_with_parents

    # Build chains from each root
    chains: Dict[str, Chain] = {}
    visited: Set[str] = set()

    for root in roots:
        if root in visited:
            continue

        # BFS to find all sessions in this chain
        chain_sessions = []
        queue = [root]

        while queue:
            current = queue.pop(0)
            if current in visited:
                continue

            visited.add(current)
            chain_sessions.append(current)

            # Add children to queue
            children = children_map.get(current, [])
            for child in children:
                if child not in visited:
                    queue.append(child)

        # Generate chain ID from root session
        chain_id = hashlib.md5(root.encode()).hexdigest()[:8]

        # Build branches map for this chain
        branches: Dict[str, List[str]] = {}
        for session in chain_sessions:
            if session in children_map:
                branches[session] = children_map[session]

        chains[chain_id] = Chain(
            chain_id=chain_id,
            root_session=root,
            sessions=chain_sessions,
            branches=branches,
            time_range=None,  # Will be populated when parsing timestamps
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )

    return chains


# ============================================================================
# Utility Functions
# ============================================================================

def get_session_chain(chains: Dict[str, Chain], session_id: str) -> Optional[str]:
    """Find which chain a session belongs to.

    Args:
        chains: Chain graph
        session_id: Session to look up

    Returns:
        chain_id if found, None otherwise
    """
    for chain_id, chain in chains.items():
        if session_id in chain.sessions:
            return chain_id
    return None


def get_session_parent(chains: Dict[str, Chain], session_id: str) -> Optional[str]:
    """Find the parent session in the chain.

    Args:
        chains: Chain graph
        session_id: Session to look up

    Returns:
        parent session_id if found, None if root or not in chain
    """
    for chain in chains.values():
        for parent, children in chain.branches.items():
            if session_id in children:
                return parent
    return None


def get_chain_depth(chain: Chain, session_id: str) -> int:
    """Calculate depth of a session in the chain (0 = root).

    Args:
        chain: Chain object
        session_id: Session to measure depth for

    Returns:
        Depth (0 for root, 1 for first child, etc.)
    """
    if session_id == chain.root_session:
        return 0

    depth = 0
    current = session_id

    # Walk up the tree
    while current != chain.root_session:
        for parent, children in chain.branches.items():
            if current in children:
                current = parent
                depth += 1
                break
        else:
            # No parent found, session might not be in chain
            break

    return depth


# ============================================================================
# Database Persistence
# ============================================================================

def persist_chains(db, chains: Dict[str, Chain]) -> Dict[str, int]:
    """Persist chain graph to database.

    Args:
        db: SQLite connection
        chains: Chain graph from build_chain_graph()

    Returns:
        Stats dict: {"chains_stored": N, "sessions_stored": M}
    """
    chains_stored = 0
    sessions_stored = 0

    for chain in chains.values():
        # Build parent map for this chain
        parent_map: Dict[str, str] = {}
        for parent, children in chain.branches.items():
            for child in children:
                parent_map[child] = parent

        # Insert chain metadata
        db.execute("""
            INSERT OR REPLACE INTO chains (
                chain_id, root_session_id, session_count, branch_count,
                max_depth, files_bloom, files_json, files_count,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        """, (
            chain.chain_id,
            chain.root_session,
            len(chain.sessions),
            len(chain.branches),
            max(get_chain_depth(chain, s) for s in chain.sessions) if chain.sessions else 0,
            chain.files_bloom,
            json.dumps(chain.files_list),
            len(chain.files_list),
        ))
        chains_stored += 1

        # Insert session chain memberships
        for session_id in chain.sessions:
            parent_session = parent_map.get(session_id)
            depth = get_chain_depth(chain, session_id)
            is_root = session_id == chain.root_session
            children_count = len(chain.branches.get(session_id, []))

            # Get the leafUuid that links to parent (if any)
            parent_uuid = ""
            # Note: We don't have direct access to leafUuid here after building
            # This would need to be passed through or re-extracted

            db.execute("""
                INSERT OR REPLACE INTO chain_graph (
                    session_id, parent_session_id, parent_message_uuid,
                    chain_id, position_in_chain, is_root, children_count,
                    indexed_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            """, (
                session_id,
                parent_session,
                parent_uuid,
                chain.chain_id,
                depth,
                is_root,
                children_count,
            ))
            sessions_stored += 1

    db.commit()

    return {
        "chains_stored": chains_stored,
        "sessions_stored": sessions_stored,
    }


def load_chains(db) -> Dict[str, Chain]:
    """Load chain graph from database.

    Args:
        db: SQLite connection

    Returns:
        Dict mapping chain_id to Chain objects
    """
    chains: Dict[str, Chain] = {}

    # Load chain metadata
    cursor = db.execute("""
        SELECT chain_id, root_session_id, session_count, files_bloom, files_json
        FROM chains
    """)

    for row in cursor.fetchall():
        chain_id = row[0]
        root_session = row[1]
        files_bloom = row[3]
        files_json = row[4]

        # Load sessions for this chain
        session_cursor = db.execute("""
            SELECT session_id, parent_session_id, children_count
            FROM chain_graph
            WHERE chain_id = ?
            ORDER BY position_in_chain
        """, (chain_id,))

        sessions = []
        branches: Dict[str, List[str]] = {}

        for session_row in session_cursor.fetchall():
            session_id = session_row[0]
            parent_session = session_row[1]
            children_count = session_row[2]

            sessions.append(session_id)

            # Build branches map
            if parent_session and children_count > 0:
                if parent_session not in branches:
                    branches[parent_session] = []

        # Re-build branches from parent relationships
        for session_row in db.execute("""
            SELECT session_id, parent_session_id
            FROM chain_graph
            WHERE chain_id = ? AND parent_session_id IS NOT NULL
        """, (chain_id,)):
            session_id = session_row[0]
            parent = session_row[1]
            if parent not in branches:
                branches[parent] = []
            if session_id not in branches[parent]:
                branches[parent].append(session_id)

        chains[chain_id] = Chain(
            chain_id=chain_id,
            root_session=root_session,
            sessions=sessions,
            branches=branches,
            time_range=None,
            total_duration_seconds=0,
            files_bloom=files_bloom,
            files_list=json.loads(files_json) if files_json else [],
        )

    return chains


def get_chain_for_session(db, session_id: str) -> Optional[str]:
    """Look up which chain a session belongs to.

    Args:
        db: SQLite connection
        session_id: Session to look up

    Returns:
        chain_id if found, None otherwise
    """
    cursor = db.execute("""
        SELECT chain_id FROM chain_graph WHERE session_id = ?
    """, (session_id,))

    row = cursor.fetchone()
    return row[0] if row else None


def get_session_context(db, session_id: str) -> Optional[Dict]:
    """Get chain context for a session.

    Returns session's position in chain, parent, siblings, etc.

    Args:
        db: SQLite connection
        session_id: Session to look up

    Returns:
        Dict with chain context or None if not found
    """
    cursor = db.execute("""
        SELECT
            cg.chain_id,
            cg.parent_session_id,
            cg.position_in_chain,
            cg.is_root,
            cg.children_count,
            c.session_count,
            c.root_session_id
        FROM chain_graph cg
        JOIN chains c ON cg.chain_id = c.chain_id
        WHERE cg.session_id = ?
    """, (session_id,))

    row = cursor.fetchone()
    if not row:
        return None

    return {
        "chain_id": row[0],
        "parent_session_id": row[1],
        "position_in_chain": row[2],
        "is_root": bool(row[3]),
        "children_count": row[4],
        "chain_session_count": row[5],
        "chain_root": row[6],
    }
