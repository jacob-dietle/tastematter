"""Daemon configuration module.

Loads, validates, and provides access to daemon configuration.
Config file location: ~/.context-os/config.yaml
"""

import copy
from pathlib import Path
from typing import Dict, List, Literal, Optional, TypedDict

import yaml


# ============================================================================
# Type Definitions
# ============================================================================


class SyncConfig(TypedDict):
    """Sync settings for git/session sync."""

    interval_minutes: int
    git_since_days: int


class WatchConfig(TypedDict):
    """Watch settings for file monitoring."""

    enabled: bool
    paths: List[str]
    debounce_ms: int


class ProjectConfig(TypedDict):
    """Project settings."""

    path: Optional[str]


class IntelligenceConfig(TypedDict):
    """Intelligence settings (v2 - AI agent integration)."""

    enabled: bool
    auto_commit: bool
    stale_detection: bool


class LoggingConfig(TypedDict):
    """Logging settings."""

    level: Literal["DEBUG", "INFO", "WARNING", "ERROR"]
    max_size_mb: int
    backup_count: int


class DaemonConfig(TypedDict):
    """Complete daemon configuration."""

    version: int
    sync: SyncConfig
    watch: WatchConfig
    project: ProjectConfig
    intelligence: IntelligenceConfig
    logging: LoggingConfig


# ============================================================================
# Default Configuration
# ============================================================================

DEFAULT_CONFIG: DaemonConfig = {
    "version": 1,
    "sync": {
        "interval_minutes": 30,
        "git_since_days": 7,
    },
    "watch": {
        "enabled": True,
        "paths": ["."],
        "debounce_ms": 100,
    },
    "project": {
        "path": None,
    },
    "intelligence": {
        "enabled": False,
        "auto_commit": False,
        "stale_detection": False,
    },
    "logging": {
        "level": "INFO",
        "max_size_mb": 10,
        "backup_count": 5,
    },
}

VALID_LOG_LEVELS = {"DEBUG", "INFO", "WARNING", "ERROR"}


# ============================================================================
# Functions
# ============================================================================


def get_default_config() -> DaemonConfig:
    """Return default configuration.

    Returns a deep copy to prevent accidental mutation of defaults.
    """
    return copy.deepcopy(DEFAULT_CONFIG)


def ensure_config_dir() -> Path:
    """Create ~/.context-os/ directory if it doesn't exist.

    Returns:
        Path to config directory.
    """
    # Get home directory (works on Windows and Unix)
    home = Path.home()
    config_dir = home / ".context-os"

    config_dir.mkdir(parents=True, exist_ok=True)
    return config_dir


def _deep_merge(base: Dict, overlay: Dict) -> Dict:
    """Deep merge overlay onto base, preserving nested structure.

    Args:
        base: Base dictionary (will not be modified)
        overlay: Dictionary to merge on top

    Returns:
        Merged dictionary
    """
    result = copy.deepcopy(base)

    for key, value in overlay.items():
        if key in result and isinstance(result[key], dict) and isinstance(value, dict):
            result[key] = _deep_merge(result[key], value)
        else:
            result[key] = copy.deepcopy(value)

    return result


def load_config(config_path: Optional[Path] = None) -> DaemonConfig:
    """Load config from file, with defaults for missing values.

    Args:
        config_path: Path to config file. If None, uses ~/.context-os/config.yaml

    Returns:
        Complete configuration with defaults filled in for missing values.
    """
    if config_path is None:
        config_dir = ensure_config_dir()
        config_path = config_dir / "config.yaml"

    defaults = get_default_config()

    # If file doesn't exist, create it with defaults
    if not config_path.exists():
        config_path.parent.mkdir(parents=True, exist_ok=True)
        config_path.write_text(yaml.dump(defaults, default_flow_style=False))
        return defaults

    # Load from file
    with open(config_path, "r") as f:
        file_config = yaml.safe_load(f) or {}

    # Merge with defaults
    merged = _deep_merge(defaults, file_config)
    return merged  # type: ignore


def validate_config(config: DaemonConfig) -> List[str]:
    """Validate config, return list of errors (empty if valid).

    Args:
        config: Configuration to validate

    Returns:
        List of error messages. Empty list means config is valid.
    """
    errors: List[str] = []

    # Validate sync interval
    interval = config.get("sync", {}).get("interval_minutes", 0)
    if not isinstance(interval, int) or interval <= 0:
        errors.append(f"sync.interval_minutes must be a positive integer, got: {interval}")

    # Validate git_since_days
    git_days = config.get("sync", {}).get("git_since_days", 0)
    if not isinstance(git_days, int) or git_days <= 0:
        errors.append(f"sync.git_since_days must be a positive integer, got: {git_days}")

    # Validate log level
    log_level = config.get("logging", {}).get("level", "")
    if log_level not in VALID_LOG_LEVELS:
        errors.append(
            f"logging.level must be one of {sorted(VALID_LOG_LEVELS)}, got: {log_level}"
        )

    # Validate debounce_ms
    debounce = config.get("watch", {}).get("debounce_ms", 0)
    if not isinstance(debounce, int) or debounce < 0:
        errors.append(f"watch.debounce_ms must be a non-negative integer, got: {debounce}")

    # Validate max_size_mb
    max_size = config.get("logging", {}).get("max_size_mb", 0)
    if not isinstance(max_size, int) or max_size <= 0:
        errors.append(f"logging.max_size_mb must be a positive integer, got: {max_size}")

    # Validate backup_count
    backup_count = config.get("logging", {}).get("backup_count", 0)
    if not isinstance(backup_count, int) or backup_count < 0:
        errors.append(
            f"logging.backup_count must be a non-negative integer, got: {backup_count}"
        )

    return errors
