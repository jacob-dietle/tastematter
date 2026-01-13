"""Snapshot generator for user-visible data exports.

Generates markdown files with analysis results:
- game_trails.md - Most accessed files
- tool_patterns.md - Tool usage breakdown
- automation_candidates.md - Repeated grep patterns
- commit_hotspots.md - Most modified files
- snapshots/latest.md - Combined snapshot
"""

import json
from collections import Counter
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from sqlite3 import Connection
from typing import Dict, List, Optional


# ============================================================================
# Data Structures
# ============================================================================

@dataclass
class GameTrailEntry:
    """Single file in game trails."""
    path: str
    read_count: int
    last_accessed: datetime
    category: str  # config, state, knowledge, code, other


@dataclass
class ToolPatternEntry:
    """Tool usage statistics."""
    tool_name: str
    use_count: int
    percentage: float
    avg_per_session: float


@dataclass
class AutomationCandidate:
    """Repeated pattern that could be automated."""
    pattern: str
    use_count: int
    session_count: int
    potential: str  # HIGH, MEDIUM, LOW


@dataclass
class CommitHotspot:
    """File modification statistics."""
    path: str
    commit_count: int
    agent_commits: int
    human_commits: int
    churn: str  # HIGH, MEDIUM, LOW


@dataclass
class Snapshot:
    """Complete snapshot data."""
    generated_at: datetime
    git_commits: int
    agent_commits: int
    sessions: int
    total_messages: int
    tool_uses: int

    # Time period data
    earliest_commit: Optional[datetime]
    latest_commit: Optional[datetime]
    earliest_session: Optional[datetime]
    latest_session: Optional[datetime]
    days_span: int  # Total days covered

    game_trails: List[GameTrailEntry]
    tool_patterns: List[ToolPatternEntry]
    automation_candidates: List[AutomationCandidate]
    commit_hotspots: List[CommitHotspot]


# ============================================================================
# Snapshot Generator
# ============================================================================

class SnapshotGenerator:
    """Generates markdown snapshot files from database."""

    def __init__(self, db: Connection, output_dir: Path):
        """Initialize the snapshot generator.

        Args:
            db: Database connection
            output_dir: Directory to write snapshot files
        """
        self.db = db
        self.output_dir = Path(output_dir)

    def _categorize_path(self, path: str) -> str:
        """Categorize a file path.

        Args:
            path: File path

        Returns:
            Category string (config, state, knowledge, code, other)
        """
        if "CLAUDE.md" in path or path.endswith(".yaml") or path.endswith(".json"):
            return "config"
        elif "_system/state" in path or "state" in path.lower():
            return "state"
        elif "knowledge_base" in path or "_synthesis" in path:
            return "knowledge"
        elif path.endswith((".py", ".js", ".ts", ".go", ".rs")):
            return "code"
        else:
            return "other"

    def _calculate_churn(self, commit_count: int) -> str:
        """Calculate churn level from commit count.

        Args:
            commit_count: Number of commits

        Returns:
            Churn level string (HIGH, MEDIUM, LOW)
        """
        if commit_count >= 10:
            return "HIGH"
        elif commit_count >= 5:
            return "MEDIUM"
        else:
            return "LOW"

    def _calculate_potential(self, use_count: int, session_count: int) -> str:
        """Calculate automation potential.

        Args:
            use_count: Number of times pattern used
            session_count: Number of sessions using pattern

        Returns:
            Potential string (HIGH, MEDIUM, LOW)
        """
        if use_count >= 5 and session_count >= 3:
            return "HIGH"
        elif use_count >= 3 or session_count >= 2:
            return "MEDIUM"
        else:
            return "LOW"

    # ========================================================================
    # Data Query Methods
    # ========================================================================

    def generate_game_trails(self, limit: int = 20) -> List[GameTrailEntry]:
        """Query and return game trails data.

        Args:
            limit: Maximum number of files to return

        Returns:
            List of GameTrailEntry sorted by read count
        """
        # Count file reads across all sessions
        file_counts: Counter = Counter()

        cursor = self.db.execute("""
            SELECT files_read FROM claude_sessions WHERE files_read IS NOT NULL
        """)

        for row in cursor.fetchall():
            try:
                files = json.loads(row["files_read"])
                for f in files:
                    file_counts[f] += 1
            except (json.JSONDecodeError, TypeError):
                continue

        # Convert to entries
        entries = []
        for path, count in file_counts.most_common(limit):
            entries.append(GameTrailEntry(
                path=path,
                read_count=count,
                last_accessed=datetime.now(),  # TODO: Track actual access time
                category=self._categorize_path(path)
            ))

        return entries

    def generate_tool_patterns(self) -> List[ToolPatternEntry]:
        """Query and return tool pattern data.

        Returns:
            List of ToolPatternEntry with usage percentages
        """
        # Aggregate tool counts across all sessions
        tool_counts: Counter = Counter()
        session_count = 0

        cursor = self.db.execute("""
            SELECT tools_used FROM claude_sessions WHERE tools_used IS NOT NULL
        """)

        for row in cursor.fetchall():
            session_count += 1
            try:
                tools = json.loads(row["tools_used"])
                if isinstance(tools, dict):
                    for tool, count in tools.items():
                        tool_counts[tool] += count
            except (json.JSONDecodeError, TypeError):
                continue

        # Calculate percentages
        total_uses = sum(tool_counts.values())
        if total_uses == 0:
            return []

        entries = []
        for tool, count in tool_counts.most_common():
            entries.append(ToolPatternEntry(
                tool_name=tool,
                use_count=count,
                percentage=round((count / total_uses) * 100, 1),
                avg_per_session=round(count / max(session_count, 1), 2)
            ))

        return entries

    def generate_automation_candidates(self) -> List[AutomationCandidate]:
        """Query and return automation candidates.

        Returns:
            List of AutomationCandidate for repeated patterns
        """
        # Count grep patterns and sessions using them
        pattern_counts: Counter = Counter()
        pattern_sessions: Dict[str, set] = {}

        cursor = self.db.execute("""
            SELECT session_id, grep_patterns FROM claude_sessions
            WHERE grep_patterns IS NOT NULL
        """)

        for row in cursor.fetchall():
            session_id = row["session_id"]
            try:
                patterns = json.loads(row["grep_patterns"])
                for pattern in patterns:
                    pattern_counts[pattern] += 1
                    if pattern not in pattern_sessions:
                        pattern_sessions[pattern] = set()
                    pattern_sessions[pattern].add(session_id)
            except (json.JSONDecodeError, TypeError):
                continue

        # Filter to patterns with >1 use and create entries
        entries = []
        for pattern, count in pattern_counts.most_common():
            if count > 1:  # Only include repeated patterns
                session_count = len(pattern_sessions.get(pattern, set()))
                entries.append(AutomationCandidate(
                    pattern=pattern,
                    use_count=count,
                    session_count=session_count,
                    potential=self._calculate_potential(count, session_count)
                ))

        return entries

    def generate_commit_hotspots(self, limit: int = 20) -> List[CommitHotspot]:
        """Query and return commit hotspots.

        Args:
            limit: Maximum number of files to return

        Returns:
            List of CommitHotspot sorted by commit count
        """
        # Count file modifications from commits
        file_counts: Counter = Counter()
        file_agent_counts: Counter = Counter()

        cursor = self.db.execute("""
            SELECT files_changed, is_agent_commit FROM git_commits
            WHERE files_changed IS NOT NULL
        """)

        for row in cursor.fetchall():
            is_agent = row["is_agent_commit"]
            try:
                files = json.loads(row["files_changed"])
                for f in files:
                    file_counts[f] += 1
                    if is_agent:
                        file_agent_counts[f] += 1
            except (json.JSONDecodeError, TypeError):
                continue

        # Convert to entries
        entries = []
        for path, total in file_counts.most_common(limit):
            agent = file_agent_counts.get(path, 0)
            human = total - agent
            entries.append(CommitHotspot(
                path=path,
                commit_count=total,
                agent_commits=agent,
                human_commits=human,
                churn=self._calculate_churn(total)
            ))

        return entries

    def generate_full_snapshot(self) -> Snapshot:
        """Generate complete snapshot with all data.

        Returns:
            Complete Snapshot object
        """
        # Get summary stats and date ranges for commits
        cursor = self.db.execute("""
            SELECT COUNT(*) as total,
                   SUM(CASE WHEN is_agent_commit THEN 1 ELSE 0 END) as agent,
                   MIN(timestamp) as earliest,
                   MAX(timestamp) as latest
            FROM git_commits
        """)
        git_row = cursor.fetchone()

        # Get summary stats and date ranges for sessions
        cursor = self.db.execute("""
            SELECT COUNT(*) as total,
                   SUM(total_messages) as messages,
                   MIN(started_at) as earliest,
                   MAX(ended_at) as latest
            FROM claude_sessions
        """)
        session_row = cursor.fetchone()

        # Parse dates
        earliest_commit = None
        latest_commit = None
        if git_row["earliest"]:
            try:
                earliest_commit = datetime.fromisoformat(git_row["earliest"].replace("Z", "+00:00"))
            except (ValueError, AttributeError):
                earliest_commit = datetime.strptime(git_row["earliest"][:10], "%Y-%m-%d")
        if git_row["latest"]:
            try:
                latest_commit = datetime.fromisoformat(git_row["latest"].replace("Z", "+00:00"))
            except (ValueError, AttributeError):
                latest_commit = datetime.strptime(git_row["latest"][:10], "%Y-%m-%d")

        earliest_session = None
        latest_session = None
        if session_row["earliest"]:
            try:
                earliest_session = datetime.fromisoformat(session_row["earliest"].replace("Z", "+00:00"))
            except (ValueError, AttributeError):
                earliest_session = datetime.strptime(session_row["earliest"][:10], "%Y-%m-%d")
        if session_row["latest"]:
            try:
                latest_session = datetime.fromisoformat(session_row["latest"].replace("Z", "+00:00"))
            except (ValueError, AttributeError):
                latest_session = datetime.strptime(session_row["latest"][:10], "%Y-%m-%d")

        # Calculate days span (use the widest range)
        days_span = 0
        all_dates = [d for d in [earliest_commit, latest_commit, earliest_session, latest_session] if d]
        if len(all_dates) >= 2:
            days_span = (max(all_dates) - min(all_dates)).days + 1

        # Calculate tool uses
        tool_patterns = self.generate_tool_patterns()
        total_tool_uses = sum(t.use_count for t in tool_patterns)

        return Snapshot(
            generated_at=datetime.now(),
            git_commits=git_row["total"] or 0,
            agent_commits=git_row["agent"] or 0,
            sessions=session_row["total"] or 0,
            total_messages=session_row["messages"] or 0,
            tool_uses=total_tool_uses,
            earliest_commit=earliest_commit,
            latest_commit=latest_commit,
            earliest_session=earliest_session,
            latest_session=latest_session,
            days_span=days_span,
            game_trails=self.generate_game_trails(),
            tool_patterns=tool_patterns,
            automation_candidates=self.generate_automation_candidates(),
            commit_hotspots=self.generate_commit_hotspots()
        )

    # ========================================================================
    # Markdown Generation Methods
    # ========================================================================

    def write_game_trails_md(self, data: List[GameTrailEntry]) -> Path:
        """Write game_trails.md file.

        Args:
            data: List of game trail entries

        Returns:
            Path to written file
        """
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        lines = [
            "# Game Trails - Most Accessed Files",
            "",
            f"*Generated: {now}*",
            "",
            "## Top Accessed Files",
            "",
            "| Rank | File | Reads | Category |",
            "|------|------|-------|----------|",
        ]

        for i, entry in enumerate(data, 1):
            lines.append(f"| {i} | {entry.path} | {entry.read_count} | {entry.category} |")

        lines.extend(["", "---", ""])

        path = self.output_dir / "game_trails.md"
        path.write_text("\n".join(lines))
        return path

    def write_tool_patterns_md(self, data: List[ToolPatternEntry]) -> Path:
        """Write tool_patterns.md file.

        Args:
            data: List of tool pattern entries

        Returns:
            Path to written file
        """
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        lines = [
            "# Tool Usage Patterns",
            "",
            f"*Generated: {now}*",
            "",
            "## Tool Usage Breakdown",
            "",
            "| Tool | Uses | % of Total | Avg/Session |",
            "|------|------|------------|-------------|",
        ]

        for entry in data:
            lines.append(
                f"| {entry.tool_name} | {entry.use_count} | "
                f"{entry.percentage}% | {entry.avg_per_session} |"
            )

        lines.extend(["", "---", ""])

        path = self.output_dir / "tool_patterns.md"
        path.write_text("\n".join(lines))
        return path

    def write_automation_candidates_md(self, data: List[AutomationCandidate]) -> Path:
        """Write automation_candidates.md file.

        Args:
            data: List of automation candidates

        Returns:
            Path to written file
        """
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        lines = [
            "# Automation Candidates",
            "",
            f"*Generated: {now}*",
            "",
            "## Repeated Patterns",
            "",
            "| Pattern | Uses | Sessions | Potential |",
            "|---------|------|----------|-----------|",
        ]

        for entry in data:
            # Escape pipe characters in pattern
            safe_pattern = entry.pattern.replace("|", "\\|")
            lines.append(
                f"| `{safe_pattern}` | {entry.use_count} | "
                f"{entry.session_count} | {entry.potential} |"
            )

        if not data:
            lines.append("| (none found) | - | - | - |")

        lines.extend(["", "---", ""])

        path = self.output_dir / "automation_candidates.md"
        path.write_text("\n".join(lines))
        return path

    def write_commit_hotspots_md(self, data: List[CommitHotspot]) -> Path:
        """Write commit_hotspots.md file.

        Args:
            data: List of commit hotspots

        Returns:
            Path to written file
        """
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        lines = [
            "# Commit Hotspots - Most Modified Files",
            "",
            f"*Generated: {now}*",
            "",
            "## Most Modified Files",
            "",
            "| Rank | File | Commits | Agent | Human | Churn |",
            "|------|------|---------|-------|-------|-------|",
        ]

        for i, entry in enumerate(data, 1):
            lines.append(
                f"| {i} | {entry.path} | {entry.commit_count} | "
                f"{entry.agent_commits} | {entry.human_commits} | {entry.churn} |"
            )

        if not data:
            lines.append("| - | (no commits) | - | - | - | - |")

        lines.extend(["", "---", ""])

        path = self.output_dir / "commit_hotspots.md"
        path.write_text("\n".join(lines))
        return path

    def write_full_snapshot_md(self, snapshot: Snapshot) -> Path:
        """Write snapshots/latest.md and dated snapshot.

        Args:
            snapshot: Complete snapshot data

        Returns:
            Path to latest.md
        """
        now = snapshot.generated_at
        now_str = now.strftime("%Y-%m-%d %H:%M:%S")
        date_str = now.strftime("%Y-%m-%d")

        # Format date ranges
        commit_range = "N/A"
        if snapshot.earliest_commit and snapshot.latest_commit:
            commit_range = f"{snapshot.earliest_commit.strftime('%Y-%m-%d')} to {snapshot.latest_commit.strftime('%Y-%m-%d')}"

        session_range = "N/A"
        if snapshot.earliest_session and snapshot.latest_session:
            session_range = f"{snapshot.earliest_session.strftime('%Y-%m-%d')} to {snapshot.latest_session.strftime('%Y-%m-%d')}"

        # Calculate rates
        commits_per_day = f"{snapshot.git_commits / snapshot.days_span:.1f}" if snapshot.days_span > 0 else "N/A"
        sessions_per_day = f"{snapshot.sessions / snapshot.days_span:.1f}" if snapshot.days_span > 0 else "N/A"

        lines = [
            "# Context OS Snapshot",
            "",
            f"*Generated: {now_str}*",
            "",
            "## Quick Stats",
            "",
            "| Metric | Value |",
            "|--------|-------|",
            f"| Git Commits | {snapshot.git_commits} ({snapshot.agent_commits} agent) |",
            f"| Claude Sessions | {snapshot.sessions} |",
            f"| Total Messages | {snapshot.total_messages} |",
            f"| Tool Uses | {snapshot.tool_uses} |",
            "",
            "## Time Period",
            "",
            "| Range | Dates |",
            "|-------|-------|",
            f"| Commits | {commit_range} |",
            f"| Sessions | {session_range} |",
            f"| **Total Span** | **{snapshot.days_span} days** |",
            "",
            "## Activity Rates",
            "",
            "| Metric | Rate |",
            "|--------|------|",
            f"| Commits/day | {commits_per_day} |",
            f"| Sessions/day | {sessions_per_day} |",
            "",
            "---",
            "",
            "## Game Trails",
            "",
            "| File | Reads |",
            "|------|-------|",
        ]

        for entry in snapshot.game_trails[:10]:
            lines.append(f"| {entry.path} | {entry.read_count} |")

        lines.extend([
            "",
            "## Tool Patterns",
            "",
            "| Tool | Uses | % |",
            "|------|------|---|",
        ])

        for entry in snapshot.tool_patterns[:10]:
            lines.append(f"| {entry.tool_name} | {entry.use_count} | {entry.percentage}% |")

        lines.extend(["", "---", ""])

        content = "\n".join(lines)

        # Ensure snapshots directory exists
        snapshots_dir = self.output_dir / "snapshots"
        snapshots_dir.mkdir(parents=True, exist_ok=True)

        # Write latest.md
        latest_path = snapshots_dir / "latest.md"
        latest_path.write_text(content)

        # Write dated snapshot
        dated_path = snapshots_dir / f"{date_str}.md"
        dated_path.write_text(content)

        return latest_path

    def generate_all(self) -> None:
        """Generate all snapshot files."""
        # Generate full snapshot (includes all data and time periods)
        snapshot = self.generate_full_snapshot()

        # Write individual files
        self.write_game_trails_md(snapshot.game_trails)
        self.write_tool_patterns_md(snapshot.tool_patterns)
        self.write_automation_candidates_md(snapshot.automation_candidates)
        self.write_commit_hotspots_md(snapshot.commit_hotspots)

        # Write combined snapshot
        self.write_full_snapshot_md(snapshot)
