"""File tree index for Context OS Intelligence.

Builds an annotated trie (tree) from the inverted file index.
Each node tracks which sessions/chains touched it or its descendants.

Purpose: "Fog of war" navigation - click deeper, see more context.

Algorithm:
1. Create tree from project directory structure
2. Annotate each node with sessions/chains that touched it
3. Bubble up stats from children to parents
"""

import json
import logging
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

from .inverted_index import FileAccess

logger = logging.getLogger(__name__)


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class FileTreeNode:
    """Node in the annotated file tree.

    Can represent either a file or directory.
    Directories have children, files have access_history.
    """
    path: str                                     # Relative path (forward slashes)
    name: str                                     # Just the filename/dirname
    is_directory: bool
    chains: Set[str] = field(default_factory=set)     # Chains that touched this or children
    sessions: Set[str] = field(default_factory=set)   # Sessions that touched this or children
    session_count: int = 0                             # Cached len(sessions)
    last_accessed: Optional[datetime] = None
    children: Dict[str, 'FileTreeNode'] = field(default_factory=dict)  # For directories
    access_history: List[FileAccess] = field(default_factory=list)     # For files


# ============================================================================
# Path Normalization
# ============================================================================

def normalize_path(path: str, project_root: Optional[Path] = None) -> str:
    """Convert path to normalized relative format.

    - Converts Windows backslashes to forward slashes
    - Removes project root prefix if provided
    - Strips leading slashes

    Args:
        path: Path to normalize
        project_root: Optional project root to strip

    Returns:
        Normalized relative path with forward slashes
    """
    # Convert Windows to Unix separators
    path = path.replace('\\', '/')

    # Strip project root if provided
    if project_root:
        root_str = str(project_root).replace('\\', '/')
        if path.startswith(root_str):
            path = path[len(root_str):]

    # Strip leading slashes
    path = path.lstrip('/')

    return path


# ============================================================================
# Tree Building
# ============================================================================

def _ensure_path_exists(root: FileTreeNode, file_path: str) -> FileTreeNode:
    """Navigate/create path from root to leaf, return leaf node.

    Creates intermediate directory nodes as needed.

    Args:
        root: Root node of tree
        file_path: Normalized file path (forward slashes)

    Returns:
        The leaf node (file) at the given path
    """
    # Handle empty path
    if not file_path:
        return root

    parts = file_path.split('/')
    current = root

    for i, part in enumerate(parts):
        if not part:  # Skip empty parts
            continue

        if part not in current.children:
            is_dir = i < len(parts) - 1  # All but last are directories
            current.children[part] = FileTreeNode(
                path='/'.join(parts[:i+1]),
                name=part,
                is_directory=is_dir,
                chains=set(),
                sessions=set(),
                session_count=0,
                last_accessed=None,
                children={},
                access_history=[],
            )
        current = current.children[part]

    return current


def bubble_up_stats(node: FileTreeNode) -> None:
    """Recursively bubble up stats from children to parents.

    Post-order traversal: process children first, then update self.

    Updates:
    - chains: union of all child chains
    - sessions: union of all child sessions
    - session_count: len(sessions)
    - last_accessed: max of child last_accessed

    Args:
        node: Node to process (and all descendants)
    """
    # First, recursively process all children
    for child in node.children.values():
        bubble_up_stats(child)

        # Merge child stats into this node
        node.chains.update(child.chains)
        node.sessions.update(child.sessions)

        # Update last_accessed to max
        if child.last_accessed:
            if node.last_accessed is None or child.last_accessed > node.last_accessed:
                node.last_accessed = child.last_accessed

    # Update session count
    node.session_count = len(node.sessions)


def build_file_tree(
    inverted_index: Dict[str, List[FileAccess]],
    project_root: Optional[Path] = None
) -> FileTreeNode:
    """Build annotated file tree from inverted index.

    Args:
        inverted_index: Dict mapping file_path -> List[FileAccess]
        project_root: Optional project root for path normalization

    Returns:
        Root FileTreeNode with all files and stats bubbled up
    """
    # Create root node
    root = FileTreeNode(
        path="",
        name="",
        is_directory=True,
        chains=set(),
        sessions=set(),
        session_count=0,
        last_accessed=None,
        children={},
        access_history=[],
    )

    # Add each file from inverted index
    for file_path, accesses in inverted_index.items():
        # Normalize path
        normalized = normalize_path(file_path, project_root)

        if not normalized:
            continue

        # Navigate/create path to file
        file_node = _ensure_path_exists(root, normalized)

        # Store access history
        file_node.access_history = accesses

        # Extract sessions and chains from accesses
        for access in accesses:
            file_node.sessions.add(access.session_id)
            if access.chain_id:
                file_node.chains.add(access.chain_id)

            # Update last_accessed
            if access.timestamp:
                if file_node.last_accessed is None or access.timestamp > file_node.last_accessed:
                    file_node.last_accessed = access.timestamp

        # Update session count for file
        file_node.session_count = len(file_node.sessions)

    # Bubble up stats from leaves to root
    bubble_up_stats(root)

    return root


# ============================================================================
# Query Functions
# ============================================================================

def get_node_by_path(root: FileTreeNode, path: str) -> Optional[FileTreeNode]:
    """Navigate to a specific node by path.

    Args:
        root: Root node of tree
        path: Path to navigate to (forward slashes)

    Returns:
        Node at path, or None if not found
    """
    if not path:
        return root

    # Normalize path
    path = path.replace('\\', '/').strip('/')

    parts = path.split('/')
    current = root

    for part in parts:
        if not part:
            continue

        if part not in current.children:
            return None

        current = current.children[part]

    return current


def get_children(node: FileTreeNode) -> List[FileTreeNode]:
    """Get immediate children of a directory node.

    Args:
        node: Directory node

    Returns:
        List of child nodes
    """
    return list(node.children.values())


def get_directory_stats(node: FileTreeNode) -> Dict[str, Any]:
    """Get aggregated stats for a directory.

    Args:
        node: Directory node

    Returns:
        Dict with session_count, chain_count, file_count, last_accessed
    """
    def count_files(n: FileTreeNode) -> int:
        """Recursively count files under a node."""
        if not n.is_directory:
            return 1

        return sum(count_files(child) for child in n.children.values())

    return {
        "session_count": node.session_count,
        "chain_count": len(node.chains),
        "file_count": count_files(node),
        "last_accessed": node.last_accessed,
        "path": node.path,
    }


# ============================================================================
# Database Persistence
# ============================================================================

def persist_file_tree(db, root: FileTreeNode) -> Dict[str, int]:
    """Persist file tree to database.

    Writes to file_tree table.

    Args:
        db: SQLite connection
        root: Root node of tree

    Returns:
        Stats dict: {"nodes_stored": N}
    """
    nodes_stored = 0

    def persist_node(node: FileTreeNode, parent_path: Optional[str], depth: int):
        nonlocal nodes_stored

        # Serialize sets to JSON
        chains_json = json.dumps(list(node.chains))
        sessions_json = json.dumps(list(node.sessions))
        last_accessed_str = node.last_accessed.isoformat() if node.last_accessed else None

        db.execute("""
            INSERT OR REPLACE INTO file_tree (
                path, name, is_directory, parent_path,
                chains_json, sessions_json, session_count,
                last_accessed, depth
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            node.path,
            node.name,
            node.is_directory,
            parent_path,
            chains_json,
            sessions_json,
            node.session_count,
            last_accessed_str,
            depth,
        ))
        nodes_stored += 1

        # Recursively persist children
        for child in node.children.values():
            persist_node(child, node.path, depth + 1)

    # Start from root
    persist_node(root, None, 0)
    db.commit()

    return {"nodes_stored": nodes_stored}


def load_file_tree(db) -> FileTreeNode:
    """Load file tree from database.

    Args:
        db: SQLite connection

    Returns:
        Root FileTreeNode
    """
    # Load all nodes ordered by depth (root first)
    cursor = db.execute("""
        SELECT path, name, is_directory, parent_path,
               chains_json, sessions_json, session_count,
               last_accessed, depth
        FROM file_tree
        ORDER BY depth ASC
    """)

    # Build node lookup
    nodes: Dict[str, FileTreeNode] = {}
    root: Optional[FileTreeNode] = None

    for row in cursor.fetchall():
        path = row[0]
        name = row[1]
        is_directory = bool(row[2])
        parent_path = row[3]
        chains_json = row[4]
        sessions_json = row[5]
        session_count = row[6] or 0
        last_accessed_str = row[7]
        # depth = row[8]

        # Parse JSON fields
        chains = set(json.loads(chains_json)) if chains_json else set()
        sessions = set(json.loads(sessions_json)) if sessions_json else set()

        # Parse timestamp
        if last_accessed_str:
            try:
                last_accessed = datetime.fromisoformat(last_accessed_str)
            except ValueError:
                last_accessed = None
        else:
            last_accessed = None

        # Create node
        node = FileTreeNode(
            path=path,
            name=name,
            is_directory=is_directory,
            chains=chains,
            sessions=sessions,
            session_count=session_count,
            last_accessed=last_accessed,
            children={},
            access_history=[],  # Not persisted
        )

        nodes[path] = node

        # Link to parent
        # Root node has path="" and parent_path=None
        # Top-level dirs have parent_path="" (parent is root)
        if path == "":
            # This IS the root node
            root = node
        elif parent_path is None:
            # Also root (parent_path not set)
            root = node
        elif parent_path == "":
            # Parent is root - add as child of root
            if "" in nodes:
                nodes[""].children[name] = node
        elif parent_path in nodes:
            nodes[parent_path].children[name] = node

    # Return root (or empty root if nothing loaded)
    if root is None:
        root = FileTreeNode(
            path="",
            name="",
            is_directory=True,
        )

    return root
