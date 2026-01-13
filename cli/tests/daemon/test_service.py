"""Tests for daemon service module.

Phase 3 of daemon implementation - NSSM Windows service integration.
Following TDD Red-Green-Refactor cycle.
"""

import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest


class TestServiceConstants:
    """Tests for service name and path constants."""

    def test_get_service_name_returns_constant(self):
        """Service name should be 'ContextOSEvents'."""
        from context_os_events.daemon.service import get_service_name

        name = get_service_name()
        assert name == "ContextOSEvents"

    def test_get_python_path_returns_pythonw(self):
        """Should return path to pythonw.exe (windowless Python)."""
        from context_os_events.daemon.service import get_python_path

        python_path = get_python_path()

        assert python_path.exists()
        # Should be pythonw.exe for windowless execution
        assert python_path.name == "pythonw.exe"

    def test_get_runner_module_returns_correct_module(self):
        """Should return the daemon runner module path."""
        from context_os_events.daemon.service import get_runner_module

        module = get_runner_module()
        assert module == "context_os_events.daemon.runner"


class TestNssmAvailability:
    """Tests for NSSM availability checking."""

    def test_get_nssm_path_returns_path_or_none(self):
        """Should return Path to nssm.exe or None if not found."""
        from context_os_events.daemon.service import get_nssm_path

        result = get_nssm_path()
        assert result is None or isinstance(result, Path)

    def test_is_nssm_available_returns_bool(self):
        """Should return True/False based on NSSM in PATH."""
        from context_os_events.daemon.service import is_nssm_available

        result = is_nssm_available()
        assert isinstance(result, bool)

    def test_is_nssm_available_returns_false_when_not_found(self):
        """Should return False when nssm is not installed."""
        from context_os_events.daemon.service import is_nssm_available

        # Mock get_nssm_path to return None
        with patch("context_os_events.daemon.service.get_nssm_path", return_value=None):
            result = is_nssm_available()
            assert result is False

    def test_is_nssm_available_returns_true_when_found(self):
        """Should return True when nssm responds."""
        from context_os_events.daemon.service import is_nssm_available

        # Mock get_nssm_path and subprocess.run
        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock(returncode=0)
                result = is_nssm_available()
                assert result is True

    def test_is_servy_available_is_alias_for_nssm(self):
        """is_servy_available should be an alias for backwards compatibility."""
        from context_os_events.daemon.service import is_servy_available, is_nssm_available

        # They should be the same function
        assert is_servy_available is is_nssm_available


class TestServiceStatus:
    """Tests for service status checking."""

    def test_get_service_status_when_not_installed(self):
        """Should return None when service is not installed."""
        from context_os_events.daemon.service import get_service_status

        # Mock sc query to return error (service not found)
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=1,
                stdout="The specified service does not exist",
            )
            status = get_service_status()
            assert status is None

    def test_get_service_status_returns_running(self):
        """Should return 'Running' when service is running."""
        from context_os_events.daemon.service import get_service_status

        # Mock sc query to return running status
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="STATE              : 4  RUNNING",
            )
            status = get_service_status()
            assert status == "Running"

    def test_get_service_status_returns_stopped(self):
        """Should return 'Stopped' when service is stopped."""
        from context_os_events.daemon.service import get_service_status

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="STATE              : 1  STOPPED",
            )
            status = get_service_status()
            assert status == "Stopped"


class TestInstallCommand:
    """Tests for service install command generation."""

    def test_build_install_command_has_required_args(self):
        """Install command should have all required NSSM arguments."""
        from context_os_events.daemon.service import build_install_command

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            cmd = build_install_command()

            # Check NSSM command structure
            assert "nssm" in cmd[0].lower()
            assert "install" in cmd
            assert "ContextOSEvents" in cmd
            # Should include pythonw.exe path and module args
            assert any("pythonw.exe" in str(arg) for arg in cmd)
            assert any("-m context_os_events.daemon.runner" in str(arg) for arg in cmd)

    def test_build_install_command_includes_config_path(self):
        """Install command should include config path when provided."""
        from context_os_events.daemon.service import build_install_command

        config_path = Path("/path/to/config.yaml")
        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            cmd = build_install_command(config_path)
            cmd_str = " ".join(str(c) for c in cmd)

            assert "--config" in cmd_str
            assert "config.yaml" in cmd_str

    def test_build_install_command_uses_nssm_path(self):
        """Install command should use discovered NSSM path."""
        from context_os_events.daemon.service import build_install_command

        nssm_path = Path("C:/tools/nssm.exe")
        with patch("context_os_events.daemon.service.get_nssm_path", return_value=nssm_path):
            cmd = build_install_command()

            assert str(nssm_path) in cmd[0]


class TestServiceInstallation:
    """Tests for actual service installation (mocked)."""

    def test_install_service_checks_nssm_first(self):
        """install_service should fail gracefully if NSSM not available."""
        from context_os_events.daemon.service import install_service

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=None):
            success, message = install_service()
            assert success is False
            assert "nssm" in message.lower()

    def test_install_service_runs_command(self):
        """install_service should run the install command."""
        from context_os_events.daemon.service import install_service

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock(returncode=0, stdout="Installed")
                success, message = install_service()

                assert mock_run.called
                assert success is True

    def test_install_service_sets_properties(self):
        """install_service should set service properties after install."""
        from context_os_events.daemon.service import install_service

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock(returncode=0, stdout="OK")
                success, message = install_service()

                # Should have multiple calls: install + properties
                assert mock_run.call_count > 1
                assert success is True

    def test_uninstall_service_checks_nssm_first(self):
        """uninstall_service should fail gracefully if NSSM not available."""
        from context_os_events.daemon.service import uninstall_service

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=None):
            success, message = uninstall_service()
            assert success is False
            assert "nssm" in message.lower()

    def test_uninstall_service_runs_command(self):
        """uninstall_service should run the uninstall command."""
        from context_os_events.daemon.service import uninstall_service

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock(returncode=0, stdout="Uninstalled")
                success, message = uninstall_service()

                assert mock_run.called
                assert success is True

    def test_uninstall_service_stops_first(self):
        """uninstall_service should try to stop the service before removing."""
        from context_os_events.daemon.service import uninstall_service

        with patch("context_os_events.daemon.service.get_nssm_path", return_value=Path("nssm.exe")):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock(returncode=0, stdout="OK")
                success, message = uninstall_service()

                # Should have at least 2 calls: stop + remove
                assert mock_run.call_count >= 2
                # First call should be stop
                first_call_args = mock_run.call_args_list[0][0][0]
                assert "stop" in first_call_args
