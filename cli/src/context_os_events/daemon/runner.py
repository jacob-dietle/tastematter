"""Daemon runner module.

Main daemon loop orchestrating:
- File watcher (continuous)
- Git/session sync (periodic via scheduler)
- Event emission for future AI agent hooks
"""

import logging
import threading
import time
from datetime import datetime
from pathlib import Path
from typing import Callable, Dict, List, Optional

import schedule

from context_os_events.daemon.config import DaemonConfig, ensure_config_dir
from context_os_events.daemon.state import DaemonState

logger = logging.getLogger(__name__)

# Event handler type for future AI agent integration
EventHandler = Callable[[str, dict], None]


class ContextOSDaemon:
    """Main daemon orchestrator.

    Coordinates:
    - File watching (continuous)
    - Git/session sync (periodic)
    - Event emission (for future AI agents)
    """

    def __init__(
        self,
        config: DaemonConfig,
        state_file: Optional[Path] = None,
    ):
        """Initialize daemon with configuration.

        Args:
            config: Daemon configuration
            state_file: Path to state persistence file.
                        Defaults to ~/.context-os/daemon.state.json
        """
        self.config = config
        self._running = False
        self._event_handlers: Dict[str, List[EventHandler]] = {}
        self._scheduler_thread: Optional[threading.Thread] = None
        self._watcher = None

        # State persistence
        if state_file is None:
            config_dir = ensure_config_dir()
            state_file = config_dir / "daemon.state.json"
        self._state_file = state_file

        # Load existing state or create fresh
        self.state = DaemonState.load(state_file)

    def start(self) -> None:
        """Start daemon: file watcher + scheduler.

        Idempotent - can be called multiple times.
        """
        if self._running:
            logger.debug("Daemon already running, ignoring start()")
            return

        self._running = True
        self.state.started_at = datetime.now()

        # Setup scheduler for periodic sync
        interval_minutes = self.config["sync"]["interval_minutes"]
        schedule.every(interval_minutes).minutes.do(self.run_sync)

        # Start scheduler thread
        self._scheduler_thread = threading.Thread(
            target=self._run_scheduler,
            daemon=True,
            name="DaemonScheduler",
        )
        self._scheduler_thread.start()

        # Start file watcher if enabled
        if self.config["watch"]["enabled"]:
            self._start_file_watcher()

        logger.info(
            f"Daemon started (sync interval: {interval_minutes}min, "
            f"watch: {self.config['watch']['enabled']})"
        )

    def stop(self) -> None:
        """Graceful shutdown."""
        if not self._running:
            return

        self._running = False

        # Stop file watcher
        if self._watcher:
            self._stop_file_watcher()

        # Clear scheduled jobs
        schedule.clear()

        # Wait for scheduler thread to finish
        if self._scheduler_thread and self._scheduler_thread.is_alive():
            self._scheduler_thread.join(timeout=2.0)

        # Save final state
        self.state.save(self._state_file)

        logger.info("Daemon stopped")

    def run_sync(self) -> None:
        """Run git sync + session parse + chain building.

        Called by scheduler at configured interval.
        Also can be called manually for immediate sync.
        """
        logger.debug("Running sync...")

        git_commits = self._sync_git()
        sessions = self._sync_sessions()
        chains = self._build_chains()

        # Update state
        self.state.last_git_sync = datetime.now()
        self.state.last_session_parse = datetime.now()
        self.state.last_chain_build = datetime.now()
        self.state.git_commits_synced += git_commits
        self.state.sessions_parsed += sessions
        self.state.chains_built += chains

        # Persist state
        self.state.save(self._state_file)

        # Emit event for handlers (future AI agents)
        self.emit("sync_complete", {
            "git_commits": git_commits,
            "sessions": sessions,
            "chains": chains,
            "timestamp": datetime.now().isoformat(),
        })

        logger.info(f"Sync complete: {git_commits} commits, {sessions} sessions, {chains} chains")

    def on(self, event: str, handler: EventHandler) -> None:
        """Register event handler.

        Args:
            event: Event name to listen for
            handler: Callback function(event_name, data_dict)
        """
        if event not in self._event_handlers:
            self._event_handlers[event] = []
        self._event_handlers[event].append(handler)

    def emit(self, event: str, data: dict) -> None:
        """Emit event to registered handlers.

        Args:
            event: Event name
            data: Event data dictionary
        """
        handlers = self._event_handlers.get(event, [])
        for handler in handlers:
            try:
                handler(event, data)
            except Exception as e:
                logger.error(f"Error in event handler for {event}: {e}")

    def _run_scheduler(self) -> None:
        """Background thread running scheduler."""
        while self._running:
            schedule.run_pending()
            time.sleep(1)  # Check every second

    def _start_file_watcher(self) -> None:
        """Start file watcher for continuous event capture."""
        # Import here to avoid circular imports
        try:
            from context_os_events.capture.file_watcher import start_watcher
            from context_os_events.db.connection import get_connection

            watch_paths = self.config["watch"]["paths"]
            debounce_ms = self.config["watch"]["debounce_ms"]

            # For now, watch first path
            # TODO: Support multiple watch paths
            watch_path = watch_paths[0] if watch_paths else "."

            db = get_connection()
            self._watcher = start_watcher(
                watch_path=watch_path,
                db=db,
            )
            logger.info(f"File watcher started on {watch_path}")
        except Exception as e:
            logger.error(f"Failed to start file watcher: {e}")
            self._watcher = None

    def _stop_file_watcher(self) -> None:
        """Stop file watcher."""
        if self._watcher:
            try:
                from context_os_events.capture.file_watcher import stop_watcher
                stats = stop_watcher(self._watcher)
                self.state.file_events_captured += stats.get("events_captured", 0)
                logger.info(f"File watcher stopped: {stats}")
            except Exception as e:
                logger.error(f"Error stopping file watcher: {e}")
            self._watcher = None

    def _sync_git(self) -> int:
        """Sync git commits. Returns number of commits synced."""
        try:
            from context_os_events.capture.git_sync import sync_commits
            from context_os_events.db.connection import get_connection

            db = get_connection()
            days = self.config["sync"]["git_since_days"]

            # sync_commits takes (db, options: SyncOptions)
            result = sync_commits(db, {"since": f"{days} days", "incremental": True})
            db.close()
            return result.get("commits_synced", 0)
        except Exception as e:
            logger.error(f"Git sync failed: {e}")
            return 0

    def _sync_sessions(self) -> int:
        """Sync Claude sessions. Returns number of sessions synced."""
        try:
            from context_os_events.capture.jsonl_parser import sync_sessions
            from context_os_events.db.connection import get_connection
            import os

            db = get_connection()

            # Get project path from config or use cwd
            project_path = self.config.get("project", {}).get("path") or os.getcwd()

            # sync_sessions takes (db, options: ParseOptions)
            result = sync_sessions(db, {"project_path": project_path, "incremental": True})
            db.close()
            return result.get("sessions_parsed", 0)
        except Exception as e:
            logger.error(f"Session sync failed: {e}")
            return 0

    def _build_chains(self) -> int:
        """Build chain graph from session data. Returns number of chains built."""
        try:
            from context_os_events.index.chain_graph import build_chain_graph, persist_chains
            from context_os_events.db.connection import get_connection
            from pathlib import Path
            import os

            db = get_connection()

            # Get project path from config or use cwd
            project_path = self.config.get("project", {}).get("path") or os.getcwd()

            # Find Claude project directory
            claude_projects_dir = Path.home() / ".claude" / "projects"
            # Convert path to Claude's format (replace special chars with dashes)
            project_key = project_path.replace(":", "-").replace("/", "-").replace("\\", "-")
            jsonl_dir = claude_projects_dir / project_key

            if not jsonl_dir.exists():
                logger.warning(f"No Claude project directory found at {jsonl_dir}")
                db.close()
                return 0

            # Build and persist chains
            chains = build_chain_graph(jsonl_dir)
            stats = persist_chains(db, chains)
            db.close()

            return stats.get("chains_stored", 0)
        except Exception as e:
            logger.error(f"Chain build failed: {e}")
            return 0


def main():
    """Main entry point for running daemon as a service."""
    import signal
    import sys

    from context_os_events.daemon.config import load_config

    # Setup logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    )

    logger.info("Context OS Events Daemon starting...")

    # Load config
    config = load_config()
    daemon = ContextOSDaemon(config)

    # Handle shutdown signals
    def shutdown_handler(signum, frame):
        logger.info(f"Received signal {signum}, shutting down...")
        daemon.stop()
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    # Start daemon
    daemon.start()

    # Run initial sync
    daemon.run_sync()

    # Keep running until stopped
    logger.info("Daemon running. Press Ctrl+C to stop.")
    try:
        while daemon._running:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("Keyboard interrupt received")
        daemon.stop()


if __name__ == "__main__":
    main()
