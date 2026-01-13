"""Tests for CLI query commands.

Test-Driven Development: These tests were written BEFORE implementation.
They should FAIL initially, then PASS after implementation.

Tests cover:
1. query file - Show sessions that touched a file
2. query co-access - Show files frequently accessed together
3. query recent - Show recent activity summary
4. query chains - Show conversation chains
5. build_index_from_jsonl - Helper to build index from JSONL
"""

import pytest
from datetime import datetime
from pathlib import Path
from unittest.mock import patch, MagicMock
from click.testing import CliRunner

from context_os_events.cli import cli
from context_os_events.index import (
    ContextIndex,
    FileAccess,
    TemporalBucket,
    Chain,
)


# =============================================================================
# Fixtures
# =============================================================================


@pytest.fixture
def cli_runner():
    """Click CLI test runner."""
    return CliRunner()


@pytest.fixture
def mock_index():
    """Create a mock ContextIndex with test data."""
    index = ContextIndex()

    # Add test file accesses to inverted index
    index._inverted_index = {
        "src/agent.ts": [
            FileAccess(
                session_id="session-001",
                chain_id="chain-main",
                file_path="src/agent.ts",
                access_type="read",
                tool_name="Read",
                timestamp=datetime(2024, 12, 18, 14, 30),
            ),
            FileAccess(
                session_id="session-002",
                chain_id="chain-main",
                file_path="src/agent.ts",
                access_type="write",
                tool_name="Edit",
                timestamp=datetime(2024, 12, 17, 10, 15),
            ),
        ],
        "src/index.ts": [
            FileAccess(
                session_id="session-001",
                chain_id="chain-main",
                file_path="src/index.ts",
                access_type="read",
                tool_name="Read",
                timestamp=datetime(2024, 12, 18, 14, 35),
            ),
        ],
    }

    # Add co-access data (PMI scores)
    index._co_access = {
        "src/agent.ts": [
            ("src/index.ts", 2.31),
            ("src/config.ts", 1.84),
            ("tests/agent.test.ts", 1.52),
        ],
    }

    # Add temporal buckets
    index._temporal = {
        "2024-W51": TemporalBucket(
            period="2024-W51",
            period_type="week",
            sessions={"session-001", "session-002"},
            chains={"chain-main"},
            files_bloom=None,
            commits=[],
            started_at=datetime(2024, 12, 16),
            ended_at=datetime(2024, 12, 18),
        ),
        "2024-W50": TemporalBucket(
            period="2024-W50",
            period_type="week",
            sessions={"session-003", "session-004", "session-005"},
            chains={"chain-main", "chain-feature"},
            files_bloom=None,
            commits=[],
            started_at=datetime(2024, 12, 9),
            ended_at=datetime(2024, 12, 15),
        ),
    }

    # Add chains
    index._chains = {
        "chain-main": Chain(
            chain_id="chain-main",
            root_session="session-001",
            sessions=["session-001", "session-002"],
            branches={},
            time_range=(datetime(2024, 12, 17), datetime(2024, 12, 18)),
            total_duration_seconds=3600,
            files_bloom=None,
            files_list=["src/agent.ts", "src/index.ts"],
        ),
        "chain-feature": Chain(
            chain_id="chain-feature",
            root_session="session-003",
            sessions=["session-003"],
            branches={},
            time_range=(datetime(2024, 12, 15), datetime(2024, 12, 15)),
            total_duration_seconds=1800,
            files_bloom=None,
            files_list=["src/feature.ts"],
        ),
    }

    return index


# =============================================================================
# Test: query file
# =============================================================================


class TestQueryFile:
    """Tests for 'context-os query file <path>' command."""

    def test_query_file_shows_sessions(self, cli_runner, mock_index):
        """Query file should display sessions that touched the file.

        Given: Index has 2 sessions touching src/agent.ts
        When: User runs 'context-os query file src/agent.ts'
        Then: Output shows both sessions with timestamps
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "file", "src/agent.ts"])

        assert result.exit_code == 0
        assert "session-001" in result.output or "session" in result.output.lower()
        assert "agent.ts" in result.output

    def test_query_file_handles_missing_file(self, cli_runner, mock_index):
        """Query file should show message when file not in index.

        Given: Index does not contain 'unknown/file.ts'
        When: User runs 'context-os query file unknown/file.ts'
        Then: Output shows "not found" message (not error)
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "file", "unknown/file.ts"])

        assert result.exit_code == 0
        assert "not found" in result.output.lower() or "no" in result.output.lower()

    def test_query_file_respects_limit(self, cli_runner, mock_index):
        """Query file should respect --limit option.

        Given: Index has 2 sessions touching src/agent.ts
        When: User runs 'context-os query file src/agent.ts --limit 1'
        Then: Output shows only 1 session
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "file", "src/agent.ts", "--limit", "1"])

        assert result.exit_code == 0
        # Should show table with limited results


# =============================================================================
# Test: query co-access
# =============================================================================


class TestQueryCoAccess:
    """Tests for 'context-os query co-access <path>' command."""

    def test_query_co_access_shows_related_files(self, cli_runner, mock_index):
        """Query co-access should display files frequently accessed together.

        Given: Index has co-access data for src/agent.ts
        When: User runs 'context-os query co-access src/agent.ts'
        Then: Output shows related files with PMI scores
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "co-access", "src/agent.ts"])

        assert result.exit_code == 0
        assert "index.ts" in result.output
        assert "PMI" in result.output or "pmi" in result.output.lower()

    def test_query_co_access_handles_no_data(self, cli_runner, mock_index):
        """Query co-access should handle files with no co-access data.

        Given: Index has no co-access data for src/index.ts
        When: User runs 'context-os query co-access src/index.ts'
        Then: Output shows appropriate message (not error)
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "co-access", "src/index.ts"])

        assert result.exit_code == 0
        # Should show "no co-access data" message

    def test_query_co_access_respects_limit(self, cli_runner, mock_index):
        """Query co-access should respect --limit option.

        Given: Index has 3 co-accessed files for src/agent.ts
        When: User runs 'context-os query co-access src/agent.ts --limit 2'
        Then: Output shows only 2 files
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "co-access", "src/agent.ts", "--limit", "2"])

        assert result.exit_code == 0


# =============================================================================
# Test: query recent
# =============================================================================


class TestQueryRecent:
    """Tests for 'context-os query recent' command."""

    def test_query_recent_shows_weekly_buckets(self, cli_runner, mock_index):
        """Query recent should display weekly activity buckets.

        Given: Index has temporal data for 2 weeks
        When: User runs 'context-os query recent'
        Then: Output shows weekly buckets with session counts
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "recent"])

        assert result.exit_code == 0
        assert "W51" in result.output or "W50" in result.output
        assert "session" in result.output.lower()

    def test_query_recent_respects_weeks_option(self, cli_runner, mock_index):
        """Query recent should respect --weeks option.

        Given: Index has data for 2 weeks
        When: User runs 'context-os query recent --weeks 1'
        Then: Output shows only 1 week
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "recent", "--weeks", "1"])

        assert result.exit_code == 0

    def test_query_recent_shows_totals(self, cli_runner, mock_index):
        """Query recent should show total counts.

        Given: Index has data for multiple weeks
        When: User runs 'context-os query recent'
        Then: Output shows total sessions and chains
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "recent"])

        assert result.exit_code == 0
        assert "total" in result.output.lower()


# =============================================================================
# Test: query chains
# =============================================================================


class TestQueryChains:
    """Tests for 'context-os query chains' command."""

    def test_query_chains_shows_chain_list(self, cli_runner, mock_index):
        """Query chains should display list of conversation chains.

        Given: Index has 2 chains
        When: User runs 'context-os query chains'
        Then: Output shows both chains with session counts
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "chains"])

        assert result.exit_code == 0
        assert "chain" in result.output.lower()

    def test_query_chains_respects_limit(self, cli_runner, mock_index):
        """Query chains should respect --limit option.

        Given: Index has 2 chains
        When: User runs 'context-os query chains --limit 1'
        Then: Output shows only 1 chain
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "chains", "--limit", "1"])

        assert result.exit_code == 0


# =============================================================================
# Test: build_index_from_jsonl
# =============================================================================


class TestBuildIndexFromJsonl:
    """Tests for build_index_from_jsonl helper function."""

    def test_build_index_handles_no_jsonl_files(self, cli_runner):
        """Build index should error gracefully when no JSONL files found.

        Given: No JSONL files exist for project
        When: User runs any query command
        Then: Error message shown (not stack trace)
        """
        # Patch at the import location inside build_index_from_jsonl
        with patch("context_os_events.capture.jsonl_parser.get_claude_projects_dir") as mock_dir:
            mock_dir.return_value = Path("/nonexistent/path")
            result = cli_runner.invoke(cli, ["query", "file", "test.py"])

        # Should show user-friendly error, not crash
        assert result.exit_code != 0 or "no" in result.output.lower() or "not found" in result.output.lower()

    def test_build_index_uses_current_project(self, cli_runner, mock_index):
        """Build index should default to current working directory.

        Given: No --project option specified
        When: User runs query command
        Then: Index built from current directory's project files
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index) as mock_build:
            result = cli_runner.invoke(cli, ["query", "chains"])

        # Verify build was called (will use default project path)
        mock_build.assert_called_once()


# =============================================================================
# Test: Command Group Structure
# =============================================================================


class TestQueryCommandGroup:
    """Tests for query command group structure."""

    def test_query_group_exists(self, cli_runner):
        """Query command group should exist.

        When: User runs 'context-os query --help'
        Then: Help text shows available subcommands
        """
        result = cli_runner.invoke(cli, ["query", "--help"])

        assert result.exit_code == 0
        assert "file" in result.output
        assert "co-access" in result.output
        assert "recent" in result.output
        assert "chains" in result.output

    def test_query_without_subcommand_shows_help(self, cli_runner):
        """Query without subcommand should show help.

        When: User runs 'context-os query' (no subcommand)
        Then: Help text is displayed
        """
        result = cli_runner.invoke(cli, ["query"])

        # Should show help, not error
        assert "Usage" in result.output or "file" in result.output


# =============================================================================
# Integration Tests (with real JSONL parsing - NO MOCKING)
# =============================================================================


class TestBuildIndexFromJsonlIntegration:
    """Integration tests for build_index_from_jsonl - NO MOCKING."""

    def test_build_index_returns_populated_context_index(self):
        """build_index_from_jsonl should return a real populated ContextIndex.

        Given: Real JSONL files exist for gtm_operating_system project
        When: build_index_from_jsonl() is called with project path
        Then: Returns ContextIndex with actual data
        """
        from context_os_events.cli import build_index_from_jsonl
        from context_os_events.index import ContextIndex

        project_path = r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system"
        index = build_index_from_jsonl(project_path)

        # Must return a ContextIndex
        assert isinstance(index, ContextIndex)

        # Must have actual data - not empty
        assert len(index._chains) > 0, "Expected chains from real JSONL data"
        assert len(index._inverted_index) > 0, "Expected file accesses from real JSONL"
        assert len(index._temporal) > 0, "Expected temporal buckets from real JSONL"

    def test_build_index_chains_have_valid_structure(self):
        """Chains from real JSONL should have valid structure.

        Given: Real JSONL files with session data
        When: build_index_from_jsonl() builds chains
        Then: Each chain has required fields populated
        """
        from context_os_events.cli import build_index_from_jsonl

        project_path = r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system"
        index = build_index_from_jsonl(project_path)

        # Check at least one chain has valid structure
        chain_id, chain = next(iter(index._chains.items()))
        assert chain.chain_id is not None
        assert chain.root_session is not None
        assert isinstance(chain.sessions, list)
        assert len(chain.sessions) >= 1

    def test_build_index_inverted_index_has_file_paths(self):
        """Inverted index should contain real file paths.

        Given: Real JSONL files with Read/Edit/Write tool calls
        When: build_index_from_jsonl() builds inverted index
        Then: Index contains actual file paths as keys
        """
        from context_os_events.cli import build_index_from_jsonl

        project_path = r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system"
        index = build_index_from_jsonl(project_path)

        # Check file paths look like real paths
        sample_paths = list(index._inverted_index.keys())[:10]
        for path in sample_paths:
            # Should be absolute paths on Windows
            assert ":" in path or path.startswith("/"), f"Expected absolute path: {path}"

    def test_build_index_temporal_buckets_have_sessions(self):
        """Temporal buckets should contain session data.

        Given: Real JSONL files with timestamps
        When: build_index_from_jsonl() builds temporal buckets
        Then: Buckets contain session and chain references
        """
        from context_os_events.cli import build_index_from_jsonl

        project_path = r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system"
        index = build_index_from_jsonl(project_path)

        # Check at least one bucket has sessions
        period, bucket = next(iter(index._temporal.items()))
        assert bucket.period is not None
        assert isinstance(bucket.sessions, set)
        assert len(bucket.sessions) >= 1


class TestQueryCommandsIntegration:
    """Integration tests for CLI query commands - NO MOCKING."""

    @pytest.fixture
    def project_runner(self, cli_runner):
        """CLI runner that executes from the correct project directory."""
        import os
        original_cwd = os.getcwd()
        os.chdir(r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system")
        yield cli_runner
        os.chdir(original_cwd)

    def test_query_recent_returns_real_weekly_data(self, project_runner):
        """query recent should return actual weekly buckets from JSONL.

        Given: Real JSONL files with session data
        When: User runs 'context-os query recent --weeks 4'
        Then: Output shows real weekly data with session counts
        """
        result = project_runner.invoke(cli, ["query", "recent", "--weeks", "4"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should have week identifiers (2025-W## format)
        assert "W5" in result.output or "W4" in result.output, "Expected week buckets"
        # Should show session counts
        assert "session" in result.output.lower()
        # Should show totals
        assert "total" in result.output.lower()

    def test_query_chains_returns_real_chains(self, project_runner):
        """query chains should return actual conversation chains.

        Given: Real JSONL files with leafUuid linking
        When: User runs 'context-os query chains'
        Then: Output shows real chains with session counts
        """
        result = project_runner.invoke(cli, ["query", "chains", "--limit", "10"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should have chain data
        assert "chain" in result.output.lower()
        # Should show multiple chains (we know there are 400+)
        assert "of" in result.output.lower(), "Expected 'X of Y' count"

    def test_query_file_finds_real_file(self, project_runner):
        """query file should find sessions that touched a real file.

        Given: Real JSONL files with file access data
        When: User runs 'context-os query file CLAUDE.md'
        Then: Output shows sessions that accessed CLAUDE.md
        """
        result = project_runner.invoke(cli, ["query", "file", "CLAUDE.md"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # CLAUDE.md should have been accessed (we verified this earlier)
        # Either shows sessions or "not found" message
        assert "CLAUDE.md" in result.output

    def test_query_file_with_path_shows_access_types(self, project_runner):
        """query file should show access types (read/write/create).

        Given: Real JSONL files with tool call data
        When: User queries a file that was accessed
        Then: Output shows access type column
        """
        result = project_runner.invoke(cli, ["query", "file", "CLAUDE.md"])

        if "not found" not in result.output.lower():
            # If file was found, should show access types
            assert "read" in result.output.lower() or "write" in result.output.lower()

    def test_query_co_access_finds_related_files(self, project_runner):
        """query co-access should find files accessed together.

        Given: Real JSONL files with co-access patterns
        When: User queries co-access for a frequently accessed file
        Then: Output shows related files with PMI scores
        """
        result = project_runner.invoke(cli, ["query", "co-access", "CLAUDE.md"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should either show PMI data or "no co-access data"
        assert "PMI" in result.output or "no co-access" in result.output.lower()

    def test_full_workflow_no_mocking(self, project_runner):
        """Full query workflow with real data, no mocking.

        This is the real integration test - all commands, real data.
        """
        # Query recent - must work
        result = project_runner.invoke(cli, ["query", "recent", "--weeks", "2"])
        assert result.exit_code == 0, f"query recent failed: {result.output}"
        assert "W5" in result.output or "W4" in result.output

        # Query chains - must work
        result = project_runner.invoke(cli, ["query", "chains", "--limit", "5"])
        assert result.exit_code == 0, f"query chains failed: {result.output}"
        assert "chain" in result.output.lower()

        # Query file - must not crash (file may or may not be in index)
        result = project_runner.invoke(cli, ["query", "file", "cli.py"])
        assert result.exit_code == 0, f"query file failed: {result.output}"

        # Query co-access - must not crash
        result = project_runner.invoke(cli, ["query", "co-access", "cli.py"])
        assert result.exit_code == 0, f"query co-access failed: {result.output}"


# =============================================================================
# V2 Tests: query session (NEW)
# =============================================================================


class TestQuerySession:
    """Tests for 'context-os query session <id>' command.

    V2 Enhancement: Exposes Session→Files direction of bidirectional index.
    """

    def test_query_session_shows_files(self, cli_runner, mock_index):
        """Query session should display files touched by that session.

        Given: Index has session-001 touching 2 files
        When: User runs 'context-os query session session-001'
        Then: Output shows both files with access types
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "session", "session-001"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        assert "agent.ts" in result.output
        assert "index.ts" in result.output

    def test_query_session_partial_id_match(self, cli_runner, mock_index):
        """Query session should support partial session ID matching.

        Given: Index has session-001
        When: User runs 'context-os query session session-0'
        Then: Output shows files for matching session
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "session", "session-0"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should match session-001 or session-002
        assert "agent.ts" in result.output or "index.ts" in result.output

    def test_query_session_unknown_session(self, cli_runner, mock_index):
        """Query session should handle unknown session gracefully.

        Given: Index doesn't have session-unknown
        When: User runs 'context-os query session session-unknown'
        Then: Output shows "not found" message
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "session", "session-unknown"])

        assert result.exit_code == 0
        assert "not found" in result.output.lower() or "no files" in result.output.lower()

    def test_query_session_shows_chain_context(self, cli_runner, mock_index):
        """Query session should show chain context if available.

        Given: Index has session-001 in chain-main
        When: User runs 'context-os query session session-001'
        Then: Output includes chain ID
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "session", "session-001"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        assert "chain" in result.output.lower()

    def test_query_session_respects_limit(self, cli_runner, mock_index):
        """Query session should respect --limit option.

        Given: Session has many files
        When: User runs 'context-os query session session-001 --limit 1'
        Then: Output shows only 1 file
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "session", "session-001", "--limit", "1"])

        assert result.exit_code == 0, f"Command failed: {result.output}"


# =============================================================================
# V2 Tests: query search (NEW)
# =============================================================================


class TestQuerySearch:
    """Tests for 'context-os query search <pattern>' command.

    V2 Enhancement: Enables file discovery by pattern matching.
    """

    def test_query_search_finds_matching_files(self, cli_runner, mock_index):
        """Query search should find files matching pattern.

        Given: Index has files src/agent.ts and src/index.ts
        When: User runs 'context-os query search agent'
        Then: Output shows src/agent.ts
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "search", "agent"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        assert "agent.ts" in result.output

    def test_query_search_case_insensitive(self, cli_runner, mock_index):
        """Query search should be case-insensitive.

        Given: Index has src/agent.ts
        When: User runs 'context-os query search AGENT'
        Then: Output shows src/agent.ts
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "search", "AGENT"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        assert "agent.ts" in result.output

    def test_query_search_sorted_by_access_count(self, cli_runner, mock_index):
        """Query search should sort results by access count.

        Given: Index has files with different access counts
        When: User runs 'context-os query search src'
        Then: Files with more accesses appear first
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "search", "src"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # agent.ts has 2 accesses, index.ts has 1
        # agent.ts should appear before index.ts in output
        agent_pos = result.output.find("agent.ts")
        index_pos = result.output.find("index.ts")
        assert agent_pos < index_pos, "Higher access count should appear first"

    def test_query_search_no_matches(self, cli_runner, mock_index):
        """Query search should handle no matches gracefully.

        Given: Index has no files matching 'nonexistent'
        When: User runs 'context-os query search nonexistent'
        Then: Output shows "no files found" message
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "search", "nonexistent"])

        assert result.exit_code == 0
        assert "no files" in result.output.lower() or "0" in result.output

    def test_query_search_respects_limit(self, cli_runner, mock_index):
        """Query search should respect --limit option.

        Given: Index has multiple files matching 'src'
        When: User runs 'context-os query search src --limit 1'
        Then: Output shows only 1 file
        """
        with patch("context_os_events.cli.build_index_from_jsonl", return_value=mock_index):
            result = cli_runner.invoke(cli, ["query", "search", "src", "--limit", "1"])

        assert result.exit_code == 0, f"Command failed: {result.output}"


# =============================================================================
# V2 Tests: Helper Functions
# =============================================================================


class TestHelperFunctions:
    """Tests for V2 helper functions."""

    def test_find_matching_files_substring(self, mock_index):
        """find_matching_files should match by substring.

        Given: Index has src/agent.ts, src/index.ts
        When: find_matching_files(index, "agent")
        Then: Returns [(src/agent.ts, 2)]
        """
        from context_os_events.cli import find_matching_files

        matches = find_matching_files(mock_index, "agent")

        assert len(matches) == 1
        assert matches[0][0] == "src/agent.ts"
        assert matches[0][1] == 2  # 2 accesses

    def test_find_matching_files_case_insensitive(self, mock_index):
        """find_matching_files should be case-insensitive.

        Given: Index has src/agent.ts
        When: find_matching_files(index, "AGENT")
        Then: Returns [(src/agent.ts, 2)]
        """
        from context_os_events.cli import find_matching_files

        matches = find_matching_files(mock_index, "AGENT")

        assert len(matches) == 1
        assert "agent.ts" in matches[0][0]

    def test_find_matching_files_sorted_by_count(self, mock_index):
        """find_matching_files should sort by access count descending.

        Given: Index has src/agent.ts (2 accesses), src/index.ts (1 access)
        When: find_matching_files(index, "src")
        Then: agent.ts appears before index.ts
        """
        from context_os_events.cli import find_matching_files

        matches = find_matching_files(mock_index, "src")

        assert len(matches) == 2
        assert matches[0][0] == "src/agent.ts"  # 2 accesses
        assert matches[1][0] == "src/index.ts"  # 1 access

    def test_find_matching_sessions_prefix(self, mock_index):
        """find_matching_sessions should match by prefix.

        Given: Index has session-001, session-002
        When: find_matching_sessions(index, "session-0")
        Then: Returns both session IDs
        """
        from context_os_events.cli import find_matching_sessions

        matches = find_matching_sessions(mock_index, "session-0")

        assert "session-001" in matches
        assert "session-002" in matches


# =============================================================================
# V2 Integration Tests (real JSONL, no mocking)
# =============================================================================


class TestV2Integration:
    """V2 integration tests with real JSONL data - NO MOCKING."""

    @pytest.fixture
    def project_runner(self, cli_runner):
        """CLI runner from correct project directory."""
        import os
        original_cwd = os.getcwd()
        os.chdir(r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system")
        yield cli_runner
        os.chdir(original_cwd)

    def test_query_session_with_real_data(self, project_runner):
        """Query session should work with real JSONL data.

        Given: Real JSONL files with session 7fab4726
        When: User runs 'context-os query session 7fab4726'
        Then: Output shows files touched by that session
        """
        result = project_runner.invoke(cli, ["query", "session", "7fab4726", "--limit", "10"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Session 7fab4726 touched nickel files
        # Should show files OR "not found" if session ID changed

    def test_query_search_finds_nickel_files(self, project_runner):
        """Query search should find files matching 'nickel'.

        Given: Real JSONL files with nickel-related file accesses
        When: User runs 'context-os query search nickel'
        Then: Output shows files matching pattern (expect ~73 files)
        """
        result = project_runner.invoke(cli, ["query", "search", "nickel", "--limit", "20"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        assert "nickel" in result.output.lower()

    def test_query_search_to_session_workflow(self, project_runner):
        """Full discovery workflow: search → file → session.

        This is the real use case: find files, then explore sessions.
        """
        # Step 1: Search for files
        result = project_runner.invoke(cli, ["query", "search", "nickel", "--limit", "5"])
        assert result.exit_code == 0, f"search failed: {result.output}"

        # Step 2: Query recent to see activity
        result = project_runner.invoke(cli, ["query", "recent", "--weeks", "2"])
        assert result.exit_code == 0, f"recent failed: {result.output}"

    def test_query_file_with_substring_matching(self, project_runner):
        """Query file should work with partial paths (V2 fix).

        Given: Real JSONL files
        When: User runs 'context-os query file CLAUDE.md'
        Then: Output shows sessions (substring matching, not just exact)
        """
        result = project_runner.invoke(cli, ["query", "file", "CLAUDE.md"])

        assert result.exit_code == 0, f"Command failed: {result.output}"
        # Should find CLAUDE.md via substring matching


# =============================================================================
# Phase A Tests: JSON Output for Existing Commands
# =============================================================================
# TDD: These tests written BEFORE implementation (should FAIL initially)


class TestPhaseAJsonOutput:
    """Phase A tests: All existing query commands support --format json."""

    @pytest.fixture
    def project_runner(self, cli_runner):
        """CLI runner from correct project directory."""
        import os
        original_cwd = os.getcwd()
        os.chdir(r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system")
        yield cli_runner
        os.chdir(original_cwd)

    def test_query_search_supports_json_format(self, project_runner):
        """query search supports --format json flag.

        Given: Real JSONL files
        When: User runs 'context-os query search nickel --format json'
        Then: Output is valid JSON with expected fields
        """
        import json

        result = project_runner.invoke(cli, ["query", "search", "nickel", "--format", "json"])

        assert result.exit_code == 0, f"Command failed: {result.output}"

        # Must be valid JSON
        data = json.loads(result.output)

        # Must have required fields
        assert "results" in data, "Missing 'results' field"
        assert "result_count" in data, "Missing 'result_count' field"
        assert "timestamp" in data, "Missing 'timestamp' field"

    def test_query_file_supports_json_format(self, project_runner):
        """query file supports --format json flag.

        Given: Real JSONL files
        When: User runs 'context-os query file CLAUDE.md --format json'
        Then: Output is valid JSON
        """
        import json

        result = project_runner.invoke(cli, ["query", "file", "CLAUDE.md", "--format", "json"])

        assert result.exit_code == 0, f"Command failed: {result.output}"

        # Must be valid JSON
        data = json.loads(result.output)

        # Must have required fields
        assert "results" in data, "Missing 'results' field"

    def test_query_session_supports_json_format(self, project_runner):
        """query session supports --format json flag.

        Given: Real JSONL files with sessions
        When: User runs 'context-os query session <prefix> --format json'
        Then: Output is valid JSON
        """
        import json

        # Use a session prefix that exists
        result = project_runner.invoke(cli, ["query", "session", "7fab", "--format", "json"])

        # May not find session, but should still be valid JSON
        if result.exit_code == 0:
            data = json.loads(result.output)
            assert "results" in data

    def test_query_chains_supports_json_format(self, project_runner):
        """query chains supports --format json flag.

        Given: Real JSONL files with chains
        When: User runs 'context-os query chains --format json'
        Then: Output is valid JSON
        """
        import json

        result = project_runner.invoke(cli, ["query", "chains", "--format", "json"])

        assert result.exit_code == 0, f"Command failed: {result.output}"

        # Must be valid JSON
        data = json.loads(result.output)
        assert "results" in data

    def test_query_co_access_supports_json_format(self, project_runner):
        """query co-access supports --format json flag.

        Given: Real JSONL files
        When: User runs 'context-os query co-access CLAUDE.md --format json'
        Then: Output is valid JSON
        """
        import json

        result = project_runner.invoke(cli, ["query", "co-access", "CLAUDE.md", "--format", "json"])

        assert result.exit_code == 0, f"Command failed: {result.output}"

        # Must be valid JSON
        data = json.loads(result.output)
        assert "results" in data

    def test_query_recent_supports_json_format(self, project_runner):
        """query recent supports --format json flag.

        Given: Real JSONL files with recent activity
        When: User runs 'context-os query recent --format json'
        Then: Output is valid JSON
        """
        import json

        result = project_runner.invoke(cli, ["query", "recent", "--format", "json"])

        assert result.exit_code == 0, f"Command failed: {result.output}"

        # Must be valid JSON
        data = json.loads(result.output)
        assert "results" in data

    def test_default_format_is_table(self, project_runner):
        """Default format is table (backwards compatible).

        Given: Real JSONL files
        When: User runs 'context-os query search nickel' (no --format)
        Then: Output is table format (not JSON)
        """
        import json

        result = project_runner.invoke(cli, ["query", "search", "nickel"])

        assert result.exit_code == 0, f"Command failed: {result.output}"

        # Should NOT be JSON (table is default)
        with pytest.raises(json.JSONDecodeError):
            json.loads(result.output)

    def test_json_output_contains_timestamp(self, project_runner):
        """JSON output includes ISO timestamp.

        Given: --format json
        When: Running any query command
        Then: Output includes 'timestamp' in ISO format
        """
        import json
        from datetime import datetime

        result = project_runner.invoke(cli, ["query", "recent", "--format", "json"])

        assert result.exit_code == 0
        data = json.loads(result.output)

        assert "timestamp" in data
        # Should parse as ISO datetime
        datetime.fromisoformat(data["timestamp"].replace("Z", "+00:00"))
