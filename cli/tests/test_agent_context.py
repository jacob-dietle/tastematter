"""TDD tests for agent-context CLI command.

Agent 3 of AGENT_CONTEXT_LOGGING_SPEC.md implementation.
Tests follow Kent Beck RED→GREEN→REFACTOR pattern.
"""

import json
import tempfile
from datetime import datetime, timedelta
from pathlib import Path

import pytest
from click.testing import CliRunner


# =============================================================================
# Cycle 1: Command Registration
# =============================================================================


class TestAgentContextCommand:
    """Tests for agent-context command registration."""

    def test_command_exists(self):
        """agent-context command is registered in CLI."""
        from context_os_events.cli import cli

        runner = CliRunner()
        result = runner.invoke(cli, ["agent-context", "--help"])

        assert result.exit_code == 0
        assert "agent-context" in result.output.lower()

    def test_format_option_exists(self):
        """--format option accepts json and markdown."""
        from context_os_events.cli import cli

        runner = CliRunner()
        # Check help shows format option
        result = runner.invoke(cli, ["agent-context", "--help"])

        assert "--format" in result.output
        assert "json" in result.output
        assert "markdown" in result.output

    def test_include_errors_flag_exists(self):
        """--include-errors flag is available."""
        from context_os_events.cli import cli

        runner = CliRunner()
        result = runner.invoke(cli, ["agent-context", "--help"])

        assert "--include-errors" in result.output

    def test_since_option_exists(self):
        """--since option is available."""
        from context_os_events.cli import cli

        runner = CliRunner()
        result = runner.invoke(cli, ["agent-context", "--help"])

        assert "--since" in result.output


# =============================================================================
# Cycle 2: Health Section Generation
# =============================================================================


class TestHealthSection:
    """Tests for health section generation."""

    def test_generates_health_section(self, tmp_path):
        """Generates markdown health section from health.json."""
        from context_os_events.agent_context import generate_health_section

        # Create test health.json
        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {
                "path": "/path/to/db",
                "size_mb": 12.5,
                "tables": {
                    "claude_sessions": {"rows": 460, "last_updated": None},
                    "chains": {"rows": 614, "last_updated": None},
                }
            },
            "recent_errors": [],
            "warnings": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_health_section(state_dir)

        assert "## System Health" in result
        assert "claude_sessions" in result
        assert "460" in result

    def test_includes_table_counts(self, tmp_path):
        """Health section shows row counts for each table."""
        from context_os_events.agent_context import generate_health_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {
                "path": "/path/to/db",
                "size_mb": 12.5,
                "tables": {
                    "claude_sessions": {"rows": 460, "last_updated": None},
                    "chains": {"rows": 614, "last_updated": None},
                    "chain_graph": {"rows": 743, "last_updated": None},
                }
            },
            "recent_errors": [],
            "warnings": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_health_section(state_dir)

        assert "460" in result
        assert "614" in result
        assert "743" in result

    def test_shows_healthy_status(self, tmp_path):
        """Shows Healthy when no errors."""
        from context_os_events.agent_context import generate_health_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [],
            "warnings": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_health_section(state_dir)

        assert "Healthy" in result

    def test_shows_degraded_status(self, tmp_path):
        """Shows Degraded when warnings present."""
        from context_os_events.agent_context import generate_health_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [],
            "warnings": [{"message": "Stale data"}],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_health_section(state_dir)

        assert "Degraded" in result or "Warning" in result


# =============================================================================
# Cycle 3: Activity Section Generation
# =============================================================================


class TestActivitySection:
    """Tests for activity section generation."""

    def test_generates_activity_section(self, tmp_path):
        """Generates markdown activity section from activity.json."""
        from context_os_events.agent_context import generate_activity_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 12, "errors": 0},
            "recent_commands": [
                {"ts": "2026-01-02T16:45:00Z", "command": "build-chains", "status": "success", "duration_ms": 3200},
            ],
        }
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_activity_section(state_dir)

        assert "## Recent Activity" in result
        assert "build-chains" in result

    def test_formats_recent_commands(self, tmp_path):
        """Recent commands show time, name, status, duration."""
        from context_os_events.agent_context import generate_activity_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 2, "errors": 0},
            "recent_commands": [
                {"ts": "2026-01-02T16:45:00Z", "command": "build-chains", "status": "success", "duration_ms": 3200},
                {"ts": "2026-01-02T16:30:00Z", "command": "parse-sessions", "status": "success", "duration_ms": 1200},
            ],
        }
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_activity_section(state_dir)

        assert "build-chains" in result
        assert "parse-sessions" in result
        assert "3.2s" in result or "3200" in result

    def test_shows_24h_summary(self, tmp_path):
        """Shows commands_run and errors count for 24h."""
        from context_os_events.agent_context import generate_activity_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 12, "errors": 2},
            "recent_commands": [],
        }
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_activity_section(state_dir)

        assert "12" in result or "commands" in result.lower()

    def test_handles_no_activity(self, tmp_path):
        """Shows 'No activity' when empty."""
        from context_os_events.agent_context import generate_activity_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 0, "errors": 0},
            "recent_commands": [],
        }
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_activity_section(state_dir)

        assert "No" in result or "activity" in result.lower() or "0" in result


# =============================================================================
# Cycle 4: Error Section Generation
# =============================================================================


class TestErrorSection:
    """Tests for error section generation."""

    def test_generates_error_section(self, tmp_path):
        """Generates error section from health.json recent_errors."""
        from context_os_events.agent_context import generate_error_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [
                {"ts": "2026-01-02T16:00:00Z", "message": "Connection failed", "suggestion": "Check network"}
            ],
            "warnings": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_error_section(state_dir)

        assert "## Recent Errors" in result

    def test_shows_no_errors_message(self, tmp_path):
        """Shows 'None in last 24 hours' when no errors."""
        from context_os_events.agent_context import generate_error_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [],
            "warnings": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_error_section(state_dir)

        assert "None" in result or "no error" in result.lower()

    def test_includes_error_suggestion(self, tmp_path):
        """Error includes suggestion when present."""
        from context_os_events.agent_context import generate_error_section

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [
                {"ts": "2026-01-02T16:00:00Z", "message": "No chain data", "suggestion": "Run build-chains"}
            ],
            "warnings": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)

        result = generate_error_section(state_dir, include_errors=True)

        assert "No chain data" in result or "build-chains" in result


# =============================================================================
# Cycle 5: Quick Reference Section
# =============================================================================


class TestQuickReference:
    """Tests for quick reference section."""

    def test_generates_quick_reference(self):
        """Generates quick reference section with common commands."""
        from context_os_events.agent_context import generate_quick_reference

        result = generate_quick_reference()

        assert "## Quick Reference" in result

    def test_includes_key_commands(self):
        """Includes build-chains, parse-sessions, status, agent-context."""
        from context_os_events.agent_context import generate_quick_reference

        result = generate_quick_reference()

        assert "build-chains" in result
        assert "parse-sessions" in result
        assert "agent-context" in result


# =============================================================================
# Cycle 6: JSON Output Format
# =============================================================================


class TestJsonFormat:
    """Tests for JSON output format."""

    def test_json_format_is_valid(self, tmp_path):
        """--format json outputs valid JSON."""
        from context_os_events.agent_context import generate_agent_context

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        # Create minimal state files
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [],
            "warnings": [],
        }
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 0, "errors": 0},
            "recent_commands": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_agent_context(state_dir, output_format="json")

        # Should be valid JSON
        parsed = json.loads(result)
        assert isinstance(parsed, dict)

    def test_json_includes_all_sections(self, tmp_path):
        """JSON output has health, activity, quick_reference."""
        from context_os_events.agent_context import generate_agent_context

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [],
            "warnings": [],
        }
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 0, "errors": 0},
            "recent_commands": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_agent_context(state_dir, output_format="json")
        parsed = json.loads(result)

        assert "health" in parsed
        assert "activity" in parsed
        assert "quick_reference" in parsed


# =============================================================================
# Cycle 7: Full Integration
# =============================================================================


class TestFullCommand:
    """Tests for full command output."""

    def test_markdown_output_complete(self, tmp_path):
        """Full command produces complete markdown document."""
        from context_os_events.agent_context import generate_agent_context

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {"claude_sessions": {"rows": 100, "last_updated": None}}},
            "recent_errors": [],
            "warnings": [],
        }
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 5, "errors": 0},
            "recent_commands": [
                {"ts": "2026-01-02T16:45:00Z", "command": "build-chains", "status": "success", "duration_ms": 1000}
            ],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_agent_context(state_dir, output_format="markdown")

        assert "# Context OS - Agent Context Summary" in result
        assert "## System Health" in result
        assert "## Recent Activity" in result
        assert "## Quick Reference" in result

    def test_generated_timestamp_present(self, tmp_path):
        """Output includes 'Generated:' timestamp."""
        from context_os_events.agent_context import generate_agent_context

        state_dir = tmp_path / "state"
        state_dir.mkdir()
        health_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "database": {"path": "/db", "size_mb": 10, "tables": {}},
            "recent_errors": [],
            "warnings": [],
        }
        activity_data = {
            "generated_at": "2026-01-02T16:45:00Z",
            "last_24h": {"commands_run": 0, "errors": 0},
            "recent_commands": [],
        }
        with open(state_dir / "health.json", "w") as f:
            json.dump(health_data, f)
        with open(state_dir / "activity.json", "w") as f:
            json.dump(activity_data, f)

        result = generate_agent_context(state_dir, output_format="markdown")

        assert "Generated:" in result
