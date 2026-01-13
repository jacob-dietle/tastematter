"""Tests for JSONL context extraction module.

TDD Phase: RED - These tests should FAIL until jsonl_context.py is implemented.
"""

import json
from pathlib import Path

import pytest


class TestJsonlContext:
    """Test suite for JSONL context extraction functions."""

    @pytest.fixture
    def sample_jsonl(self, tmp_path):
        """Create a sample JSONL file with realistic session data."""
        jsonl_path = tmp_path / "test-session.jsonl"

        messages = [
            # User message with voice memo
            {
                "type": "user",
                "timestamp": "2025-12-14T10:00:00Z",
                "message": {
                    "role": "user",
                    "content": "![[Voice Memos/Desktop/2025-12-14.webm]]\nI want to build a meta-intelligence layer for context tracking."
                }
            },
            # Assistant response
            {
                "type": "assistant",
                "timestamp": "2025-12-14T10:00:30Z",
                "message": {
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "I'll help you build that."},
                        {"type": "tool_use", "name": "Read", "input": {"file_path": "/test/file.py"}}
                    ]
                }
            },
            # User with slash command
            {
                "type": "user",
                "timestamp": "2025-12-14T10:01:00Z",
                "message": {
                    "role": "user",
                    "content": "<command-name>/chief-of-staff</command-name>\n<command-args>weekly planning</command-args>"
                }
            },
            # Another user message
            {
                "type": "user",
                "timestamp": "2025-12-14T10:02:00Z",
                "message": {
                    "role": "user",
                    "content": "Let's focus on the query module first."
                }
            },
            # Assistant response
            {
                "type": "assistant",
                "timestamp": "2025-12-14T10:02:30Z",
                "message": {
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Starting with queries.py"}]
                }
            },
            # User with another slash command
            {
                "type": "user",
                "timestamp": "2025-12-14T10:03:00Z",
                "message": {
                    "role": "user",
                    "content": "<command-name>/map-work</command-name>"
                }
            },
            # Final messages
            {
                "type": "user",
                "timestamp": "2025-12-14T10:04:00Z",
                "message": {
                    "role": "user",
                    "content": "Tests are passing now."
                }
            },
            {
                "type": "assistant",
                "timestamp": "2025-12-14T10:04:30Z",
                "message": {
                    "role": "assistant",
                    "content": [{"type": "text", "text": "All 7 tests GREEN. Ready to commit."}]
                }
            },
        ]

        with open(jsonl_path, 'w') as f:
            for msg in messages:
                f.write(json.dumps(msg) + '\n')

        return jsonl_path

    def test_get_user_messages_returns_first_n(self, sample_jsonl):
        """get_user_messages should return first N user messages."""
        from context_os_events.intelligence.jsonl_context import get_user_messages

        messages = get_user_messages(sample_jsonl, limit=3)

        assert len(messages) == 3
        # First message should contain voice memo reference
        assert "meta-intelligence" in messages[0]

    def test_get_user_messages_extracts_content(self, sample_jsonl):
        """get_user_messages should extract message content correctly."""
        from context_os_events.intelligence.jsonl_context import get_user_messages

        messages = get_user_messages(sample_jsonl, limit=1)

        # Should extract the text content
        assert "I want to build" in messages[0]

    def test_get_voice_memos_extracts_transcripts(self, sample_jsonl):
        """get_voice_memos should find and extract voice memo references."""
        from context_os_events.intelligence.jsonl_context import get_voice_memos

        memos = get_voice_memos(sample_jsonl)

        # Should find the voice memo
        assert len(memos) >= 1
        # Should include the transcript text after the reference
        assert any("meta-intelligence" in m for m in memos)

    def test_get_voice_memos_handles_no_memos(self, tmp_path):
        """get_voice_memos should return empty list when no memos present."""
        from context_os_events.intelligence.jsonl_context import get_voice_memos

        # Create JSONL without voice memos
        jsonl_path = tmp_path / "no-memos.jsonl"
        with open(jsonl_path, 'w') as f:
            f.write(json.dumps({
                "type": "user",
                "message": {"role": "user", "content": "Just text, no voice memo."}
            }) + '\n')

        memos = get_voice_memos(jsonl_path)

        assert memos == []

    def test_get_slash_commands_extracts_commands(self, sample_jsonl):
        """get_slash_commands should extract slash command names."""
        from context_os_events.intelligence.jsonl_context import get_slash_commands

        commands = get_slash_commands(sample_jsonl)

        # Should find both /chief-of-staff and /map-work
        assert "/chief-of-staff" in commands
        assert "/map-work" in commands
        assert len(commands) == 2

    def test_get_slash_commands_handles_no_commands(self, tmp_path):
        """get_slash_commands should return empty list when no commands used."""
        from context_os_events.intelligence.jsonl_context import get_slash_commands

        jsonl_path = tmp_path / "no-commands.jsonl"
        with open(jsonl_path, 'w') as f:
            f.write(json.dumps({
                "type": "user",
                "message": {"role": "user", "content": "Regular message."}
            }) + '\n')

        commands = get_slash_commands(jsonl_path)

        assert commands == []

    def test_get_last_messages_returns_last_n(self, sample_jsonl):
        """get_last_messages should return last N messages (any role)."""
        from context_os_events.intelligence.jsonl_context import get_last_messages

        messages = get_last_messages(sample_jsonl, limit=2)

        assert len(messages) == 2
        # Last message should be assistant's "Ready to commit"
        assert "commit" in messages[-1].lower() or "green" in messages[-1].lower()

    def test_get_last_messages_includes_both_roles(self, sample_jsonl):
        """get_last_messages should include both user and assistant messages."""
        from context_os_events.intelligence.jsonl_context import get_last_messages

        messages = get_last_messages(sample_jsonl, limit=4)

        # Should have mix of user and assistant content
        assert len(messages) == 4
