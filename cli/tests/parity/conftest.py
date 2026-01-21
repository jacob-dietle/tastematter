"""Shared fixtures for Python vs Rust parity tests."""
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any

import pytest

# Project paths
GTM_PROJECT = Path(r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system")
PIXEE_PROJECT = GTM_PROJECT / "03_gtm_engagements/03_active_client/pixee_ai_gtm/Pixee"
RUST_CLI = GTM_PROJECT / "apps/tastematter/core/target/release/context-os"
CLAUDE_DIR = Path.home() / ".claude"


@pytest.fixture
def gtm_project() -> Path:
    """GTM Operating System project path."""
    return GTM_PROJECT


@pytest.fixture
def pixee_project() -> Path:
    """Pixee AI GTM project path."""
    return PIXEE_PROJECT


@pytest.fixture
def claude_dir() -> Path:
    """Claude data directory."""
    return CLAUDE_DIR


@pytest.fixture
def rust_cli() -> Path:
    """Path to Rust CLI binary."""
    return RUST_CLI


def run_rust_cli(args: list[str], timeout: int = 300) -> str:
    """Execute Rust CLI and return stdout.

    Args:
        args: Command line arguments
        timeout: Timeout in seconds (default 5 minutes for large projects)

    Returns:
        stdout as string

    Raises:
        subprocess.CalledProcessError: If command fails
    """
    result = subprocess.run(
        [str(RUST_CLI)] + args,
        capture_output=True,
        text=True,
        timeout=timeout,
        encoding='utf-8',
        errors='replace',
    )
    if result.returncode != 0:
        raise subprocess.CalledProcessError(
            result.returncode,
            [str(RUST_CLI)] + args,
            result.stdout,
            result.stderr
        )
    return result.stdout


def run_rust_cli_json(args: list[str], timeout: int = 300) -> dict[str, Any]:
    """Execute Rust CLI and return parsed JSON output.

    Args:
        args: Command line arguments (--format json is added automatically)
        timeout: Timeout in seconds

    Returns:
        Parsed JSON dict
    """
    if "--format" not in args:
        args = args + ["--format", "json"]
    output = run_rust_cli(args, timeout)
    return json.loads(output)


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test files."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def create_test_jsonl(temp_dir):
    """Factory fixture to create test JSONL files."""
    def _create(filename: str, records: list[dict]) -> Path:
        filepath = temp_dir / filename
        with open(filepath, "w") as f:
            for record in records:
                f.write(json.dumps(record) + "\n")
        return filepath
    return _create
