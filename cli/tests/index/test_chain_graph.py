"""Tests for chain_graph module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).

The chain graph is built from Claude Code's leafUuid mechanism:
- Summary records at start of JSONL have {"type":"summary","leafUuid":"..."}
- The leafUuid points to a message.uuid in the parent conversation
- This gives us explicit chain linking without heuristics
"""

import json
import tempfile
from datetime import datetime
from pathlib import Path

import pytest


class TestExtractLeafUuids:
    """Test extracting leafUuid references from JSONL files."""

    def test_extract_leaf_uuids_finds_summary_records(self):
        """Should extract LAST leafUuid from summary records.

        IMPORTANT: Claude Code stacks summaries oldest-first when continuing:
        - Session C continues B continues A
        - C gets: [summary from A, summary from B]
        - FIRST summary → original root (A)
        - LAST summary → immediate parent (B) ← We want this

        See Package 11 investigation (2026-01-15) for empirical verification.
        """
        from context_os_events.index.chain_graph import extract_leaf_uuids

        # Create test JSONL with summary records
        jsonl_content = [
            '{"type":"summary","summary":"First topic","leafUuid":"uuid-parent-1"}',
            '{"type":"summary","summary":"Second topic","leafUuid":"uuid-parent-2"}',
            '{"type":"user","message":{"role":"user","content":"Hello"}}',
            '{"type":"assistant","message":{"role":"assistant","content":"Hi"}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            leaf_uuids = extract_leaf_uuids(filepath)

            # Implementation returns LAST leafUuid only (immediate parent)
            # See Package 11: Claude Code stacks summaries oldest-first
            assert len(leaf_uuids) == 1
            assert leaf_uuids[0] == "uuid-parent-2"  # LAST summary's leafUuid
        finally:
            filepath.unlink()

    def test_extract_leaf_uuids_handles_no_summaries(self):
        """Files without summary records should return empty list."""
        from context_os_events.index.chain_graph import extract_leaf_uuids

        jsonl_content = [
            '{"type":"user","message":{"role":"user","content":"Hello"}}',
            '{"type":"assistant","message":{"role":"assistant","content":"Hi"}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            leaf_uuids = extract_leaf_uuids(filepath)
            assert len(leaf_uuids) == 0
        finally:
            filepath.unlink()

    def test_extract_leaf_uuids_handles_malformed_json(self):
        """Malformed JSON lines should be skipped, not crash."""
        from context_os_events.index.chain_graph import extract_leaf_uuids

        jsonl_content = [
            '{"type":"summary","summary":"Valid","leafUuid":"uuid-1"}',
            'this is not valid json',
            '{"type":"summary","summary":"Also valid","leafUuid":"uuid-2"}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            leaf_uuids = extract_leaf_uuids(filepath)
            # Returns LAST valid leafUuid only
            assert len(leaf_uuids) == 1
            assert leaf_uuids[0] == "uuid-2"  # LAST valid summary
        finally:
            filepath.unlink()


class TestExtractMessageUuids:
    """Test extracting message.uuid from JSONL files."""

    def test_extract_message_uuids_finds_uuid_field(self):
        """Should extract uuid field from message records.

        RED: Run before implementation - should fail
        GREEN: Implement extract_message_uuids()

        Real JSONL structure has uuid at top level of message records.
        """
        from context_os_events.index.chain_graph import extract_message_uuids

        jsonl_content = [
            '{"type":"summary","summary":"Topic","leafUuid":"leaf-1"}',
            '{"type":"user","uuid":"msg-uuid-1","message":{"role":"user","content":"Hello"}}',
            '{"type":"assistant","uuid":"msg-uuid-2","message":{"role":"assistant","content":"Hi"}}',
            '{"type":"user","uuid":"msg-uuid-3","message":{"role":"user","content":"Bye"}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            uuids = extract_message_uuids(filepath)

            assert len(uuids) == 3
            assert "msg-uuid-1" in uuids
            assert "msg-uuid-2" in uuids
            assert "msg-uuid-3" in uuids
        finally:
            filepath.unlink()

    def test_extract_message_uuids_excludes_leaf_uuids(self):
        """leafUuid in summary records should NOT be extracted as message uuid."""
        from context_os_events.index.chain_graph import extract_message_uuids

        jsonl_content = [
            '{"type":"summary","summary":"Topic","leafUuid":"leaf-uuid-1"}',
            '{"type":"user","uuid":"msg-uuid-1","message":{"role":"user","content":"Hello"}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            uuids = extract_message_uuids(filepath)

            assert "msg-uuid-1" in uuids
            assert "leaf-uuid-1" not in uuids  # This is a leafUuid, not a message uuid
        finally:
            filepath.unlink()


class TestBuildChainGraph:
    """Test building chain graph from leafUuid relationships."""

    def test_build_chain_graph_links_parent_child(self):
        """Should link sessions via leafUuid -> uuid relationship.

        RED: Run before implementation - should fail
        GREEN: Implement build_chain_graph()

        Example:
        - Session A has message with uuid "msg-123"
        - Session B has summary with leafUuid "msg-123"
        - Therefore: B continues from A (B's parent is A)
        """
        from context_os_events.index.chain_graph import build_chain_graph

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Session A: Parent (owns the uuid)
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('\n'.join([
                '{"type":"user","uuid":"msg-123","message":{"role":"user","content":"Start"}}',
                '{"type":"assistant","uuid":"msg-124","message":{"role":"assistant","content":"OK"}}',
            ]))

            # Session B: Child (references parent's uuid via leafUuid)
            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text('\n'.join([
                '{"type":"summary","summary":"Continued work","leafUuid":"msg-123"}',
                '{"type":"user","uuid":"msg-200","message":{"role":"user","content":"Continue"}}',
            ]))

            chains = build_chain_graph(tmpdir)

            # Should have one chain with both sessions
            assert len(chains) == 1

            chain = list(chains.values())[0]
            assert chain.root_session == "session-a"
            assert "session-a" in chain.sessions
            assert "session-b" in chain.sessions

    def test_build_chain_graph_handles_branches(self):
        """Multiple sessions can continue from the same parent (branching).

        Session A (parent)
           ├── Session B (child 1)
           └── Session C (child 2)
        """
        from context_os_events.index.chain_graph import build_chain_graph

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Session A: Parent
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('\n'.join([
                '{"type":"user","uuid":"branch-point","message":{"role":"user","content":"Start"}}',
            ]))

            # Session B: Branch 1
            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text('\n'.join([
                '{"type":"summary","summary":"Branch 1","leafUuid":"branch-point"}',
                '{"type":"user","uuid":"b-1","message":{"role":"user","content":"Path B"}}',
            ]))

            # Session C: Branch 2
            session_c = tmpdir / "session-c.jsonl"
            session_c.write_text('\n'.join([
                '{"type":"summary","summary":"Branch 2","leafUuid":"branch-point"}',
                '{"type":"user","uuid":"c-1","message":{"role":"user","content":"Path C"}}',
            ]))

            chains = build_chain_graph(tmpdir)

            # Should have one chain with all three sessions
            assert len(chains) == 1
            chain = list(chains.values())[0]
            assert len(chain.sessions) == 3
            assert chain.root_session == "session-a"

    def test_build_chain_graph_handles_deep_chains(self):
        """Chains can be multiple levels deep.

        Session A -> Session B -> Session C
        """
        from context_os_events.index.chain_graph import build_chain_graph

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Session A: Root
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('\n'.join([
                '{"type":"user","uuid":"a-end","message":{"role":"user","content":"Start"}}',
            ]))

            # Session B: Middle (continues from A)
            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text('\n'.join([
                '{"type":"summary","summary":"Middle","leafUuid":"a-end"}',
                '{"type":"user","uuid":"b-end","message":{"role":"user","content":"Middle"}}',
            ]))

            # Session C: Leaf (continues from B)
            session_c = tmpdir / "session-c.jsonl"
            session_c.write_text('\n'.join([
                '{"type":"summary","summary":"End","leafUuid":"b-end"}',
                '{"type":"user","uuid":"c-end","message":{"role":"user","content":"End"}}',
            ]))

            chains = build_chain_graph(tmpdir)

            assert len(chains) == 1
            chain = list(chains.values())[0]
            assert chain.root_session == "session-a"
            assert len(chain.sessions) == 3

    def test_build_chain_graph_handles_orphan_sessions(self):
        """Sessions without leafUuid are standalone (single-session chains)."""
        from context_os_events.index.chain_graph import build_chain_graph

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Standalone session (no leafUuid, no one references it)
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('\n'.join([
                '{"type":"user","uuid":"a-1","message":{"role":"user","content":"Standalone"}}',
            ]))

            chains = build_chain_graph(tmpdir)

            # Should create a single-session chain
            assert len(chains) == 1
            chain = list(chains.values())[0]
            assert chain.root_session == "session-a"
            assert len(chain.sessions) == 1

    def test_build_chain_graph_handles_multiple_independent_chains(self):
        """Multiple unconnected chains should be separate."""
        from context_os_events.index.chain_graph import build_chain_graph

        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Chain 1: A -> B
            session_a = tmpdir / "session-a.jsonl"
            session_a.write_text('{"type":"user","uuid":"chain1-link","message":{}}')

            session_b = tmpdir / "session-b.jsonl"
            session_b.write_text(
                '{"type":"summary","leafUuid":"chain1-link"}\n'
                '{"type":"user","uuid":"b-1","message":{}}'
            )

            # Chain 2: C -> D (completely separate)
            session_c = tmpdir / "session-c.jsonl"
            session_c.write_text('{"type":"user","uuid":"chain2-link","message":{}}')

            session_d = tmpdir / "session-d.jsonl"
            session_d.write_text(
                '{"type":"summary","leafUuid":"chain2-link"}\n'
                '{"type":"user","uuid":"d-1","message":{}}'
            )

            chains = build_chain_graph(tmpdir)

            # Should have two separate chains
            assert len(chains) == 2


class TestChainDataclass:
    """Test Chain dataclass properties."""

    def test_chain_has_required_fields(self):
        """Chain should have all required fields."""
        from context_os_events.index.chain_graph import Chain

        chain = Chain(
            chain_id="test-chain",
            root_session="session-a",
            sessions=["session-a", "session-b"],
            branches={},
            time_range=None,
            total_duration_seconds=0,
            files_bloom=None,
            files_list=[],
        )

        assert chain.chain_id == "test-chain"
        assert chain.root_session == "session-a"
        assert len(chain.sessions) == 2


class TestChainNodeDataclass:
    """Test ChainNode dataclass properties."""

    def test_chain_node_has_required_fields(self):
        """ChainNode should track parent-child relationships."""
        from context_os_events.index.chain_graph import ChainNode

        node = ChainNode(
            session_id="session-b",
            parent_session_id="session-a",
            parent_message_uuid="msg-123",
            children=["session-c", "session-d"],
        )

        assert node.session_id == "session-b"
        assert node.parent_session_id == "session-a"
        assert node.parent_message_uuid == "msg-123"
        assert len(node.children) == 2


class TestRealWorldScenarios:
    """Test with realistic data patterns observed from actual JSONL files."""

    def test_handles_multiple_summary_records(self):
        """Real JSONL files have multiple summaries (conversation history stack).

        Claude Code stacks summaries oldest-first when continuing sessions.
        Only the LAST summary's leafUuid indicates the immediate parent.
        Earlier summaries point to ancestors in the chain.
        """
        from context_os_events.index.chain_graph import extract_leaf_uuids

        # This pattern was observed in real files
        jsonl_content = [
            '{"type":"summary","summary":"Context OS Daemon","leafUuid":"uuid-1"}',
            '{"type":"summary","summary":"Building Event Capture","leafUuid":"uuid-2"}',
            '{"type":"summary","summary":"Phase 1 Complete","leafUuid":"uuid-3"}',
            '{"type":"file-history-snapshot","messageId":"snap-1"}',
            '{"type":"user","uuid":"msg-1","message":{"role":"user","content":"Continue"}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            leaf_uuids = extract_leaf_uuids(filepath)

            # Only LAST leafUuid should be extracted (immediate parent)
            # uuid-1, uuid-2 are ancestors; uuid-3 is immediate parent
            assert len(leaf_uuids) == 1
            assert leaf_uuids[0] == "uuid-3"  # LAST = immediate parent
        finally:
            filepath.unlink()

    def test_ignores_file_history_snapshots(self):
        """file-history-snapshot records should not be treated as chain links."""
        from context_os_events.index.chain_graph import extract_leaf_uuids

        jsonl_content = [
            '{"type":"summary","summary":"Work","leafUuid":"real-link"}',
            '{"type":"file-history-snapshot","messageId":"snapshot-id","snapshot":{}}',
        ]

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.jsonl', delete=False
        ) as f:
            f.write('\n'.join(jsonl_content))
            f.flush()
            filepath = Path(f.name)

        try:
            leaf_uuids = extract_leaf_uuids(filepath)

            assert len(leaf_uuids) == 1
            assert "real-link" in leaf_uuids
            assert "snapshot-id" not in leaf_uuids
        finally:
            filepath.unlink()
