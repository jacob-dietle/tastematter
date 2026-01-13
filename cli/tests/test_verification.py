"""
Tests for Phase C: Verification Layer.

Following TDD: These tests are written BEFORE implementation.
Run first to confirm RED (all fail), then implement to GREEN (all pass).

Test Categories:
1. QueryReceipt - Receipt ID generation and hashing (10 tests)
2. QueryLedger - Storage with 30-day TTL (7 tests)
3. Verification - Match/Drift detection (6 tests)
4. E2E - Full CLI workflow (5 tests)

Total: 28 tests
"""

import json
import pytest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest.mock import Mock


# =============================================================================
# QueryReceipt Tests (10 tests)
# =============================================================================

class TestQueryReceipt:
    """Tests for QueryReceipt dataclass."""

    def test_receipt_id_is_deterministic(self):
        """Same query at same timestamp produces same receipt_id."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        ts = "2025-12-21T21:45:00Z"
        spec = QuerySpec(files="*pixee*")
        results = [{"file_path": "a.py"}, {"file_path": "b.py"}]

        id1 = QueryReceipt.generate_id(ts, spec, results)
        id2 = QueryReceipt.generate_id(ts, spec, results)

        assert id1 == id2

    def test_receipt_id_starts_with_prefix(self):
        """Receipt ID starts with 'q_' prefix."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        ts = "2025-12-21T21:45:00Z"
        spec = QuerySpec(files="*pixee*")
        results = []

        receipt_id = QueryReceipt.generate_id(ts, spec, results)

        assert receipt_id.startswith("q_")

    def test_receipt_id_has_correct_length(self):
        """Receipt ID is 8 characters: q_ + 6 hex chars."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        ts = "2025-12-21T21:45:00Z"
        spec = QuerySpec(files="*pixee*")
        results = []

        receipt_id = QueryReceipt.generate_id(ts, spec, results)

        assert len(receipt_id) == 8

    def test_receipt_id_differs_for_different_queries(self):
        """Different queries produce different receipt_ids."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        ts = "2025-12-21T21:45:00Z"
        results = []

        id1 = QueryReceipt.generate_id(ts, QuerySpec(files="*pixee*"), results)
        id2 = QueryReceipt.generate_id(ts, QuerySpec(files="*nickel*"), results)

        assert id1 != id2

    def test_receipt_id_differs_for_different_timestamps(self):
        """Different timestamps produce different receipt_ids."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec(files="*pixee*")
        results = []

        id1 = QueryReceipt.generate_id("2025-12-21T21:45:00Z", spec, results)
        id2 = QueryReceipt.generate_id("2025-12-21T21:46:00Z", spec, results)

        assert id1 != id2

    def test_receipt_hash_is_deterministic(self):
        """Same results produce same hash."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec(files="*pixee*")
        results = [{"a": 1}, {"b": 2}]

        receipt = QueryReceipt(
            receipt_id="q_test01",
            timestamp="2025-12-21T21:45:00Z",
            query_spec=spec,
            result_hash="",  # Will compute
            result_count=2,
            result_snapshot=results,
        )

        hash1 = receipt.compute_result_hash()
        hash2 = receipt.compute_result_hash()

        assert hash1 == hash2

    def test_receipt_hash_starts_with_sha256(self):
        """Result hash starts with 'sha256:' prefix."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec()
        receipt = QueryReceipt(
            receipt_id="q_test01",
            timestamp="2025-12-21T21:45:00Z",
            query_spec=spec,
            result_hash="",
            result_count=0,
            result_snapshot=[],
        )

        hash_val = receipt.compute_result_hash()

        assert hash_val.startswith("sha256:")

    def test_receipt_hash_differs_for_different_results(self):
        """Different results produce different hashes."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec()

        receipt1 = QueryReceipt(
            receipt_id="q_test01",
            timestamp="2025-12-21T21:45:00Z",
            query_spec=spec,
            result_hash="",
            result_count=1,
            result_snapshot=[{"a": 1}],
        )
        receipt2 = QueryReceipt(
            receipt_id="q_test02",
            timestamp="2025-12-21T21:45:00Z",
            query_spec=spec,
            result_hash="",
            result_count=1,
            result_snapshot=[{"a": 2}],
        )

        assert receipt1.compute_result_hash() != receipt2.compute_result_hash()

    def test_receipt_serialization_roundtrip(self):
        """Receipt survives to_dict/from_dict roundtrip."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec(files="*pixee*", time="7d", agg=["count", "recency"])
        original = QueryReceipt(
            receipt_id="q_test01",
            timestamp="2025-12-21T21:45:00Z",
            query_spec=spec,
            result_hash="sha256:abc123",
            result_count=5,
            result_snapshot=[{"file": "a.py"}, {"file": "b.py"}],
        )

        serialized = original.to_dict()
        restored = QueryReceipt.from_dict(serialized)

        assert restored.receipt_id == original.receipt_id
        assert restored.timestamp == original.timestamp
        assert restored.result_hash == original.result_hash
        assert restored.result_count == original.result_count
        assert restored.query_spec.files == original.query_spec.files
        assert restored.query_spec.time == original.query_spec.time

    def test_receipt_to_dict_includes_all_fields(self):
        """to_dict includes all required fields."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec(files="*test*", time="7d", agg=["count"])
        receipt = QueryReceipt(
            receipt_id="q_abc123",
            timestamp="2025-12-21T21:45:00Z",
            query_spec=spec,
            result_hash="sha256:xyz789",
            result_count=10,
            result_snapshot=[{"file": "test.py"}],
        )

        d = receipt.to_dict()

        assert "receipt_id" in d
        assert "timestamp" in d
        assert "query_spec" in d
        assert "result_hash" in d
        assert "result_count" in d
        assert "result_snapshot" in d


# =============================================================================
# QueryLedger Tests (7 tests)
# =============================================================================

class TestQueryLedger:
    """Tests for QueryLedger storage class."""

    @pytest.fixture
    def tmp_ledger(self, tmp_path):
        """Create a temporary ledger directory."""
        from context_os_events.query_engine import QueryLedger

        return QueryLedger(ledger_dir=tmp_path, ttl_days=30)

    @pytest.fixture
    def sample_receipt(self):
        """Create a sample receipt for testing."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt

        spec = QuerySpec(files="*pixee*", agg=["count"])
        return QueryReceipt(
            receipt_id="q_abc123",
            timestamp=datetime.now(timezone.utc).isoformat(),
            query_spec=spec,
            result_hash="sha256:test",
            result_count=10,
            result_snapshot=[{"file": "test.py"}],
        )

    def test_ledger_saves_receipt(self, tmp_ledger, sample_receipt):
        """Receipts are saved to ledger directory."""
        path = tmp_ledger.save(sample_receipt)

        assert path.exists()
        assert path.name == "q_abc123.json"

    def test_ledger_loads_receipt(self, tmp_ledger, sample_receipt):
        """Receipts can be loaded by ID."""
        tmp_ledger.save(sample_receipt)
        loaded = tmp_ledger.load("q_abc123")

        assert loaded is not None
        assert loaded.receipt_id == sample_receipt.receipt_id
        assert loaded.result_hash == sample_receipt.result_hash
        assert loaded.result_count == sample_receipt.result_count

    def test_ledger_returns_none_for_missing(self, tmp_ledger):
        """Loading non-existent receipt returns None."""
        loaded = tmp_ledger.load("q_nonexistent")

        assert loaded is None

    def test_ledger_ttl_expires_old_receipts(self, tmp_path):
        """Receipts older than TTL are treated as expired."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt, QueryLedger

        ledger = QueryLedger(ledger_dir=tmp_path, ttl_days=30)
        spec = QuerySpec()
        old_time = datetime.now(timezone.utc) - timedelta(days=31)

        receipt = QueryReceipt(
            receipt_id="q_old123",
            timestamp=old_time.isoformat(),
            query_spec=spec,
            result_hash="sha256:old",
            result_count=5,
            result_snapshot=[],
        )

        ledger.save(receipt)
        loaded = ledger.load("q_old123")

        assert loaded is None  # Expired

    def test_ledger_keeps_recent_receipts(self, tmp_path):
        """Receipts within TTL are kept."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt, QueryLedger

        ledger = QueryLedger(ledger_dir=tmp_path, ttl_days=30)
        spec = QuerySpec()
        recent_time = datetime.now(timezone.utc) - timedelta(days=15)

        receipt = QueryReceipt(
            receipt_id="q_recent",
            timestamp=recent_time.isoformat(),
            query_spec=spec,
            result_hash="sha256:recent",
            result_count=5,
            result_snapshot=[],
        )

        ledger.save(receipt)
        loaded = ledger.load("q_recent")

        assert loaded is not None

    def test_ledger_cleanup_deletes_old(self, tmp_path):
        """Cleanup removes receipts past TTL."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt, QueryLedger

        ledger = QueryLedger(ledger_dir=tmp_path, ttl_days=30)
        spec = QuerySpec()

        # Create old receipt (31 days ago)
        old_time = datetime.now(timezone.utc) - timedelta(days=31)
        old_receipt = QueryReceipt(
            receipt_id="q_old123",
            timestamp=old_time.isoformat(),
            query_spec=spec,
            result_hash="sha256:old",
            result_count=5,
            result_snapshot=[],
        )
        ledger.save(old_receipt)

        # Create recent receipt
        recent_receipt = QueryReceipt(
            receipt_id="q_recent",
            timestamp=datetime.now(timezone.utc).isoformat(),
            query_spec=spec,
            result_hash="sha256:recent",
            result_count=5,
            result_snapshot=[],
        )
        ledger.save(recent_receipt)

        deleted_count = ledger.cleanup()

        assert deleted_count == 1
        assert not (tmp_path / "q_old123.json").exists()
        assert (tmp_path / "q_recent.json").exists()

    def test_ledger_list_receipts_returns_recent(self, tmp_path):
        """list_receipts returns metadata for recent receipts."""
        from context_os_events.query_engine import QuerySpec, QueryReceipt, QueryLedger

        ledger = QueryLedger(ledger_dir=tmp_path, ttl_days=30)
        spec = QuerySpec(files="*test*")

        for i in range(5):
            receipt = QueryReceipt(
                receipt_id=f"q_test{i:02d}",
                timestamp=datetime.now(timezone.utc).isoformat(),
                query_spec=spec,
                result_hash=f"sha256:hash{i}",
                result_count=i * 10,
                result_snapshot=[],
            )
            ledger.save(receipt)

        receipts = ledger.list_receipts(limit=3)

        assert len(receipts) == 3
        assert all("receipt_id" in r for r in receipts)
        assert all("timestamp" in r for r in receipts)
        assert all("result_count" in r for r in receipts)


# =============================================================================
# Verification Tests (6 tests)
# =============================================================================

class TestVerification:
    """Tests for QueryEngine.verify() method."""

    @pytest.fixture
    def mock_index(self):
        """Create a mock index for testing."""
        mock = Mock()
        mock.file_sessions = {
            "pixee_file1.py": ["sess_001", "sess_002"],
            "pixee_file2.py": ["sess_001"],
            "other_file.py": ["sess_003"],
        }
        mock.file_access_counts = {
            "pixee_file1.py": 5,
            "pixee_file2.py": 3,
            "other_file.py": 2,
        }
        mock.session_timestamps = {
            "sess_001": datetime.now(timezone.utc) - timedelta(days=1),
            "sess_002": datetime.now(timezone.utc) - timedelta(days=2),
            "sess_003": datetime.now(timezone.utc) - timedelta(days=3),
        }
        return mock

    @pytest.fixture
    def verification_setup(self, tmp_path, mock_index):
        """Set up engine and ledger for verification tests."""
        from context_os_events.query_engine import QueryLedger, QueryEngine

        ledger = QueryLedger(ledger_dir=tmp_path, ttl_days=30)
        engine = QueryEngine(mock_index, ledger=ledger)
        return engine, ledger

    def test_verify_match_when_data_unchanged(self, verification_setup):
        """Verification returns MATCH when data hasn't changed."""
        from context_os_events.query_engine import QuerySpec

        engine, ledger = verification_setup

        # Run query, get receipt
        spec = QuerySpec(files="*pixee*")
        result = engine.execute(spec)
        receipt_id = result.receipt_id

        # Verify immediately (no data changes)
        verification = engine.verify(receipt_id)

        assert verification.status == "MATCH"
        assert verification.original_hash == verification.current_hash
        assert verification.drift_summary is None

    def test_verify_drift_when_data_changed(self, verification_setup, mock_index):
        """Verification returns DRIFT when results differ."""
        from context_os_events.query_engine import QuerySpec

        engine, ledger = verification_setup

        # Run query, get receipt
        spec = QuerySpec(files="*pixee*")
        result = engine.execute(spec)
        receipt_id = result.receipt_id
        original_count = result.result_count

        # Modify underlying data (add new file)
        mock_index.file_sessions["new_pixee_file.py"] = ["sess_new"]
        mock_index.file_access_counts["new_pixee_file.py"] = 1
        mock_index.session_timestamps["sess_new"] = datetime.now(timezone.utc)

        # Verify (data changed)
        verification = engine.verify(receipt_id)

        assert verification.status == "DRIFT"
        assert verification.original_count == original_count
        assert verification.current_count == original_count + 1
        assert verification.drift_summary is not None

    def test_verify_not_found_for_missing_receipt(self, verification_setup):
        """Verification returns NOT_FOUND for unknown receipt."""
        engine, ledger = verification_setup

        verification = engine.verify("q_nonexistent")

        assert verification.status == "NOT_FOUND"
        assert verification.current_hash is None

    def test_verify_includes_original_timestamp(self, verification_setup):
        """Verification result includes original query timestamp."""
        from context_os_events.query_engine import QuerySpec

        engine, ledger = verification_setup

        spec = QuerySpec(files="*")
        result = engine.execute(spec)

        verification = engine.verify(result.receipt_id)

        assert verification.original_timestamp == result.timestamp

    def test_verify_verbose_shows_diff(self, verification_setup, mock_index):
        """Verification with verbose=True shows detailed diff."""
        from context_os_events.query_engine import QuerySpec

        engine, ledger = verification_setup

        # Run query
        spec = QuerySpec(files="*pixee*")
        result = engine.execute(spec)
        receipt_id = result.receipt_id

        # Add new file
        mock_index.file_sessions["new_pixee_doc.md"] = ["sess_new"]
        mock_index.file_access_counts["new_pixee_doc.md"] = 1
        mock_index.session_timestamps["sess_new"] = datetime.now(timezone.utc)

        # Verify with verbose
        verification = engine.verify(receipt_id, verbose=True)

        assert verification.status == "DRIFT"
        assert verification.diff is not None
        assert "added" in verification.diff

    def test_verify_drift_summary_describes_changes(self, verification_setup, mock_index):
        """Drift summary describes what changed."""
        from context_os_events.query_engine import QuerySpec

        engine, ledger = verification_setup

        spec = QuerySpec(files="*pixee*")
        result = engine.execute(spec)

        # Add 2 files
        mock_index.file_sessions["new1_pixee.py"] = ["sess"]
        mock_index.file_access_counts["new1_pixee.py"] = 1
        mock_index.file_sessions["new2_pixee.py"] = ["sess"]
        mock_index.file_access_counts["new2_pixee.py"] = 1
        mock_index.session_timestamps["sess"] = datetime.now(timezone.utc)

        verification = engine.verify(result.receipt_id)

        assert verification.status == "DRIFT"
        # Summary should mention the change
        summary_lower = verification.drift_summary.lower()
        assert "new" in summary_lower or "added" in summary_lower or "2" in summary_lower


# =============================================================================
# E2E Tests: Full CLI Workflow (5 tests)
# =============================================================================

class TestVerificationE2E:
    """End-to-end tests for verify CLI commands.

    These tests mock build_index_from_jsonl to avoid requiring real Claude data.
    """

    @pytest.fixture
    def runner(self):
        """Create CLI test runner."""
        from click.testing import CliRunner

        return CliRunner()

    @pytest.fixture
    def mock_cli_index(self, monkeypatch):
        """Mock build_index_from_jsonl to return a test index."""
        mock_idx = Mock()
        mock_idx.file_sessions = {
            "pixee_file1.py": ["sess_001", "sess_002"],
            "pixee_file2.py": ["sess_001"],
            "nickel_file.py": ["sess_003"],
            "other.py": ["sess_002"],
        }
        mock_idx.file_access_counts = {
            "pixee_file1.py": 5,
            "pixee_file2.py": 3,
            "nickel_file.py": 2,
            "other.py": 1,
        }
        mock_idx.session_timestamps = {
            "sess_001": datetime.now(timezone.utc) - timedelta(days=1),
            "sess_002": datetime.now(timezone.utc) - timedelta(days=2),
            "sess_003": datetime.now(timezone.utc) - timedelta(days=3),
        }

        # Patch build_index_from_jsonl to return mock index
        monkeypatch.setattr(
            "context_os_events.cli.build_index_from_jsonl",
            lambda: mock_idx
        )
        return mock_idx

    def test_e2e_query_receipt_verify_cycle(self, runner, tmp_path, monkeypatch, mock_cli_index):
        """Complete workflow: query → receipt → verify."""
        from context_os_events.cli import cli

        # Patch ledger directory to use temp path
        monkeypatch.setenv("CONTEXT_OS_LEDGER_DIR", str(tmp_path))

        # Run query
        result = runner.invoke(cli, [
            "query", "flex",
            "--files", "*",
            "--format", "json"
        ])

        assert result.exit_code == 0, f"Exit code: {result.exit_code}, Output: {result.output}"
        data = json.loads(result.output)
        receipt_id = data["receipt_id"]

        assert receipt_id.startswith("q_")

        # Verify
        verify_result = runner.invoke(cli, [
            "query", "verify", receipt_id
        ])

        assert verify_result.exit_code == 0
        assert "MATCH" in verify_result.output or "verified" in verify_result.output.lower()

    def test_e2e_verify_unknown_receipt(self, runner, tmp_path, monkeypatch, mock_cli_index):
        """Verify command handles unknown receipt gracefully."""
        from context_os_events.cli import cli

        # Patch ledger directory to use temp path
        monkeypatch.setenv("CONTEXT_OS_LEDGER_DIR", str(tmp_path))

        result = runner.invoke(cli, [
            "query", "verify", "q_unknwn"
        ])

        assert result.exit_code == 0  # Not an error, just not found
        assert "NOT_FOUND" in result.output or "not found" in result.output.lower()

    def test_e2e_list_receipts(self, runner, tmp_path, monkeypatch, mock_cli_index):
        """List receipts shows recent queries."""
        from context_os_events.cli import cli

        # Patch ledger directory to use temp path
        monkeypatch.setenv("CONTEXT_OS_LEDGER_DIR", str(tmp_path))

        # Run a few queries
        for pattern in ["*pixee*", "*nickel*", "*.py"]:
            runner.invoke(cli, ["query", "flex", "--files", pattern, "--format", "json"])

        # List receipts
        result = runner.invoke(cli, ["query", "receipts", "--limit", "5"])

        assert result.exit_code == 0

    def test_e2e_verify_json_output(self, runner, tmp_path, monkeypatch, mock_cli_index):
        """Verify command supports JSON output."""
        from context_os_events.cli import cli

        # Patch ledger directory to use temp path
        monkeypatch.setenv("CONTEXT_OS_LEDGER_DIR", str(tmp_path))

        # Run query
        query_result = runner.invoke(cli, [
            "query", "flex", "--files", "*", "--format", "json"
        ])
        assert query_result.exit_code == 0, f"Output: {query_result.output}"
        data = json.loads(query_result.output)
        receipt_id = data["receipt_id"]

        # Verify with JSON output
        verify_result = runner.invoke(cli, [
            "query", "verify", receipt_id, "--format", "json"
        ])

        assert verify_result.exit_code == 0
        verify_data = json.loads(verify_result.output)
        assert "status" in verify_data
        assert "receipt_id" in verify_data

    def test_e2e_agent_citation_workflow(self, runner, tmp_path, monkeypatch, mock_cli_index):
        """Agent can cite receipts and user can verify claims."""
        from context_os_events.cli import cli

        # Patch ledger directory to use temp path
        monkeypatch.setenv("CONTEXT_OS_LEDGER_DIR", str(tmp_path))

        # Agent runs query
        result = runner.invoke(cli, [
            "query", "flex",
            "--files", "*",
            "--format", "json"
        ])
        assert result.exit_code == 0, f"Output: {result.output}"
        data = json.loads(result.output)

        # Agent could cite: "Found {count} files [receipt_id]"
        receipt_id = data["receipt_id"]
        count = data["result_count"]

        # Agent's claim
        claim = f"Found {count} files [{receipt_id}]"
        assert receipt_id in claim

        # User verifies
        verify_result = runner.invoke(cli, ["query", "verify", receipt_id])

        assert verify_result.exit_code == 0
        assert "MATCH" in verify_result.output or "verified" in verify_result.output.lower()
