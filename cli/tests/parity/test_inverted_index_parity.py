"""Inverted Index parity tests: Python vs Rust file indexing.

These are TRUE parity tests - they run BOTH implementations on the same data
and compare results. Tests FAIL if implementations disagree, not skip.
"""
import pytest
from collections import defaultdict

from context_os_events.capture.jsonl_parser import encode_project_path, find_session_files, parse_session_file
from context_os_events.index.inverted_index import (
    build_inverted_index,
    READ_TOOLS,
    WRITE_TOOLS,
    CREATE_TOOLS,
)

from .conftest import GTM_PROJECT, CLAUDE_DIR, run_rust_cli_json


def classify_access_type(tool_name: str) -> str | None:
    """Classify tool to access type."""
    if tool_name in READ_TOOLS:
        return "read"
    elif tool_name in WRITE_TOOLS:
        return "write"
    elif tool_name in CREATE_TOOLS:
        return "create"
    return None


class TestInvertedIndexParity:
    """True parity tests for file indexing."""

    def test_unique_file_count_parity(self, gtm_project):
        """Unique file counts must match within tolerance."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        # build_inverted_index returns Dict[str, List[FileAccess]]
        py_index = build_inverted_index(project_dir)
        py_files = len(py_index)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "index-files",
            "--project", str(gtm_project),
        ])
        rs_files = rs_output["unique_files"]

        # === COMPARE ===
        diff_pct = abs(py_files - rs_files) / max(py_files, 1) * 100
        assert diff_pct < 5, \
            f"Unique file count diff {diff_pct:.2f}%: Python={py_files}, Rust={rs_files}"

    def test_total_accesses_parity(self, gtm_project):
        """Total file accesses must match within tolerance."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_index = build_inverted_index(project_dir)
        py_accesses = sum(len(v) for v in py_index.values())

        # === RUST ===
        rs_output = run_rust_cli_json([
            "index-files",
            "--project", str(gtm_project),
        ])
        rs_accesses = rs_output["accesses_indexed"]

        # === COMPARE ===
        diff_pct = abs(py_accesses - rs_accesses) / max(py_accesses, 1) * 100
        assert diff_pct < 5, \
            f"Total accesses diff {diff_pct:.2f}%: Python={py_accesses}, Rust={rs_accesses}"

    def test_unique_sessions_parity(self, gtm_project):
        """Unique sessions with file access must match within tolerance."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_index = build_inverted_index(project_dir)
        # Build session set from all accesses
        py_sessions = set()
        for accesses in py_index.values():
            for access in accesses:
                py_sessions.add(access.session_id)
        py_session_count = len(py_sessions)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "index-files",
            "--project", str(gtm_project),
        ])
        rs_sessions = rs_output["unique_sessions"]

        # === COMPARE ===
        diff_pct = abs(py_session_count - rs_sessions) / max(py_session_count, 1) * 100
        assert diff_pct < 10, \
            f"Unique sessions diff {diff_pct:.2f}%: Python={py_session_count}, Rust={rs_sessions}"


class TestAccessTypeClassification:
    """Test access type classification matches."""

    def test_read_tools_classification(self):
        """Read tools should be classified as 'read'."""
        read_tools = ["Read", "Glob", "Grep", "WebFetch", "WebSearch"]
        for tool in read_tools:
            result = classify_access_type(tool)
            assert result == "read", f"{tool} should be 'read', got {result}"

    def test_write_tools_classification(self):
        """Write tools should be classified as 'write'."""
        write_tools = ["Edit", "NotebookEdit"]
        for tool in write_tools:
            result = classify_access_type(tool)
            assert result == "write", f"{tool} should be 'write', got {result}"

    def test_create_tools_classification(self):
        """Create tools should be classified as 'create'."""
        create_tools = ["Write"]
        for tool in create_tools:
            result = classify_access_type(tool)
            assert result == "create", f"{tool} should be 'create', got {result}"

    def test_non_file_tools_excluded(self):
        """Non-file tools should return None."""
        non_file_tools = ["Bash", "Task", "AskUserQuestion", "TodoWrite"]
        for tool in non_file_tools:
            result = classify_access_type(tool)
            assert result is None, f"{tool} should be None, got {result}"


class TestIndexConsistency:
    """Test internal consistency of index."""

    def test_no_grep_glob_patterns_in_index(self, gtm_project):
        """Grep/Glob patterns should NOT appear as file paths."""
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_index = build_inverted_index(project_dir)

        pattern_files = [
            f for f in py_index.keys()
            if f.startswith("GREP:") or f.startswith("GLOB:") or "*" in f
        ]

        assert not pattern_files, \
            f"Found {len(pattern_files)} pattern entries in index: {pattern_files[:5]}"

    def test_accesses_have_valid_session_ids(self, gtm_project):
        """All accesses should have non-empty session IDs."""
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_index = build_inverted_index(project_dir)

        invalid_accesses = []
        for file_path, accesses in py_index.items():
            for access in accesses:
                if not access.session_id:
                    invalid_accesses.append(f"{file_path}: empty session_id")

        assert not invalid_accesses, \
            f"Found {len(invalid_accesses)} invalid accesses:\n" + "\n".join(invalid_accesses[:10])
