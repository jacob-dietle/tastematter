"""Agent context generation for CLI command.

Generates markdown or JSON summaries optimized for Claude Code agents
to quickly understand system state.
"""

import json
from datetime import datetime
from pathlib import Path
from typing import Dict, Any, Optional


def generate_health_section(state_dir: Path) -> str:
    """Generate health section markdown from health.json.

    Args:
        state_dir: Path to state directory containing health.json

    Returns:
        Markdown string with health status and table counts.
    """
    health_path = state_dir / "health.json"

    if not health_path.exists():
        return "## System Health: Unknown\n\nNo health data available.\n"

    with open(health_path, "r", encoding="utf-8") as f:
        health_data = json.load(f)

    # Determine status
    warnings = health_data.get("warnings", [])
    errors = health_data.get("recent_errors", [])

    if errors:
        status = "Error"
        status_icon = "[X]"
    elif warnings:
        status = "Degraded"
        status_icon = "[!]"
    else:
        status = "Healthy"
        status_icon = "[OK]"

    lines = [f"## System Health: {status_icon} {status}", ""]

    # Add table info
    tables = health_data.get("database", {}).get("tables", {})
    if tables:
        lines.append("| Table | Rows | Last Updated |")
        lines.append("|-------|------|--------------|")

        for table_name, table_info in tables.items():
            rows = table_info.get("rows", 0)
            last_updated = table_info.get("last_updated")
            if last_updated:
                # Format relative time (simplified)
                updated_str = _format_relative_time(last_updated)
            else:
                updated_str = "-"
            lines.append(f"| {table_name} | {rows} | {updated_str} |")

        lines.append("")

    return "\n".join(lines)


def generate_activity_section(state_dir: Path) -> str:
    """Generate activity section markdown from activity.json.

    Args:
        state_dir: Path to state directory containing activity.json

    Returns:
        Markdown string with recent activity and 24h summary.
    """
    activity_path = state_dir / "activity.json"

    if not activity_path.exists():
        return "## Recent Activity (24h)\n\nNo activity data available.\n"

    with open(activity_path, "r", encoding="utf-8") as f:
        activity_data = json.load(f)

    last_24h = activity_data.get("last_24h", {})
    commands_run = last_24h.get("commands_run", 0)
    errors = last_24h.get("errors", 0)
    recent_commands = activity_data.get("recent_commands", [])

    lines = [f"## Recent Activity (24h)"]

    # Summary line
    if commands_run == 0:
        lines.append("")
        lines.append("No activity in last 24 hours.")
    else:
        lines.append(f"Commands: {commands_run}, Errors: {errors}")
        lines.append("")

        # Recent commands list
        for cmd in recent_commands[:10]:  # Limit to 10
            ts = cmd.get("ts", "")
            time_str = ts[11:16] if len(ts) >= 16 else ts  # Extract HH:MM
            command = cmd.get("command", "unknown")
            status = cmd.get("status", "unknown")
            duration_ms = cmd.get("duration_ms", 0)
            duration_str = f"{duration_ms / 1000:.1f}s"

            status_icon = "[OK]" if status == "success" else "[X]"
            lines.append(f"- {time_str} `{command}` {status_icon} ({duration_str})")

    lines.append("")
    return "\n".join(lines)


def generate_error_section(state_dir: Path, include_errors: bool = False) -> str:
    """Generate error section markdown from health.json recent_errors.

    Args:
        state_dir: Path to state directory containing health.json
        include_errors: If True, show full error details

    Returns:
        Markdown string with recent errors.
    """
    health_path = state_dir / "health.json"

    if not health_path.exists():
        return "## Recent Errors\n\nNo health data available.\n"

    with open(health_path, "r", encoding="utf-8") as f:
        health_data = json.load(f)

    errors = health_data.get("recent_errors", [])

    lines = ["## Recent Errors", ""]

    if not errors:
        lines.append("None in last 24 hours.")
    else:
        for error in errors[:10]:  # Limit to 10
            ts = error.get("ts", "")
            time_str = ts[11:16] if len(ts) >= 16 else ts
            message = error.get("message", "Unknown error")
            suggestion = error.get("suggestion", "")

            if include_errors:
                lines.append(f"- {time_str} **{message}**")
                if suggestion:
                    lines.append(f"  - Suggestion: {suggestion}")
            else:
                lines.append(f"- {time_str} {message[:50]}...")

    lines.append("")
    return "\n".join(lines)


def generate_quick_reference() -> str:
    """Generate quick reference section with common commands.

    Returns:
        Markdown string with quick reference commands.
    """
    lines = [
        "## Quick Reference",
        "",
        "- Rebuild chains: `context-os build-chains`",
        "- Parse new sessions: `context-os parse-sessions`",
        "- Check status: `context-os status`",
        "- View this context: `context-os agent-context`",
        "",
    ]
    return "\n".join(lines)


def generate_agent_context(
    state_dir: Path,
    output_format: str = "markdown",
    include_errors: bool = False,
    since: str = "24h",
) -> str:
    """Generate full agent context summary.

    Args:
        state_dir: Path to state directory (contains health.json, activity.json)
        output_format: "markdown" or "json"
        include_errors: If True, show full error details
        since: Time filter (e.g., "1h", "24h", "7d") - for future use

    Returns:
        Complete agent context as markdown or JSON string.
    """
    if output_format == "json":
        # Load raw data for JSON output
        health_data = {}
        activity_data = {}

        health_path = state_dir / "health.json"
        if health_path.exists():
            with open(health_path, "r", encoding="utf-8") as f:
                health_data = json.load(f)

        activity_path = state_dir / "activity.json"
        if activity_path.exists():
            with open(activity_path, "r", encoding="utf-8") as f:
                activity_data = json.load(f)

        result = {
            "generated_at": datetime.utcnow().isoformat() + "Z",
            "health": health_data,
            "activity": activity_data,
            "quick_reference": {
                "build_chains": "context-os build-chains",
                "parse_sessions": "context-os parse-sessions",
                "status": "context-os status",
                "agent_context": "context-os agent-context",
            },
        }
        return json.dumps(result, indent=2)

    # Markdown output
    now = datetime.utcnow()
    timestamp = now.strftime("%Y-%m-%d %H:%M:%S UTC")

    sections = [
        f"# Context OS - Agent Context Summary",
        f"Generated: {timestamp}",
        "",
        generate_health_section(state_dir),
        generate_activity_section(state_dir),
        generate_error_section(state_dir, include_errors=include_errors),
        generate_quick_reference(),
    ]

    return "\n".join(sections)


def _format_relative_time(iso_timestamp: str) -> str:
    """Format ISO timestamp as relative time (e.g., '2 min ago').

    Args:
        iso_timestamp: ISO8601 timestamp string

    Returns:
        Human-readable relative time string.
    """
    try:
        # Parse ISO timestamp
        if iso_timestamp.endswith("Z"):
            dt = datetime.fromisoformat(iso_timestamp.replace("Z", "+00:00"))
        else:
            dt = datetime.fromisoformat(iso_timestamp)

        now = datetime.now(dt.tzinfo) if dt.tzinfo else datetime.now()
        delta = now - dt.replace(tzinfo=None) if dt.tzinfo else now - dt

        seconds = delta.total_seconds()
        if seconds < 60:
            return "just now"
        elif seconds < 3600:
            mins = int(seconds / 60)
            return f"{mins} min ago"
        elif seconds < 86400:
            hours = int(seconds / 3600)
            return f"{hours} hour{'s' if hours > 1 else ''} ago"
        else:
            days = int(seconds / 86400)
            return f"{days} day{'s' if days > 1 else ''} ago"
    except Exception:
        return iso_timestamp[:19]
