"""JSONL context extraction module.

Helpers for extracting specific context from raw JSONL session files.
These functions extract RAW text - the agent interprets meaning.
"""

import json
import re
from pathlib import Path
from typing import List, Union


def get_user_messages(
    jsonl_path: Union[str, Path],
    limit: int = 10
) -> List[str]:
    """Extract first N user messages from a session.

    Useful for detecting user intent at the start of a session.

    Args:
        jsonl_path: Path to JSONL file
        limit: Maximum number of messages to return

    Returns:
        List of user message content strings
    """
    messages = []
    jsonl_path = Path(jsonl_path)

    if not jsonl_path.exists():
        return []

    with open(jsonl_path, 'r', encoding='utf-8', errors='replace') as f:
        for line in f:
            if len(messages) >= limit:
                break

            line = line.strip()
            if not line:
                continue

            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            if data.get("type") != "user":
                continue

            message = data.get("message", {})
            content = message.get("content", "")

            if isinstance(content, str) and content:
                messages.append(content)

    return messages


def get_voice_memos(jsonl_path: Union[str, Path]) -> List[str]:
    """Extract voice memo transcripts from a session.

    Voice memos appear as ![[Voice Memos/...]] followed by transcript text.

    Args:
        jsonl_path: Path to JSONL file

    Returns:
        List of voice memo content (reference + transcript)
    """
    memos = []
    jsonl_path = Path(jsonl_path)

    if not jsonl_path.exists():
        return []

    # Pattern to match voice memo references
    voice_memo_pattern = re.compile(r'!\[\[Voice Memos/[^\]]+\]\]')

    with open(jsonl_path, 'r', encoding='utf-8', errors='replace') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            if data.get("type") != "user":
                continue

            message = data.get("message", {})
            content = message.get("content", "")

            if isinstance(content, str) and voice_memo_pattern.search(content):
                # Include the full message (reference + transcript)
                memos.append(content)

    return memos


def get_slash_commands(jsonl_path: Union[str, Path]) -> List[str]:
    """Extract slash commands used in a session.

    Slash commands appear as <command-name>/command</command-name>.

    Args:
        jsonl_path: Path to JSONL file

    Returns:
        List of unique slash command names (e.g., ["/chief-of-staff", "/map-work"])
    """
    commands = set()
    jsonl_path = Path(jsonl_path)

    if not jsonl_path.exists():
        return []

    # Pattern to match command names
    command_pattern = re.compile(r'<command-name>(/[^<]+)</command-name>')

    with open(jsonl_path, 'r', encoding='utf-8', errors='replace') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            if data.get("type") != "user":
                continue

            message = data.get("message", {})
            content = message.get("content", "")

            if isinstance(content, str):
                matches = command_pattern.findall(content)
                commands.update(matches)

    return list(commands)


def get_last_messages(
    jsonl_path: Union[str, Path],
    limit: int = 5
) -> List[str]:
    """Extract last N messages from a session (any role).

    Useful for detecting session status (complete, in-progress, etc.).

    Args:
        jsonl_path: Path to JSONL file
        limit: Maximum number of messages to return

    Returns:
        List of message content strings (user and assistant)
    """
    all_messages = []
    jsonl_path = Path(jsonl_path)

    if not jsonl_path.exists():
        return []

    with open(jsonl_path, 'r', encoding='utf-8', errors='replace') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            msg_type = data.get("type")
            if msg_type not in ("user", "assistant"):
                continue

            message = data.get("message", {})
            content = message.get("content", "")

            # Handle assistant messages (may be list of content blocks)
            if isinstance(content, list):
                # Extract text from content blocks
                text_parts = []
                for block in content:
                    if isinstance(block, dict) and block.get("type") == "text":
                        text_parts.append(block.get("text", ""))
                content = " ".join(text_parts)

            if isinstance(content, str) and content:
                all_messages.append(content)

    # Return last N messages
    return all_messages[-limit:] if len(all_messages) > limit else all_messages
