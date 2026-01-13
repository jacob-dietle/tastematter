"""Temporal buckets for Context OS Intelligence.

Groups sessions by time period (ISO week) for temporal queries.
Enables: "What was I working on last week?"

Key features:
- Sessions grouped by ISO week (2025-W50)
- Bloom filters for fast file membership checks
- Chain aggregation per bucket
"""

import json
import logging
from dataclasses import dataclass, field
from datetime import datetime
from typing import Dict, List, Optional, Set

from .bloom import BloomFilter
from .inverted_index import FileAccess

logger = logging.getLogger(__name__)


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class TemporalBucket:
    """Represents a time period (week or day) with aggregated session data.

    Attributes:
        period: ISO format string (e.g., "2025-W50" for week, "2025-12-16" for day)
        period_type: "week" or "day"
        sessions: Set of session IDs in this period
        chains: Set of chain IDs in this period
        files_bloom: Serialized bloom filter of files touched
        commits: List of git commit hashes in this period
        started_at: Start of the period
        ended_at: End of the period
    """
    period: str
    period_type: str
    sessions: Set[str]
    chains: Set[str]
    files_bloom: bytes
    commits: List[str] = field(default_factory=list)
    started_at: Optional[datetime] = None
    ended_at: Optional[datetime] = None


# ============================================================================
# Helper Functions
# ============================================================================

def _get_iso_week(dt: datetime) -> str:
    """Get ISO week string from datetime.

    Args:
        dt: Datetime to convert

    Returns:
        ISO week string in format "YYYY-WXX"
    """
    iso_year, iso_week, _ = dt.isocalendar()
    return f"{iso_year}-W{iso_week:02d}"


def _get_week_boundaries(dt: datetime) -> tuple:
    """Get start and end of the ISO week containing dt.

    Args:
        dt: Datetime within the week

    Returns:
        Tuple of (week_start, week_end) datetimes
    """
    from datetime import timedelta

    # ISO week starts on Monday
    iso_year, iso_week, iso_day = dt.isocalendar()

    # Find Monday of this week
    days_since_monday = iso_day - 1
    monday = dt - timedelta(days=days_since_monday)
    monday = monday.replace(hour=0, minute=0, second=0, microsecond=0)

    # Sunday is 6 days after Monday
    sunday = monday + timedelta(days=6)
    sunday = sunday.replace(hour=23, minute=59, second=59, microsecond=999999)

    return (monday, sunday)


# ============================================================================
# Build Functions
# ============================================================================

def build_temporal_buckets(
    inverted_index: Dict[str, List[FileAccess]]
) -> Dict[str, TemporalBucket]:
    """Build temporal buckets from inverted file index.

    Groups sessions by ISO week and builds bloom filters for file membership.

    Args:
        inverted_index: Dict mapping file_path -> List[FileAccess]

    Returns:
        Dict mapping period string -> TemporalBucket
    """
    if not inverted_index:
        return {}

    buckets: Dict[str, TemporalBucket] = {}

    # Track bloom filters separately (not yet serialized)
    bucket_blooms: Dict[str, BloomFilter] = {}

    # Process all file accesses
    for file_path, accesses in inverted_index.items():
        for access in accesses:
            week = _get_iso_week(access.timestamp)

            # Create bucket if needed
            if week not in buckets:
                week_start, week_end = _get_week_boundaries(access.timestamp)
                buckets[week] = TemporalBucket(
                    period=week,
                    period_type="week",
                    sessions=set(),
                    chains=set(),
                    files_bloom=b"",  # Will serialize later
                    commits=[],
                    started_at=week_start,
                    ended_at=week_end,
                )
                bucket_blooms[week] = BloomFilter(expected_items=1000, false_positive_rate=0.01)

            bucket = buckets[week]

            # Add session
            bucket.sessions.add(access.session_id)

            # Add chain if present
            if access.chain_id:
                bucket.chains.add(access.chain_id)

            # Add file to bloom filter
            bucket_blooms[week].add(file_path)

    # Serialize bloom filters
    for week, bloom in bucket_blooms.items():
        buckets[week].files_bloom = bloom.serialize()

    return buckets


# ============================================================================
# Query Functions
# ============================================================================

def get_week_bucket(
    buckets: Dict[str, TemporalBucket],
    week: str
) -> Optional[TemporalBucket]:
    """Get bucket for a specific week.

    Args:
        buckets: Dict of buckets from build_temporal_buckets()
        week: ISO week string (e.g., "2025-W50")

    Returns:
        TemporalBucket or None if not found
    """
    return buckets.get(week)


def get_buckets_in_range(
    buckets: Dict[str, TemporalBucket],
    start: datetime,
    end: datetime
) -> List[TemporalBucket]:
    """Get buckets overlapping with date range.

    Args:
        buckets: Dict of buckets from build_temporal_buckets()
        start: Start of range
        end: End of range

    Returns:
        List of buckets that overlap with the range
    """
    result = []

    for bucket in buckets.values():
        if bucket.started_at is None or bucket.ended_at is None:
            continue

        # Check for overlap: bucket overlaps if bucket_start <= end AND bucket_end >= start
        if bucket.started_at <= end and bucket.ended_at >= start:
            result.append(bucket)

    return result


def file_touched_in_week(
    buckets: Dict[str, TemporalBucket],
    file_path: str,
    week: str
) -> bool:
    """Check if file was touched in a specific week using bloom filter.

    Fast O(1) check with possible false positives.

    Args:
        buckets: Dict of buckets from build_temporal_buckets()
        file_path: Path to check
        week: ISO week string (e.g., "2025-W50")

    Returns:
        True if file was PROBABLY touched (may be false positive)
        False if file was DEFINITELY NOT touched
    """
    bucket = buckets.get(week)
    if bucket is None:
        return False

    if not bucket.files_bloom:
        return False

    # Deserialize and check (self-describing format)
    bloom = BloomFilter.deserialize(bucket.files_bloom)

    return file_path in bloom


# ============================================================================
# Database Persistence
# ============================================================================

def persist_temporal_buckets(
    db,
    buckets: Dict[str, TemporalBucket]
) -> Dict[str, int]:
    """Persist temporal buckets to database.

    Writes to temporal_buckets table.

    Args:
        db: SQLite connection
        buckets: Dict of buckets from build_temporal_buckets()

    Returns:
        Stats dict: {"buckets_stored": N}
    """
    buckets_stored = 0

    for period, bucket in buckets.items():
        sessions_json = json.dumps(sorted(bucket.sessions))
        chains_json = json.dumps(sorted(bucket.chains))
        commits_json = json.dumps(bucket.commits)

        started_at_str = bucket.started_at.isoformat() if bucket.started_at else None
        ended_at_str = bucket.ended_at.isoformat() if bucket.ended_at else None

        db.execute("""
            INSERT OR REPLACE INTO temporal_buckets
            (period, period_type, sessions_json, chains_json, files_bloom,
             commits_json, started_at, ended_at, session_count, chain_count, commit_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            period,
            bucket.period_type,
            sessions_json,
            chains_json,
            bucket.files_bloom,
            commits_json,
            started_at_str,
            ended_at_str,
            len(bucket.sessions),
            len(bucket.chains),
            len(bucket.commits),
        ))
        buckets_stored += 1

    db.commit()

    return {"buckets_stored": buckets_stored}


def load_temporal_buckets(db) -> Dict[str, TemporalBucket]:
    """Load temporal buckets from database.

    Args:
        db: SQLite connection

    Returns:
        Dict mapping period string -> TemporalBucket
    """
    cursor = db.execute("""
        SELECT period, period_type, sessions_json, chains_json, files_bloom,
               commits_json, started_at, ended_at
        FROM temporal_buckets
    """)

    buckets: Dict[str, TemporalBucket] = {}

    for row in cursor.fetchall():
        period = row[0]
        period_type = row[1]
        sessions_json = row[2]
        chains_json = row[3]
        files_bloom = row[4]
        commits_json = row[5]
        started_at_str = row[6]
        ended_at_str = row[7]

        # Parse JSON
        sessions = set(json.loads(sessions_json)) if sessions_json else set()
        chains = set(json.loads(chains_json)) if chains_json else set()
        commits = json.loads(commits_json) if commits_json else []

        # Parse datetimes
        started_at = datetime.fromisoformat(started_at_str) if started_at_str else None
        ended_at = datetime.fromisoformat(ended_at_str) if ended_at_str else None

        buckets[period] = TemporalBucket(
            period=period,
            period_type=period_type,
            sessions=sessions,
            chains=chains,
            files_bloom=files_bloom if files_bloom else b"",
            commits=commits,
            started_at=started_at,
            ended_at=ended_at,
        )

    return buckets
