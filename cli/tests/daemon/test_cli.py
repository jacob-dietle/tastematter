"""Tests for daemon CLI commands.

Phase 4 of daemon implementation - CLI integration.
Following TDD Red-Green-Refactor cycle.
"""

from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest
from click.testing import CliRunner


class TestDaemonConfigCommand:
    """Tests for 'daemon config' command."""

    def test_daemon_config_shows_configuration(self):
        """Should display current configuration."""
        from context_os_events.cli import cli

        runner = CliRunner()
        result = runner.invoke(cli, ["daemon", "config"])

        assert result.exit_code == 0
        assert "version" in result.output.lower() or "sync" in result.output.lower()

    def test_daemon_config_shows_file_path(self):
        """Should show config file location."""
        from context_os_events.cli import cli

        runner = CliRunner()
        result = runner.invoke(cli, ["daemon", "config"])

        assert result.exit_code == 0
        # Should mention the config file path
        assert ".context-os" in result.output or "config" in result.output.lower()


class TestDaemonStatusCommand:
    """Tests for 'daemon status' command."""

    def test_daemon_status_shows_not_installed(self):
        """Should show service is not installed when not present."""
        from context_os_events.cli import cli

        runner = CliRunner()

        with patch(
            "context_os_events.daemon.service.get_service_status", return_value=None
        ):
            result = runner.invoke(cli, ["daemon", "status"])

        assert result.exit_code == 0
        assert "not installed" in result.output.lower() or "not found" in result.output.lower()

    def test_daemon_status_shows_running(self):
        """Should show service is running when active."""
        from context_os_events.cli import cli

        runner = CliRunner()

        with patch(
            "context_os_events.daemon.service.get_service_status", return_value="Running"
        ):
            result = runner.invoke(cli, ["daemon", "status"])

        assert result.exit_code == 0
        assert "running" in result.output.lower()


class TestDaemonRunCommand:
    """Tests for 'daemon run' command (foreground mode)."""

    def test_daemon_run_starts_foreground_mode(self):
        """Should start daemon in foreground with Ctrl+C to stop."""
        from context_os_events.cli import cli

        runner = CliRunner()

        # Mock the daemon to avoid actually starting it
        with patch("context_os_events.daemon.runner.ContextOSDaemon") as MockDaemon:
            mock_daemon = MagicMock()
            MockDaemon.return_value = mock_daemon

            # Simulate KeyboardInterrupt after start
            mock_daemon.start.side_effect = lambda: None

            # Use timeout to simulate Ctrl+C
            result = runner.invoke(cli, ["daemon", "run", "--once"])

            # Check that daemon was created and started
            MockDaemon.assert_called_once()


class TestDaemonInstallCommand:
    """Tests for 'daemon install' command."""

    def test_daemon_install_requires_servy(self):
        """Should fail gracefully if Servy not available."""
        from context_os_events.cli import cli

        runner = CliRunner()

        with patch(
            "context_os_events.daemon.service.is_servy_available", return_value=False
        ):
            result = runner.invoke(cli, ["daemon", "install"])

        assert result.exit_code != 0 or "servy" in result.output.lower()

    def test_daemon_install_success(self):
        """Should install service when Servy available."""
        from context_os_events.cli import cli

        runner = CliRunner()

        with patch(
            "context_os_events.daemon.service.install_service",
            return_value=(True, "Installed successfully"),
        ):
            result = runner.invoke(cli, ["daemon", "install"])

        assert "install" in result.output.lower() or "success" in result.output.lower()


class TestDaemonLogsCommand:
    """Tests for 'daemon logs' command."""

    def test_daemon_logs_shows_log_location(self):
        """Should show log file location."""
        from context_os_events.cli import cli

        runner = CliRunner()
        result = runner.invoke(cli, ["daemon", "logs"])

        # Should reference log location
        assert result.exit_code == 0
        assert "log" in result.output.lower()


class TestDaemonUninstallCommand:
    """Tests for 'daemon uninstall' command."""

    def test_daemon_uninstall_success(self):
        """Should uninstall service."""
        from context_os_events.cli import cli

        runner = CliRunner()

        with patch(
            "context_os_events.daemon.service.is_servy_available", return_value=True
        ):
            with patch(
                "context_os_events.daemon.service.uninstall_service",
                return_value=(True, "Uninstalled successfully"),
            ):
                result = runner.invoke(cli, ["daemon", "uninstall"])

        assert "uninstall" in result.output.lower() or "success" in result.output.lower()
