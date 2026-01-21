"""Chain Graph parity tests: Python vs Rust chain building.

These are TRUE parity tests - they run BOTH implementations on the same data
and compare results. Tests FAIL if implementations disagree, not skip.
"""
import pytest

from context_os_events.capture.jsonl_parser import encode_project_path
from context_os_events.index.chain_graph import build_chain_graph

from .conftest import GTM_PROJECT, CLAUDE_DIR, run_rust_cli_json


class TestChainGraphParity:
    """True parity tests for chain building."""

    def test_chain_count_parity(self, gtm_project):
        """Chain counts must match exactly."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_chains = build_chain_graph(project_dir)
        py_count = len(py_chains)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "build-chains",
            "--project", str(gtm_project),
        ])
        rs_count = rs_output["result"]["chains_built"]

        # === COMPARE ===
        assert py_count == rs_count, \
            f"Chain count mismatch: Python={py_count}, Rust={rs_count}"

    def test_sessions_linked_parity(self, gtm_project):
        """Total sessions linked to chains must match."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_chains = build_chain_graph(project_dir)
        py_linked = sum(len(c.sessions) for c in py_chains.values())

        # === RUST ===
        rs_output = run_rust_cli_json([
            "build-chains",
            "--project", str(gtm_project),
        ])
        rs_linked = rs_output["result"]["sessions_linked"]

        # === COMPARE ===
        assert py_linked == rs_linked, \
            f"Sessions linked mismatch: Python={py_linked}, Rust={rs_linked}"

    def test_largest_chain_parity(self, gtm_project):
        """Largest chain size must match."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_chains = build_chain_graph(project_dir)
        py_largest = max(len(c.sessions) for c in py_chains.values()) if py_chains else 0

        # === RUST ===
        rs_output = run_rust_cli_json([
            "build-chains",
            "--project", str(gtm_project),
        ])
        rs_largest = rs_output["largest_chain"]

        # === COMPARE ===
        assert py_largest == rs_largest, \
            f"Largest chain mismatch: Python={py_largest}, Rust={rs_largest}"

    def test_orphan_count_parity(self, gtm_project):
        """Orphan session counts must match within tolerance."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_chains = build_chain_graph(project_dir)
        # Orphans = chains with only 1 session (no links)
        py_orphans = sum(1 for c in py_chains.values() if len(c.sessions) == 1)

        # === RUST ===
        rs_output = run_rust_cli_json([
            "build-chains",
            "--project", str(gtm_project),
        ])
        rs_orphans = rs_output["result"]["orphan_sessions"]

        # === COMPARE ===
        # Allow 5% tolerance for orphan counting (different heuristics)
        diff_pct = abs(py_orphans - rs_orphans) / max(py_orphans, 1) * 100
        assert diff_pct < 5, \
            f"Orphan count diff {diff_pct:.1f}%: Python={py_orphans}, Rust={rs_orphans}"


class TestChainTopologyParity:
    """Test chain topology matches between implementations."""

    def test_root_sessions_match(self, gtm_project):
        """Root sessions (chain starters) should largely match."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_chains = build_chain_graph(project_dir)
        py_roots = {c.root_session for c in py_chains.values() if c.root_session}

        # === RUST ===
        rs_output = run_rust_cli_json([
            "build-chains",
            "--project", str(gtm_project),
        ])
        rs_roots = {
            chain["root_session"]
            for chain in rs_output.get("chains", {}).values()
            if chain.get("root_session")
        }

        # === COMPARE ===
        # Check overlap percentage
        common = py_roots & rs_roots
        overlap_pct = len(common) / max(len(py_roots), len(rs_roots), 1) * 100

        assert overlap_pct > 90, \
            f"Root session overlap only {overlap_pct:.1f}%:\n" \
            f"  Python roots: {len(py_roots)}\n" \
            f"  Rust roots: {len(rs_roots)}\n" \
            f"  Common: {len(common)}"

    def test_agent_sessions_in_chains(self, gtm_project):
        """Agent sessions (agent-*) should be linked in both."""
        # === PYTHON ===
        encoded = encode_project_path(gtm_project)
        project_dir = CLAUDE_DIR / "projects" / encoded
        py_chains = build_chain_graph(project_dir)
        py_agents = 0
        for chain in py_chains.values():
            for sid in chain.sessions:
                if sid.startswith("agent-"):
                    py_agents += 1

        # === RUST ===
        rs_output = run_rust_cli_json([
            "build-chains",
            "--project", str(gtm_project),
        ])
        rs_agents = 0
        for chain in rs_output.get("chains", {}).values():
            for sid in chain.get("sessions", []):
                if sid.startswith("agent-"):
                    rs_agents += 1

        # === COMPARE ===
        diff_pct = abs(py_agents - rs_agents) / max(py_agents, 1) * 100
        assert diff_pct < 5, \
            f"Agent session diff {diff_pct:.1f}%: Python={py_agents}, Rust={rs_agents}"
