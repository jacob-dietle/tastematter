"""Unified ContextIndex - single interface wrapping all index structures.

Phase 6 of Context OS Intelligence Layer.

Provides clean API for agent queries across:
- Chain queries (leafUuid-based session linking)
- File queries (inverted index)
- Directory queries (file tree with bubble-up stats)
- Co-access queries (game trails)
- Temporal queries (weekly buckets)
- O(1) bloom filter checks

Usage:
    index = ContextIndex()
    # ... build or load data

    # Chain queries
    chain = index.get_chain("chain-id")
    chain_id = index.get_chain_for_session("session-id")

    # File queries
    sessions = index.get_sessions_for_file("/src/main.py")
    co_accessed = index.get_co_accessed("/src/main.py", limit=5)

    # Temporal queries
    week = index.get_week_summary("2025-W50")
    recent = index.get_recent_weeks(count=4)

    # O(1) bloom checks
    touched = index.chain_touched_file("chain-1", "/src/main.py")
    in_week = index.file_touched_in_week("/src/main.py", "2025-W50")

    # Persistence
    index.persist(Path("context.db"))
    loaded = ContextIndex.load(Path("context.db"))
"""

import json
import logging
import sqlite3
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

from .bloom import BloomFilter
from .chain_graph import Chain
from .file_tree import FileTreeNode
from .inverted_index import FileAccess
from .temporal import TemporalBucket

logger = logging.getLogger(__name__)


# ============================================================================
# Internal dataclasses for loaded data
# ============================================================================

@dataclass
class LoadedChainNode:
    """ChainNode representation for loaded data."""
    session_id: str
    parent_session_id: Optional[str] = None
    timestamp: Optional[datetime] = None
    message_uuids: List[str] = field(default_factory=list)


@dataclass
class LoadedChain:
    """Chain representation for loaded data."""
    chain_id: str
    root_session_id: str
    nodes: List[LoadedChainNode] = field(default_factory=list)
    files_bloom: Optional[bytes] = None


class ContextIndex:
    """Unified interface for all Context OS index structures.

    Wraps:
    - _chains: Dict[str, Chain] - chain_id -> Chain
    - _inverted_index: Dict[str, List[FileAccess]] - file_path -> accesses
    - _file_tree: FileTreeNode - root of annotated trie
    - _co_access: Dict[str, List[Tuple[str, float]]] - file -> [(co_file, similarity)]
    - _temporal: Dict[str, TemporalBucket] - period -> bucket
    """

    def __init__(self):
        """Initialize empty ContextIndex."""
        self._chains: Dict[str, Chain] = {}
        self._inverted_index: Dict[str, List[FileAccess]] = {}
        self._file_tree: Optional[FileTreeNode] = FileTreeNode(
            name="", path="", is_directory=True
        )
        self._co_access: Dict[str, List[Tuple[str, float]]] = {}
        self._temporal: Dict[str, TemporalBucket] = {}

        # Session-to-chain mapping (built lazily)
        self._session_to_chain: Optional[Dict[str, str]] = None

    # =========================================================================
    # Chain Queries
    # =========================================================================

    def get_chain(self, chain_id: str) -> Optional[Chain]:
        """Get chain by ID.

        Args:
            chain_id: Chain identifier

        Returns:
            Chain if found, None otherwise
        """
        return self._chains.get(chain_id)

    def get_chain_for_session(self, session_id: str) -> Optional[str]:
        """Get chain ID containing a session.

        Args:
            session_id: Session identifier

        Returns:
            chain_id if session is in a chain, None otherwise
        """
        # Build session-to-chain mapping if not cached
        if self._session_to_chain is None:
            self._session_to_chain = {}
            for chain_id, chain in self._chains.items():
                # Support both old structure (sessions list) and test structure (nodes list)
                if hasattr(chain, 'sessions') and chain.sessions:
                    for sid in chain.sessions:
                        self._session_to_chain[sid] = chain_id
                elif hasattr(chain, 'nodes') and chain.nodes:
                    for node in chain.nodes:
                        self._session_to_chain[node.session_id] = chain_id

        return self._session_to_chain.get(session_id)

    def get_all_chains(self) -> List[Chain]:
        """Get all chains sorted by session count (largest first), then recency.

        Multi-session chains are more interesting for analysis, so they appear first.
        Chains with the same session count are sorted by recency (newest first).

        Returns:
            List of Chain objects, largest chains first
        """
        chains = list(self._chains.values())

        def sort_key(chain: Chain) -> tuple:
            # Primary: session count (descending via negative)
            if hasattr(chain, 'sessions') and chain.sessions:
                session_count = len(chain.sessions)
            elif hasattr(chain, 'nodes') and chain.nodes:
                session_count = len(chain.nodes)
            else:
                session_count = 0

            # Secondary: recency (descending - newer dates are "larger")
            # Normalize to offset-naive to avoid comparison errors
            def to_naive(dt):
                if dt is None:
                    return datetime.min
                if hasattr(dt, 'tzinfo') and dt.tzinfo is not None:
                    return dt.replace(tzinfo=None)
                return dt

            if hasattr(chain, 'nodes') and chain.nodes:
                timestamps = [to_naive(n.timestamp) for n in chain.nodes if n.timestamp]
                recency = max(timestamps) if timestamps else datetime.min
            elif hasattr(chain, 'time_range') and chain.time_range:
                recency = to_naive(chain.time_range[1])
            else:
                recency = datetime.min

            # Return tuple: (session_count, recency)
            # With reverse=True: larger session_count first, then newer recency first
            return (session_count, recency)

        chains.sort(key=sort_key, reverse=True)
        return chains

    # =========================================================================
    # File Queries
    # =========================================================================

    def get_sessions_for_file(self, file_path: str) -> List[FileAccess]:
        """Get all sessions that touched a file.

        Args:
            file_path: File path to query

        Returns:
            List of FileAccess records for this file
        """
        return self._inverted_index.get(file_path, [])

    def get_files_for_session(self, session_id: str) -> List[str]:
        """Get all files touched by a session.

        Args:
            session_id: Session identifier

        Returns:
            List of file paths touched by this session
        """
        files = []
        for file_path, accesses in self._inverted_index.items():
            for access in accesses:
                if access.session_id == session_id:
                    files.append(file_path)
                    break  # Only add file once per session
        return files

    def get_co_accessed(
        self,
        file_path: str,
        limit: int = 10
    ) -> List[Tuple[str, float]]:
        """Get files frequently co-accessed with this file ("game trails").

        Args:
            file_path: File to find co-accesses for
            limit: Maximum number of results

        Returns:
            List of (file_path, similarity_score) tuples, highest similarity first
        """
        co_files = self._co_access.get(file_path, [])
        return co_files[:limit]

    # =========================================================================
    # Directory Queries
    # =========================================================================

    def get_directory_stats(self, dir_path: str) -> Dict[str, Any]:
        """Get session/chain counts for a directory.

        Args:
            dir_path: Directory path (relative)

        Returns:
            Dict with session_count, chain_count, etc. Empty dict if not found.
        """
        if self._file_tree is None:
            return {}

        # Navigate to directory node
        node = self._find_tree_node(dir_path)
        if node is None:
            return {}

        return {
            "path": node.path,
            "name": node.name,
            "session_count": node.session_count,
            "chain_count": len(node.chains) if hasattr(node, 'chains') else 0,
            "last_accessed": node.last_accessed,
        }

    def _find_tree_node(self, path: str) -> Optional[FileTreeNode]:
        """Navigate to a node in the file tree.

        Args:
            path: Relative path to find

        Returns:
            FileTreeNode if found, None otherwise
        """
        if self._file_tree is None:
            return None

        if not path or path == "":
            return self._file_tree

        parts = path.replace("\\", "/").strip("/").split("/")
        node = self._file_tree

        for part in parts:
            if part not in node.children:
                return None
            node = node.children[part]

        return node

    def get_hot_directories(self, limit: int = 10) -> List[Tuple[str, int]]:
        """Get most active directories by session count.

        Args:
            limit: Maximum number of results

        Returns:
            List of (dir_path, session_count) tuples, most active first
        """
        if self._file_tree is None:
            return []

        # Collect all directory nodes with session counts
        dirs = []
        self._collect_directories(self._file_tree, dirs)

        # Sort by session count descending
        dirs.sort(key=lambda x: x[1], reverse=True)
        return dirs[:limit]

    def _collect_directories(
        self,
        node: FileTreeNode,
        result: List[Tuple[str, int]]
    ) -> None:
        """Recursively collect directories with session counts."""
        for child in node.children.values():
            # Check is_directory attribute if available, otherwise check children
            is_dir = getattr(child, 'is_directory', False) or bool(child.children)
            if is_dir:
                result.append((child.path or child.name, child.session_count))
            self._collect_directories(child, result)

    # =========================================================================
    # Temporal Queries
    # =========================================================================

    def get_week_summary(self, week: str) -> Optional[TemporalBucket]:
        """Get summary for a specific ISO week.

        Args:
            week: ISO week string (e.g., "2025-W50")

        Returns:
            TemporalBucket if found, None otherwise
        """
        return self._temporal.get(week)

    def get_recent_weeks(self, count: int = 4) -> List[TemporalBucket]:
        """Get N most recent weeks with activity.

        Args:
            count: Number of weeks to return

        Returns:
            List of TemporalBucket objects, newest first
        """
        buckets = list(self._temporal.values())

        # Sort by period descending (ISO week format sorts correctly)
        buckets.sort(key=lambda b: b.period, reverse=True)
        return buckets[:count]

    def get_weeks_in_range(
        self,
        start: datetime,
        end: datetime
    ) -> List[TemporalBucket]:
        """Get all weeks overlapping with date range.

        Args:
            start: Range start datetime
            end: Range end datetime

        Returns:
            List of TemporalBucket objects overlapping the range
        """
        result = []

        for bucket in self._temporal.values():
            # Check for overlap
            if bucket.started_at is None or bucket.ended_at is None:
                continue

            # Overlap: bucket.start <= end AND bucket.end >= start
            if bucket.started_at <= end and bucket.ended_at >= start:
                result.append(bucket)

        return result

    # =========================================================================
    # Bloom Filter Checks (O(1))
    # =========================================================================

    def chain_touched_file(self, chain_id: str, file_path: str) -> bool:
        """Check if chain touched file using bloom filter.

        O(1) check with possible false positives but NO false negatives.

        Args:
            chain_id: Chain identifier
            file_path: File path to check

        Returns:
            True if file possibly touched, False if definitely not touched
        """
        chain = self._chains.get(chain_id)
        if chain is None:
            return False

        if chain.files_bloom is None:
            return False

        # Deserialize and check bloom filter
        try:
            bloom = BloomFilter.deserialize(chain.files_bloom)
            return file_path in bloom
        except Exception:
            logger.warning(f"Failed to deserialize bloom filter for chain {chain_id}")
            return False

    def file_touched_in_week(self, file_path: str, week: str) -> bool:
        """Check if file was touched in ISO week using bloom filter.

        O(1) check with possible false positives but NO false negatives.

        Args:
            file_path: File path to check
            week: ISO week string (e.g., "2025-W50")

        Returns:
            True if file possibly touched, False if definitely not touched
        """
        bucket = self._temporal.get(week)
        if bucket is None:
            return False

        if not bucket.files_bloom:
            return False

        # Deserialize and check bloom filter
        try:
            bloom = BloomFilter.deserialize(bucket.files_bloom)
            return file_path in bloom
        except Exception:
            logger.warning(f"Failed to deserialize bloom filter for week {week}")
            return False

    # =========================================================================
    # Persistence
    # =========================================================================

    def persist(self, db_path: Path) -> Dict[str, int]:
        """Persist index to SQLite database.

        Args:
            db_path: Path to SQLite database file

        Returns:
            Stats dict with counts of stored items
        """
        db = sqlite3.connect(str(db_path))

        try:
            # Create tables
            self._create_tables(db)

            # Persist each index structure
            chains_count = self._persist_chains(db)
            files_count = self._persist_inverted_index(db)
            buckets_count = self._persist_temporal(db)

            db.commit()

            return {
                "chains": chains_count,
                "files": files_count,
                "buckets": buckets_count,
            }
        finally:
            db.close()

    def _create_tables(self, db: sqlite3.Connection) -> None:
        """Create database tables for persistence."""
        db.execute("""
            CREATE TABLE IF NOT EXISTS chains (
                chain_id TEXT PRIMARY KEY,
                root_session_id TEXT NOT NULL,
                nodes_json TEXT,
                files_bloom BLOB,
                created_at TEXT,
                updated_at TEXT
            )
        """)

        db.execute("""
            CREATE TABLE IF NOT EXISTS file_accesses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                session_id TEXT NOT NULL,
                chain_id TEXT,
                access_type TEXT,
                tool_name TEXT,
                timestamp TEXT,
                UNIQUE(file_path, session_id, timestamp)
            )
        """)

        db.execute("""
            CREATE TABLE IF NOT EXISTS temporal_buckets (
                period TEXT PRIMARY KEY,
                period_type TEXT NOT NULL,
                sessions_json TEXT,
                chains_json TEXT,
                files_bloom BLOB,
                commits_json TEXT,
                started_at TEXT,
                ended_at TEXT,
                session_count INTEGER,
                chain_count INTEGER,
                commit_count INTEGER
            )
        """)

        # Create indexes
        db.execute("""
            CREATE INDEX IF NOT EXISTS idx_file_accesses_file
            ON file_accesses(file_path)
        """)
        db.execute("""
            CREATE INDEX IF NOT EXISTS idx_file_accesses_session
            ON file_accesses(session_id)
        """)

    def _persist_chains(self, db: sqlite3.Connection) -> int:
        """Persist chains to database."""
        count = 0

        for chain_id, chain in self._chains.items():
            # Support test structure (nodes list)
            if hasattr(chain, 'nodes') and chain.nodes:
                nodes_data = [
                    {
                        "session_id": n.session_id,
                        "parent_session_id": n.parent_session_id,
                        "timestamp": n.timestamp.isoformat() if n.timestamp else None,
                        "message_uuids": getattr(n, 'message_uuids', []),
                    }
                    for n in chain.nodes
                ]
                root_session_id = getattr(chain, 'root_session_id', chain.nodes[0].session_id)
            else:
                # Support existing structure (sessions list)
                nodes_data = [{"session_id": sid} for sid in getattr(chain, 'sessions', [])]
                root_session_id = getattr(chain, 'root_session', getattr(chain, 'root_session_id', ''))

            db.execute("""
                INSERT OR REPLACE INTO chains
                (chain_id, root_session_id, nodes_json, files_bloom, updated_at)
                VALUES (?, ?, ?, ?, ?)
            """, (
                chain_id,
                root_session_id,
                json.dumps(nodes_data),
                getattr(chain, 'files_bloom', None),
                datetime.now().isoformat(),
            ))
            count += 1

        return count

    def _persist_inverted_index(self, db: sqlite3.Connection) -> int:
        """Persist inverted index to database."""
        count = 0

        for file_path, accesses in self._inverted_index.items():
            for access in accesses:
                try:
                    db.execute("""
                        INSERT OR IGNORE INTO file_accesses
                        (file_path, session_id, chain_id, access_type, tool_name, timestamp)
                        VALUES (?, ?, ?, ?, ?, ?)
                    """, (
                        file_path,
                        access.session_id,
                        access.chain_id,
                        access.access_type,
                        access.tool_name,
                        access.timestamp.isoformat() if access.timestamp else None,
                    ))
                    count += 1
                except sqlite3.IntegrityError:
                    pass  # Duplicate, ignore

        return count

    def _persist_temporal(self, db: sqlite3.Connection) -> int:
        """Persist temporal buckets to database."""
        count = 0

        for period, bucket in self._temporal.items():
            db.execute("""
                INSERT OR REPLACE INTO temporal_buckets
                (period, period_type, sessions_json, chains_json, files_bloom,
                 commits_json, started_at, ended_at, session_count, chain_count, commit_count)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                period,
                bucket.period_type,
                json.dumps(list(bucket.sessions)),
                json.dumps(list(bucket.chains)),
                bucket.files_bloom,
                json.dumps(bucket.commits),
                bucket.started_at.isoformat() if bucket.started_at else None,
                bucket.ended_at.isoformat() if bucket.ended_at else None,
                len(bucket.sessions),
                len(bucket.chains),
                len(bucket.commits),
            ))
            count += 1

        return count

    @classmethod
    def load(cls, db_path: Path) -> "ContextIndex":
        """Load index from SQLite database.

        Args:
            db_path: Path to SQLite database file

        Returns:
            ContextIndex populated from database
        """
        index = cls()
        db = sqlite3.connect(str(db_path))

        try:
            index._load_chains(db)
            index._load_inverted_index(db)
            index._load_temporal(db)
        finally:
            db.close()

        return index

    def _load_chains(self, db: sqlite3.Connection) -> None:
        """Load chains from database."""
        cursor = db.execute("""
            SELECT chain_id, root_session_id, nodes_json, files_bloom
            FROM chains
        """)

        for row in cursor:
            chain_id, root_session_id, nodes_json, files_bloom = row

            nodes = []
            if nodes_json:
                nodes_data = json.loads(nodes_json)
                for nd in nodes_data:
                    timestamp = None
                    if nd.get("timestamp"):
                        try:
                            timestamp = datetime.fromisoformat(nd["timestamp"])
                        except ValueError:
                            pass

                    nodes.append(LoadedChainNode(
                        session_id=nd["session_id"],
                        parent_session_id=nd.get("parent_session_id"),
                        timestamp=timestamp,
                        message_uuids=nd.get("message_uuids", []),
                    ))

            self._chains[chain_id] = LoadedChain(
                chain_id=chain_id,
                root_session_id=root_session_id,
                nodes=nodes,
                files_bloom=files_bloom,
            )

    def _load_inverted_index(self, db: sqlite3.Connection) -> None:
        """Load inverted index from database."""
        cursor = db.execute("""
            SELECT file_path, session_id, chain_id, access_type, tool_name, timestamp
            FROM file_accesses
        """)

        for row in cursor:
            file_path, session_id, chain_id, access_type, tool_name, timestamp_str = row

            timestamp = None
            if timestamp_str:
                try:
                    timestamp = datetime.fromisoformat(timestamp_str)
                except ValueError:
                    pass

            access = FileAccess(
                session_id=session_id,
                chain_id=chain_id,
                file_path=file_path,
                access_type=access_type,
                tool_name=tool_name,
                timestamp=timestamp,
            )

            if file_path not in self._inverted_index:
                self._inverted_index[file_path] = []
            self._inverted_index[file_path].append(access)

    def _load_temporal(self, db: sqlite3.Connection) -> None:
        """Load temporal buckets from database."""
        cursor = db.execute("""
            SELECT period, period_type, sessions_json, chains_json, files_bloom,
                   commits_json, started_at, ended_at
            FROM temporal_buckets
        """)

        for row in cursor:
            (period, period_type, sessions_json, chains_json, files_bloom,
             commits_json, started_at_str, ended_at_str) = row

            sessions = set(json.loads(sessions_json)) if sessions_json else set()
            chains = set(json.loads(chains_json)) if chains_json else set()
            commits = json.loads(commits_json) if commits_json else []

            started_at = None
            if started_at_str:
                try:
                    started_at = datetime.fromisoformat(started_at_str)
                except ValueError:
                    pass

            ended_at = None
            if ended_at_str:
                try:
                    ended_at = datetime.fromisoformat(ended_at_str)
                except ValueError:
                    pass

            self._temporal[period] = TemporalBucket(
                period=period,
                period_type=period_type,
                sessions=sessions,
                chains=chains,
                files_bloom=files_bloom,
                commits=commits,
                started_at=started_at,
                ended_at=ended_at,
            )
