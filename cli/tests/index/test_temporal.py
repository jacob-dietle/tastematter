"""Tests for temporal buckets module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).

Temporal buckets group sessions by time period (ISO week or day).
Enables: "What was I working on last week?"

Key features:
- Sessions grouped by ISO week (2025-W50)
- Bloom filters for fast file membership checks
- Chain aggregation per bucket
"""

import sqlite3
from datetime import datetime, timedelta

import pytest


class TestTemporalBucketDataclass:
    """Test TemporalBucket dataclass properties."""

    def test_temporal_bucket_has_required_fields(self):
        """TemporalBucket should track period, sessions, chains, and bloom filter.

        RED: Run before implementation - should fail
        GREEN: Implement TemporalBucket dataclass
        """
        from context_os_events.index.temporal import TemporalBucket

        bucket = TemporalBucket(
            period="2025-W50",
            period_type="week",
            sessions={"session-a", "session-b"},
            chains={"chain-1"},
            files_bloom=b"",
            commits=["abc123"],
            started_at=datetime(2025, 12, 9),
            ended_at=datetime(2025, 12, 15),
        )

        assert bucket.period == "2025-W50"
        assert bucket.period_type == "week"
        assert bucket.sessions == {"session-a", "session-b"}
        assert bucket.chains == {"chain-1"}
        assert bucket.started_at == datetime(2025, 12, 9)


class TestBuildTemporalBuckets:
    """Test building temporal buckets from inverted index."""

    def test_groups_sessions_by_iso_week(self):
        """Sessions should be grouped by ISO week (YYYY-WXX format).

        RED: Run before implementation - should fail
        GREEN: Implement build_temporal_buckets()
        """
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        # Create sessions in different weeks
        # Week 50 (Dec 9-15, 2025) and Week 51 (Dec 16-22, 2025)
        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="session-w50-a",
                    chain_id="chain-1",
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 10, 0),  # Week 50
                ),
                FileAccess(
                    session_id="session-w50-b",
                    chain_id="chain-1",
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 12, 14, 0),  # Week 50
                ),
                FileAccess(
                    session_id="session-w51-a",
                    chain_id="chain-2",
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 17, 9, 0),  # Week 51
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        # Should have 2 buckets
        assert len(buckets) == 2
        assert "2025-W50" in buckets
        assert "2025-W51" in buckets

        # Week 50 should have 2 sessions
        assert buckets["2025-W50"].sessions == {"session-w50-a", "session-w50-b"}

        # Week 51 should have 1 session
        assert buckets["2025-W51"].sessions == {"session-w51-a"}

    def test_aggregates_chains_per_bucket(self):
        """Each bucket should aggregate unique chain IDs."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s1",
                    chain_id="chain-1",
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 10, 0),
                ),
                FileAccess(
                    session_id="s2",
                    chain_id="chain-1",  # Same chain
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 11, 10, 0),
                ),
                FileAccess(
                    session_id="s3",
                    chain_id="chain-2",  # Different chain
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 12, 10, 0),
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        assert "2025-W50" in buckets
        assert buckets["2025-W50"].chains == {"chain-1", "chain-2"}

    def test_handles_sessions_without_chain_id(self):
        """Sessions without chain_id should not add None to chains set."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,  # No chain
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 10, 0),
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        assert "2025-W50" in buckets
        assert None not in buckets["2025-W50"].chains
        assert buckets["2025-W50"].chains == set()

    def test_builds_bloom_filter_for_files(self):
        """Each bucket should have bloom filter of files touched."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 10, 0),
                ),
            ],
            "/src/b.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 11, 0),
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        assert "2025-W50" in buckets
        # files_bloom should be bytes (serialized bloom filter)
        assert isinstance(buckets["2025-W50"].files_bloom, bytes)
        assert len(buckets["2025-W50"].files_bloom) > 0

    def test_handles_empty_inverted_index(self):
        """Empty inverted index should return empty buckets dict."""
        from context_os_events.index.temporal import build_temporal_buckets

        buckets = build_temporal_buckets({})

        assert buckets == {}


class TestGetWeekBucket:
    """Test querying buckets by week."""

    def test_get_week_bucket_returns_bucket(self):
        """Should return bucket for specified week.

        RED: Run before implementation - should fail
        GREEN: Implement get_week_bucket()
        """
        from context_os_events.index.temporal import (
            TemporalBucket,
            get_week_bucket,
        )

        buckets = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            ),
        }

        result = get_week_bucket(buckets, "2025-W50")

        assert result is not None
        assert result.period == "2025-W50"

    def test_get_week_bucket_not_found(self):
        """Should return None for non-existent week."""
        from context_os_events.index.temporal import get_week_bucket

        buckets = {}

        result = get_week_bucket(buckets, "2025-W50")

        assert result is None


class TestGetBucketsInRange:
    """Test querying buckets by date range."""

    def test_get_buckets_in_range(self):
        """Should return buckets overlapping with date range.

        RED: Run before implementation - should fail
        GREEN: Implement get_buckets_in_range()
        """
        from context_os_events.index.temporal import (
            TemporalBucket,
            get_buckets_in_range,
        )

        buckets = {
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

        # Query Dec 5 - Dec 12 (should get W49 and W50)
        result = get_buckets_in_range(
            buckets,
            start=datetime(2025, 12, 5),
            end=datetime(2025, 12, 12),
        )

        assert len(result) == 2
        periods = {b.period for b in result}
        assert "2025-W49" in periods
        assert "2025-W50" in periods
        assert "2025-W51" not in periods

    def test_get_buckets_in_range_empty(self):
        """Should return empty list if no buckets in range."""
        from context_os_events.index.temporal import (
            TemporalBucket,
            get_buckets_in_range,
        )

        buckets = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1"},
                chains=set(),
                files_bloom=b"",
                commits=[],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            ),
        }

        # Query January - no overlap
        result = get_buckets_in_range(
            buckets,
            start=datetime(2025, 1, 1),
            end=datetime(2025, 1, 31),
        )

        assert result == []


class TestFileTouchedInWeek:
    """Test bloom filter file membership checks."""

    def test_file_touched_in_week_bloom_check(self):
        """Should use bloom filter for fast file membership check.

        RED: Run before implementation - should fail
        GREEN: Implement file_touched_in_week()
        """
        from context_os_events.index.temporal import (
            build_temporal_buckets,
            file_touched_in_week,
        )
        from context_os_events.index.inverted_index import FileAccess

        inverted_index = {
            "/src/main.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 10, 10, 0),
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        # File we added
        assert file_touched_in_week(buckets, "/src/main.py", "2025-W50") is True

        # File we didn't add
        assert file_touched_in_week(buckets, "/src/other.py", "2025-W50") is False

    def test_file_touched_in_week_nonexistent_week(self):
        """Should return False for non-existent week."""
        from context_os_events.index.temporal import file_touched_in_week

        buckets = {}

        assert file_touched_in_week(buckets, "/src/main.py", "2025-W99") is False


class TestTemporalPersistence:
    """Test persisting temporal buckets to database."""

    def test_persist_temporal_buckets(self):
        """Should write to temporal_buckets table.

        RED: Run before implementation - should fail
        GREEN: Implement persist_temporal_buckets()
        """
        from context_os_events.index.temporal import (
            TemporalBucket,
            persist_temporal_buckets,
        )

        # Create in-memory database with schema
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE temporal_buckets (
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

        buckets = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1", "s2"},
                chains={"chain-1"},
                files_bloom=b"\x00\x01\x02",
                commits=["abc123"],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            ),
        }

        stats = persist_temporal_buckets(db, buckets)

        assert stats["buckets_stored"] >= 1

        # Verify in database
        cursor = db.execute("SELECT * FROM temporal_buckets")
        rows = cursor.fetchall()
        assert len(rows) == 1

    def test_load_temporal_buckets(self):
        """Should load temporal buckets from database.

        RED: Run before implementation - should fail
        GREEN: Implement load_temporal_buckets()
        """
        from context_os_events.index.temporal import load_temporal_buckets

        # Create in-memory database with test data
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE temporal_buckets (
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
        db.execute("""
            INSERT INTO temporal_buckets
            (period, period_type, sessions_json, chains_json, files_bloom,
             commits_json, started_at, ended_at, session_count, chain_count, commit_count)
            VALUES ('2025-W50', 'week', '["s1", "s2"]', '["chain-1"]', X'000102',
                    '["abc123"]', '2025-12-09T00:00:00', '2025-12-15T23:59:59', 2, 1, 1)
        """)
        db.commit()

        buckets = load_temporal_buckets(db)

        assert "2025-W50" in buckets
        assert buckets["2025-W50"].sessions == {"s1", "s2"}
        assert buckets["2025-W50"].chains == {"chain-1"}

    def test_persist_and_load_roundtrip(self):
        """Buckets should survive persist -> load roundtrip."""
        from context_os_events.index.temporal import (
            TemporalBucket,
            load_temporal_buckets,
            persist_temporal_buckets,
        )

        # Create in-memory database with schema
        db = sqlite3.connect(":memory:")
        db.execute("""
            CREATE TABLE temporal_buckets (
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

        original = {
            "2025-W50": TemporalBucket(
                period="2025-W50",
                period_type="week",
                sessions={"s1", "s2", "s3"},
                chains={"chain-1", "chain-2"},
                files_bloom=b"\x00\x01\x02\x03",
                commits=["abc123", "def456"],
                started_at=datetime(2025, 12, 9),
                ended_at=datetime(2025, 12, 15),
            ),
        }

        persist_temporal_buckets(db, original)
        loaded = load_temporal_buckets(db)

        assert "2025-W50" in loaded
        assert loaded["2025-W50"].sessions == original["2025-W50"].sessions
        assert loaded["2025-W50"].chains == original["2025-W50"].chains
        assert loaded["2025-W50"].period_type == "week"


class TestISOWeekCalculation:
    """Test ISO week handling edge cases."""

    def test_iso_week_format(self):
        """Buckets should use YYYY-WXX format."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s1",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 1, 6, 10, 0),  # Week 02
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        # Should have week format, not just date
        keys = list(buckets.keys())
        assert len(keys) == 1
        assert keys[0].startswith("2025-W")

    def test_week_boundary_handling(self):
        """Sessions at week boundaries should group correctly."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        # Sunday Dec 15, 2025 = end of W50
        # Monday Dec 16, 2025 = start of W51
        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s-sunday",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 14, 23, 59),  # Sunday W50
                ),
                FileAccess(
                    session_id="s-monday",
                    chain_id=None,
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=datetime(2025, 12, 15, 0, 1),  # Monday W51
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        # Should be in different weeks
        assert len(buckets) == 2


class TestRealWorldScenarios:
    """Test with realistic usage patterns."""

    def test_multiple_files_same_session(self):
        """Multiple files in same session should deduplicate in bucket."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        ts = datetime(2025, 12, 10, 10, 0)
        inverted_index = {
            "/src/a.py": [
                FileAccess(
                    session_id="s1",
                    chain_id="chain-1",
                    file_path="/src/a.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=ts,
                ),
            ],
            "/src/b.py": [
                FileAccess(
                    session_id="s1",  # Same session
                    chain_id="chain-1",  # Same chain
                    file_path="/src/b.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=ts,
                ),
            ],
            "/src/c.py": [
                FileAccess(
                    session_id="s1",  # Same session
                    chain_id="chain-1",  # Same chain
                    file_path="/src/c.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=ts,
                ),
            ],
        }

        buckets = build_temporal_buckets(inverted_index)

        assert "2025-W50" in buckets
        # Should only have 1 session, not 3
        assert len(buckets["2025-W50"].sessions) == 1
        # Should only have 1 chain
        assert len(buckets["2025-W50"].chains) == 1

    def test_many_weeks_of_activity(self):
        """Should handle activity spanning many weeks."""
        from context_os_events.index.temporal import build_temporal_buckets
        from context_os_events.index.inverted_index import FileAccess

        # 10 weeks of activity
        inverted_index = {"/src/main.py": []}
        base_date = datetime(2025, 10, 1)

        for week in range(10):
            inverted_index["/src/main.py"].append(
                FileAccess(
                    session_id=f"s-week-{week}",
                    chain_id=f"chain-{week}",
                    file_path="/src/main.py",
                    access_type="read",
                    tool_name="Read",
                    timestamp=base_date + timedelta(weeks=week),
                )
            )

        buckets = build_temporal_buckets(inverted_index)

        # Should have 10 different weeks
        assert len(buckets) == 10
        # Each week should have 1 session
        for bucket in buckets.values():
            assert len(bucket.sessions) == 1
