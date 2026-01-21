"""CLI for Context OS Events."""

import json
import sys
import time
from datetime import datetime
from pathlib import Path
from functools import wraps
from typing import Dict, List

import click
from rich.console import Console
from rich.table import Table

from .db.connection import get_connection, init_database, get_database_path
from .capture.git_sync import sync_commits, GitError
from .capture.jsonl_parser import sync_sessions, get_claude_projects_dir, encode_project_path
from .capture.file_watcher import start_watcher, stop_watcher
from .visibility.snapshot import SnapshotGenerator
from .index.chain_graph import build_chain_graph, persist_chains
from .observability import Event, event_logger, update_state

console = Console()


# =============================================================================
# Event Logging Helpers
# =============================================================================

def _utc_now() -> str:
    """Return current UTC timestamp in ISO8601 format."""
    return datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")


def log_command_start(command: str, context: dict = None) -> float:
    """Log command start and return start time for duration tracking.

    Args:
        command: Command name (e.g., "parse-sessions")
        context: Additional context dict (optional)

    Returns:
        Start time (from time.time()) for duration calculation
    """
    event = Event(
        ts=_utc_now(),
        level="info",
        source="cli",
        event="command_start",
        command=command,
        context=context or {},
    )
    event_logger.log(event)
    return time.time()


def log_command_complete(command: str, start_time: float, context: dict = None) -> None:
    """Log successful command completion.

    Args:
        command: Command name
        start_time: Start time from log_command_start()
        context: Result context (e.g., {"sessions_parsed": 5})
    """
    duration_ms = int((time.time() - start_time) * 1000)
    event = Event(
        ts=_utc_now(),
        level="info",
        source="cli",
        event="command_complete",
        command=command,
        duration_ms=duration_ms,
        context=context or {},
    )
    event_logger.log(event)


def log_command_error(command: str, start_time: float, error: str, suggestion: str = None) -> None:
    """Log command error.

    Args:
        command: Command name
        start_time: Start time from log_command_start()
        error: Error message
        suggestion: Actionable fix suggestion
    """
    duration_ms = int((time.time() - start_time) * 1000)
    event = Event(
        ts=_utc_now(),
        level="error",
        source="cli",
        event="command_error",
        command=command,
        duration_ms=duration_ms,
        context={"error": error},
        suggestion=suggestion,
    )
    event_logger.log(event)


# =============================================================================
# Query Logging
# =============================================================================

def get_query_log_path() -> Path:
    """Get path to query log file.

    Returns:
        Path to ~/.context-os/query_log.jsonl
    """
    log_dir = Path.home() / ".context-os"
    log_dir.mkdir(parents=True, exist_ok=True)
    return log_dir / "query_log.jsonl"


def log_query(command: str, args: dict, result_summary: dict, duration_ms: float, error: str = None):
    """Log a CLI query to JSONL file.

    Args:
        command: Command name (e.g., "query search")
        args: Command arguments
        result_summary: Summary of results (counts, etc.)
        duration_ms: Execution time in milliseconds
        error: Error message if command failed
    """
    log_path = get_query_log_path()

    entry = {
        "timestamp": datetime.now().isoformat(),
        "command": command,
        "args": args,
        "result": result_summary,
        "duration_ms": round(duration_ms, 2),
        "project": str(Path.cwd()),
    }

    if error:
        entry["error"] = error

    try:
        with open(log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry) + "\n")
    except Exception:
        pass  # Don't fail on logging errors


def query_logged(command_name: str):
    """Decorator to log query commands.

    Logs to both query_log.jsonl (legacy) and events.jsonl (new).

    Usage:
        @query_logged("query search")
        def query_search(pattern, limit):
            ...
            return {"matches": 10}  # Return result summary for logging
    """
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            start = time.time()
            error = None
            result_summary = {}

            # Log command start to events.jsonl
            start_time = log_command_start(command_name, {"args": kwargs})

            try:
                result = func(*args, **kwargs)
                if isinstance(result, dict):
                    result_summary = result
            except Exception as e:
                error = str(e)
                # Log error to events.jsonl
                log_command_error(command_name, start_time, str(e))
                raise
            finally:
                duration_ms = (time.time() - start) * 1000
                # Log to legacy query_log.jsonl
                log_query(
                    command=command_name,
                    args=kwargs,
                    result_summary=result_summary,
                    duration_ms=duration_ms,
                    error=error
                )

            # Log success to events.jsonl
            log_command_complete(command_name, start_time, result_summary)
            return result
        return wrapper
    return decorator


# =============================================================================
# JSON Output Support (Phase A - Hypercube Refactor)
# =============================================================================


# Common format option for query commands
format_option = click.option(
    "--format", "output_format",
    type=click.Choice(["json", "table"]),
    default="table",
    help="Output format: json or table (default: table)"
)


def results_to_json(results: list, command: str, **extra_fields) -> str:
    """Serialize query results to JSON for agent consumption.

    Args:
        results: List of result dictionaries
        command: Command name for logging
        **extra_fields: Additional fields to include in output

    Returns:
        JSON string with timestamp, results, and result_count
    """
    output = {
        "command": command,
        "timestamp": datetime.now().isoformat(),
        "result_count": len(results),
        "results": results,
        **extra_fields
    }
    return json.dumps(output, indent=2, default=str)


def get_data_dir() -> Path:
    """Get the _data directory for snapshot outputs.

    Returns:
        Path to _data directory (created if doesn't exist)
    """
    # Store alongside the database
    db_path = get_database_path()
    data_dir = db_path.parent / "_data"
    data_dir.mkdir(parents=True, exist_ok=True)
    (data_dir / "snapshots").mkdir(exist_ok=True)
    return data_dir


@click.group()
@click.version_option(version="0.1.0")
def cli():
    """Context OS Events - Event capture layer for Context OS.

    Tracks file events, Claude sessions, and git commits for analysis.

    Run 'context-os agent-context' for a quick system overview.
    """
    pass


@cli.command()
def init():
    """Initialize the database.

    Creates the SQLite database and schema if it doesn't exist.
    """
    db_path = get_database_path()

    if db_path.exists():
        console.print(f"[yellow]Database already exists:[/yellow] {db_path}")
        return

    init_database(db_path)
    console.print(f"[green]Database initialized:[/green] {db_path}")


@cli.command("sync-git")
@click.option("--since", default="90 days", help="Sync commits since date (e.g., '90 days', '2025-01-01')")
@click.option("--full", is_flag=True, help="Full resync (ignore incremental)")
@click.option("--repo", default=".", help="Repository path")
def sync_git(since: str, full: bool, repo: str):
    """Sync git commits to database.

    Parses git log and stores commit data for analysis.

    Examples:

        context-os sync-git

        context-os sync-git --since "30 days"

        context-os sync-git --full
    """
    db = get_connection()
    repo_path = str(Path(repo).resolve())

    console.print(f"[dim]Syncing commits from:[/dim] {repo_path}")
    console.print(f"[dim]Since:[/dim] {since}")

    # Log command start
    start_time = log_command_start("sync-git", {"repo": repo_path, "since": since})

    try:
        result = sync_commits(db, {
            "since": since,
            "repo_path": repo_path,
            "incremental": not full
        })

        console.print(f"[green]Synced {result['commits_synced']} commits[/green]")
        if result["commits_skipped"]:
            console.print(f"[dim]Skipped {result['commits_skipped']} existing[/dim]")
        if result["errors"]:
            console.print(f"[yellow]Warnings: {len(result['errors'])}[/yellow]")
            for err in result["errors"][:5]:
                console.print(f"  [dim]{err}[/dim]")

        # Log command complete and update state
        log_command_complete("sync-git", start_time, {
            "commits_synced": result['commits_synced'],
            "commits_skipped": result['commits_skipped'],
        })
        update_state()

    except GitError as e:
        log_command_error(
            "sync-git", start_time, str(e),
            suggestion="Check that the path is a valid git repository"
        )
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)
    finally:
        db.close()


@cli.command("parse-sessions")
@click.option("--project", default=".", help="Project path to parse sessions for")
@click.option("--full", is_flag=True, help="Full reparse (ignore incremental)")
def parse_sessions(project: str, full: bool):
    """Parse Claude session JSONL files.

    Reads Claude Code session files and extracts:
    - Files read and written
    - Tool usage patterns
    - Grep patterns (automation candidates)

    Examples:

        context-os parse-sessions

        context-os parse-sessions --project /path/to/project

        context-os parse-sessions --full
    """
    db = get_connection()
    project_path = str(Path(project).resolve())

    console.print(f"[dim]Parsing sessions for:[/dim] {project_path}")

    # Log command start
    start_time = log_command_start("parse-sessions", {"project": project_path})

    try:
        result = sync_sessions(db, {
            "project_path": project_path,
            "incremental": not full
        })

        console.print(f"[green]Parsed {result['sessions_parsed']} sessions[/green]")
        console.print(f"[dim]Total tool uses: {result['total_tool_uses']}[/dim]")
        if result["sessions_skipped"]:
            console.print(f"[dim]Skipped {result['sessions_skipped']} unchanged[/dim]")
        if result["errors"]:
            console.print(f"[yellow]Warnings: {len(result['errors'])}[/yellow]")
            for err in result["errors"][:5]:
                console.print(f"  [dim]{err}[/dim]")

        # Log command complete and update state
        log_command_complete("parse-sessions", start_time, {
            "sessions_parsed": result['sessions_parsed'],
            "sessions_skipped": result['sessions_skipped'],
            "total_tool_uses": result['total_tool_uses'],
        })
        update_state()

    except Exception as e:
        log_command_error(
            "parse-sessions", start_time, str(e),
            suggestion="Check that Claude sessions exist in ~/.claude/projects/"
        )
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)
    finally:
        db.close()


def get_jsonl_dir_for_project(project_path: str) -> Path:
    """Get the JSONL directory for a project path.

    Args:
        project_path: Absolute path to the project

    Returns:
        Path to ~/.claude/projects/{encoded_path}/
    """
    claude_projects_dir = get_claude_projects_dir()
    encoded_path = encode_project_path(project_path)
    return claude_projects_dir / encoded_path


@cli.command("build-chains")
@click.option("--project", default=".", help="Project path to build chains for")
def build_chains(project: str):
    """Build and persist chain graph from JSONL leafUuid linking.

    Scans JSONL session files and builds the chain graph based on
    Claude Code's leafUuid mechanism, which tracks conversation
    continuations explicitly.

    The chain graph is persisted to:
    - chains: Chain metadata (root session, session count, files)
    - chain_graph: Session-to-chain mappings with parent links

    Examples:

        context-os build-chains

        context-os build-chains --project /path/to/project
    """
    db = get_connection()
    project_path = str(Path(project).resolve())

    console.print(f"[dim]Building chain graph for:[/dim] {project_path}")

    # Log command start
    start_time = log_command_start("build-chains", {"project": project_path})

    try:
        # Get JSONL directory
        jsonl_dir = get_jsonl_dir_for_project(project_path)

        if not jsonl_dir.exists():
            console.print(f"[yellow]No JSONL directory found:[/yellow] {jsonl_dir}")
            console.print("[dim]Run 'parse-sessions' first to sync session data.[/dim]")
            log_command_complete("build-chains", start_time, {"chains_built": 0, "sessions_linked": 0})
            return

        # Count JSONL files (recursive to include subagents/ directories)
        jsonl_files = list(jsonl_dir.glob("**/*.jsonl"))
        if not jsonl_files:
            console.print(f"[yellow]No JSONL files found in:[/yellow] {jsonl_dir}")
            console.print("[dim]0 chains built, 0 sessions linked.[/dim]")
            log_command_complete("build-chains", start_time, {"chains_built": 0, "sessions_linked": 0})
            return

        console.print(f"[dim]Found {len(jsonl_files)} session files[/dim]")

        # Build chain graph
        chains = build_chain_graph(jsonl_dir)

        if not chains:
            console.print("[dim]0 chains built, 0 sessions linked.[/dim]")
            log_command_complete("build-chains", start_time, {"chains_built": 0, "sessions_linked": 0})
            return

        # Persist to database
        stats = persist_chains(db, chains)

        console.print(f"[green]Built {stats['chains_stored']} chains[/green]")
        console.print(f"[green]Linked {stats['sessions_stored']} sessions[/green]")

        # Show chain summary
        if len(chains) <= 5:
            for chain_id, chain in chains.items():
                console.print(
                    f"  [dim]{chain_id}:[/dim] {len(chain.sessions)} sessions, "
                    f"root={chain.root_session[:8]}..."
                )

        # Log command complete and update state
        log_command_complete("build-chains", start_time, {
            "chains_built": stats['chains_stored'],
            "sessions_linked": stats['sessions_stored'],
        })
        update_state()

    except Exception as e:
        log_command_error(
            "build-chains", start_time, str(e),
            suggestion="Run 'parse-sessions' first to ensure session data exists"
        )
        console.print(f"[red]Error:[/red] {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
    finally:
        db.close()


@cli.command()
@click.option("--path", default=".", help="Directory to watch")
def watch(path: str):
    """Watch directory for file events.

    Captures file creation, modification, and deletion events
    in real-time. Press Ctrl+C to stop.

    Examples:

        context-os watch

        context-os watch --path /path/to/project
    """
    import signal

    db = get_connection()
    watch_path = str(Path(path).resolve())

    console.print(f"[cyan]Watching:[/cyan] {watch_path}")
    console.print("[dim]Press Ctrl+C to stop[/dim]\n")

    watcher = None

    def handle_shutdown(signum, frame):
        nonlocal watcher
        console.print("\n[yellow]Stopping watcher...[/yellow]")
        if watcher:
            stats = stop_watcher(watcher)
            console.print(f"[green]Events captured:[/green] {stats.get('events_captured', 0)}")
            console.print(f"[dim]Events filtered:[/dim] {stats.get('events_filtered', 0)}")
        db.close()
        sys.exit(0)

    signal.signal(signal.SIGINT, handle_shutdown)

    try:
        watcher = start_watcher(watch_path, db)
        console.print("[green]Watcher started[/green]")

        # Keep running
        while True:
            import time
            time.sleep(1)

    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        if watcher:
            stop_watcher(watcher)
        db.close()
        sys.exit(1)


@cli.command()
def status():
    """Show database status and statistics.

    Displays counts of events, sessions, and commits with time periods.
    """
    db = get_connection()

    try:
        # Git commits stats with date range
        cursor = db.execute("""
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_agent_commit THEN 1 ELSE 0 END) as agent_commits,
                MIN(timestamp) as earliest,
                MAX(timestamp) as latest
            FROM git_commits
        """)
        git_row = cursor.fetchone()

        # Claude sessions stats with date range
        cursor = db.execute("""
            SELECT
                COUNT(*) as total,
                SUM(total_messages) as total_messages,
                MIN(started_at) as earliest,
                MAX(ended_at) as latest
            FROM claude_sessions
        """)
        session_row = cursor.fetchone()

        # File events stats
        cursor = db.execute("""
            SELECT
                COUNT(*) as total,
                COUNT(DISTINCT path) as unique_files
            FROM file_events
        """)
        file_row = cursor.fetchone()

        # Display counts table
        table = Table(title="Context OS Events Status")
        table.add_column("Source", style="cyan")
        table.add_column("Count", justify="right")
        table.add_column("Details")

        table.add_row(
            "Git Commits",
            str(git_row["total"]),
            f"Agent: {git_row['agent_commits'] or 0}"
        )
        table.add_row(
            "Claude Sessions",
            str(session_row["total"]),
            f"Messages: {session_row['total_messages'] or 0}"
        )
        table.add_row(
            "File Events",
            str(file_row["total"]),
            f"Unique files: {file_row['unique_files']}"
        )

        console.print(table)

        # Calculate time period
        from datetime import datetime

        dates = []
        commit_earliest = git_row["earliest"][:10] if git_row["earliest"] else None
        commit_latest = git_row["latest"][:10] if git_row["latest"] else None
        session_earliest = session_row["earliest"][:10] if session_row["earliest"] else None
        session_latest = session_row["latest"][:10] if session_row["latest"] else None

        if commit_earliest:
            dates.append(datetime.strptime(commit_earliest, "%Y-%m-%d"))
        if commit_latest:
            dates.append(datetime.strptime(commit_latest, "%Y-%m-%d"))
        if session_earliest:
            dates.append(datetime.strptime(session_earliest, "%Y-%m-%d"))
        if session_latest:
            dates.append(datetime.strptime(session_latest, "%Y-%m-%d"))

        # Show time period info
        console.print("")
        if commit_earliest and commit_latest:
            console.print(f"[dim]Commit range:[/dim]  {commit_earliest} to {commit_latest}")
        if session_earliest and session_latest:
            console.print(f"[dim]Session range:[/dim] {session_earliest} to {session_latest}")

        # Calculate and show rates
        if len(dates) >= 2:
            days_span = (max(dates) - min(dates)).days + 1
            console.print(f"[dim]Total span:[/dim]    {days_span} days")

            if days_span > 0:
                commits_per_day = git_row["total"] / days_span
                sessions_per_day = session_row["total"] / days_span
                console.print("")
                console.print(f"[cyan]Rate:[/cyan] {commits_per_day:.1f} commits/day, {sessions_per_day:.1f} sessions/day")

    finally:
        db.close()


@cli.command("game-trails")
@click.option("--limit", default=20, help="Number of results to show")
def game_trails(limit: int):
    """Show most accessed files (game trails).

    Files that are read most often across Claude sessions and file events.
    """
    db = get_connection()

    try:
        # Query from claude_sessions files_read
        cursor = db.execute("""
            SELECT
                json_each.value as path,
                COUNT(*) as access_count
            FROM claude_sessions, json_each(files_read)
            GROUP BY json_each.value
            ORDER BY access_count DESC
            LIMIT ?
        """, (limit,))

        rows = cursor.fetchall()

        if not rows:
            console.print("[yellow]No data yet. Run 'parse-sessions' first.[/yellow]")
            return

        table = Table(title="Game Trails - Most Accessed Files")
        table.add_column("Rank", justify="right", style="dim")
        table.add_column("File", style="cyan")
        table.add_column("Accesses", justify="right")

        for i, row in enumerate(rows, 1):
            table.add_row(str(i), row["path"], str(row["access_count"]))

        console.print(table)

    finally:
        db.close()


@cli.command("commit-patterns")
@click.option("--limit", default=20, help="Number of results to show")
def commit_patterns(limit: int):
    """Show commit patterns and hotspots.

    Files that are modified most often in git commits.
    """
    db = get_connection()

    try:
        cursor = db.execute("""
            SELECT
                json_each.value as path,
                COUNT(DISTINCT hash) as commit_count
            FROM git_commits, json_each(files_changed)
            GROUP BY json_each.value
            ORDER BY commit_count DESC
            LIMIT ?
        """, (limit,))

        rows = cursor.fetchall()

        if not rows:
            console.print("[yellow]No commits synced yet. Run 'sync-git' first.[/yellow]")
            return

        table = Table(title="Commit Hotspots - Most Modified Files")
        table.add_column("Rank", justify="right", style="dim")
        table.add_column("File", style="cyan")
        table.add_column("Commits", justify="right")

        for i, row in enumerate(rows, 1):
            table.add_row(str(i), row["path"], str(row["commit_count"]))

        console.print(table)

    finally:
        db.close()


@cli.command("agent-commits")
@click.option("--limit", default=10, help="Number of results to show")
def agent_commits(limit: int):
    """Show recent Claude-generated commits.

    Lists commits that were created by Claude Code.
    """
    db = get_connection()

    try:
        cursor = db.execute("""
            SELECT short_hash, timestamp, message, files_count
            FROM git_commits
            WHERE is_agent_commit = 1
            ORDER BY timestamp DESC
            LIMIT ?
        """, (limit,))

        rows = cursor.fetchall()

        if not rows:
            console.print("[yellow]No agent commits found.[/yellow]")
            return

        table = Table(title="Recent Claude-Generated Commits")
        table.add_column("Hash", style="cyan")
        table.add_column("Date", style="dim")
        table.add_column("Message")
        table.add_column("Files", justify="right")

        for row in rows:
            date = row["timestamp"][:10] if row["timestamp"] else ""
            msg = row["message"][:50] + "..." if len(row["message"] or "") > 50 else row["message"]
            table.add_row(row["short_hash"], date, msg, str(row["files_count"]))

        console.print(table)

    finally:
        db.close()


@cli.command()
@click.option("--days", default=7, help="Number of days to analyze")
def intelligence(days: int):
    """Generate contextual intelligence report.

    Analyzes recent sessions and produces a contextual report that transforms
    raw analytics (31% Bash) into actionable insights (Bash used for git ops).

    This command provides a quick summary. For deeper analysis, invoke the
    meta-intelligence skill: /meta-intelligence

    Examples:

        context-os intelligence

        context-os intelligence --days 14
    """
    from datetime import datetime, timedelta

    db = get_connection()
    data_dir = get_data_dir()

    console.print(f"[cyan]Generating Intelligence Report[/cyan] (last {days} days)")
    console.print("")

    try:
        # Get recent sessions
        cutoff = (datetime.now() - timedelta(days=days)).isoformat()

        cursor = db.execute("""
            SELECT session_id, started_at, total_messages, tools_used
            FROM claude_sessions
            WHERE started_at >= ?
            ORDER BY started_at DESC
            LIMIT 10
        """, (cutoff,))
        sessions = cursor.fetchall()

        # Get tool patterns
        cursor = db.execute("SELECT * FROM tool_patterns")
        tools = cursor.fetchall()

        # Get game trails
        cursor = db.execute("SELECT * FROM game_trails LIMIT 10")
        trails = cursor.fetchall()

        # Get recent commits
        cursor = db.execute("""
            SELECT short_hash, message, is_agent_commit
            FROM git_commits
            WHERE timestamp >= ?
            ORDER BY timestamp DESC
            LIMIT 5
        """, (cutoff,))
        commits = cursor.fetchall()

        # Display summary
        console.print(f"[bold]Sessions:[/bold] {len(sessions)} in last {days} days")

        if tools:
            console.print("")
            console.print("[bold]Tool Usage:[/bold]")
            for tool in tools[:5]:
                console.print(f"  {tool['tool']}: {tool['total_uses']} uses")

        if trails:
            console.print("")
            console.print("[bold]Top Game Trails:[/bold]")
            for trail in trails[:5]:
                # Shorten path for display
                path = trail['path']
                if len(path) > 60:
                    path = "..." + path[-57:]
                console.print(f"  {path}: {trail['total_accesses']} accesses")

        if commits:
            console.print("")
            console.print("[bold]Recent Commits:[/bold]")
            for commit in commits:
                agent = "[agent]" if commit['is_agent_commit'] else ""
                msg = commit['message'][:40] + "..." if len(commit['message'] or "") > 40 else commit['message']
                console.print(f"  {commit['short_hash']} {msg} {agent}")

        # Point to skill for deeper analysis
        console.print("")
        console.print("[dim]-------------------------------------[/dim]")
        console.print("")
        console.print("[cyan]For contextual intelligence[/cyan] (themes, intent, decisions):")
        console.print("  Use the meta-intelligence skill:")
        console.print("  [bold]/meta-intelligence[/bold] or read .claude/skills/meta-intelligence/SKILL.md")

    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)
    finally:
        db.close()


@cli.command()
@click.option("--daily", is_flag=True, help="Generate dated daily snapshot")
def snapshot(daily: bool):
    """Generate analysis snapshot files.

    Creates browsable markdown files in _data/:
    - game_trails.md - Most accessed files
    - tool_patterns.md - Tool usage breakdown
    - automation_candidates.md - Repeated patterns
    - commit_hotspots.md - Most modified files
    - snapshots/latest.md - Combined snapshot

    Examples:

        context-os snapshot

        context-os snapshot --daily
    """
    db = get_connection()
    data_dir = get_data_dir()

    console.print(f"[dim]Generating snapshots to:[/dim] {data_dir}")

    try:
        generator = SnapshotGenerator(db, data_dir)
        generator.generate_all()

        # Count generated files
        files = list(data_dir.glob("*.md")) + list((data_dir / "snapshots").glob("*.md"))

        console.print(f"[green]Generated {len(files)} snapshot files[/green]")
        console.print("")
        console.print("[cyan]Files created:[/cyan]")
        console.print(f"  {data_dir / 'game_trails.md'}")
        console.print(f"  {data_dir / 'tool_patterns.md'}")
        console.print(f"  {data_dir / 'automation_candidates.md'}")
        console.print(f"  {data_dir / 'commit_hotspots.md'}")
        console.print(f"  {data_dir / 'snapshots' / 'latest.md'}")

    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)
    finally:
        db.close()


# =============================================================================
# Agent Context Command
# =============================================================================


@cli.command("agent-context")
@click.option(
    "--format", "output_format",
    type=click.Choice(["json", "markdown"]),
    default="markdown",
    help="Output format: json or markdown (default: markdown)"
)
@click.option(
    "--include-errors", is_flag=True,
    help="Show full error details (last 10)"
)
@click.option(
    "--since", default="24h",
    help="Only activity since timeframe (e.g., 1h, 24h, 7d)"
)
def agent_context(output_format: str, include_errors: bool, since: str):
    """Generate agent context summary.

    Outputs a markdown (or JSON) summary optimized for Claude Code agents
    to quickly understand system state.

    Examples:

        context-os agent-context

        context-os agent-context --format json

        context-os agent-context --include-errors --since 1h
    """
    from .agent_context import generate_agent_context
    from .observability.state import DEFAULT_STATE_DIR, update_state

    # Ensure state is up to date
    try:
        update_state()
    except Exception:
        pass  # Continue even if state update fails

    # Generate context
    result = generate_agent_context(
        state_dir=DEFAULT_STATE_DIR,
        output_format=output_format,
        include_errors=include_errors,
        since=since,
    )

    # Output (use click.echo for both to handle Windows encoding)
    click.echo(result)


# =============================================================================
# Daemon Commands
# =============================================================================


@cli.group()
def daemon():
    """Daemon management commands.

    Background daemon for continuous event capture.
    Runs as Windows service via Servy.
    """
    pass


@daemon.command("config")
def daemon_config():
    """Show current daemon configuration.

    Displays the configuration loaded from ~/.context-os/config.yaml
    """
    from .daemon.config import load_config, ensure_config_dir

    config_dir = ensure_config_dir()
    config_file = config_dir / "config.yaml"
    config = load_config(config_file)

    console.print(f"[cyan]Config file:[/cyan] {config_file}")
    console.print("")

    # Display config sections
    console.print("[bold]Sync Settings:[/bold]")
    console.print(f"  Interval: {config['sync']['interval_minutes']} minutes")
    console.print(f"  Git since: {config['sync']['git_since_days']} days")

    console.print("")
    console.print("[bold]Watch Settings:[/bold]")
    console.print(f"  Enabled: {config['watch']['enabled']}")
    console.print(f"  Paths: {config['watch']['paths']}")
    console.print(f"  Debounce: {config['watch']['debounce_ms']}ms")

    console.print("")
    console.print("[bold]Logging:[/bold]")
    console.print(f"  Level: {config['logging']['level']}")
    console.print(f"  Max size: {config['logging']['max_size_mb']}MB")


@daemon.command("status")
def daemon_status():
    """Show daemon service status.

    Displays whether the service is installed, running, or stopped.
    """
    from .daemon.service import get_service_status, get_service_name

    service_name = get_service_name()
    status = get_service_status()

    if status is None:
        console.print(f"[yellow]Service not installed:[/yellow] {service_name}")
        console.print("")
        console.print("[dim]To install:[/dim] context-os daemon install")
    elif status == "Running":
        console.print(f"[green]Service running:[/green] {service_name}")
    elif status == "Stopped":
        console.print(f"[yellow]Service stopped:[/yellow] {service_name}")
        console.print("")
        console.print("[dim]To start:[/dim] context-os daemon start")
    else:
        console.print(f"[dim]Service status:[/dim] {status}")


@daemon.command("run")
@click.option("--once", is_flag=True, help="Run sync once and exit (for testing)")
def daemon_run(once: bool):
    """Run daemon in foreground mode.

    Useful for debugging. Press Ctrl+C to stop.

    Examples:

        context-os daemon run

        context-os daemon run --once
    """
    from .daemon.config import load_config
    from .daemon.runner import ContextOSDaemon

    config = load_config()
    daemon_instance = ContextOSDaemon(config)

    if once:
        console.print("[dim]Running single sync...[/dim]")
        daemon_instance.run_sync()
        console.print("[green]Sync complete[/green]")
        return

    console.print("[cyan]Starting daemon in foreground mode...[/cyan]")
    console.print("[dim]Press Ctrl+C to stop[/dim]")
    console.print("")

    try:
        daemon_instance.start()
        # Keep running until interrupted
        import time
        while daemon_instance._running:
            time.sleep(1)
    except KeyboardInterrupt:
        console.print("")
        console.print("[yellow]Stopping daemon...[/yellow]")
    finally:
        daemon_instance.stop()
        console.print("[green]Daemon stopped[/green]")


@daemon.command("install")
def daemon_install():
    """Install daemon as Windows service.

    Requires:
    - Servy CLI installed (https://github.com/aelassas/servy/releases)
    - Admin privileges

    The service will auto-start on boot.
    """
    from .daemon.service import install_service, is_servy_available

    if not is_servy_available():
        console.print("[red]Servy CLI not found[/red]")
        console.print("")
        console.print("Please install Servy from:")
        console.print("  https://github.com/aelassas/servy/releases")
        console.print("")
        console.print("Then add servy-cli.exe to your PATH")
        sys.exit(1)

    console.print("[dim]Installing service...[/dim]")

    success, message = install_service()

    if success:
        console.print(f"[green]{message}[/green]")
        console.print("")
        console.print("To start the service:")
        console.print("  context-os daemon start")
    else:
        console.print(f"[red]Installation failed:[/red] {message}")
        sys.exit(1)


@daemon.command("uninstall")
def daemon_uninstall():
    """Uninstall daemon Windows service.

    Requires admin privileges.
    """
    from .daemon.service import uninstall_service, is_servy_available

    if not is_servy_available():
        console.print("[red]Servy CLI not found[/red]")
        sys.exit(1)

    console.print("[dim]Uninstalling service...[/dim]")

    success, message = uninstall_service()

    if success:
        console.print(f"[green]{message}[/green]")
    else:
        console.print(f"[red]Uninstallation failed:[/red] {message}")
        sys.exit(1)


@daemon.command("start")
def daemon_start():
    """Start the daemon service."""
    from .daemon.service import start_service, get_service_status

    status = get_service_status()
    if status is None:
        console.print("[red]Service not installed[/red]")
        console.print("Run: context-os daemon install")
        sys.exit(1)

    if status == "Running":
        console.print("[yellow]Service already running[/yellow]")
        return

    success, message = start_service()
    if success:
        console.print("[green]Service started[/green]")
    else:
        console.print(f"[red]Failed to start:[/red] {message}")
        sys.exit(1)


@daemon.command("stop")
def daemon_stop():
    """Stop the daemon service."""
    from .daemon.service import stop_service, get_service_status

    status = get_service_status()
    if status is None:
        console.print("[red]Service not installed[/red]")
        sys.exit(1)

    if status == "Stopped":
        console.print("[yellow]Service already stopped[/yellow]")
        return

    success, message = stop_service()
    if success:
        console.print("[green]Service stopped[/green]")
    else:
        console.print(f"[red]Failed to stop:[/red] {message}")
        sys.exit(1)


@daemon.command("logs")
@click.option("--lines", "-n", default=50, help="Number of lines to show")
def daemon_logs(lines: int):
    """Show daemon log file.

    Displays recent log entries from the daemon.
    """
    from .daemon.config import ensure_config_dir

    config_dir = ensure_config_dir()
    log_dir = config_dir / "logs"
    log_file = log_dir / "daemon.log"

    console.print(f"[dim]Log file:[/dim] {log_file}")
    console.print("")

    if not log_file.exists():
        console.print("[yellow]No log file found[/yellow]")
        console.print("The daemon may not have run yet.")
        return

    # Read last N lines
    with open(log_file, "r") as f:
        all_lines = f.readlines()
        recent = all_lines[-lines:] if len(all_lines) > lines else all_lines

    for line in recent:
        console.print(line.rstrip())


# =============================================================================
# Query Commands
# =============================================================================


def build_index_from_jsonl(project_path=None):
    """Build ContextIndex from JSONL files for a project.

    Args:
        project_path: Path to project directory. If None, uses current working dir.

    Returns:
        ContextIndex populated with session data from JSONL files
    """
    from .capture.jsonl_parser import find_session_files, extract_session_id, get_claude_projects_dir, encode_project_path
    from .index import (
        ContextIndex,
        FileAccess,
        Chain,
        TemporalBucket,
        extract_file_accesses,
        build_chain_graph,
        build_co_access_matrix,
        build_temporal_buckets,
    )
    from datetime import datetime
    import json

    # Use provided path or current directory
    if project_path is None:
        project_path = Path.cwd()
    else:
        project_path = Path(project_path)

    # Check if .claude directory exists
    claude_dir = project_path / ".claude"
    if not claude_dir.exists():
        click.echo(
            f"Error: No Claude session directory found for project: {project_path}\n"
            "Ensure Claude Code has been used in this project.",
            err=True
        )
        sys.exit(1)

    # Find JSONL files
    jsonl_files = find_session_files(str(project_path))

    if not jsonl_files:
        click.echo(
            f"Error: No JSONL session files found for project: {project_path}",
            err=True
        )
        sys.exit(1)

    # Get the JSONL directory for chain building
    claude_projects_dir = get_claude_projects_dir()
    encoded_path = encode_project_path(str(project_path))
    jsonl_dir = claude_projects_dir / encoded_path

    # Build index
    index = ContextIndex()
    all_file_accesses = []
    session_to_chain = {}

    for jsonl_path in jsonl_files:
        session_id = extract_session_id(jsonl_path)

        # Extract file accesses
        accesses = extract_file_accesses(jsonl_path, session_id)
        all_file_accesses.extend(accesses)

        # Extract chain info (leafUuid)
        try:
            with open(jsonl_path, 'r', encoding='utf-8', errors='replace') as f:
                for line in f:
                    try:
                        data = json.loads(line)
                        if data.get("type") == "summary" and data.get("leafUuid"):
                            leaf_uuid = data["leafUuid"]
                            session_to_chain[session_id] = leaf_uuid
                            break
                    except json.JSONDecodeError:
                        continue
        except Exception:
            pass

    # Build inverted index (file -> accesses)
    for access in all_file_accesses:
        if access.file_path not in index._inverted_index:
            index._inverted_index[access.file_path] = []
        index._inverted_index[access.file_path].append(access)

    # Build chain graph (expects directory path, not file list)
    chain_dict = build_chain_graph(jsonl_dir)

    # Build session -> file accesses map for enrichment
    session_files: Dict[str, List] = {}
    session_times: Dict[str, List[datetime]] = {}
    for access in all_file_accesses:
        sid = access.session_id
        if sid not in session_files:
            session_files[sid] = []
        session_files[sid].append(access.file_path)
        if access.timestamp:
            if sid not in session_times:
                session_times[sid] = []
            session_times[sid].append(access.timestamp)

    # Enrich chains with files and time_range
    for chain_id, chain in chain_dict.items():
        # Collect all files and timestamps from sessions in this chain
        chain_files = set()
        chain_times = []

        for session_id in chain.sessions:
            if session_id in session_files:
                chain_files.update(session_files[session_id])
            if session_id in session_times:
                chain_times.extend(session_times[session_id])

        # Update chain with enriched data
        chain.files_list = sorted(chain_files)
        if chain_times:
            chain.time_range = (min(chain_times), max(chain_times))

        index._chains[chain_id] = chain

    # Build co-access matrix (expects inverted index dict)
    co_access = build_co_access_matrix(index._inverted_index)
    index._co_access = co_access

    # Build temporal buckets (expects inverted index dict)
    temporal = build_temporal_buckets(index._inverted_index)
    index._temporal = temporal

    return index


def find_matching_files(index, pattern: str):
    """Find files containing pattern in path (case-insensitive).

    Returns:
        List of (file_path, access_count) tuples sorted by count descending
    """
    matches = []
    pattern_lower = pattern.lower()

    for file_path, accesses in index._inverted_index.items():
        if pattern_lower in file_path.lower():
            matches.append((file_path, len(accesses)))

    return sorted(matches, key=lambda x: -x[1])


def find_matching_sessions(index, prefix: str):
    """Find session IDs matching prefix.

    Returns:
        List of matching session IDs
    """
    all_sessions = set()

    for accesses in index._inverted_index.values():
        for access in accesses:
            if access.session_id.startswith(prefix):
                all_sessions.add(access.session_id)

    return list(all_sessions)


@cli.group()
def query():
    """Query session and file data.

    Commands to explore the bidirectional session↔file index.

    Examples:

        context-os query recent

        context-os query file src/main.py

        context-os query session abc123

        context-os query search nickel
    """
    pass


@query.command("file")
@click.argument("file_path")
@click.option("--limit", "-n", default=10, help="Max sessions to show")
@format_option
def query_file(file_path: str, limit: int, output_format: str):
    """Show sessions that touched a file.

    Supports exact match, suffix match, or substring match.

    Examples:

        context-os query file src/main.py

        context-os query file main.py --format json

        context-os query file cli
    """
    start_time = time.time()
    original_query = file_path
    index = build_index_from_jsonl()

    # Try exact match first
    accesses = index.get_sessions_for_file(file_path)

    # Try suffix match if no exact
    if not accesses:
        for path in index._inverted_index.keys():
            if path.endswith(file_path):
                accesses = index._inverted_index[path]
                file_path = path
                break

    # Try substring match if still nothing
    if not accesses:
        matches = find_matching_files(index, file_path)
        if matches:
            best_match = matches[0][0]
            accesses = index._inverted_index[best_match]
            file_path = best_match

    if not accesses:
        log_query(
            command="query file",
            args={"file_path": original_query, "limit": limit, "format": output_format},
            result_summary={"found": False, "matched_path": None, "sessions": 0},
            duration_ms=(time.time() - start_time) * 1000
        )
        if output_format == "json":
            click.echo(results_to_json([], "file", file_path=original_query, found=False))
        else:
            console.print(f"[yellow]File not found in index:[/yellow] {file_path}")
            console.print("\nUse 'context-os query search <pattern>' to find files.")
        return

    # Group by session
    sessions = {}
    for access in accesses:
        if access.session_id not in sessions:
            sessions[access.session_id] = []
        sessions[access.session_id].append(access)

    # Sort by most recent
    sorted_sessions = sorted(
        sessions.items(),
        key=lambda x: max(a.timestamp for a in x[1] if a.timestamp) if any(a.timestamp for a in x[1]) else "",
        reverse=True
    )[:limit]

    # Log successful query
    log_query(
        command="query file",
        args={"file_path": original_query, "limit": limit, "format": output_format},
        result_summary={
            "found": True,
            "matched_path": file_path.split("\\")[-1],
            "total_sessions": len(sessions),
            "shown": len(sorted_sessions)
        },
        duration_ms=(time.time() - start_time) * 1000
    )

    if output_format == "json":
        results = []
        for session_id, session_accesses in sorted_sessions:
            access_types = list(set(a.access_type for a in session_accesses if a.access_type))
            last_access = max(
                (a.timestamp for a in session_accesses if a.timestamp),
                default=None
            )
            chain_id = index.get_chain_for_session(session_id)
            results.append({
                "session_id": session_id,
                "access_types": access_types,
                "last_access": last_access.isoformat() if last_access else None,
                "chain_id": chain_id,
            })
        click.echo(results_to_json(results, "file", file_path=file_path, found=True))
        return

    table = Table(title=f"Sessions touching '{file_path}'")
    table.add_column("Session", style="cyan")
    table.add_column("Access Type")
    table.add_column("Last Access")
    table.add_column("Chain")

    for session_id, session_accesses in sorted_sessions:
        access_types = set(a.access_type for a in session_accesses if a.access_type)
        last_access = max(
            (a.timestamp for a in session_accesses if a.timestamp),
            default=None
        )
        chain_id = index.get_chain_for_session(session_id) or "-"

        table.add_row(
            session_id[:12] + "...",
            ", ".join(access_types) if access_types else "-",
            last_access.strftime("%Y-%m-%d %H:%M") if last_access else "-",
            chain_id[:12] + "..." if len(chain_id) > 12 else chain_id
        )

    console.print(table)
    console.print(f"\nShowing {len(sorted_sessions)} of {len(sessions)} sessions.")
    console.print("\nUse 'context-os query session <id>' to see files in a session.")


@query.command("co-access")
@click.argument("file_path")
@click.option("--limit", "-n", default=10, help="Max files to show")
@format_option
def query_co_access(file_path: str, limit: int, output_format: str):
    """Show files frequently co-accessed with this file.

    Uses PMI (Pointwise Mutual Information) scoring.

    Examples:

        context-os query co-access src/main.py

        context-os query co-access cli.py --limit 20 --format json
    """
    index = build_index_from_jsonl()

    # Try to find file (exact, suffix, or substring)
    matched_path = file_path
    if file_path not in index._co_access:
        # Try suffix
        for path in index._co_access.keys():
            if path.endswith(file_path):
                matched_path = path
                break
        else:
            # Try substring
            matches = find_matching_files(index, file_path)
            if matches:
                matched_path = matches[0][0]

    co_files = index.get_co_accessed(matched_path, limit)

    if output_format == "json":
        results = [
            {"file_path": co_file, "pmi_score": score}
            for co_file, score in co_files
        ]
        click.echo(results_to_json(results, "co-access", query_file=matched_path))
        return

    if not co_files:
        console.print(f"[yellow]No co-access data for:[/yellow] {file_path}")
        console.print("\nThis file may not have been accessed with other files.")
        return

    table = Table(title=f"Files co-accessed with '{matched_path}'")
    table.add_column("File", style="cyan")
    table.add_column("PMI Score", justify="right")

    for co_file, score in co_files:
        table.add_row(co_file, f"{score:.2f}")

    console.print(table)


@query.command("recent")
@click.option("--weeks", "-w", default=4, help="Number of weeks to show")
@format_option
def query_recent(weeks: int, output_format: str):
    """Show recent activity summary.

    Displays weekly buckets with session and chain counts.

    Examples:

        context-os query recent

        context-os query recent --weeks 8 --format json
    """
    start_time = time.time()
    index = build_index_from_jsonl()
    buckets = index.get_recent_weeks(weeks)

    total_sessions = 0
    total_chains = set()

    for bucket in buckets:
        total_sessions += len(bucket.sessions)
        total_chains.update(bucket.chains)

    # Log query
    log_query(
        command="query recent",
        args={"weeks": weeks, "format": output_format},
        result_summary={
            "weeks_shown": len(buckets),
            "total_sessions": total_sessions,
            "total_chains": len(total_chains)
        },
        duration_ms=(time.time() - start_time) * 1000
    )

    if output_format == "json":
        results = [
            {
                "period": bucket.period,
                "session_count": len(bucket.sessions),
                "chain_count": len(bucket.chains),
                "sessions": list(bucket.sessions)[:10],  # Limit to first 10
                "chains": list(bucket.chains),
            }
            for bucket in buckets
        ]
        click.echo(results_to_json(
            results, "recent",
            weeks=weeks,
            total_sessions=total_sessions,
            total_chains=len(total_chains)
        ))
        return

    if not buckets:
        console.print("[yellow]No activity data found.[/yellow]")
        return

    table = Table(title=f"Recent Activity (last {weeks} weeks)")
    table.add_column("Week")
    table.add_column("Sessions", justify="right")
    table.add_column("Chains", justify="right")

    for bucket in buckets:
        table.add_row(
            bucket.period,
            str(len(bucket.sessions)),
            str(len(bucket.chains))
        )

    console.print(table)
    console.print(f"\nTotal: {total_sessions} sessions across {len(total_chains)} chains.")


@query.command("chains")
@click.option("--limit", "-n", default=10, help="Max chains to show")
@format_option
def query_chains(limit: int, output_format: str):
    """Show conversation chains.

    Chains are linked sessions connected via leafUuid.

    Examples:

        context-os query chains

        context-os query chains --limit 20 --format json
    """
    index = build_index_from_jsonl()
    chains = index.get_all_chains()[:limit]

    if output_format == "json":
        results = []
        for chain in chains:
            session_count = len(chain.sessions) if hasattr(chain, 'sessions') else 0
            files_count = len(chain.files_list) if hasattr(chain, 'files_list') and chain.files_list else 0
            time_range = None
            if hasattr(chain, 'time_range') and chain.time_range:
                start, end = chain.time_range
                time_range = {
                    "start": start.isoformat() if start else None,
                    "end": end.isoformat() if end else None
                }
            results.append({
                "chain_id": chain.chain_id,
                "session_count": session_count,
                "file_count": files_count,
                "time_range": time_range,
            })
        click.echo(results_to_json(results, "chains", total_chains=len(index._chains)))
        return

    if not chains:
        console.print("[yellow]No chains found.[/yellow]")
        return

    table = Table(title="Conversation Chains")
    table.add_column("Chain ID", style="cyan")
    table.add_column("Sessions", justify="right")
    table.add_column("Files", justify="right")
    table.add_column("Time Range")

    for chain in chains:
        session_count = len(chain.sessions) if hasattr(chain, 'sessions') else 0
        files_count = len(chain.files_list) if hasattr(chain, 'files_list') and chain.files_list else 0

        time_range = "-"
        if hasattr(chain, 'time_range') and chain.time_range:
            start, end = chain.time_range
            time_range = f"{start.strftime('%m/%d')} - {end.strftime('%m/%d')}"

        table.add_row(
            chain.chain_id[:12] + "...",
            str(session_count),
            str(files_count),
            time_range
        )

    console.print(table)
    console.print(f"\nShowing {len(chains)} of {len(index._chains)} chains.")


@query.command("session")
@click.argument("session_id")
@click.option("--limit", "-n", default=20, help="Max files to show")
@format_option
def query_session(session_id: str, limit: int, output_format: str):
    """Show files touched by a session.

    Supports prefix matching for session IDs.

    Examples:

        context-os query session abc123

        context-os query session 7fab4726-9e5... --format json
    """
    start_time = time.time()
    index = build_index_from_jsonl()

    # Support prefix matching
    matched_session = None
    for accesses in index._inverted_index.values():
        for access in accesses:
            if access.session_id.startswith(session_id):
                matched_session = access.session_id
                break
        if matched_session:
            break

    if not matched_session:
        log_query(
            command="query session",
            args={"session_id": session_id, "limit": limit, "format": output_format},
            result_summary={"found": False, "files": 0},
            duration_ms=(time.time() - start_time) * 1000
        )
        if output_format == "json":
            click.echo(results_to_json([], "session", session_id=session_id, found=False))
        else:
            console.print(f"[yellow]Session not found:[/yellow] {session_id}")
            console.print("\nUse 'context-os query recent' to see active sessions.")
        return

    # Get files for session
    files = index.get_files_for_session(matched_session)

    # Get chain context
    chain_id = index.get_chain_for_session(matched_session) or "-"

    # Get access details for each file
    file_details = []
    for file_path in files:
        accesses = index._inverted_index.get(file_path, [])
        session_accesses = [a for a in accesses if a.session_id == matched_session]
        access_types = list(set(a.access_type for a in session_accesses if a.access_type))
        file_details.append({"file_path": file_path, "access_types": access_types})

    # Log successful query
    log_query(
        command="query session",
        args={"session_id": session_id, "limit": limit, "format": output_format},
        result_summary={
            "found": True,
            "matched_session": matched_session[:12],
            "total_files": len(files),
            "shown": min(limit, len(files)),
            "chain_id": chain_id[:8] if chain_id != "-" else None
        },
        duration_ms=(time.time() - start_time) * 1000
    )

    if output_format == "json":
        click.echo(results_to_json(
            file_details[:limit], "session",
            session_id=matched_session,
            chain_id=chain_id if chain_id != "-" else None,
            total_files=len(files),
            found=True
        ))
        return

    if not files:
        console.print(f"[yellow]No files found for session:[/yellow] {matched_session}")
        return

    console.print(f"[bold]Files touched by session {matched_session[:12]}...[/bold] ({len(files)} total)")
    console.print(f"Chain: {chain_id[:12]}..." if len(chain_id) > 12 else f"Chain: {chain_id}")
    console.print()

    table = Table()
    table.add_column("File", style="cyan")
    table.add_column("Access Type")

    for detail in file_details[:limit]:
        table.add_row(detail["file_path"], ", ".join(detail["access_types"]) if detail["access_types"] else "-")

    console.print(table)

    if len(files) > limit:
        console.print(f"\nShowing {limit} of {len(files)}. Use --limit to show more.")


@query.command("log")
@click.option("--limit", "-n", default=20, help="Max entries to show")
@click.option("--command", "-c", default=None, help="Filter by command name")
def query_log(limit: int, command: str):
    """View query history log.

    Shows recent CLI queries for analysis and improvement.

    Examples:

        context-os query log

        context-os query log --limit 50

        context-os query log -c "query search"
    """
    log_path = get_query_log_path()

    if not log_path.exists():
        console.print("[yellow]No query log found yet.[/yellow]")
        return

    # Read all entries
    entries = []
    with open(log_path, "r", encoding="utf-8") as f:
        for line in f:
            try:
                entry = json.loads(line.strip())
                if command is None or entry.get("command") == command:
                    entries.append(entry)
            except json.JSONDecodeError:
                continue

    if not entries:
        console.print(f"[yellow]No matching queries found.[/yellow]")
        return

    # Show most recent first
    entries = entries[-limit:][::-1]

    table = Table(title=f"Query Log ({len(entries)} entries)")
    table.add_column("Time", style="dim")
    table.add_column("Command", style="cyan")
    table.add_column("Args")
    table.add_column("Result")
    table.add_column("Duration", justify="right")

    for entry in entries:
        # Format timestamp
        ts = entry.get("timestamp", "")[:19].replace("T", " ")

        # Format args
        args = entry.get("args", {})
        args_str = ", ".join(f"{k}={v}" for k, v in args.items())
        if len(args_str) > 30:
            args_str = args_str[:27] + "..."

        # Format result
        result = entry.get("result", {})
        if "total_matches" in result:
            result_str = f"{result['total_matches']} files"
        elif "total_sessions" in result:
            result_str = f"{result['total_sessions']} sessions"
        elif "total_files" in result:
            result_str = f"{result['total_files']} files"
        elif "found" in result:
            result_str = "found" if result["found"] else "not found"
        else:
            result_str = str(result)[:20]

        # Format duration
        duration = entry.get("duration_ms", 0)
        duration_str = f"{duration/1000:.1f}s"

        table.add_row(ts, entry.get("command", ""), args_str, result_str, duration_str)

    console.print(table)
    console.print(f"\nLog file: {log_path}")


@query.command("search")
@click.argument("pattern")
@click.option("--limit", "-n", default=20, help="Max files to show")
@format_option
def query_search(pattern: str, limit: int, output_format: str):
    """Search files by pattern (substring match, case-insensitive).

    Examples:

        context-os query search nickel

        context-os query search cli.py

        context-os query search test --format json
    """
    start_time = time.time()
    index = build_index_from_jsonl()
    matches = find_matching_files(index, pattern)

    # Log the query
    log_query(
        command="query search",
        args={"pattern": pattern, "limit": limit, "format": output_format},
        result_summary={
            "total_matches": len(matches),
            "shown": min(limit, len(matches)),
            "top_files": [m[0].split("\\")[-1] for m in matches[:3]]  # Just filenames
        },
        duration_ms=(time.time() - start_time) * 1000
    )

    if output_format == "json":
        results = [
            {"file_path": file_path, "access_count": count}
            for file_path, count in matches[:limit]
        ]
        click.echo(results_to_json(results, "search", pattern=pattern))
        return

    if not matches:
        console.print(f"[yellow]No files matching '{pattern}'[/yellow]")
        return

    table = Table(title=f"Files matching '{pattern}' ({len(matches)} found, showing {min(limit, len(matches))})")
    table.add_column("Accesses", justify="right")
    table.add_column("File", style="cyan")

    for file_path, count in matches[:limit]:
        table.add_row(str(count), file_path)

    console.print(table)

    if len(matches) > limit:
        console.print(f"\nShowing {limit} of {len(matches)}. Use --limit to show more.")

    console.print("\nUse 'context-os query file <path>' to see sessions for a specific file.")
    console.print("Use 'context-os query session <id>' to see all files in a session.")


# =============================================================================
# Query Flex - Flexible Hypercube Query (Phase B)
# =============================================================================

@query.command("flex")
@click.option("--files", help="File pattern filter (glob or substring)")
@click.option("--time", "time_range", help="Time range: 7d, 2w, 2025-W50")
@click.option("--chain", help="Chain ID prefix or 'active'")
@click.option("--session", "session_prefix", help="Session ID prefix")
@click.option("--access", help="Access types: r, w, c, rw, rwc")
@click.option("--agg", default="count", help="Aggregations: count,recency,sessions,chains")
@click.option("--format", "output_format", type=click.Choice(["json", "table"]), default="json", help="Output format")
@click.option("--limit", default=20, help="Max results (default: 20)")
@click.option("--sort", type=click.Choice(["count", "recency", "alpha"]), default="count", help="Sort order")
def query_flex(files, time_range, chain, session_prefix, access, agg, output_format, limit, sort):
    """Flexible hypercube query with multi-dimensional slicing.

    Query the context index with flexible filtering across 5 dimensions:
    Files × Sessions × Time × Chains × AccessType

    Examples:

        # Client context with recency
        context-os query flex --files "*pixee*" --agg count,recency

        # Recent activity
        context-os query flex --time 7d --agg files,sessions

        # Multi-filter: Pixee files in last 2 weeks
        context-os query flex --files "*pixee*" --time 2w --format json
    """
    from .query_engine import QuerySpec, QueryEngine

    start_time = time.time()

    # Build query spec
    spec = QuerySpec(
        files=files,
        time=time_range,
        chain=chain,
        session=session_prefix,
        access=access,
        agg=agg.split(",") if agg else ["count"],
        format=output_format,
        limit=limit,
        sort=sort,
    )

    # Validate
    errors = spec.validate()
    if errors:
        for error in errors:
            click.echo(f"Error: {error}", err=True)
        sys.exit(1)

    # Build index and execute with ledger
    from .query_engine import QueryLedger
    index = build_index_from_jsonl()
    ledger = QueryLedger()  # Uses default ~/.context-os/query_ledger/
    engine = QueryEngine(index, ledger=ledger)
    result = engine.execute(spec)

    # Log the query
    log_query(
        command="query flex",
        args={
            "files": files,
            "time": time_range,
            "chain": chain,
            "session": session_prefix,
            "access": access,
            "agg": agg,
            "format": output_format,
            "limit": limit,
            "sort": sort,
        },
        result_summary={
            "receipt_id": result.receipt_id,
            "result_count": result.result_count,
        },
        duration_ms=(time.time() - start_time) * 1000
    )

    # Output
    if output_format == "json":
        click.echo(result.to_json())
    else:
        click.echo(result.to_table())


# ─────────────────────────────────────────────────────────────────────────────
# query verify - Verify a query receipt (Phase C)
# ─────────────────────────────────────────────────────────────────────────────

@query.command("verify")
@click.argument("receipt_id")
@click.option("--verbose", is_flag=True, help="Show detailed diff (added/removed files)")
@format_option
def query_verify(receipt_id: str, verbose: bool, output_format: str):
    """Verify a query receipt against current data.

    RECEIPT_ID is the receipt ID to verify (e.g., "q_7f3a2b").

    Verification checks if the query results have changed since the original
    query was run. Returns MATCH if unchanged, DRIFT if results differ.

    Examples:

        # Basic verification
        context-os query verify q_7f3a2b

        # Show detailed diff
        context-os query verify q_7f3a2b --verbose

        # JSON output
        context-os query verify q_7f3a2b --format json
    """
    from .query_engine import QuerySpec, QueryEngine, QueryLedger, VerificationResult
    import time

    start_time = time.time()

    # Build index and ledger
    index = build_index_from_jsonl()
    ledger = QueryLedger()
    engine = QueryEngine(index, ledger=ledger)

    # Verify
    result = engine.verify(receipt_id, verbose=verbose)

    # Log the verification
    log_query(
        command="query verify",
        args={"receipt_id": receipt_id, "verbose": verbose},
        result_summary={
            "status": result.status,
            "original_count": result.original_count,
            "current_count": result.current_count,
        },
        duration_ms=(time.time() - start_time) * 1000
    )

    # Output
    if output_format == "json":
        click.echo(json.dumps(result.to_dict(), indent=2))
    else:
        # Table format
        from rich.console import Console
        from rich.panel import Panel
        from rich.table import Table

        console = Console()

        if result.status == "MATCH":
            panel = Panel(
                f"[green]MATCH: Results verified[/green]\n\n"
                f"Receipt:   {result.receipt_id}\n"
                f"Original:  {result.original_timestamp}\n"
                f"Verified:  {result.verification_timestamp}\n"
                f"Count:     {result.original_count} files\n"
                f"Hash:      {result.original_hash[:40]}...",
                title="Verification Result",
                border_style="green"
            )
        elif result.status == "DRIFT":
            drift_info = f"[yellow]DRIFT: Results changed[/yellow]\n\n"
            drift_info += f"Receipt:   {result.receipt_id}\n"
            drift_info += f"Original:  {result.original_count} files ({result.original_timestamp})\n"
            drift_info += f"Current:   {result.current_count} files ({result.drift_summary})\n"
            drift_info += f"\nSuggestion: Re-run query for current data"

            if verbose and result.diff:
                if result.diff.get("added"):
                    drift_info += f"\n\n[green]Added:[/green]\n"
                    for f in result.diff["added"][:10]:
                        drift_info += f"  + {f}\n"
                if result.diff.get("removed"):
                    drift_info += f"\n[red]Removed:[/red]\n"
                    for f in result.diff["removed"][:10]:
                        drift_info += f"  - {f}\n"

            panel = Panel(drift_info, title="Verification Result", border_style="yellow")
        else:  # NOT_FOUND or EXPIRED
            panel = Panel(
                f"[red]{result.status}[/red]\n\n"
                f"Receipt:   {result.receipt_id}\n"
                f"Details:   {result.drift_summary or 'Receipt not found in ledger'}",
                title="Verification Result",
                border_style="red"
            )

        console.print(panel)


# ─────────────────────────────────────────────────────────────────────────────
# query receipts - List recent query receipts (Phase C)
# ─────────────────────────────────────────────────────────────────────────────

@query.command("receipts")
@click.option("--limit", default=20, help="Number of receipts to show (default: 20)")
@format_option
def query_receipts(limit: int, output_format: str):
    """List recent query receipts.

    Shows receipt IDs, timestamps, query summaries, and result counts.

    Examples:

        # List last 20 receipts
        context-os query receipts

        # List last 5 receipts
        context-os query receipts --limit 5

        # JSON output
        context-os query receipts --format json
    """
    from .query_engine import QueryLedger
    import time

    start_time = time.time()

    # Get receipts from ledger
    ledger = QueryLedger()
    receipts = ledger.list_receipts(limit=limit)

    # Log
    log_query(
        command="query receipts",
        args={"limit": limit},
        result_summary={"count": len(receipts)},
        duration_ms=(time.time() - start_time) * 1000
    )

    # Output
    if output_format == "json":
        click.echo(json.dumps({"receipts": receipts}, indent=2))
    else:
        from rich.console import Console
        from rich.table import Table

        console = Console()
        table = Table(title=f"Recent Query Receipts ({len(receipts)} shown)")

        table.add_column("Receipt", style="cyan")
        table.add_column("Timestamp")
        table.add_column("Query")
        table.add_column("Results", justify="right")

        for r in receipts:
            # Format timestamp to be more readable
            ts = r["timestamp"][:19]  # Trim to YYYY-MM-DDTHH:MM:SS
            table.add_row(
                r["receipt_id"],
                ts,
                r["query_summary"],
                str(r["result_count"])
            )

        console.print(table)


if __name__ == "__main__":
    cli()
