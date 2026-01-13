"""Tests for file_tree module.

TDD Phase 3: File Tree Index (Annotated Trie)
Purpose: Hierarchical navigation with "fog of war" - session/chain stats at each node

Test categories:
1. FileTreeNode dataclass
2. Build file tree from inverted index
3. Bubble up stats from leaves to parents
4. Query functions
5. Database persistence
6. Real-world scenarios (Windows paths, absolute paths)
"""

import json
import sqlite3
import tempfile
from datetime import datetime
from pathlib import Path

import pytest

from context_os_events.index.file_tree import (
    FileTreeNode,
    build_file_tree,
    bubble_up_stats,
    get_node_by_path,
    get_children,
    get_directory_stats,
    normalize_path,
    persist_file_tree,
    load_file_tree,
)
from context_os_events.index.inverted_index import FileAccess


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def sample_file_access():
    """Create a sample FileAccess for testing."""
    return FileAccess(
        session_id="session-001",
        chain_id="chain-abc",
        file_path="src/main.py",
        access_type="read",
        tool_name="Read",
        timestamp=datetime(2025, 12, 16, 10, 0, 0),
        access_count=1,
    )


@pytest.fixture
def sample_inverted_index():
    """Create a sample inverted index for testing."""
    return {
        "src/main.py": [
            FileAccess(
                session_id="session-001",
                chain_id="chain-abc",
                file_path="src/main.py",
                access_type="read",
                tool_name="Read",
                timestamp=datetime(2025, 12, 16, 10, 0, 0),
                access_count=1,
            ),
        ],
        "src/utils/helpers.py": [
            FileAccess(
                session_id="session-001",
                chain_id="chain-abc",
                file_path="src/utils/helpers.py",
                access_type="write",
                tool_name="Edit",
                timestamp=datetime(2025, 12, 16, 11, 0, 0),
                access_count=2,
            ),
            FileAccess(
                session_id="session-002",
                chain_id="chain-def",
                file_path="src/utils/helpers.py",
                access_type="read",
                tool_name="Read",
                timestamp=datetime(2025, 12, 16, 12, 0, 0),
                access_count=1,
            ),
        ],
        "tests/test_main.py": [
            FileAccess(
                session_id="session-002",
                chain_id="chain-def",
                file_path="tests/test_main.py",
                access_type="create",
                tool_name="Write",
                timestamp=datetime(2025, 12, 16, 13, 0, 0),
                access_count=1,
            ),
        ],
    }


@pytest.fixture
def temp_db():
    """Create a temporary SQLite database for persistence tests."""
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
        db_path = f.name

    conn = sqlite3.connect(db_path)

    # Create file_tree table
    conn.execute("""
        CREATE TABLE file_tree (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            is_directory BOOLEAN NOT NULL,
            parent_path TEXT,
            chains_json TEXT,
            sessions_json TEXT,
            session_count INTEGER,
            last_accessed TEXT,
            depth INTEGER
        )
    """)
    conn.commit()

    yield conn

    conn.close()
    Path(db_path).unlink(missing_ok=True)


# ============================================================================
# Test: FileTreeNode Dataclass
# ============================================================================

class TestFileTreeNode:
    """Tests for FileTreeNode dataclass."""

    def test_file_tree_node_has_required_fields(self):
        """FileTreeNode should have all required fields."""
        node = FileTreeNode(
            path="src/main.py",
            name="main.py",
            is_directory=False,
            chains={"chain-abc"},
            sessions={"session-001"},
            session_count=1,
            last_accessed=datetime(2025, 12, 16, 10, 0, 0),
            children={},
            access_history=[],
        )

        assert node.path == "src/main.py"
        assert node.name == "main.py"
        assert node.is_directory is False
        assert "chain-abc" in node.chains
        assert "session-001" in node.sessions
        assert node.session_count == 1
        assert node.last_accessed == datetime(2025, 12, 16, 10, 0, 0)
        assert node.children == {}
        assert node.access_history == []

    def test_file_tree_node_defaults(self):
        """FileTreeNode should have sensible defaults for optional fields."""
        node = FileTreeNode(
            path="",
            name="",
            is_directory=True,
        )

        assert node.chains == set() or node.chains is not None
        assert node.sessions == set() or node.sessions is not None
        assert node.session_count == 0
        assert node.last_accessed is None
        assert node.children == {}
        assert node.access_history == []


# ============================================================================
# Test: Build File Tree
# ============================================================================

class TestBuildFileTree:
    """Tests for building file tree from inverted index."""

    def test_build_empty_tree_from_empty_index(self):
        """Empty inverted index should return empty root node."""
        tree = build_file_tree({})

        assert tree.path == ""
        assert tree.name == ""
        assert tree.is_directory is True
        assert tree.session_count == 0
        assert len(tree.children) == 0

    def test_build_tree_single_file(self):
        """Single file should create path from root to leaf."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id="chain-abc",
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # Root should have 'src' directory
        assert "src" in tree.children
        src_node = tree.children["src"]
        assert src_node.is_directory is True
        assert src_node.path == "src"

        # 'src' should have 'main.py' file
        assert "main.py" in src_node.children
        main_node = src_node.children["main.py"]
        assert main_node.is_directory is False
        assert main_node.path == "src/main.py"
        assert "session-001" in main_node.sessions
        assert "chain-abc" in main_node.chains

    def test_build_tree_multiple_files_same_dir(self):
        """Multiple files in same directory should share parent."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
            "src/utils.py": [
                FileAccess(
                    session_id="session-002",
                    chain_id=None,
                    file_path="src/utils.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 11, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # Both files should be under same 'src' directory
        assert "src" in tree.children
        src_node = tree.children["src"]
        assert len(src_node.children) == 2
        assert "main.py" in src_node.children
        assert "utils.py" in src_node.children

    def test_build_tree_nested_directories(self):
        """Deeply nested files should create full path hierarchy."""
        index = {
            "src/utils/helpers/format.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,
                    file_path="src/utils/helpers/format.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # Navigate through hierarchy
        assert "src" in tree.children
        src = tree.children["src"]
        assert src.is_directory is True

        assert "utils" in src.children
        utils = src.children["utils"]
        assert utils.is_directory is True

        assert "helpers" in utils.children
        helpers = utils.children["helpers"]
        assert helpers.is_directory is True

        assert "format.py" in helpers.children
        format_file = helpers.children["format.py"]
        assert format_file.is_directory is False

    def test_build_tree_with_chain_context(self):
        """Chain IDs should propagate to tree nodes."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id="chain-abc",
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
                FileAccess(
                    session_id="session-002",
                    chain_id="chain-def",
                    file_path="src/main.py",
                    access_type="write",
                    tool_name="Edit",
                    timestamp=datetime(2025, 12, 16, 11, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        main_node = tree.children["src"].children["main.py"]
        assert "chain-abc" in main_node.chains
        assert "chain-def" in main_node.chains
        assert len(main_node.chains) == 2


# ============================================================================
# Test: Bubble Up Stats
# ============================================================================

class TestBubbleUpStats:
    """Tests for bubble_up_stats function."""

    def test_bubble_up_sessions_to_parent(self):
        """Parent directory should include child sessions after bubble up."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # After bubble up, 'src' directory should have session-001
        src_node = tree.children["src"]
        assert "session-001" in src_node.sessions

        # Root should also have session-001
        assert "session-001" in tree.sessions

    def test_bubble_up_chains_to_parent(self):
        """Parent directory should include child chains after bubble up."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id="chain-abc",
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # After bubble up, 'src' directory should have chain-abc
        src_node = tree.children["src"]
        assert "chain-abc" in src_node.chains

        # Root should also have chain-abc
        assert "chain-abc" in tree.chains

    def test_bubble_up_session_count(self):
        """session_count should equal len(sessions) after bubble up."""
        index = {
            "src/a.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,
                    file_path="src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
            "src/b.py": [
                FileAccess(
                    session_id="session-002",
                    chain_id=None,
                    file_path="src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 11, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # 'src' directory should have 2 unique sessions
        src_node = tree.children["src"]
        assert src_node.session_count == 2
        assert len(src_node.sessions) == 2

    def test_bubble_up_last_accessed(self):
        """Parent last_accessed should be max of children."""
        earlier = datetime(2025, 12, 16, 10, 0, 0)
        later = datetime(2025, 12, 16, 14, 0, 0)

        index = {
            "src/old.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,
                    file_path="src/old.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=earlier,
                    access_count=1,
                ),
            ],
            "src/new.py": [
                FileAccess(
                    session_id="session-002",
                    chain_id=None,
                    file_path="src/new.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=later,
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # 'src' last_accessed should be the later timestamp
        src_node = tree.children["src"]
        assert src_node.last_accessed == later

    def test_bubble_up_recursive(self):
        """Stats should bubble up through multiple levels."""
        index = {
            "src/utils/deep/file.py": [
                FileAccess(
                    session_id="session-deep",
                    chain_id="chain-deep",
                    file_path="src/utils/deep/file.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # All ancestors should have the session/chain
        assert "session-deep" in tree.sessions
        assert "chain-deep" in tree.chains

        src = tree.children["src"]
        assert "session-deep" in src.sessions
        assert "chain-deep" in src.chains

        utils = src.children["utils"]
        assert "session-deep" in utils.sessions
        assert "chain-deep" in utils.chains


# ============================================================================
# Test: Query Functions
# ============================================================================

class TestFileTreeQueries:
    """Tests for query functions."""

    def test_get_node_by_path(self, sample_inverted_index):
        """Should navigate to specific node by path."""
        tree = build_file_tree(sample_inverted_index)

        node = get_node_by_path(tree, "src/utils/helpers.py")

        assert node is not None
        assert node.path == "src/utils/helpers.py"
        assert node.is_directory is False

    def test_get_node_by_path_directory(self, sample_inverted_index):
        """Should navigate to directory node."""
        tree = build_file_tree(sample_inverted_index)

        node = get_node_by_path(tree, "src/utils")

        assert node is not None
        assert node.path == "src/utils"
        assert node.is_directory is True

    def test_get_node_not_found(self, sample_inverted_index):
        """Should return None for non-existent path."""
        tree = build_file_tree(sample_inverted_index)

        node = get_node_by_path(tree, "nonexistent/path.py")

        assert node is None

    def test_get_children(self, sample_inverted_index):
        """Should return immediate children of directory."""
        tree = build_file_tree(sample_inverted_index)

        src_node = get_node_by_path(tree, "src")
        children = get_children(src_node)

        # 'src' has 'main.py' and 'utils' directory
        assert len(children) == 2
        child_names = [c.name for c in children]
        assert "main.py" in child_names
        assert "utils" in child_names

    def test_get_directory_stats(self, sample_inverted_index):
        """Should return aggregated stats for directory."""
        tree = build_file_tree(sample_inverted_index)

        src_node = get_node_by_path(tree, "src")
        stats = get_directory_stats(src_node)

        assert "session_count" in stats
        assert "chain_count" in stats
        assert "file_count" in stats
        assert "last_accessed" in stats

        # src has 2 files: main.py and utils/helpers.py
        # sessions: session-001 (main.py), session-001 + session-002 (helpers.py)
        assert stats["session_count"] == 2  # session-001 and session-002


# ============================================================================
# Test: Persistence
# ============================================================================

class TestFileTreePersistence:
    """Tests for database persistence."""

    def test_persist_file_tree(self, temp_db, sample_inverted_index):
        """Should persist tree to database."""
        tree = build_file_tree(sample_inverted_index)

        stats = persist_file_tree(temp_db, tree)

        assert "nodes_stored" in stats
        assert stats["nodes_stored"] > 0

        # Verify data in database
        cursor = temp_db.execute("SELECT COUNT(*) FROM file_tree")
        count = cursor.fetchone()[0]
        assert count > 0

    def test_load_file_tree(self, temp_db, sample_inverted_index):
        """Should load tree from database."""
        original_tree = build_file_tree(sample_inverted_index)
        persist_file_tree(temp_db, original_tree)

        loaded_tree = load_file_tree(temp_db)

        # Verify structure matches
        assert loaded_tree.is_directory is True
        assert "src" in loaded_tree.children
        assert "tests" in loaded_tree.children

        # Verify stats preserved
        src_node = loaded_tree.children["src"]
        assert src_node.session_count > 0


# ============================================================================
# Test: Real-World Scenarios
# ============================================================================

class TestRealWorldScenarios:
    """Tests for real-world edge cases."""

    def test_handles_windows_paths(self):
        """Should normalize Windows backslashes to forward slashes."""
        windows_path = "src\\utils\\helpers.py"
        normalized = normalize_path(windows_path)
        assert normalized == "src/utils/helpers.py"

    def test_handles_absolute_paths(self):
        """Should convert absolute paths to relative."""
        project_root = Path("C:/Users/test/project")
        absolute_path = "C:/Users/test/project/src/main.py"

        normalized = normalize_path(absolute_path, project_root)

        assert normalized == "src/main.py"
        assert not normalized.startswith("C:")

    def test_handles_absolute_paths_unix(self):
        """Should convert Unix absolute paths to relative."""
        project_root = Path("/home/user/project")
        absolute_path = "/home/user/project/src/main.py"

        normalized = normalize_path(absolute_path, project_root)

        assert normalized == "src/main.py"
        assert not normalized.startswith("/")

    def test_build_tree_with_mixed_paths(self):
        """Should handle inverted index with mixed path formats."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
            # Windows-style path (should be normalized before building)
            "src\\utils.py": [
                FileAccess(
                    session_id="session-002",
                    chain_id=None,
                    file_path="src\\utils.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 11, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        # Both files should end up under 'src'
        src_node = tree.children["src"]
        assert len(src_node.children) == 2

    def test_handles_none_chain_id(self):
        """Should handle FileAccess with None chain_id."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id=None,  # No chain context
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
            ],
        }

        tree = build_file_tree(index)

        main_node = tree.children["src"].children["main.py"]
        # Should not have None in chains set
        assert None not in main_node.chains
        assert len(main_node.chains) == 0

    def test_access_history_preserved(self):
        """File nodes should preserve full access history."""
        index = {
            "src/main.py": [
                FileAccess(
                    session_id="session-001",
                    chain_id="chain-abc",
                    file_path="src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 16, 10, 0, 0),
                    access_count=1,
                ),
                FileAccess(
                    session_id="session-002",
                    chain_id="chain-def",
                    file_path="src/main.py",
                    access_type="write",
                    tool_name="Edit",
                    timestamp=datetime(2025, 12, 16, 11, 0, 0),
                    access_count=3,
                ),
            ],
        }

        tree = build_file_tree(index)

        main_node = tree.children["src"].children["main.py"]
        assert len(main_node.access_history) == 2
        assert main_node.access_history[0].session_id == "session-001"
        assert main_node.access_history[1].access_count == 3
