"""Tests for co_access module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).

The co-access matrix tracks files frequently accessed together.
Enables: "You're working on X, you probably need Y"

Algorithm:
1. Build session sets per file from inverted index
2. Compute Jaccard similarity: |A ∩ B| / |A ∪ B|
3. Keep pairs with jaccard >= threshold (default 0.3)
"""

import sqlite3
from datetime import datetime

import pytest


class TestCoAccessDataclass:
    """Test CoAccessEntry dataclass properties."""

    def test_co_access_entry_has_required_fields(self):
        """CoAccessEntry should track file pair relationship.

        RED: Run before implementation - should fail
        GREEN: Implement CoAccessEntry dataclass
        """
        from context_os_events.index.co_access import CoAccessEntry

        entry = CoAccessEntry(
            file_a="/src/main.py",
            file_b="/src/utils.py",
            pmi_score=1.5,
            co_occurrence_count=4,
            total_sessions=6,
        )

        assert entry.file_a == "/src/main.py"
        assert entry.file_b == "/src/utils.py"
        assert entry.pmi_score == 1.5
        assert entry.co_occurrence_count == 4
        assert entry.total_sessions == 6


class TestBuildSessionSets:
    """Test extracting session sets from inverted index."""

    def test_builds_session_sets_from_index(self):
        """Should extract unique session IDs per file.

        RED: Run before implementation - should fail
        GREEN: Implement _build_session_sets() helper
        """
        from context_os_events.index.co_access import _build_session_sets
        from context_os_events.index.inverted_index import FileAccess

        # Build test inverted index
        inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/utils.py": [
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/utils.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
        }

        session_sets = _build_session_sets(inverted_index, min_sessions=1)

        assert session_sets["/src/main.py"] == {"session-a", "session-b"}
        assert session_sets["/src/utils.py"] == {"session-b"}

    def test_filters_files_with_few_sessions(self):
        """Files with < min_sessions should be excluded.

        If a file was only accessed in 1 session, it's not useful for co-access.
        """
        from context_os_events.index.co_access import _build_session_sets
        from context_os_events.index.inverted_index import FileAccess

        inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/rare.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/rare.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],  # Only 1 session - should be filtered
        }

        session_sets = _build_session_sets(inverted_index, min_sessions=2)

        assert "/src/main.py" in session_sets
        assert "/src/rare.py" not in session_sets


class TestJaccardComputation:
    """Test Jaccard similarity computation."""

    def test_computes_jaccard_correctly(self):
        """Jaccard = |A ∩ B| / |A ∪ B|

        Example:
        - sessions_a = {s1, s2, s3}
        - sessions_b = {s2, s3, s4}
        - intersection = {s2, s3} = 2
        - union = {s1, s2, s3, s4} = 4
        - jaccard = 2/4 = 0.5
        """
        from context_os_events.index.co_access import _compute_jaccard

        sessions_a = {"s1", "s2", "s3"}
        sessions_b = {"s2", "s3", "s4"}

        jaccard = _compute_jaccard(sessions_a, sessions_b)

        assert jaccard == 0.5

    def test_jaccard_identical_sets(self):
        """Identical sets should have jaccard = 1.0"""
        from context_os_events.index.co_access import _compute_jaccard

        sessions = {"s1", "s2", "s3"}

        jaccard = _compute_jaccard(sessions, sessions)

        assert jaccard == 1.0

    def test_jaccard_disjoint_sets(self):
        """Disjoint sets should have jaccard = 0.0"""
        from context_os_events.index.co_access import _compute_jaccard

        sessions_a = {"s1", "s2"}
        sessions_b = {"s3", "s4"}

        jaccard = _compute_jaccard(sessions_a, sessions_b)

        assert jaccard == 0.0

    def test_jaccard_empty_sets(self):
        """Empty sets should return 0.0 (avoid division by zero)"""
        from context_os_events.index.co_access import _compute_jaccard

        empty = set()
        non_empty = {"s1"}

        assert _compute_jaccard(empty, non_empty) == 0.0
        assert _compute_jaccard(non_empty, empty) == 0.0
        assert _compute_jaccard(empty, empty) == 0.0


class TestBuildCoAccessMatrix:
    """Test building the complete co-access matrix."""

    def test_builds_matrix_from_inverted_index(self):
        """Should compute co-access scores for all file pairs.

        RED: Run before implementation - should fail
        GREEN: Implement build_co_access_matrix()
        """
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # Build index: main.py and utils.py share sessions
        inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-c",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/utils.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/utils.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/utils.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
        }

        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # main.py: {a, b, c}, utils.py: {a, b}
        # jaccard = 2/3 = 0.67
        assert "/src/main.py" in matrix
        co_accessed = dict(matrix["/src/main.py"])
        assert "/src/utils.py" in co_accessed
        assert abs(co_accessed["/src/utils.py"] - 0.67) < 0.01

    def test_filters_low_jaccard_pairs(self):
        """Pairs with jaccard < min_jaccard should be excluded."""
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # Low overlap: 1 shared session out of 5 total = 0.2 jaccard
        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(3)
            ],
            "/src/b.py": [
                FileAccess(
                    session_id="session-0",  # Only 1 shared
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-10",  # Different
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
        }

        # a.py: {0, 1, 2}, b.py: {0, 10}
        # jaccard = 1/4 = 0.25 < 0.3 threshold
        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # Neither file should appear in matrix (below threshold)
        assert "/src/a.py" not in matrix or "/src/b.py" not in dict(
            matrix.get("/src/a.py", [])
        )

    def test_bidirectional_entries(self):
        """Co-access should be bidirectional: if A->B exists, B->A should too."""
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # High overlap
        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
            "/src/b.py": [
                FileAccess(
                    session_id="session-a",
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
                FileAccess(
                    session_id="session-b",
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                ),
            ],
        }

        # jaccard = 2/2 = 1.0
        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # Both directions should exist
        assert "/src/a.py" in matrix
        assert "/src/b.py" in dict(matrix["/src/a.py"])
        assert "/src/b.py" in matrix
        assert "/src/a.py" in dict(matrix["/src/b.py"])

    def test_handles_empty_index(self):
        """Empty inverted index should return empty matrix."""
        from context_os_events.index.co_access import build_co_access_matrix

        matrix = build_co_access_matrix({})

        assert matrix == {}

    def test_sorts_by_score_descending(self):
        """Co-accessed files should be sorted by jaccard score (highest first)."""
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # Create index where main.py has different overlap with 3 files
        inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(4)
            ],  # {0, 1, 2, 3}
            "/src/high.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/high.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(4)
            ],  # {0, 1, 2, 3} - jaccard = 1.0
            "/src/medium.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/medium.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in [0, 1, 4, 5]
            ],  # {0, 1, 4, 5} - jaccard = 2/6 = 0.33
        }

        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # main.py should have high.py first, then medium.py
        assert "/src/main.py" in matrix
        co_accessed = matrix["/src/main.py"]
        assert len(co_accessed) >= 2
        # First entry should be high.py (jaccard = 1.0)
        assert co_accessed[0][0] == "/src/high.py"


class TestGetCoAccessed:
    """Test querying co-accessed files."""

    def test_get_co_accessed_returns_top_n(self):
        """Should return top N co-accessed files for a given file.

        RED: Run before implementation - should fail
        GREEN: Implement get_co_accessed()
        """
        from context_os_events.index.co_access import get_co_accessed

        matrix = {
            "/src/main.py": [
                ("/src/a.py", 0.9),
                ("/src/b.py", 0.7),
                ("/src/c.py", 0.5),
                ("/src/d.py", 0.4),
                ("/src/e.py", 0.35),
            ]
        }

        result = get_co_accessed(matrix, "/src/main.py", limit=3)

        assert len(result) == 3
        assert result[0] == ("/src/a.py", 0.9)
        assert result[1] == ("/src/b.py", 0.7)
        assert result[2] == ("/src/c.py", 0.5)

    def test_get_co_accessed_file_not_found(self):
        """Non-existent file should return empty list."""
        from context_os_events.index.co_access import get_co_accessed

        matrix = {"/src/main.py": [("/src/a.py", 0.9)]}

        result = get_co_accessed(matrix, "/nonexistent.py")

        assert result == []

    def test_get_co_accessed_empty_matrix(self):
        """Empty matrix should return empty list."""
        from context_os_events.index.co_access import get_co_accessed

        result = get_co_accessed({}, "/src/main.py")

        assert result == []


class TestCoAccessPersistence:
    """Test persisting co-access matrix to database."""

    def test_persist_co_access(self):
        """Should write to co_access table.

        RED: Run before implementation - should fail
        GREEN: Implement persist_co_access()
        """
        from context_os_events.index.co_access import persist_co_access

        # Create in-memory database with schema
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE co_access (
                file_a TEXT NOT NULL,
                file_b TEXT NOT NULL,
                jaccard_score REAL NOT NULL,
                co_occurrence_count INTEGER,
                total_sessions INTEGER,
                PRIMARY KEY (file_a, file_b)
            )
        """)

        matrix = {
            "/src/main.py": [("/src/utils.py", 0.67), ("/src/config.py", 0.5)],
            "/src/utils.py": [("/src/main.py", 0.67)],
            "/src/config.py": [("/src/main.py", 0.5)],
        }

        stats = persist_co_access(db, matrix)

        assert stats["pairs_stored"] >= 2

        # Verify in database
        cursor = db.execute("SELECT * FROM co_access ORDER BY jaccard_score DESC")
        rows = cursor.fetchall()
        assert len(rows) >= 2

    def test_load_co_access(self):
        """Should load co-access matrix from database.

        RED: Run before implementation - should fail
        GREEN: Implement load_co_access()
        """
        from context_os_events.index.co_access import load_co_access

        # Create in-memory database with test data
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE co_access (
                file_a TEXT NOT NULL,
                file_b TEXT NOT NULL,
                jaccard_score REAL NOT NULL,
                co_occurrence_count INTEGER,
                total_sessions INTEGER,
                PRIMARY KEY (file_a, file_b)
            )
        """)
        db.execute("""
            INSERT INTO co_access (file_a, file_b, jaccard_score, co_occurrence_count, total_sessions)
            VALUES ('/src/main.py', '/src/utils.py', 0.67, 2, 3)
        """)
        db.execute("""
            INSERT INTO co_access (file_a, file_b, jaccard_score, co_occurrence_count, total_sessions)
            VALUES ('/src/main.py', '/src/config.py', 0.5, 1, 2)
        """)
        db.commit()

        matrix = load_co_access(db)

        assert "/src/main.py" in matrix
        co_accessed = dict(matrix["/src/main.py"])
        assert "/src/utils.py" in co_accessed
        assert abs(co_accessed["/src/utils.py"] - 0.67) < 0.01

    def test_persist_and_load_roundtrip(self):
        """Matrix should survive persist -> load roundtrip."""
        from context_os_events.index.co_access import (
            load_co_access,
            persist_co_access,
        )

        # Create in-memory database with schema
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE co_access (
                file_a TEXT NOT NULL,
                file_b TEXT NOT NULL,
                jaccard_score REAL NOT NULL,
                co_occurrence_count INTEGER,
                total_sessions INTEGER,
                PRIMARY KEY (file_a, file_b)
            )
        """)

        original_matrix = {
            "/src/a.py": [("/src/b.py", 0.8), ("/src/c.py", 0.6)],
            "/src/b.py": [("/src/a.py", 0.8)],
            "/src/c.py": [("/src/a.py", 0.6)],
        }

        persist_co_access(db, original_matrix)
        loaded_matrix = load_co_access(db)

        # Check main file exists with correct co-accessed
        assert "/src/a.py" in loaded_matrix
        loaded_dict = dict(loaded_matrix["/src/a.py"])
        assert "/src/b.py" in loaded_dict
        assert abs(loaded_dict["/src/b.py"] - 0.8) < 0.01


class TestRealWorldScenarios:
    """Test with realistic data patterns."""

    def test_handles_many_files_efficiently(self):
        """Should handle 100+ files without exploding."""
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # Create 100 files, each accessed by some subset of 20 sessions
        num_files = 100
        num_sessions = 20
        inverted_index = {}

        import random

        random.seed(42)  # Reproducible

        for i in range(num_files):
            # Each file accessed by 3-10 random sessions
            num_accesses = random.randint(3, 10)
            session_ids = random.sample(
                [f"session-{j}" for j in range(num_sessions)], num_accesses
            )

            inverted_index[f"/src/file_{i}.py"] = [
                FileAccess(
                    session_id=sid,
                    chain_id=None,
                    file_path=f"/src/file_{i}.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for sid in session_ids
            ]

        # Should complete without timeout
        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # Should have some entries
        assert len(matrix) > 0

    def test_game_trails_pattern(self):
        """Test the 'game trails' pattern: files always edited together.

        Real scenario: test file + implementation file
        """
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # Test file and implementation always accessed together
        inverted_index = {
            "/src/parser.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/parser.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(10)
            ],
            "/tests/test_parser.py": [
                FileAccess(
                    session_id=f"session-{i}",  # Same sessions!
                    chain_id=None,
                    file_path="/tests/test_parser.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(10)
            ],
            "/src/unrelated.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/unrelated.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in [20, 21, 22]  # Different sessions
            ],
        }

        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # parser.py should have test_parser.py as top co-accessed
        assert "/src/parser.py" in matrix
        top_co_accessed = matrix["/src/parser.py"][0]
        assert top_co_accessed[0] == "/tests/test_parser.py"
        assert top_co_accessed[1] == 1.0  # Perfect overlap

    def test_init_files_common_pattern(self):
        """__init__.py files are often accessed with many files."""
        from context_os_events.index.co_access import build_co_access_matrix
        from context_os_events.index.inverted_index import FileAccess

        # __init__.py accessed in all sessions
        inverted_index = {
            "/src/__init__.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/__init__.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(10)
            ],
            "/src/a.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(5)
            ],  # {0..4}
            "/src/b.py": [
                FileAccess(
                    session_id=f"session-{i}",
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime.now(),
                )
                for i in range(5, 10)
            ],  # {5..9}
        }

        matrix = build_co_access_matrix(inverted_index, min_sessions=2, use_pmi=False)

        # __init__.py should co-access with both a.py and b.py
        assert "/src/__init__.py" in matrix
        co_accessed_files = {f for f, _ in matrix["/src/__init__.py"]}
        assert "/src/a.py" in co_accessed_files
        assert "/src/b.py" in co_accessed_files
