"""Tests for unified ContextIndex module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).

ContextIndex provides a single interface wrapping ALL index structures:
- Chain queries (leafUuid-based session linking)
- File queries (inverted index)
- Directory queries (file tree with bubble-up stats)
- Co-access queries (game trails)
- Temporal queries (weekly buckets)
- O(1) bloom filter checks

This is Phase 6 of Context OS Intelligence Layer.
"""

import sqlite3
import tempfile
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Optional

import pytest


# ============================================================================
# Test-specific dataclasses (simplified for testing)
# Prefixed with underscore to avoid pytest collection warnings
# ============================================================================

@dataclass
class _MockChainNode:
    """Simplified ChainNode for testing."""
    session_id: str
    parent_session_id: Optional[str] = None
    timestamp: Optional[datetime] = None
    message_uuids: List[str] = field(default_factory=list)


@dataclass
class _MockChain:
    """Simplified Chain for testing with nodes list."""
    chain_id: str
    root_session_id: str
    nodes: List[_MockChainNode] = field(default_factory=list)
    files_bloom: Optional[bytes] = None


# Aliases for cleaner test code
TestChainNode = _MockChainNode
TestChain = _MockChain


class TestContextIndexBuild:
    """Test building ContextIndex from data sources."""

    def test_build_from_empty_creates_empty_indexes(self):
        """Empty input should create empty indexes.

        RED: Run before implementation - should fail
        GREEN: Implement ContextIndex with empty handling
        """
        from context_os_events.index.context_index import ContextIndex

        # Build from empty (no JSONL data)
        index = ContextIndex()

        assert index.get_all_chains() == []
        assert index.get_sessions_for_file("/any/file.py") == []
        assert index.get_hot_directories() == []

    def test_build_with_chains(self):
        """ContextIndex should store chains from build."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()

        # Manually add chains for testing using test dataclasses
        chain = TestChain(
            chain_id="chain-1",
            root_session_id="session-a",
            nodes=[
                TestChainNode(
                    session_id="session-a",
                    parent_session_id=None,
                    timestamp=datetime(2025, 12, 10, 10, 0),
                    message_uuids=["msg-1"],
                ),
                TestChainNode(
                    session_id="session-b",
                    parent_session_id="session-a",
                    timestamp=datetime(2025, 12, 10, 11, 0),
                    message_uuids=["msg-2"],
                ),
            ],
        )
        index._chains = {"chain-1": chain}

        chains = index.get_all_chains()
        assert len(chains) == 1
        assert chains[0].chain_id == "chain-1"


class TestContextIndexChainQueries:
    """Test chain-related queries."""

    def test_get_chain_returns_chain(self):
        """get_chain should return Chain for valid ID.

        RED: Run before implementation - should fail
        GREEN: Implement get_chain()
        """
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._chains = {
            "chain-1": TestChain(
                chain_id="chain-1",
                root_session_id="session-a",
                nodes=[
                    TestChainNode(
                        session_id="session-a",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 10, 10, 0),
                        message_uuids=[],
                    )
                ],
            )
        }

        result = index.get_chain("chain-1")

        assert result is not None
        assert result.chain_id == "chain-1"

    def test_get_chain_not_found(self):
        """get_chain should return None for invalid ID."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._chains = {}

        result = index.get_chain("nonexistent")

        assert result is None

    def test_get_chain_for_session(self):
        """get_chain_for_session should return chain containing session."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._chains = {
            "chain-1": TestChain(
                chain_id="chain-1",
                root_session_id="session-a",
                nodes=[
                    TestChainNode(
                        session_id="session-a",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 10, 10, 0),
                        message_uuids=[],
                    ),
                    TestChainNode(
                        session_id="session-b",
                        parent_session_id="session-a",
                        timestamp=datetime(2025, 12, 10, 11, 0),
                        message_uuids=[],
                    ),
                ],
            )
        }

        result = index.get_chain_for_session("session-b")

        assert result == "chain-1"

    def test_get_chain_for_session_not_found(self):
        """get_chain_for_session should return None for orphan session."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._chains = {}

        result = index.get_chain_for_session("orphan-session")

        assert result is None

    def test_get_all_chains_sorted_by_recency(self):
        """get_all_chains should return chains sorted newest first."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._chains = {
            "chain-old": TestChain(
                chain_id="chain-old",
                root_session_id="s1",
                nodes=[
                    TestChainNode(
                        session_id="s1",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 1, 10, 0),
                        message_uuids=[],
                    )
                ],
            ),
            "chain-new": TestChain(
                chain_id="chain-new",
                root_session_id="s2",
                nodes=[
                    TestChainNode(
                        session_id="s2",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 15, 10, 0),
                        message_uuids=[],
                    )
                ],
            ),
        }

        chains = index.get_all_chains()

        assert len(chains) == 2
        # Newest first
        assert chains[0].chain_id == "chain-new"
        assert chains[1].chain_id == "chain-old"


class TestContextIndexFileQueries:
    """Test file-related queries."""

    def test_get_sessions_for_file(self):
        """get_sessions_for_file should return all sessions touching file.

        RED: Run before implementation - should fail
        GREEN: Implement get_sessions_for_file()
        """
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.inverted_index import FileAccess

        index = ContextIndex()
        index._inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="s1",
                    chain_id="c1",
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 10, 0),
                ),
                FileAccess(
                    session_id="s2",
                    chain_id="c1",
                    file_path="/src/main.py",
                    access_type="write",
                    tool_name="Write",
                    timestamp=datetime(2025, 12, 11, 10, 0),
                ),
            ]
        }

        result = index.get_sessions_for_file("/src/main.py")

        assert len(result) == 2
        session_ids = {a.session_id for a in result}
        assert session_ids == {"s1", "s2"}

    def test_get_sessions_for_file_not_found(self):
        """get_sessions_for_file should return empty list for unknown file."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._inverted_index = {}

        result = index.get_sessions_for_file("/unknown/file.py")

        assert result == []

    def test_get_files_for_session(self):
        """get_files_for_session should return all files touched by session."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.inverted_index import FileAccess

        ts = datetime(2025, 12, 10, 10, 0)
        index = ContextIndex()
        index._inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=ts,
                ),
            ],
            "/src/b.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=ts,
                ),
            ],
            "/src/c.py": [
                FileAccess(
                    session_id="s2",  # Different session
                    chain_id=None,
                    file_path="/src/c.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=ts,
                ),
            ],
        }

        result = index.get_files_for_session("s1")

        assert set(result) == {"/src/a.py", "/src/b.py"}

    def test_get_co_accessed(self):
        """get_co_accessed should return top N game trails."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._co_access = {
            "/src/main.py": [
                ("/src/utils.py", 0.8),
                ("/src/config.py", 0.6),
                ("/src/test.py", 0.4),
            ]
        }

        result = index.get_co_accessed("/src/main.py", limit=2)

        assert len(result) == 2
        assert result[0] == ("/src/utils.py", 0.8)
        assert result[1] == ("/src/config.py", 0.6)

    def test_get_co_accessed_not_found(self):
        """get_co_accessed should return empty list for unknown file."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._co_access = {}

        result = index.get_co_accessed("/unknown.py")

        assert result == []


class TestContextIndexDirectoryQueries:
    """Test directory-related queries."""

    def test_get_directory_stats(self):
        """get_directory_stats should return session/chain counts.

        RED: Run before implementation - should fail
        GREEN: Implement get_directory_stats()
        """
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.file_tree import FileTreeNode

        index = ContextIndex()
        # Build a simple tree
        root = FileTreeNode(name="", path="", is_directory=True)
        src = FileTreeNode(name="src", path="src", is_directory=True)
        src.session_count = 10
        src.chains = {"c1", "c2", "c3"}  # Use chains set for chain_count
        root.children["src"] = src
        index._file_tree = root

        result = index.get_directory_stats("src")

        assert result["session_count"] == 10
        assert result["chain_count"] == 3

    def test_get_directory_stats_not_found(self):
        """get_directory_stats should return empty dict for unknown path."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.file_tree import FileTreeNode

        index = ContextIndex()
        index._file_tree = FileTreeNode(name="", path="", is_directory=True)

        result = index.get_directory_stats("nonexistent")

        assert result == {}

    def test_get_hot_directories(self):
        """get_hot_directories should return top N active directories."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.file_tree import FileTreeNode

        index = ContextIndex()
        root = FileTreeNode(name="", path="", is_directory=True)

        # Create directories with different activity levels
        for name, count in [("src", 50), ("tests", 30), ("docs", 10)]:
            node = FileTreeNode(name=name, path=name, is_directory=True)
            node.session_count = count
            root.children[name] = node

        index._file_tree = root

        result = index.get_hot_directories(limit=2)

        assert len(result) == 2
        assert result[0] == ("src", 50)
        assert result[1] == ("tests", 30)


class TestContextIndexTemporalQueries:
    """Test temporal-related queries."""

    def test_get_week_summary(self):
        """get_week_summary should return bucket for ISO week.

        RED: Run before implementation - should fail
        GREEN: Implement get_week_summary()
        """
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.temporal import TemporalBucket

        index = ContextIndex()
        index._temporal = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1", "s2"},
                chains={"c1"},
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            )
        }

        result = index.get_week_summary("2025-W50")

        assert result is not None
        assert result.period == "2025-W50"
        assert len(result.sessions) == 2

    def test_get_week_summary_not_found(self):
        """get_week_summary should return None for unknown week."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._temporal = {}

        result = index.get_week_summary("2025-W99")

        assert result is None

    def test_get_recent_weeks(self):
        """get_recent_weeks should return N most recent weeks with activity."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.temporal import TemporalBucket

        index = ContextIndex()
        index._temporal = {
            "2025-W48": TemporalBucket(
                period="2025-W48",
                period_type="week",
                sessions={"s1"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 11, 25),
                ended_at=datetime(2025, 12, 1),
            ),
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s2"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            ),
            "2025-W49": TemporalBucket(
                period="2025-W49",
                period_type="week",
                sessions={"s3"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 2),
                ended_at=datetime(2025, 12, 8),
            ),
        }

        result = index.get_recent_weeks(count=2)

        assert len(result) == 2
        # Should be sorted by recency (W50, W49)
        assert result[0].period == "2025-W50"
        assert result[1].period == "2025-W49"

    def test_get_weeks_in_range(self):
        """get_weeks_in_range should return all weeks overlapping date range."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.temporal import TemporalBucket

        index = ContextIndex()
        index._temporal = {
            "2025-W49": TemporalBucket(
                period="2025-W49",
                period_type="week",
                sessions={"s1"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 2),
                ended_at=datetime(2025, 12, 8),
            ),
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s2"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            ),
            "2025-W51": TemporalBucket(
                period="2025-W51",
                period_type="week",
                sessions={"s3"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 16),
                ended_at=datetime(2025, 12, 22),
            ),
        }

        # Query Dec 5 - Dec 12 (overlaps W49 and W50)
        result = index.get_weeks_in_range(
            start=datetime(2025, 12, 5),
            end=datetime(2025, 12, 12),
        )

        assert len(result) == 2
        periods = {b.period for b in result}
        assert "2025-W49" in periods
        assert "2025-W50" in periods


class TestContextIndexBloomChecks:
    """Test O(1) bloom filter checks."""

    def test_chain_touched_file_bloom(self):
        """chain_touched_file should use bloom filter for O(1) check.

        RED: Run before implementation - should fail
        GREEN: Implement chain_touched_file()
        """
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.bloom import BloomFilter

        index = ContextIndex()

        # Create chain with bloom filter
        bloom = BloomFilter(expected_items=100, false_positive_rate=0.01)
        bloom.add("/src/main.py")
        bloom.add("/src/utils.py")

        chain = TestChain(
            chain_id="chain-1",
            root_session_id="s1",
            nodes=[
                TestChainNode(
                    session_id="s1",
                    parent_session_id=None,
                    timestamp=datetime(2025, 12, 10),
                    message_uuids=[],
                )
            ],
            files_bloom=bloom.serialize(),
        )
        index._chains = {"chain-1": chain}

        # File we added
        assert index.chain_touched_file("chain-1", "/src/main.py") is True

        # File we didn't add (should be False, no false negatives)
        assert index.chain_touched_file("chain-1", "/totally/different.py") is False

    def test_chain_touched_file_no_bloom(self):
        """chain_touched_file should return False if chain has no bloom."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        chain = TestChain(
            chain_id="chain-1",
            root_session_id="s1",
            nodes=[
                TestChainNode(
                    session_id="s1",
                    parent_session_id=None,
                    timestamp=datetime(2025, 12, 10),
                    message_uuids=[],
                )
            ],
            files_bloom=None,  # No bloom filter
        )
        index._chains = {"chain-1": chain}

        result = index.chain_touched_file("chain-1", "/src/main.py")

        assert result is False

    def test_file_touched_in_week_bloom(self):
        """file_touched_in_week should use bloom filter for O(1) check."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.temporal import TemporalBucket
        from context_os_events.index.bloom import BloomFilter

        index = ContextIndex()

        # Create bucket with bloom filter
        bloom = BloomFilter(expected_items=100, false_positive_rate=0.01)
        bloom.add("/src/main.py")

        index._temporal = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1"},
                chains=set(),
                files_bloom=bloom.serialize(),
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            )
        }

        # File we added
        assert index.file_touched_in_week("/src/main.py", "2025-W50") is True

        # File we didn't add
        assert index.file_touched_in_week("/other.py", "2025-W50") is False

    def test_file_touched_in_week_unknown_week(self):
        """file_touched_in_week should return False for unknown week."""
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()
        index._temporal = {}

        result = index.file_touched_in_week("/src/main.py", "2025-W99")

        assert result is False


class TestContextIndexPersistence:
    """Test persisting and loading ContextIndex."""

    def test_persist_creates_database(self):
        """persist should create SQLite database with all tables.

        RED: Run before implementation - should fail
        GREEN: Implement persist()
        """
        from context_os_events.index.context_index import ContextIndex

        index = ContextIndex()

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "context.db"

            stats = index.persist(db_path)

            assert db_path.exists()
            assert "chains" in stats
            assert "files" in stats
            assert "buckets" in stats

    def test_load_restores_index(self):
        """load should restore ContextIndex from database.

        RED: Run before implementation - should fail
        GREEN: Implement load()
        """
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.inverted_index import FileAccess

        original = ContextIndex()
        original._chains = {
            "chain-1": TestChain(
                chain_id="chain-1",
                root_session_id="s1",
                nodes=[
                    TestChainNode(
                        session_id="s1",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 10),
                        message_uuids=["msg-1"],
                    )
                ],
            )
        }
        original._inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="s1",
                    chain_id="chain-1",
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10),
                )
            ]
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "context.db"

            original.persist(db_path)
            loaded = ContextIndex.load(db_path)

            assert "chain-1" in loaded._chains
            assert "/src/main.py" in loaded._inverted_index

    def test_persist_and_load_roundtrip(self):
        """Index should survive persist -> load roundtrip."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.temporal import TemporalBucket

        original = ContextIndex()
        original._chains = {
            "chain-1": TestChain(
                chain_id="chain-1",
                root_session_id="s1",
                nodes=[
                    TestChainNode(
                        session_id="s1",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 10),
                        message_uuids=[],
                    )
                ],
            )
        }
        original._temporal = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1"},
                chains={"chain-1"},
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            )
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = Path(tmpdir) / "context.db"

            original.persist(db_path)
            loaded = ContextIndex.load(db_path)

            # Chains survived
            assert len(loaded._chains) == 1
            assert "chain-1" in loaded._chains

            # Temporal buckets survived
            assert len(loaded._temporal) == 1
            assert "2025-W50" in loaded._temporal


class TestContextIndexIntegration:
    """Integration tests with real index structures."""

    def test_cross_index_query(self):
        """Should be able to query across multiple index structures."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.inverted_index import FileAccess
        from context_os_events.index.temporal import TemporalBucket

        index = ContextIndex()

        # Set up chain
        index._chains = {
            "chain-1": TestChain(
                chain_id="chain-1",
                root_session_id="s1",
                nodes=[
                    TestChainNode(
                        session_id="s1",
                        parent_session_id=None,
                        timestamp=datetime(2025, 12, 10),
                        message_uuids=[],
                    )
                ],
            )
        }

        # Set up inverted index
        index._inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="s1",
                    chain_id="chain-1",
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10),
                )
            ]
        }

        # Set up temporal
        index._temporal = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1"},
                chains={"chain-1"},
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            )
        }

        # Cross-index queries
        # 1. Get chain for session
        chain_id = index.get_chain_for_session("s1")
        assert chain_id == "chain-1"

        # 2. Get files for that session
        files = index.get_files_for_session("s1")
        assert "/src/main.py" in files

        # 3. Get week for that session
        week = index.get_week_summary("2025-W50")
        assert "s1" in week.sessions


class TestGetAllChainsSorting:
    """Test get_all_chains() returns chains sorted by session count.

    Bug: Multi-session chains were buried after 600+ single-session chains
    because time_range was always None, making sort order effectively random.

    Fix: Sort by session_count (primary), recency (secondary).
    """

    def test_sorts_by_session_count_descending(self):
        """Chains with more sessions should appear first."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.chain_graph import Chain

        index = ContextIndex()

        # Add chains with different session counts
        chain_1_session = Chain(
            chain_id="small",
            root_session="s1",
            sessions=["s1"],
            branches={},
            time_range=None,
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )
        chain_5_sessions = Chain(
            chain_id="large",
            root_session="s2",
            sessions=["s2", "s3", "s4", "s5", "s6"],
            branches={},
            time_range=None,
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )
        chain_3_sessions = Chain(
            chain_id="medium",
            root_session="s7",
            sessions=["s7", "s8", "s9"],
            branches={},
            time_range=None,
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )

        # Add in random order
        index._chains["small"] = chain_1_session
        index._chains["large"] = chain_5_sessions
        index._chains["medium"] = chain_3_sessions

        result = index.get_all_chains()

        # Should be sorted by session count descending
        assert len(result) == 3
        assert result[0].chain_id == "large"   # 5 sessions
        assert result[1].chain_id == "medium"  # 3 sessions
        assert result[2].chain_id == "small"   # 1 session

    def test_secondary_sort_by_recency_when_same_session_count(self):
        """Chains with same session count should sort by recency."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.chain_graph import Chain

        index = ContextIndex()

        old_chain = Chain(
            chain_id="old",
            root_session="s1",
            sessions=["s1", "s2"],
            branches={},
            time_range=(datetime(2025, 1, 1), datetime(2025, 1, 2)),
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )
        new_chain = Chain(
            chain_id="new",
            root_session="s3",
            sessions=["s3", "s4"],
            branches={},
            time_range=(datetime(2025, 12, 1), datetime(2025, 12, 2)),
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )

        index._chains["old"] = old_chain
        index._chains["new"] = new_chain

        result = index.get_all_chains()

        # Same session count (2), so newer should come first
        assert result[0].chain_id == "new"
        assert result[1].chain_id == "old"

    def test_handles_none_time_range_gracefully(self):
        """Chains with None time_range should still sort by session count."""
        from context_os_events.index.context_index import ContextIndex
        from context_os_events.index.chain_graph import Chain

        index = ContextIndex()

        # All have None time_range (the bug scenario)
        chain_big = Chain(
            chain_id="big",
            root_session="s1",
            sessions=["s1", "s2", "s3"],
            branches={},
            time_range=None,
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )
        chain_small = Chain(
            chain_id="small",
            root_session="s4",
            sessions=["s4"],
            branches={},
            time_range=None,
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )

        index._chains["big"] = chain_big
        index._chains["small"] = chain_small

        result = index.get_all_chains()

        # big (3 sessions) should come before small (1 session)
        assert result[0].chain_id == "big"
        assert result[1].chain_id == "small"
