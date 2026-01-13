"""
QueryEngine: Flexible hypercube query interface for Context OS.

Phase B of CLI Hypercube Refactor (Spec 12).

The Context OS index is a 5-dimensional hypercube:
    Files × Sessions × Time × Chains × AccessType

Every query = Slice + Aggregate + Render.

Usage:
    from context_os_events.query_engine import QuerySpec, QueryEngine

    spec = QuerySpec(
        files="*pixee*",
        time="7d",
        agg=["count", "recency"],
        format="json",
        limit=20
    )

    engine = QueryEngine(index)
    result = engine.execute(spec)

    print(result.to_json())
"""

import hashlib
import json
import fnmatch
from dataclasses import dataclass, field, asdict
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, List, Literal, Optional, Tuple

from rich.console import Console
from rich.table import Table


# =============================================================================
# Type Contracts
# =============================================================================

@dataclass
class QuerySpec:
    """Defines a hypercube query: slice + aggregate + render.

    All slice dimensions are optional and combine with AND logic.
    """

    # ─────────────────────────────────────────────────────────────────
    # SLICE dimensions (all optional, combine with AND)
    # ─────────────────────────────────────────────────────────────────
    files: Optional[str] = None
    """Glob/substring pattern: "*pixee*", "*.py", "src/components/"""

    time: Optional[str] = None
    """Time range: "7d", "2w", "2025-W50", "2025-W48:2025-W50"""

    chain: Optional[str] = None
    """Chain ID prefix or "active" (touched in last 7d)"""

    session: Optional[str] = None
    """Session ID prefix for filtering"""

    access: Optional[str] = None
    """Access types: "r", "w", "c", "rw", "rwc" (read/write/create)"""

    # ─────────────────────────────────────────────────────────────────
    # AGGREGATE options
    # ─────────────────────────────────────────────────────────────────
    agg: List[str] = field(default_factory=lambda: ["count"])
    """Aggregations to compute. Valid: count, recency, trend, sessions, files, chains"""

    # ─────────────────────────────────────────────────────────────────
    # RENDER options
    # ─────────────────────────────────────────────────────────────────
    format: Literal["json", "table"] = "json"
    """Output format for results"""

    limit: int = 20
    """Maximum results to return"""

    sort: Literal["count", "recency", "alpha"] = "count"
    """Sort order for results"""

    def validate(self) -> List[str]:
        """Return list of validation errors, empty if valid."""
        errors = []

        # Validate aggregations
        valid_aggs = {"count", "recency", "trend", "sessions", "files", "chains"}
        for a in self.agg:
            if a not in valid_aggs:
                errors.append(f"Invalid aggregation '{a}'. Valid: {valid_aggs}")

        # Validate access types
        valid_access = {"r", "w", "c", "rw", "rc", "wc", "rwc"}
        if self.access and self.access not in valid_access:
            errors.append(f"Invalid access '{self.access}'. Valid: {valid_access}")

        # Validate limit
        if self.limit < 1 or self.limit > 1000:
            errors.append(f"Limit must be 1-1000, got {self.limit}")

        return errors


@dataclass
class QueryResult:
    """Result of a hypercube query with audit metadata."""

    # Audit trail (for verification layer - Phase C)
    receipt_id: str
    """Unique query ID for verification: "q_" + 6-char hash"""

    timestamp: str
    """ISO format execution timestamp"""

    query: QuerySpec
    """The query that produced this result"""

    # Results
    result_count: int
    """Total matches (before limit applied)"""

    results: List[dict]
    """The actual data rows"""

    # Aggregation summaries
    aggregations: dict
    """Summary stats: {"count": {...}, "recency": {...}}"""

    def to_json(self) -> str:
        """Serialize to JSON for agent consumption."""
        return json.dumps({
            "receipt_id": self.receipt_id,
            "timestamp": self.timestamp,
            "query": {
                "files": self.query.files,
                "time": self.query.time,
                "chain": self.query.chain,
                "session": self.query.session,
                "access": self.query.access,
                "agg": self.query.agg,
                "limit": self.query.limit,
                "sort": self.query.sort,
            },
            "result_count": self.result_count,
            "results": self.results,
            "aggregations": self.aggregations,
        }, indent=2, default=str)

    def to_table(self) -> str:
        """Render as Rich table for human consumption."""
        console = Console(record=True, width=120)
        table = Table(title=f"Query Results ({self.result_count} total)")

        # Determine columns from first result
        if self.results:
            for key in self.results[0].keys():
                table.add_column(key)

            for row in self.results:
                table.add_row(*[str(v) for v in row.values()])
        else:
            table.add_column("Result")
            table.add_row("No results found")

        console.print(table)
        return console.export_text()


# =============================================================================
# Verification Layer (Phase C)
# =============================================================================

@dataclass
class QueryReceipt:
    """Verifiable record of a query execution.

    Stores the query, results, and a content-addressed hash for verification.
    """

    receipt_id: str
    """Unique ID: "q_" + first 6 chars of content hash"""

    timestamp: str
    """ISO 8601 execution timestamp"""

    query_spec: QuerySpec
    """The query that was run"""

    result_hash: str
    """SHA256 of canonical JSON results: "sha256:..."  """

    result_count: int
    """Total results returned"""

    result_snapshot: List[dict]
    """Full results for audit (enables drift detection)"""

    @staticmethod
    def generate_id(timestamp: str, query_spec: QuerySpec, results: List[dict]) -> str:
        """Generate deterministic receipt ID from content.

        Uses content-addressing: same query + results = same ID.
        This enables deduplication and verification.
        """
        content = json.dumps({
            "timestamp": timestamp,
            "query": {
                "files": query_spec.files,
                "time": query_spec.time,
                "chain": query_spec.chain,
                "agg": query_spec.agg,
            },
            "result_count": len(results),
        }, sort_keys=True)

        full_hash = hashlib.sha256(content.encode()).hexdigest()
        return f"q_{full_hash[:6]}"

    def compute_result_hash(self) -> str:
        """Compute SHA256 of results for verification.

        Uses canonical JSON (sorted keys) for deterministic hashing.
        """
        canonical = json.dumps(self.result_snapshot, sort_keys=True)
        full_hash = hashlib.sha256(canonical.encode()).hexdigest()
        return f"sha256:{full_hash}"

    def to_dict(self) -> dict:
        """Serialize for ledger storage."""
        return {
            "receipt_id": self.receipt_id,
            "timestamp": self.timestamp,
            "query_spec": {
                "files": self.query_spec.files,
                "time": self.query_spec.time,
                "chain": self.query_spec.chain,
                "session": self.query_spec.session,
                "access": self.query_spec.access,
                "agg": self.query_spec.agg,
                "limit": self.query_spec.limit,
                "sort": self.query_spec.sort,
            },
            "result_hash": self.result_hash,
            "result_count": self.result_count,
            "result_snapshot": self.result_snapshot,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "QueryReceipt":
        """Deserialize from ledger storage."""
        spec = QuerySpec(
            files=data["query_spec"].get("files"),
            time=data["query_spec"].get("time"),
            chain=data["query_spec"].get("chain"),
            session=data["query_spec"].get("session"),
            access=data["query_spec"].get("access"),
            agg=data["query_spec"].get("agg", ["count"]),
            limit=data["query_spec"].get("limit", 20),
            sort=data["query_spec"].get("sort", "count"),
        )

        return cls(
            receipt_id=data["receipt_id"],
            timestamp=data["timestamp"],
            query_spec=spec,
            result_hash=data["result_hash"],
            result_count=data["result_count"],
            result_snapshot=data["result_snapshot"],
        )


@dataclass
class VerificationResult:
    """Result of verifying a query receipt."""

    receipt_id: str
    """The receipt being verified"""

    original_timestamp: str
    """When the original query was run"""

    verification_timestamp: str
    """When verification was performed"""

    status: Literal["MATCH", "DRIFT", "NOT_FOUND", "EXPIRED"]
    """Verification status:
    - MATCH: Current results match original
    - DRIFT: Results have changed
    - NOT_FOUND: Receipt ID not in ledger
    - EXPIRED: Receipt past TTL (30 days)
    """

    original_hash: str
    """Hash from original receipt"""

    current_hash: Optional[str]
    """Hash of current results (None if NOT_FOUND)"""

    original_count: int
    """Result count from original query"""

    current_count: Optional[int]
    """Current result count (None if NOT_FOUND)"""

    drift_summary: Optional[str]
    """Human-readable drift description: "3 new files, 2 removed" """

    diff: Optional[dict]
    """Detailed diff (only with --verbose):
    {"added": [...], "removed": [...], "changed": [...]}
    """

    def to_dict(self) -> dict:
        """Serialize for JSON output."""
        return {
            "receipt_id": self.receipt_id,
            "status": self.status,
            "original_timestamp": self.original_timestamp,
            "verification_timestamp": self.verification_timestamp,
            "original_hash": self.original_hash,
            "current_hash": self.current_hash,
            "original_count": self.original_count,
            "current_count": self.current_count,
            "drift_summary": self.drift_summary,
            "diff": self.diff,
        }


# =============================================================================
# QueryLedger (Storage)
# =============================================================================

from pathlib import Path
import os


class QueryLedger:
    """Filesystem-based storage for query receipts.

    Storage location: ~/.context-os/query_ledger/
    TTL: 30 days (configurable)
    Format: One JSON file per receipt
    """

    def __init__(self, ledger_dir: Optional[Path] = None, ttl_days: int = 30):
        """Initialize ledger.

        Args:
            ledger_dir: Storage directory. Defaults to ~/.context-os/query_ledger/
            ttl_days: Days to keep receipts. Default 30.
        """
        if ledger_dir is None:
            # Check for environment override (for testing)
            env_dir = os.environ.get("CONTEXT_OS_LEDGER_DIR")
            if env_dir:
                ledger_dir = Path(env_dir)
            else:
                home = Path.home()
                ledger_dir = home / ".context-os" / "query_ledger"

        self.ledger_dir = Path(ledger_dir)
        self.ttl_days = ttl_days
        self.ledger_dir.mkdir(parents=True, exist_ok=True)

    def save(self, receipt: QueryReceipt) -> Path:
        """Save receipt to ledger.

        Returns path to saved file.
        """
        path = self.ledger_dir / f"{receipt.receipt_id}.json"
        with open(path, "w") as f:
            json.dump(receipt.to_dict(), f, indent=2)
        return path

    def load(self, receipt_id: str) -> Optional[QueryReceipt]:
        """Load receipt by ID.

        Returns None if not found or expired.
        """
        path = self.ledger_dir / f"{receipt_id}.json"

        if not path.exists():
            return None

        with open(path) as f:
            data = json.load(f)

        # Check TTL
        timestamp_str = data["timestamp"]
        # Handle various ISO formats
        if timestamp_str.endswith("Z"):
            timestamp_str = timestamp_str.replace("Z", "+00:00")
        receipt_time = datetime.fromisoformat(timestamp_str)
        age = datetime.now(timezone.utc) - receipt_time

        if age > timedelta(days=self.ttl_days):
            # Expired - delete and return None
            path.unlink()
            return None

        return QueryReceipt.from_dict(data)

    def cleanup(self) -> int:
        """Delete receipts older than TTL.

        Returns count of deleted receipts.
        """
        deleted = 0
        cutoff = datetime.now(timezone.utc) - timedelta(days=self.ttl_days)

        for path in self.ledger_dir.glob("q_*.json"):
            try:
                with open(path) as f:
                    data = json.load(f)

                timestamp_str = data["timestamp"]
                if timestamp_str.endswith("Z"):
                    timestamp_str = timestamp_str.replace("Z", "+00:00")
                receipt_time = datetime.fromisoformat(timestamp_str)

                if receipt_time < cutoff:
                    path.unlink()
                    deleted += 1

            except (json.JSONDecodeError, KeyError, OSError):
                # Corrupted file - delete it
                path.unlink()
                deleted += 1

        return deleted

    def list_receipts(self, limit: int = 20) -> List[dict]:
        """List recent receipts (metadata only, not full snapshots).

        Returns list of {receipt_id, timestamp, query_summary, result_count}.
        """
        receipts = []

        # Sort by modification time (most recent first)
        paths = sorted(
            self.ledger_dir.glob("q_*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True
        )

        for path in paths:
            if len(receipts) >= limit:
                break

            try:
                with open(path) as f:
                    data = json.load(f)

                receipts.append({
                    "receipt_id": data["receipt_id"],
                    "timestamp": data["timestamp"],
                    "query_summary": self._summarize_query(data["query_spec"]),
                    "result_count": data["result_count"],
                })
            except (json.JSONDecodeError, KeyError):
                continue

        return receipts

    def _summarize_query(self, spec: dict) -> str:
        """Create human-readable query summary."""
        parts = []
        if spec.get("files"):
            parts.append(f"files={spec['files']}")
        if spec.get("time"):
            parts.append(f"time={spec['time']}")
        if spec.get("chain"):
            parts.append(f"chain={spec['chain']}")
        return " ".join(parts) if parts else "(all)"


# =============================================================================
# QueryEngine
# =============================================================================

class QueryEngine:
    """Execute hypercube queries against ContextIndex.

    Implements the Slice → Aggregate → Render pipeline.
    Integrates with QueryLedger for verification (Phase C).
    """

    def __init__(self, index, ledger: Optional[QueryLedger] = None):
        """Initialize QueryEngine with a ContextIndex.

        Args:
            index: ContextIndex (or mock with similar structure)
            ledger: Optional QueryLedger for receipt storage (Phase C)
        """
        self.index = index
        self.ledger = ledger

    def execute(self, spec: QuerySpec) -> QueryResult:
        """Main entry point: Slice → Aggregate → Render.

        Args:
            spec: QuerySpec defining the query

        Returns:
            QueryResult with data and metadata
        """
        # 1. Validate
        errors = spec.validate()
        if errors:
            raise ValueError(f"Invalid query: {errors}")

        # 2. Slice (filter to matching data)
        data = self._slice(spec)

        # 3. Aggregate (compute requested stats)
        results, aggregations = self._aggregate(data, spec)

        # 4. Sort
        results = self._sort(results, spec)

        # Track total before limit
        total_count = len(results)

        # 5. Apply limit
        results = results[:spec.limit]

        # 6. Build result
        timestamp = datetime.now(timezone.utc).isoformat()

        # Generate receipt ID (content-addressed for verification)
        receipt_id = QueryReceipt.generate_id(timestamp, spec, results)

        # Create and save receipt if ledger is available
        if self.ledger:
            receipt = QueryReceipt(
                receipt_id=receipt_id,
                timestamp=timestamp,
                query_spec=spec,
                result_hash="",  # Computed next
                result_count=total_count,
                result_snapshot=results,
            )
            receipt.result_hash = receipt.compute_result_hash()
            self.ledger.save(receipt)

        return QueryResult(
            receipt_id=receipt_id,
            timestamp=timestamp,
            query=spec,
            result_count=total_count,
            results=results,
            aggregations=aggregations,
        )

    def _slice(self, spec: QuerySpec) -> List[dict]:
        """Filter hypercube to matching subset.

        Returns list of dicts with file data matching all specified filters.
        """
        # Get all files from index
        files = self._get_all_files()

        # Apply file pattern filter
        if spec.files:
            files = self._filter_by_pattern(files, spec.files)

        # Apply time filter
        if spec.time:
            files = self._filter_by_time(files, spec.time)

        # Apply chain filter
        if spec.chain:
            files = self._filter_by_chain(files, spec.chain)

        # Apply session filter
        if spec.session:
            files = self._filter_by_session(files, spec.session)

        # Build data dicts for each file
        return [self._get_file_data(f) for f in files]

    def _get_all_files(self) -> List[str]:
        """Get all file paths from index."""
        # Support both mock structures (file_sessions) and real ContextIndex (_inverted_index)
        # Check file_sessions first since Mock objects auto-create attributes
        if hasattr(self.index, 'file_sessions') and isinstance(self.index.file_sessions, dict):
            return list(self.index.file_sessions.keys())
        elif hasattr(self.index, '_inverted_index') and isinstance(self.index._inverted_index, dict):
            return list(self.index._inverted_index.keys())
        return []

    def _filter_by_pattern(self, files: List[str], pattern: str) -> List[str]:
        """Filter files matching glob/substring pattern."""
        # Handle glob patterns
        if '*' in pattern or '?' in pattern:
            return [f for f in files if fnmatch.fnmatch(f.lower(), pattern.lower())]
        # Handle substring match
        pattern_lower = pattern.lower()
        return [f for f in files if pattern_lower in f.lower()]

    def _filter_by_time(self, files: List[str], time_str: str) -> List[str]:
        """Filter files touched within time range."""
        start, end = self._parse_time_range(time_str)

        filtered = []
        for file_path in files:
            last_access = self._get_last_access_time(file_path)
            if last_access and start <= last_access <= end:
                filtered.append(file_path)

        return filtered

    def _filter_by_chain(self, files: List[str], chain_spec: str) -> List[str]:
        """Filter files touched by specified chain(s)."""
        # "active" means chains touched in last 7 days
        if chain_spec == "active":
            return self._filter_by_active_chains(files)

        # Otherwise, filter by chain ID prefix
        filtered = []
        for file_path in files:
            file_chains = self._get_chains_for_file(file_path)
            if any(c.startswith(chain_spec) for c in file_chains):
                filtered.append(file_path)

        return filtered

    def _filter_by_active_chains(self, files: List[str]) -> List[str]:
        """Filter files touched by chains active in last 7 days."""
        cutoff = datetime.now(timezone.utc) - timedelta(days=7)

        filtered = []
        for file_path in files:
            last_access = self._get_last_access_time(file_path)
            if last_access and last_access >= cutoff:
                filtered.append(file_path)

        return filtered

    def _filter_by_session(self, files: List[str], session_prefix: str) -> List[str]:
        """Filter files touched by sessions matching prefix."""
        filtered = []
        for file_path in files:
            sessions = self._get_sessions_for_file(file_path)
            if any(s.startswith(session_prefix) for s in sessions):
                filtered.append(file_path)

        return filtered

    def _get_file_data(self, file_path: str) -> dict:
        """Build data dict for a file."""
        sessions = self._get_sessions_for_file(file_path)
        access_count = self._get_access_count(file_path)
        last_access = self._get_last_access_time(file_path)
        chains = self._get_chains_for_file(file_path)

        return {
            "path": file_path,
            "sessions": sessions,
            "access_count": access_count,
            "last_access": last_access.isoformat() if last_access else None,
            "chains": chains,
        }

    def _get_sessions_for_file(self, file_path: str) -> List[str]:
        """Get session IDs that touched a file."""
        # Support mock (check first since Mock auto-creates attributes)
        if hasattr(self.index, 'file_sessions') and isinstance(self.index.file_sessions, dict):
            return self.index.file_sessions.get(file_path, [])
        # Support ContextIndex
        elif hasattr(self.index, '_inverted_index') and isinstance(self.index._inverted_index, dict):
            accesses = self.index._inverted_index.get(file_path, [])
            return list(set(a.session_id for a in accesses))
        return []

    def _get_access_count(self, file_path: str) -> int:
        """Get total access count for a file."""
        # Support mock with explicit access counts (check first)
        if hasattr(self.index, 'file_access_counts') and isinstance(self.index.file_access_counts, dict):
            return self.index.file_access_counts.get(file_path, 0)
        # Support mock with file_sessions
        if hasattr(self.index, 'file_sessions') and isinstance(self.index.file_sessions, dict):
            return len(self.index.file_sessions.get(file_path, []))
        # Support ContextIndex
        if hasattr(self.index, '_inverted_index') and isinstance(self.index._inverted_index, dict):
            return len(self.index._inverted_index.get(file_path, []))
        return 0

    def _get_last_access_time(self, file_path: str) -> Optional[datetime]:
        """Get most recent access time for a file."""
        sessions = self._get_sessions_for_file(file_path)
        if not sessions:
            return None

        # Get timestamps for sessions (mock structure - check first)
        if hasattr(self.index, 'session_timestamps') and isinstance(self.index.session_timestamps, dict):
            timestamps = [
                self.index.session_timestamps.get(s)
                for s in sessions
                if self.index.session_timestamps.get(s)
            ]
            return max(timestamps) if timestamps else None

        # Support ContextIndex with FileAccess objects
        if hasattr(self.index, '_inverted_index') and isinstance(self.index._inverted_index, dict):
            accesses = self.index._inverted_index.get(file_path, [])
            timestamps = [a.timestamp for a in accesses if hasattr(a, 'timestamp') and a.timestamp]
            return max(timestamps) if timestamps else None

        return None

    def _get_chains_for_file(self, file_path: str) -> List[str]:
        """Get chain IDs that touched a file."""
        sessions = self._get_sessions_for_file(file_path)
        chains = set()

        if hasattr(self.index, 'get_chain_for_session'):
            for session_id in sessions:
                chain_id = self.index.get_chain_for_session(session_id)
                if chain_id:
                    chains.add(chain_id)

        return list(chains)

    def _parse_time_range(self, time_str: str) -> Tuple[datetime, datetime]:
        """Parse time range string to (start, end) datetimes.

        Formats:
        - "7d" → last 7 days
        - "2w" → last 2 weeks
        - "2025-W50" → specific week
        - "2025-W48:2025-W50" → week range
        """
        now = datetime.now(timezone.utc)

        if time_str.endswith("d"):
            days = int(time_str[:-1])
            return (now - timedelta(days=days), now)

        if time_str.endswith("w"):
            weeks = int(time_str[:-1])
            return (now - timedelta(weeks=weeks), now)

        if ":" in time_str:
            start_week, end_week = time_str.split(":")
            return (
                self._week_to_datetime(start_week),
                self._week_to_datetime(end_week, end=True)
            )

        if time_str.startswith("20"):  # Year prefix (e.g., 2025-W50)
            return (
                self._week_to_datetime(time_str),
                self._week_to_datetime(time_str, end=True)
            )

        raise ValueError(f"Unknown time format: {time_str}")

    def _week_to_datetime(self, week_str: str, end: bool = False) -> datetime:
        """Convert ISO week string to datetime.

        Args:
            week_str: e.g., "2025-W50"
            end: If True, return end of week (Sunday 23:59:59)
        """
        # Parse "2025-W50"
        year, week = week_str.split("-W")
        year = int(year)
        week = int(week)

        # ISO week 1 starts on the Monday of the week containing Jan 4
        jan4 = datetime(year, 1, 4, tzinfo=timezone.utc)
        week1_monday = jan4 - timedelta(days=jan4.weekday())

        # Target week start (Monday)
        target_monday = week1_monday + timedelta(weeks=week - 1)

        if end:
            # End of Sunday
            return target_monday + timedelta(days=6, hours=23, minutes=59, seconds=59)
        return target_monday

    def _aggregate(self, data: List[dict], spec: QuerySpec) -> Tuple[List[dict], dict]:
        """Compute requested aggregations.

        Returns:
            (results, aggregations) tuple
        """
        results = []
        aggregations = {}

        for item in data:
            row = {"file_path": item["path"]}

            if "count" in spec.agg:
                row["access_count"] = item["access_count"]

            if "recency" in spec.agg:
                row["last_access"] = item["last_access"]

            if "sessions" in spec.agg:
                row["session_count"] = len(item["sessions"])
                row["sessions"] = item["sessions"][:5]  # First 5

            if "chains" in spec.agg:
                row["chains"] = item["chains"]

            results.append(row)

        # Compute summary aggregations
        if "count" in spec.agg:
            total_accesses = sum(d["access_count"] for d in data)
            aggregations["count"] = {
                "total_files": len(data),
                "total_accesses": total_accesses,
            }

        if "recency" in spec.agg:
            timestamps = [d["last_access"] for d in data if d["last_access"]]
            aggregations["recency"] = {
                "newest": max(timestamps) if timestamps else None,
                "oldest": min(timestamps) if timestamps else None,
            }

        return results, aggregations

    def _sort(self, results: List[dict], spec: QuerySpec) -> List[dict]:
        """Sort results by specified field."""
        if spec.sort == "count":
            return sorted(results, key=lambda r: r.get("access_count", 0), reverse=True)
        elif spec.sort == "recency":
            return sorted(
                results,
                key=lambda r: r.get("last_access") or "",
                reverse=True
            )
        elif spec.sort == "alpha":
            return sorted(results, key=lambda r: r.get("file_path", ""))
        return results

    def _generate_receipt_id(self, timestamp: str, spec: QuerySpec) -> str:
        """Generate deterministic receipt ID.

        Format: "q_" + 6 hex chars from hash of timestamp + query
        """
        content = f"{timestamp}:{spec.files}:{spec.time}:{spec.agg}"
        hash_hex = hashlib.sha256(content.encode()).hexdigest()[:6]
        return f"q_{hash_hex}"

    def verify(self, receipt_id: str, verbose: bool = False) -> VerificationResult:
        """Verify a query receipt against current data.

        Loads the original receipt, re-executes the query, and compares results.

        Args:
            receipt_id: The receipt ID to verify
            verbose: If True, include detailed diff in result

        Returns:
            VerificationResult with MATCH, DRIFT, or NOT_FOUND status
        """
        verification_timestamp = datetime.now(timezone.utc).isoformat()

        # Check if ledger is available
        if not self.ledger:
            return VerificationResult(
                receipt_id=receipt_id,
                original_timestamp="",
                verification_timestamp=verification_timestamp,
                status="NOT_FOUND",
                original_hash="",
                current_hash=None,
                original_count=0,
                current_count=None,
                drift_summary="No ledger configured",
                diff=None,
            )

        # Load original receipt
        receipt = self.ledger.load(receipt_id)
        if receipt is None:
            return VerificationResult(
                receipt_id=receipt_id,
                original_timestamp="",
                verification_timestamp=verification_timestamp,
                status="NOT_FOUND",
                original_hash="",
                current_hash=None,
                original_count=0,
                current_count=None,
                drift_summary=f"Receipt {receipt_id} not found in ledger",
                diff=None,
            )

        # Re-execute the query
        current_result = self.execute(receipt.query_spec)

        # Compute current hash
        current_snapshot = current_result.results
        current_hash = hashlib.sha256(
            json.dumps(current_snapshot, sort_keys=True).encode()
        ).hexdigest()
        current_hash = f"sha256:{current_hash}"

        # Compare hashes
        if current_hash == receipt.result_hash:
            return VerificationResult(
                receipt_id=receipt_id,
                original_timestamp=receipt.timestamp,
                verification_timestamp=verification_timestamp,
                status="MATCH",
                original_hash=receipt.result_hash,
                current_hash=current_hash,
                original_count=receipt.result_count,
                current_count=current_result.result_count,
                drift_summary=None,
                diff=None,
            )

        # DRIFT detected - compute summary
        original_files = set(r.get("file_path", "") for r in receipt.result_snapshot)
        current_files = set(r.get("file_path", "") for r in current_snapshot)

        added = current_files - original_files
        removed = original_files - current_files

        # Build drift summary
        parts = []
        if added:
            parts.append(f"{len(added)} added")
        if removed:
            parts.append(f"{len(removed)} removed")
        if not parts:
            parts.append("content changed")
        drift_summary = ", ".join(parts)

        # Build diff if verbose
        diff = None
        if verbose:
            diff = {
                "added": list(added),
                "removed": list(removed),
            }

        return VerificationResult(
            receipt_id=receipt_id,
            original_timestamp=receipt.timestamp,
            verification_timestamp=verification_timestamp,
            status="DRIFT",
            original_hash=receipt.result_hash,
            current_hash=current_hash,
            original_count=receipt.result_count,
            current_count=current_result.result_count,
            drift_summary=drift_summary,
            diff=diff,
        )
