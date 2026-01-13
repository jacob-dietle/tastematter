"""
Phase B Tests: QuerySpec + QueryEngine + query flex command

TDD Workflow:
1. Run these tests FIRST (should fail - RED)
2. Implement query_engine.py
3. Run tests again (should pass - GREEN)

Tests extracted from: specs/context_os_intelligence/12_CLI_HYPERCUBE_SPEC.md
"""

import pytest
import json
from datetime import datetime, timedelta, timezone
from click.testing import CliRunner


# =============================================================================
# UNIT TESTS: QuerySpec Validation
# =============================================================================

class TestQuerySpecValidation:
    """QuerySpec dataclass validates inputs correctly."""

    def test_query_spec_validates_invalid_aggregations(self):
        """QuerySpec.validate() rejects invalid aggregation names."""
        from context_os_events.query_engine import QuerySpec

        spec = QuerySpec(files="*.py", agg=["invalid_agg"])
        errors = spec.validate()
        assert len(errors) == 1
        assert "Invalid aggregation 'invalid_agg'" in errors[0]

    def test_query_spec_validates_multiple_invalid_aggregations(self):
        """QuerySpec.validate() catches all invalid aggregations."""
        from context_os_events.query_engine import QuerySpec

        spec = QuerySpec(agg=["invalid1", "count", "invalid2"])
        errors = spec.validate()
        assert len(errors) == 2

    def test_query_spec_accepts_all_valid_aggregations(self):
        """QuerySpec.validate() accepts all valid aggregation names."""
        from context_os_events.query_engine import QuerySpec

        spec = QuerySpec(agg=["count", "recency", "trend", "sessions", "files", "chains"])
        errors = spec.validate()
        assert errors == []

    def test_query_spec_validates_invalid_access_type(self):
        """QuerySpec.validate() rejects invalid access types."""
        from context_os_events.query_engine import QuerySpec

        spec = QuerySpec(access="xyz")
        errors = spec.validate()
        assert len(errors) == 1
        assert "Invalid access" in errors[0]

    def test_query_spec_accepts_valid_access_types(self):
        """QuerySpec.validate() accepts all valid access type combos."""
        from context_os_events.query_engine import QuerySpec

        for access in ["r", "w", "c", "rw", "rc", "wc", "rwc"]:
            spec = QuerySpec(access=access)
            errors = spec.validate()
            assert errors == [], f"Failed for access={access}"

    def test_query_spec_validates_limit_bounds(self):
        """QuerySpec.validate() rejects limit outside 1-1000."""
        from context_os_events.query_engine import QuerySpec

        spec_low = QuerySpec(limit=0)
        spec_high = QuerySpec(limit=1001)

        assert len(spec_low.validate()) == 1
        assert len(spec_high.validate()) == 1

    def test_query_spec_defaults(self):
        """QuerySpec has sensible defaults."""
        from context_os_events.query_engine import QuerySpec

        spec = QuerySpec()

        assert spec.files is None
        assert spec.time is None
        assert spec.format == "json"
        assert spec.limit == 20
        assert spec.agg == ["count"]
        assert spec.sort == "count"

    def test_query_spec_all_slices_optional(self):
        """QuerySpec allows all slice dimensions to be None."""
        from context_os_events.query_engine import QuerySpec

        spec = QuerySpec()  # No slices specified
        errors = spec.validate()
        assert errors == []


# =============================================================================
# UNIT TESTS: QueryEngine Slicing
# =============================================================================

class TestQueryEngineSlicing:
    """QueryEngine correctly slices the hypercube."""

    @pytest.fixture
    def mock_index(self):
        """Create a mock ContextIndex with test data."""
        from unittest.mock import Mock

        mock = Mock()
        # 10 files: 5 pixee, 3 nickel, 2 other
        mock.file_sessions = {
            "gtm_engagements/03_active_client/pixee_ai_gtm/docs/file1.md": ["s1", "s2"],
            "gtm_engagements/03_active_client/pixee_ai_gtm/docs/file2.md": ["s1"],
            "gtm_engagements/03_active_client/pixee_ai_gtm/docs/file3.md": ["s2", "s3"],
            "gtm_engagements/03_active_client/pixee_ai_gtm/docs/file4.md": ["s1", "s2", "s3"],
            "gtm_engagements/03_active_client/pixee_ai_gtm/docs/file5.md": ["s3"],
            "gtm_engagements/03_active_client/nickel_ivan/file1.md": ["s1"],
            "gtm_engagements/03_active_client/nickel_ivan/file2.md": ["s2"],
            "gtm_engagements/03_active_client/nickel_ivan/file3.md": ["s3"],
            "knowledge_base/technical/context-engineering.md": ["s1", "s2", "s3", "s4"],
            "knowledge_base/technical/agentic-systems.md": ["s2", "s4"],
        }

        # Session timestamps (for time filtering)
        now = datetime.now(timezone.utc)
        mock.session_timestamps = {
            "s1": now - timedelta(days=2),
            "s2": now - timedelta(days=5),
            "s3": now - timedelta(days=10),
            "s4": now - timedelta(days=20),
        }

        return mock

    def test_engine_slices_by_file_pattern(self, mock_index):
        """Engine filters files matching glob pattern."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*pixee*")

        result = engine.execute(spec)

        assert all("pixee" in r["file_path"].lower() for r in result.results)
        assert result.result_count == 5  # Only pixee files

    def test_engine_slices_by_file_extension(self, mock_index):
        """Engine filters files by extension pattern."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*.md")

        result = engine.execute(spec)

        assert all(r["file_path"].endswith(".md") for r in result.results)

    def test_engine_slices_by_time_days(self, mock_index):
        """Engine filters to sessions within last N days."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(time="7d")

        result = engine.execute(spec)

        # Files with only s1, s2 sessions (within 7 days) should be included
        # Files with only s3, s4 sessions (outside 7 days) should be excluded
        assert result.result_count > 0

    def test_engine_slices_by_time_weeks(self, mock_index):
        """Engine filters to sessions within last N weeks."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(time="2w")

        result = engine.execute(spec)

        # All sessions within 2 weeks should be included
        assert result.result_count > 0

    def test_engine_combines_multiple_slices_with_and(self, mock_index):
        """Engine applies multiple filters with AND logic."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*pixee*", time="7d")

        result = engine.execute(spec)

        # Results match BOTH criteria
        for r in result.results:
            assert "pixee" in r["file_path"].lower()

    def test_engine_empty_slice_returns_all(self, mock_index):
        """Engine with no slices returns all files."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec()  # No filters

        result = engine.execute(spec)

        assert result.result_count == 10  # All mock files


# =============================================================================
# UNIT TESTS: QueryEngine Aggregation
# =============================================================================

class TestQueryEngineAggregation:
    """QueryEngine correctly computes aggregations."""

    @pytest.fixture
    def mock_index(self):
        """Create a mock ContextIndex with test data."""
        from unittest.mock import Mock

        mock = Mock()
        now = datetime.now(timezone.utc)

        mock.file_sessions = {
            "file1.py": ["s1", "s2", "s3"],
            "file2.py": ["s1"],
            "file3.py": ["s1", "s2"],
        }

        mock.session_timestamps = {
            "s1": now - timedelta(days=1),
            "s2": now - timedelta(days=3),
            "s3": now - timedelta(days=5),
        }

        mock.file_access_counts = {
            "file1.py": 10,
            "file2.py": 3,
            "file3.py": 5,
        }

        return mock

    def test_engine_aggregates_count(self, mock_index):
        """Engine includes access_count when count aggregation requested."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", agg=["count"])

        result = engine.execute(spec)

        assert all("access_count" in r for r in result.results)
        assert "count" in result.aggregations
        assert "total_files" in result.aggregations["count"]
        assert "total_accesses" in result.aggregations["count"]

    def test_engine_aggregates_recency(self, mock_index):
        """Engine includes last_access when recency aggregation requested."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", agg=["recency"])

        result = engine.execute(spec)

        assert all("last_access" in r for r in result.results)
        assert "recency" in result.aggregations
        assert "newest" in result.aggregations["recency"]
        assert "oldest" in result.aggregations["recency"]

    def test_engine_aggregates_sessions(self, mock_index):
        """Engine includes session list when sessions aggregation requested."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", agg=["sessions"])

        result = engine.execute(spec)

        assert all("session_count" in r for r in result.results)
        assert all("sessions" in r for r in result.results)

    def test_engine_multiple_aggregations(self, mock_index):
        """Engine handles multiple aggregations simultaneously."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", agg=["count", "recency", "sessions"])

        result = engine.execute(spec)

        assert all("access_count" in r for r in result.results)
        assert all("last_access" in r for r in result.results)
        assert all("session_count" in r for r in result.results)

    def test_engine_sorts_by_count_descending(self, mock_index):
        """Engine sorts results by count (highest first) by default."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", agg=["count"], sort="count")

        result = engine.execute(spec)

        counts = [r["access_count"] for r in result.results]
        assert counts == sorted(counts, reverse=True)

    def test_engine_sorts_by_recency(self, mock_index):
        """Engine sorts by last_access when sort=recency."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", agg=["recency"], sort="recency")

        result = engine.execute(spec)

        timestamps = [r["last_access"] for r in result.results]
        assert timestamps == sorted(timestamps, reverse=True)

    def test_engine_respects_limit(self, mock_index):
        """Engine limits results to specified count."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", limit=2)

        result = engine.execute(spec)

        assert len(result.results) == 2
        assert result.result_count == 3  # Total before limit


# =============================================================================
# UNIT TESTS: QueryEngine Rendering
# =============================================================================

class TestQueryEngineRendering:
    """QueryEngine correctly renders output."""

    @pytest.fixture
    def mock_index(self):
        """Create a mock ContextIndex with test data."""
        from unittest.mock import Mock

        mock = Mock()
        now = datetime.now(timezone.utc)

        mock.file_sessions = {
            "file1.py": ["s1", "s2"],
            "file2.py": ["s1"],
        }

        mock.session_timestamps = {
            "s1": now - timedelta(days=1),
            "s2": now - timedelta(days=3),
        }

        mock.file_access_counts = {
            "file1.py": 5,
            "file2.py": 2,
        }

        return mock

    def test_engine_result_to_json_valid(self, mock_index):
        """QueryResult.to_json() produces valid JSON."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", format="json")

        result = engine.execute(spec)
        output = result.to_json()

        parsed = json.loads(output)  # Should not raise
        assert "receipt_id" in parsed
        assert "timestamp" in parsed
        assert "results" in parsed
        assert "query" in parsed

    def test_engine_result_json_contains_receipt(self, mock_index):
        """JSON output includes receipt_id for verification."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*")

        result = engine.execute(spec)
        parsed = json.loads(result.to_json())

        assert parsed["receipt_id"].startswith("q_")
        assert len(parsed["receipt_id"]) == 8  # "q_" + 6 chars

    def test_engine_result_json_contains_query(self, mock_index):
        """JSON output includes original query for reproducibility."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*pixee*", time="7d", agg=["count", "recency"])

        result = engine.execute(spec)
        parsed = json.loads(result.to_json())

        assert parsed["query"]["files"] == "*pixee*"
        assert parsed["query"]["time"] == "7d"
        assert parsed["query"]["agg"] == ["count", "recency"]

    def test_engine_result_to_table_contains_borders(self, mock_index):
        """QueryResult.to_table() produces Rich table format."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec(files="*", format="table")

        result = engine.execute(spec)
        output = result.to_table()

        # Rich tables use box-drawing characters
        assert "│" in output or "|" in output  # Table borders

    def test_engine_result_timestamp_is_iso(self, mock_index):
        """Result timestamp is ISO 8601 format."""
        from context_os_events.query_engine import QueryEngine, QuerySpec

        engine = QueryEngine(mock_index)
        spec = QuerySpec()

        result = engine.execute(spec)

        # Should parse without error
        datetime.fromisoformat(result.timestamp.replace("Z", "+00:00"))


# =============================================================================
# INTEGRATION TESTS: CLI query flex Command
# =============================================================================

class TestQueryFlexCommand:
    """CLI query flex command works correctly."""

    @pytest.fixture
    def cli_runner(self):
        return CliRunner()

    @pytest.fixture
    def project_runner(self, cli_runner):
        """CLI runner from correct project directory."""
        import os
        original_cwd = os.getcwd()
        os.chdir(r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system")
        yield cli_runner
        os.chdir(original_cwd)

    def test_cli_query_flex_accepts_all_flags(self, project_runner):
        """query flex command accepts all slice/agg/render flags."""
        from context_os_events.cli import cli

        result = project_runner.invoke(cli, [
            "query", "flex",
            "--files", "*pixee*",
            "--time", "7d",
            "--agg", "count,recency",
            "--format", "json",
            "--limit", "10"
        ])

        assert result.exit_code == 0, f"Command failed: {result.output}"

    def test_cli_query_flex_json_output_valid(self, project_runner):
        """query flex --format json produces valid JSON."""
        from context_os_events.cli import cli

        result = project_runner.invoke(cli, [
            "query", "flex",
            "--files", "*",
            "--format", "json"
        ])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        parsed = json.loads(result.output)
        assert "receipt_id" in parsed
        assert "results" in parsed

    def test_cli_query_flex_default_format_is_json(self, project_runner):
        """query flex defaults to JSON format."""
        from context_os_events.cli import cli

        result = project_runner.invoke(cli, [
            "query", "flex",
            "--files", "*"
        ])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should be valid JSON (default format)
        json.loads(result.output)

    def test_cli_query_flex_table_format(self, project_runner):
        """query flex --format table produces table output."""
        from context_os_events.cli import cli

        result = project_runner.invoke(cli, [
            "query", "flex",
            "--files", "*",
            "--format", "table"
        ])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should NOT be valid JSON (it's a table)
        with pytest.raises(json.JSONDecodeError):
            json.loads(result.output)

    def test_cli_query_flex_with_time_filter(self, project_runner):
        """query flex --time works correctly."""
        from context_os_events.cli import cli

        result = project_runner.invoke(cli, [
            "query", "flex",
            "--time", "7d",
            "--format", "json"
        ])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        data = json.loads(result.output)
        assert "results" in data

    def test_cli_query_flex_with_multiple_aggs(self, project_runner):
        """query flex --agg count,recency works correctly."""
        from context_os_events.cli import cli

        result = project_runner.invoke(cli, [
            "query", "flex",
            "--files", "*",
            "--agg", "count,recency",
            "--format", "json"
        ])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        data = json.loads(result.output)
        assert "aggregations" in data
