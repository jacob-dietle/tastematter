"""Daemon service module.

Manages Windows service installation via NSSM (Non-Sucking Service Manager).
NSSM wraps any executable as a Windows service with:
- Automatic restart on failure
- Proper service lifecycle
- Support for any executable

NSSM docs: https://nssm.cc/
"""

import logging
import os
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Tuple

logger = logging.getLogger(__name__)

# Service constants
SERVICE_NAME = "ContextOSEvents"
SERVICE_DISPLAY_NAME = "Context OS Events Daemon"
SERVICE_DESCRIPTION = "Background daemon for Context OS Events - captures file changes, git commits, and Claude sessions"

# NSSM location (installed via winget)
NSSM_PATHS = [
    Path(os.environ.get("LOCALAPPDATA", "")) / "Microsoft" / "WinGet" / "Links" / "nssm.exe",
    Path("C:/Program Files/NSSM/win64/nssm.exe"),
    Path("C:/Program Files (x86)/NSSM/win32/nssm.exe"),
]


def get_service_name() -> str:
    """Return service name constant.

    Returns:
        Service name: 'ContextOSEvents'
    """
    return SERVICE_NAME


def get_python_path() -> Path:
    """Return path to pythonw.exe (windowless Python).

    Uses the Python from the current virtual environment.
    pythonw.exe runs without a console window, suitable for services.

    Returns:
        Path to pythonw.exe
    """
    # Get the directory containing the current Python executable
    python_dir = Path(sys.executable).parent
    pythonw = python_dir / "pythonw.exe"

    if pythonw.exists():
        return pythonw

    # Fallback to python.exe if pythonw doesn't exist
    return Path(sys.executable)


def get_runner_module() -> str:
    """Return the daemon runner module path.

    Returns:
        Module path: 'context_os_events.daemon.runner'
    """
    return "context_os_events.daemon.runner"


def get_nssm_path() -> Optional[Path]:
    """Find NSSM executable.

    Returns:
        Path to nssm.exe if found, None otherwise.
    """
    # First check PATH
    try:
        result = subprocess.run(
            ["where", "nssm"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0 and result.stdout.strip():
            return Path(result.stdout.strip().split("\n")[0])
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    # Check known locations
    for path in NSSM_PATHS:
        if path.exists():
            return path

    return None


def is_nssm_available() -> bool:
    """Check if NSSM is available.

    Returns:
        True if nssm can be executed, False otherwise.
    """
    nssm_path = get_nssm_path()
    if nssm_path is None:
        return False

    try:
        result = subprocess.run(
            [str(nssm_path), "version"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        return result.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False


# Keep old name for backwards compatibility
is_servy_available = is_nssm_available


def get_service_status() -> Optional[str]:
    """Get current service status.

    Uses Windows 'sc query' command to check service state.

    Returns:
        'Running', 'Stopped', or None if not installed.
    """
    try:
        result = subprocess.run(
            ["sc", "query", SERVICE_NAME],
            capture_output=True,
            text=True,
            timeout=10,
        )

        if result.returncode != 0:
            # Service not found
            return None

        stdout = result.stdout

        if "RUNNING" in stdout:
            return "Running"
        elif "STOPPED" in stdout:
            return "Stopped"
        elif "PAUSED" in stdout:
            return "Paused"
        elif "PENDING" in stdout:
            return "Pending"
        else:
            return "Unknown"

    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        logger.error(f"Failed to query service status: {e}")
        return None


def build_install_command(config_path: Optional[Path] = None) -> List[str]:
    """Build NSSM install command.

    Args:
        config_path: Optional path to config file to pass to runner.

    Returns:
        Command as list of arguments for the initial install.
    """
    nssm_path = get_nssm_path()
    python_path = get_python_path()
    runner_module = get_runner_module()

    # Build args for pythonw.exe -m module
    args = f"-m {runner_module}"
    if config_path:
        args += f" --config \"{config_path}\""

    # NSSM install command: nssm install <servicename> <program> [<arguments>]
    cmd = [
        str(nssm_path) if nssm_path else "nssm",
        "install",
        SERVICE_NAME,
        str(python_path),
        args,
    ]

    return cmd


def install_service(config_path: Optional[Path] = None) -> Tuple[bool, str]:
    """Install daemon as Windows service via NSSM.

    Requires:
    - NSSM installed
    - Admin privileges

    Args:
        config_path: Optional path to config file.

    Returns:
        Tuple of (success: bool, message: str)
    """
    # Check NSSM availability first
    nssm_path = get_nssm_path()
    if nssm_path is None:
        return False, (
            "NSSM not found. Please install via: winget install NSSM.NSSM"
        )

    python_path = get_python_path()
    runner_module = get_runner_module()

    # Build args for pythonw.exe -m module
    args = f"-m {runner_module}"
    if config_path:
        args += f" --config \"{config_path}\""

    try:
        # Step 1: Install the service
        install_cmd = [str(nssm_path), "install", SERVICE_NAME, str(python_path), args]
        result = subprocess.run(
            install_cmd,
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            error_msg = result.stderr or result.stdout or "Unknown error"
            logger.error(f"Failed to install service: {error_msg}")
            return False, f"Installation failed: {error_msg}"

        # Step 2: Set service properties
        nssm = str(nssm_path)
        properties = [
            ([nssm, "set", SERVICE_NAME, "DisplayName", SERVICE_DISPLAY_NAME], "DisplayName"),
            ([nssm, "set", SERVICE_NAME, "Description", SERVICE_DESCRIPTION], "Description"),
            ([nssm, "set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"], "Start"),
            ([nssm, "set", SERVICE_NAME, "AppStdout", str(Path.home() / ".context-os" / "daemon.stdout.log")], "AppStdout"),
            ([nssm, "set", SERVICE_NAME, "AppStderr", str(Path.home() / ".context-os" / "daemon.stderr.log")], "AppStderr"),
            ([nssm, "set", SERVICE_NAME, "AppRotateFiles", "1"], "AppRotateFiles"),
            ([nssm, "set", SERVICE_NAME, "AppRotateBytes", "1048576"], "AppRotateBytes"),  # 1MB
        ]

        for cmd, prop_name in properties:
            prop_result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            if prop_result.returncode != 0:
                logger.warning(f"Failed to set {prop_name}: {prop_result.stderr}")

        logger.info(f"Service installed: {SERVICE_NAME}")
        return True, f"Service '{SERVICE_DISPLAY_NAME}' installed successfully"

    except subprocess.TimeoutExpired:
        return False, "Installation timed out"
    except Exception as e:
        return False, f"Installation error: {e}"


def uninstall_service() -> Tuple[bool, str]:
    """Uninstall the Windows service.

    Requires:
    - NSSM installed
    - Admin privileges

    Returns:
        Tuple of (success: bool, message: str)
    """
    nssm_path = get_nssm_path()
    if nssm_path is None:
        return False, "NSSM not found"

    try:
        # Stop service first if running
        stop_result = subprocess.run(
            [str(nssm_path), "stop", SERVICE_NAME],
            capture_output=True,
            text=True,
            timeout=30,
        )
        # Ignore stop errors - service might not be running

        # Remove the service
        result = subprocess.run(
            [str(nssm_path), "remove", SERVICE_NAME, "confirm"],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            logger.info(f"Service uninstalled: {SERVICE_NAME}")
            return True, f"Service '{SERVICE_DISPLAY_NAME}' uninstalled successfully"
        else:
            error_msg = result.stderr or result.stdout or "Unknown error"
            return False, f"Uninstallation failed: {error_msg}"

    except subprocess.TimeoutExpired:
        return False, "Uninstallation timed out"
    except Exception as e:
        return False, f"Uninstallation error: {e}"


def start_service() -> Tuple[bool, str]:
    """Start the service.

    Returns:
        Tuple of (success: bool, message: str)
    """
    try:
        result = subprocess.run(
            ["sc", "start", SERVICE_NAME],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            return True, "Service started"
        else:
            return False, result.stderr or result.stdout or "Failed to start"

    except Exception as e:
        return False, f"Start error: {e}"


def stop_service() -> Tuple[bool, str]:
    """Stop the service.

    Returns:
        Tuple of (success: bool, message: str)
    """
    try:
        result = subprocess.run(
            ["sc", "stop", SERVICE_NAME],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            return True, "Service stopped"
        else:
            return False, result.stderr or result.stdout or "Failed to stop"

    except Exception as e:
        return False, f"Stop error: {e}"
