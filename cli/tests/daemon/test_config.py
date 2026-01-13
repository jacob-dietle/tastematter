"""Tests for daemon configuration module.

Phase 1 of daemon implementation - Configuration loading and validation.
Following TDD Red-Green-Refactor cycle.
"""

import tempfile
from pathlib import Path

import pytest
import yaml


class TestGetDefaultConfig:
    """Tests for get_default_config function."""

    def test_get_default_config_returns_valid_structure(self):
        """Default config should have all required sections with valid defaults.

        Expected structure:
        - version: int (1)
        - sync: dict with interval_minutes, git_since_days
        - watch: dict with enabled, paths, debounce_ms
        - project: dict with path
        - intelligence: dict with enabled, auto_commit, stale_detection
        - logging: dict with level, max_size_mb, backup_count
        """
        from context_os_events.daemon.config import get_default_config

        config = get_default_config()

        # Version
        assert config["version"] == 1

        # Sync section
        assert "sync" in config
        assert config["sync"]["interval_minutes"] == 30
        assert config["sync"]["git_since_days"] == 7

        # Watch section
        assert "watch" in config
        assert config["watch"]["enabled"] is True
        assert config["watch"]["paths"] == ["."]
        assert config["watch"]["debounce_ms"] == 100

        # Project section
        assert "project" in config
        assert config["project"]["path"] is None

        # Intelligence section (v2, disabled by default)
        assert "intelligence" in config
        assert config["intelligence"]["enabled"] is False
        assert config["intelligence"]["auto_commit"] is False
        assert config["intelligence"]["stale_detection"] is False

        # Logging section
        assert "logging" in config
        assert config["logging"]["level"] == "INFO"
        assert config["logging"]["max_size_mb"] == 10
        assert config["logging"]["backup_count"] == 5


class TestLoadConfig:
    """Tests for load_config function."""

    def test_load_config_from_file(self, tmp_path: Path):
        """Config should be loaded from YAML file when it exists."""
        from context_os_events.daemon.config import load_config

        # Create custom config file
        config_file = tmp_path / "config.yaml"
        custom_config = {
            "version": 1,
            "sync": {"interval_minutes": 60, "git_since_days": 14},
            "watch": {"enabled": False, "paths": ["src/"], "debounce_ms": 200},
            "project": {"path": "/custom/path"},
            "intelligence": {
                "enabled": True,
                "auto_commit": True,
                "stale_detection": True,
            },
            "logging": {"level": "DEBUG", "max_size_mb": 20, "backup_count": 10},
        }
        config_file.write_text(yaml.dump(custom_config))

        # Load and verify
        config = load_config(config_file)

        assert config["sync"]["interval_minutes"] == 60
        assert config["watch"]["enabled"] is False
        assert config["logging"]["level"] == "DEBUG"

    def test_load_config_uses_defaults_for_missing_keys(self, tmp_path: Path):
        """Missing keys should be filled with defaults."""
        from context_os_events.daemon.config import load_config

        # Create partial config file
        config_file = tmp_path / "config.yaml"
        partial_config = {
            "version": 1,
            "sync": {"interval_minutes": 45},
            # Missing: watch, project, intelligence, logging
        }
        config_file.write_text(yaml.dump(partial_config))

        config = load_config(config_file)

        # Custom value preserved
        assert config["sync"]["interval_minutes"] == 45
        # Missing sync value gets default
        assert config["sync"]["git_since_days"] == 7
        # Missing sections get defaults
        assert config["watch"]["enabled"] is True
        assert config["logging"]["level"] == "INFO"

    def test_load_config_creates_default_if_file_missing(self, tmp_path: Path):
        """If config file doesn't exist, create it with defaults."""
        from context_os_events.daemon.config import load_config

        config_file = tmp_path / "nonexistent" / "config.yaml"
        assert not config_file.exists()

        config = load_config(config_file)

        # Should return defaults
        assert config["version"] == 1
        assert config["sync"]["interval_minutes"] == 30

        # File should now exist
        assert config_file.exists()


class TestValidateConfig:
    """Tests for validate_config function."""

    def test_validate_config_rejects_invalid_interval(self):
        """Interval must be positive integer."""
        from context_os_events.daemon.config import get_default_config, validate_config

        config = get_default_config()
        config["sync"]["interval_minutes"] = -5

        errors = validate_config(config)

        assert len(errors) > 0
        assert any("interval" in e.lower() for e in errors)

    def test_validate_config_rejects_invalid_log_level(self):
        """Log level must be DEBUG, INFO, WARNING, or ERROR."""
        from context_os_events.daemon.config import get_default_config, validate_config

        config = get_default_config()
        config["logging"]["level"] = "VERBOSE"  # Invalid

        errors = validate_config(config)

        assert len(errors) > 0
        assert any("level" in e.lower() for e in errors)

    def test_validate_config_accepts_valid_config(self):
        """Valid config should return empty error list."""
        from context_os_events.daemon.config import get_default_config, validate_config

        config = get_default_config()

        errors = validate_config(config)

        assert errors == []


class TestEnsureConfigDir:
    """Tests for ensure_config_dir function."""

    def test_ensure_config_dir_creates_directory(self, tmp_path: Path, monkeypatch):
        """Should create ~/.context-os/ directory if it doesn't exist."""
        from context_os_events.daemon.config import ensure_config_dir

        # Mock home directory to tmp_path
        fake_home = tmp_path / "home"
        monkeypatch.setenv("USERPROFILE", str(fake_home))  # Windows
        monkeypatch.setenv("HOME", str(fake_home))  # Unix

        config_dir = ensure_config_dir()

        assert config_dir.exists()
        assert config_dir.is_dir()
        assert config_dir.name == ".context-os"

    def test_ensure_config_dir_returns_existing(self, tmp_path: Path, monkeypatch):
        """Should return existing directory without error."""
        from context_os_events.daemon.config import ensure_config_dir

        # Create directory first
        fake_home = tmp_path / "home"
        existing_dir = fake_home / ".context-os"
        existing_dir.mkdir(parents=True)

        monkeypatch.setenv("USERPROFILE", str(fake_home))
        monkeypatch.setenv("HOME", str(fake_home))

        config_dir = ensure_config_dir()

        assert config_dir == existing_dir
        assert config_dir.exists()
