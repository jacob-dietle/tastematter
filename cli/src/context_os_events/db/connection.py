"""Database connection and initialization for Context OS Events."""

import sqlite3
from pathlib import Path
from typing import Optional

# Default database location
DEFAULT_DB_PATH = Path(__file__).parent.parent.parent.parent / "data" / "context_os_events.db"


def get_schema_path() -> Path:
    """Get path to schema.sql file."""
    return Path(__file__).parent / "schema.sql"


def init_database(db_path: Optional[Path] = None) -> sqlite3.Connection:
    """Initialize database with schema.

    Creates database file and tables if they don't exist.

    Args:
        db_path: Path to database file. Uses default if not specified.

    Returns:
        SQLite connection with foreign keys enabled.
    """
    db_path = db_path or DEFAULT_DB_PATH

    # Ensure parent directory exists
    db_path.parent.mkdir(parents=True, exist_ok=True)

    # Connect
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row  # Enable dict-like access

    # Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON")

    # Execute schema
    schema_path = get_schema_path()
    with open(schema_path, 'r') as f:
        schema_sql = f.read()

    conn.executescript(schema_sql)
    conn.commit()

    return conn


def get_connection(db_path: Optional[Path] = None) -> sqlite3.Connection:
    """Get database connection.

    For existing database, just connects. For new database, initializes schema.

    Args:
        db_path: Path to database file. Uses default if not specified.

    Returns:
        SQLite connection.
    """
    db_path = db_path or DEFAULT_DB_PATH

    if not db_path.exists():
        return init_database(db_path)

    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")

    return conn


def get_database_path() -> Path:
    """Get the default database path."""
    return DEFAULT_DB_PATH
