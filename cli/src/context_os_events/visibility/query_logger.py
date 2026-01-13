"""Query logger for tracking CLI command execution.

Logs every CLI query to a markdown file for user visibility.
New entries are prepended (most recent at top).
"""

import re
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import List, Optional


@dataclass
class QueryLogEntry:
    """Single entry in the query log."""
    timestamp: datetime
    command: str           # Full command string
    duration_seconds: float
    results_summary: str   # Markdown summary of results
    row_count: Optional[int]  # Number of rows returned (if applicable)


class QueryLogger:
    """Logs CLI queries to markdown file.

    New entries are prepended to the file (most recent first).
    """

    HEADER = "# Context OS Query Log\n\n"

    def __init__(self, log_path: Path):
        """Initialize the query logger.

        Args:
            log_path: Path to the query_log.md file
        """
        self.log_path = Path(log_path)

    def _format_entry(self, entry: QueryLogEntry) -> str:
        """Format a log entry as markdown.

        Args:
            entry: The query log entry to format

        Returns:
            Formatted markdown string
        """
        # Extract command name from full command
        parts = entry.command.split()
        cmd_name = parts[1] if len(parts) > 1 else parts[0] if parts else "unknown"

        timestamp_str = entry.timestamp.strftime("%Y-%m-%d %H:%M:%S")

        lines = [
            f"## {timestamp_str} - {cmd_name}",
            "",
            f"**Command:** `{entry.command}`",
            f"**Duration:** {entry.duration_seconds:.2f}s",
        ]

        if entry.row_count is not None:
            lines.append(f"**Results:** {entry.row_count} rows")

        if entry.results_summary:
            lines.append(f"**Summary:** {entry.results_summary}")

        lines.extend(["", "---", ""])

        return "\n".join(lines)

    def log(self, entry: QueryLogEntry) -> None:
        """Prepend entry to log file.

        Creates the file with header if it doesn't exist.
        New entries appear at the top of the file.

        Args:
            entry: The query log entry to record
        """
        formatted = self._format_entry(entry)

        # Read existing content (if any)
        existing_content = ""
        if self.log_path.exists():
            existing_content = self.log_path.read_text()
            # Remove header if present (we'll re-add it)
            if existing_content.startswith(self.HEADER):
                existing_content = existing_content[len(self.HEADER):]

        # Ensure parent directory exists
        self.log_path.parent.mkdir(parents=True, exist_ok=True)

        # Write with new entry at top
        new_content = self.HEADER + formatted + existing_content
        self.log_path.write_text(new_content)

    def get_recent(self, limit: int = 20) -> List[QueryLogEntry]:
        """Read recent entries from log.

        Parses the markdown file to extract log entries.

        Args:
            limit: Maximum number of entries to return

        Returns:
            List of QueryLogEntry objects (most recent first)
        """
        if not self.log_path.exists():
            return []

        content = self.log_path.read_text()

        # Parse entries from markdown
        # Entries start with "## YYYY-MM-DD HH:MM:SS - command"
        pattern = r"## (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) - (\S+)\n\n\*\*Command:\*\* `([^`]+)`\n\*\*Duration:\*\* ([\d.]+)s"

        entries = []
        matches = re.finditer(pattern, content)

        for match in matches:
            if len(entries) >= limit:
                break

            timestamp_str, cmd_name, full_command, duration_str = match.groups()

            # Extract row count if present
            row_count = None
            row_match = re.search(
                r"\*\*Results:\*\* (\d+) rows",
                content[match.end():match.end() + 200]
            )
            if row_match:
                row_count = int(row_match.group(1))

            # Extract summary if present
            summary = ""
            summary_match = re.search(
                r"\*\*Summary:\*\* (.+?)(?:\n|$)",
                content[match.end():match.end() + 500]
            )
            if summary_match:
                summary = summary_match.group(1)

            entry = QueryLogEntry(
                timestamp=datetime.strptime(timestamp_str, "%Y-%m-%d %H:%M:%S"),
                command=full_command,
                duration_seconds=float(duration_str),
                results_summary=summary,
                row_count=row_count
            )
            entries.append(entry)

        return entries
