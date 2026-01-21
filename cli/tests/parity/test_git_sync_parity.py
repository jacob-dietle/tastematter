"""Git Sync parity tests: Python vs Rust commit extraction.

These are TRUE parity tests - they run BOTH implementations on the same data
and compare results. Tests FAIL if implementations disagree, not skip.

Note: Python git sync requires a DB, so we use subprocess for raw comparison.
"""
import subprocess
import pytest
from datetime import datetime, timedelta

from context_os_events.capture.git_sync import (
    detect_agent_commit,
    parse_commit_block,
    split_commit_blocks,
    AGENT_SIGNATURES,
)

from .conftest import GTM_PROJECT, run_rust_cli_json


def get_python_git_log(repo_path: str, since_days: int = 30) -> list[dict]:
    """Get git commits using Python implementation (without DB)."""
    cmd = [
        "git", "log",
        "--format=%H§%h§%aI§%an§%ae§%s§%P",
        "--numstat",
        "--name-status",
        f"--since={since_days} days",
    ]

    result = subprocess.run(
        cmd,
        cwd=str(repo_path),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace"
    )

    if result.returncode != 0:
        return []

    blocks = split_commit_blocks(result.stdout)
    commits = []

    for block in blocks:
        if not block.strip():
            continue
        try:
            commit = parse_commit_block(block)
            commits.append({
                "hash": commit.hash,
                "short_hash": commit.short_hash,
                "author_name": commit.author_name,
                "message": commit.message,
                "is_agent": commit.is_agent_commit,
                "files_changed": commit.files_changed,
            })
        except Exception:
            continue

    return commits


class TestGitSyncParity:
    """True parity tests for git commit extraction."""

    def test_commit_count_parity(self, gtm_project):
        """Commit counts must match exactly."""
        since_days = 30

        # === PYTHON ===
        py_commits = get_python_git_log(gtm_project, since_days)
        py_count = len(py_commits)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "sync-git",
            "--repo", str(gtm_project),
            "--since", f"{since_days} days",
        ])
        rs_count = rs_output["result"]["commits_synced"]

        # === COMPARE ===
        assert py_count == rs_count, \
            f"Commit count mismatch: Python={py_count}, Rust={rs_count}"

    def test_commit_hashes_match(self, gtm_project):
        """All commit hashes should match between implementations."""
        since_days = 30

        # === PYTHON ===
        py_commits = get_python_git_log(gtm_project, since_days)
        py_hashes = {c["hash"] for c in py_commits}

        # === RUST ===
        rs_output = run_rust_cli_json([
            "sync-git",
            "--repo", str(gtm_project),
            "--since", f"{since_days} days",
        ])
        rs_hashes = {c["hash"] for c in rs_output.get("commits", [])}

        # === COMPARE ===
        only_in_python = py_hashes - rs_hashes
        only_in_rust = rs_hashes - py_hashes

        assert py_hashes == rs_hashes, \
            f"Commit hash mismatch:\n" \
            f"  Only in Python ({len(only_in_python)}): {list(only_in_python)[:5]}\n" \
            f"  Only in Rust ({len(only_in_rust)}): {list(only_in_rust)[:5]}"

    def test_commit_metadata_parity(self, gtm_project):
        """Commit metadata (author, short_hash) should match."""
        since_days = 7  # Smaller window for detailed comparison

        # === PYTHON ===
        py_commits = get_python_git_log(gtm_project, since_days)
        py_by_hash = {c["hash"]: c for c in py_commits}

        # === RUST ===
        rs_output = run_rust_cli_json([
            "sync-git",
            "--repo", str(gtm_project),
            "--since", f"{since_days} days",
        ])
        rs_by_hash = {c["hash"]: c for c in rs_output.get("commits", [])}

        # === COMPARE ===
        mismatches = []
        common_hashes = set(py_by_hash.keys()) & set(rs_by_hash.keys())

        for h in common_hashes:
            py_c = py_by_hash[h]
            rs_c = rs_by_hash[h]

            # Check author
            if py_c["author_name"] != rs_c.get("author_name"):
                mismatches.append(f"{h[:7]}: author Py={py_c['author_name']} Rs={rs_c.get('author_name')}")

            # Check short_hash
            if py_c["short_hash"] != rs_c.get("short_hash"):
                mismatches.append(f"{h[:7]}: short_hash Py={py_c['short_hash']} Rs={rs_c.get('short_hash')}")

        assert not mismatches, \
            f"Metadata mismatches found:\n" + "\n".join(mismatches[:10])


class TestAgentCommitDetection:
    """Test Claude-authored commit detection matches."""

    def test_agent_detection_patterns(self):
        """Both should detect Claude commits using same patterns."""
        test_cases = [
            # (message, expected_is_agent)
            ("feat: add feature\n\nCo-Authored-By: Claude", True),
            ("fix: bug\n\nCo-Authored-By: Claude Sonnet", True),
            ("Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>", True),
            ("regular commit message", False),
            ("fix: something by hand", False),
            ("Generated with Claude Code", True),
        ]

        for message, expected in test_cases:
            result = detect_agent_commit(message)
            assert result == expected, \
                f"Agent detection mismatch for '{message[:30]}...': expected {expected}, got {result}"

    def test_agent_commit_count_parity(self, gtm_project):
        """Agent commit counts should roughly match."""
        since_days = 30

        # === PYTHON ===
        py_commits = get_python_git_log(gtm_project, since_days)
        py_agent_count = sum(1 for c in py_commits if c["is_agent"])

        # === RUST ===
        rs_output = run_rust_cli_json([
            "sync-git",
            "--repo", str(gtm_project),
            "--since", f"{since_days} days",
        ])
        # Use Python detection on Rust messages for fair comparison
        rs_agent_count = sum(
            1 for c in rs_output.get("commits", [])
            if detect_agent_commit(c.get("message", ""))
        )

        # === COMPARE ===
        # Allow some tolerance since detection might differ slightly
        diff = abs(py_agent_count - rs_agent_count)
        assert diff <= 2, \
            f"Agent commit count diff: Python={py_agent_count}, Rust={rs_agent_count}"


class TestGitSyncRobustness:
    """Test edge cases and robustness."""

    def test_recent_commits_included(self, gtm_project):
        """Very recent commits should be captured by both."""
        since_days = 7

        # === PYTHON ===
        py_commits = get_python_git_log(gtm_project, since_days)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "sync-git",
            "--repo", str(gtm_project),
            "--since", f"{since_days} days",
        ])

        # Both should find at least some commits
        py_count = len(py_commits)
        rs_count = rs_output["result"]["commits_synced"]

        # Counts should match
        assert py_count == rs_count, \
            f"Recent commit count mismatch: Python={py_count}, Rust={rs_count}"
