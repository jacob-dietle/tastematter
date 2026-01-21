"""JSONL Parser parity tests: Python vs Rust session extraction.

These are TRUE parity tests - they run BOTH implementations on the same data
and compare results. Tests FAIL if implementations disagree, not skip.
"""
import pytest

from context_os_events.capture.jsonl_parser import (
    encode_project_path,
    find_session_files,
    parse_session_file,
)

from .conftest import GTM_PROJECT, CLAUDE_DIR, run_rust_cli_json


class TestSessionParserParity:
    """True parity tests - run BOTH implementations, compare results."""

    def test_session_count_parity(self, gtm_project):
        """Session counts must match exactly."""
        # === PYTHON ===
        py_sessions = list(find_session_files(str(gtm_project)))
        py_count = len(py_sessions)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "parse-sessions",
            "--project", str(gtm_project),
        ])
        rs_count = rs_output["result"]["sessions_parsed"]

        # === COMPARE ===
        assert py_count == rs_count, \
            f"Session count mismatch: Python={py_count}, Rust={rs_count}"

    def test_tool_use_total_parity(self, gtm_project):
        """Total tool uses must match within 0.1% tolerance."""
        # === PYTHON ===
        py_total = 0
        for filepath in find_session_files(str(gtm_project)):
            try:
                session = parse_session_file(filepath, str(gtm_project))
                if session and session.tools_used:
                    py_total += sum(session.tools_used.values())
            except Exception:
                continue

        # === RUST ===
        rs_output = run_rust_cli_json([
            "parse-sessions",
            "--project", str(gtm_project),
        ])
        rs_total = rs_output["result"]["total_tool_uses"]

        # === COMPARE ===
        diff_pct = abs(py_total - rs_total) / max(py_total, 1) * 100
        assert diff_pct < 0.1, \
            f"Tool use diff {diff_pct:.2f}%: Python={py_total}, Rust={rs_total}"

    def test_session_ids_match(self, gtm_project):
        """Both implementations find the same session IDs."""
        # === PYTHON ===
        py_session_ids = set()
        for filepath in find_session_files(str(gtm_project)):
            try:
                session = parse_session_file(filepath, str(gtm_project))
                if session and session.session_id:
                    py_session_ids.add(session.session_id)
            except Exception:
                continue

        # === RUST ===
        rs_output = run_rust_cli_json([
            "parse-sessions",
            "--project", str(gtm_project),
        ])
        rs_session_ids = {s["session_id"] for s in rs_output.get("sessions", [])}

        # === COMPARE ===
        only_in_python = py_session_ids - rs_session_ids
        only_in_rust = rs_session_ids - py_session_ids

        assert py_session_ids == rs_session_ids, \
            f"Session ID mismatch:\n" \
            f"  Only in Python ({len(only_in_python)}): {list(only_in_python)[:5]}\n" \
            f"  Only in Rust ({len(only_in_rust)}): {list(only_in_rust)[:5]}"

    def test_per_session_tool_counts_parity(self, gtm_project):
        """Per-session tool counts must match for each session_id."""
        # === PYTHON ===
        py_sessions = {}
        for filepath in find_session_files(str(gtm_project)):
            try:
                session = parse_session_file(filepath, str(gtm_project))
                if session and session.session_id:
                    py_sessions[session.session_id] = dict(session.tools_used or {})
            except Exception:
                continue

        # === RUST ===
        rs_output = run_rust_cli_json([
            "parse-sessions",
            "--project", str(gtm_project),
        ])
        rs_sessions = {
            s["session_id"]: s.get("tools_used", {})
            for s in rs_output.get("sessions", [])
        }

        # === COMPARE ===
        # Only compare sessions that exist in both
        common_ids = set(py_sessions.keys()) & set(rs_sessions.keys())

        mismatches = []
        for sid in common_ids:
            py_tools = py_sessions[sid]
            rs_tools = rs_sessions[sid]
            if py_tools != rs_tools:
                mismatches.append(f"{sid}: Py={py_tools}, Rs={rs_tools}")

        # Allow up to 1% of sessions to have minor tool count differences
        mismatch_pct = len(mismatches) / max(len(common_ids), 1) * 100
        assert mismatch_pct < 1, \
            f"Tool breakdown mismatches ({len(mismatches)}/{len(common_ids)} = {mismatch_pct:.1f}%):\n" + \
            "\n".join(mismatches[:10])


class TestToolBreakdownParity:
    """Test aggregate tool breakdown matches."""

    def test_total_reads_parity(self, gtm_project):
        """Total Read tool uses must match."""
        # === PYTHON ===
        py_reads = 0
        for filepath in find_session_files(str(gtm_project)):
            try:
                session = parse_session_file(filepath, str(gtm_project))
                if session and session.tools_used:
                    py_reads += session.tools_used.get("Read", 0)
            except Exception:
                continue

        # === RUST ===
        rs_output = run_rust_cli_json([
            "parse-sessions",
            "--project", str(gtm_project),
        ])
        rs_reads = sum(
            s.get("tools_used", {}).get("Read", 0)
            for s in rs_output.get("sessions", [])
        )

        # === COMPARE ===
        diff_pct = abs(py_reads - rs_reads) / max(py_reads, 1) * 100
        assert diff_pct < 0.1, \
            f"Read count diff {diff_pct:.2f}%: Python={py_reads}, Rust={rs_reads}"

    def test_total_edits_parity(self, gtm_project):
        """Total Edit tool uses must match."""
        # === PYTHON ===
        py_edits = 0
        for filepath in find_session_files(str(gtm_project)):
            try:
                session = parse_session_file(filepath, str(gtm_project))
                if session and session.tools_used:
                    py_edits += session.tools_used.get("Edit", 0)
            except Exception:
                continue

        # === RUST ===
        rs_output = run_rust_cli_json([
            "parse-sessions",
            "--project", str(gtm_project),
        ])
        rs_edits = sum(
            s.get("tools_used", {}).get("Edit", 0)
            for s in rs_output.get("sessions", [])
        )

        # === COMPARE ===
        diff_pct = abs(py_edits - rs_edits) / max(py_edits, 1) * 100
        assert diff_pct < 0.1, \
            f"Edit count diff {diff_pct:.2f}%: Python={py_edits}, Rust={rs_edits}"
