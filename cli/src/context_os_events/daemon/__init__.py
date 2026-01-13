"""Context OS Events Daemon module.

Provides background daemon functionality for continuous event capture:
- File watching (continuous)
- Git sync (periodic)
- Session parsing (periodic)
- Windows service integration via Servy
"""

from context_os_events.daemon.config import (
    DaemonConfig,
    get_default_config,
    load_config,
    validate_config,
    ensure_config_dir,
)

__all__ = [
    "DaemonConfig",
    "get_default_config",
    "load_config",
    "validate_config",
    "ensure_config_dir",
]
