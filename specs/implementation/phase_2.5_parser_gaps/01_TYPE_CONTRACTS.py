"""Type Contracts for Parser Gap Fix (Phase 2.5)

These contracts define the extraction rules for the 2 new data sources.
The existing FileAccess dataclass handles all cases - no new types needed.

Reference: specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md
"""

from typing import TypedDict, Optional, Dict, List, Literal


# =============================================================================
# Existing Type (from inverted_index.py) - NO CHANGES NEEDED
# =============================================================================

# @dataclass
# class FileAccess:
#     session_id: str
#     chain_id: Optional[str]
#     file_path: str
#     access_type: str  # 'read', 'write', 'create', 'mention'
#     tool_name: str    # Now includes: 'toolUseResult', 'file-history-snapshot'
#     timestamp: datetime
#     access_count: int = 1


# =============================================================================
# Gap 1: toolUseResult (Canonical Spec lines 256-267)
# =============================================================================

class ToolUseResultFile(TypedDict, total=False):
    """Nested file object in toolUseResult."""
    filePath: str
    content: str
    numLines: int
    startLine: int
    totalLines: int


class ToolUseResult(TypedDict, total=False):
    """toolUseResult field in user records.

    Present when Claude's tool operation returned output to the user.
    Contains the confirmed file path that was operated on.
    """
    type: Literal["create", "text", "update"]  # Operation type
    filePath: str                               # Primary file path
    content: str                                # File content (may be truncated)
    file: ToolUseResultFile                     # Alternative location for filePath


class UserRecordWithToolUseResult(TypedDict, total=False):
    """User record structure when toolUseResult is present."""
    type: Literal["user"]
    uuid: str
    timestamp: str
    sessionId: str
    parentUuid: str
    message: Dict  # Contains role, content
    toolUseResult: ToolUseResult  # THE NEW FIELD TO EXTRACT


# Extraction Rule:
# 1. Check for toolUseResult.filePath first
# 2. If not present, check toolUseResult.file.filePath
# 3. Classify access_type based on toolUseResult.type:
#    - "create" → access_type = "create"
#    - "update" → access_type = "write"
#    - "text"   → access_type = "read"

TOOL_USE_RESULT_TYPE_TO_ACCESS_TYPE: Dict[str, str] = {
    "create": "create",
    "update": "write",
    "text": "read",
}


# =============================================================================
# Gap 2: file-history-snapshot (Canonical Spec lines 353-372)
# =============================================================================

class TrackedFileBackup(TypedDict, total=False):
    """Individual file backup entry."""
    backupFileName: Optional[str]  # May be null if no backup yet
    version: int
    backupTime: str


class FileHistorySnapshot(TypedDict, total=False):
    """Snapshot of tracked files at a point in time."""
    messageId: str
    trackedFileBackups: Dict[str, TrackedFileBackup]  # Keys are file paths!
    timestamp: str


class FileHistorySnapshotRecord(TypedDict):
    """file-history-snapshot record structure.

    Contains all files Claude is currently tracking/versioning.
    The KEYS of trackedFileBackups are the file paths to extract.
    """
    type: Literal["file-history-snapshot"]
    messageId: str
    snapshot: FileHistorySnapshot
    isSnapshotUpdate: bool


# Extraction Rule:
# 1. Get all keys from snapshot.trackedFileBackups
# 2. Each key is a file path
# 3. Classify all as access_type = "read" (tracking = reading)
# 4. tool_name = "file-history-snapshot"


# =============================================================================
# Extraction Function Contracts
# =============================================================================

def extract_tool_use_result_path(record: Dict) -> Optional[str]:
    """Extract file path from user record's toolUseResult.

    Args:
        record: JSONL record with type="user"

    Returns:
        File path string or None if not present

    Extraction priority:
        1. toolUseResult.filePath (direct)
        2. toolUseResult.file.filePath (nested)
    """
    ...


def classify_tool_use_result_access(record: Dict) -> str:
    """Classify access type from toolUseResult.type.

    Args:
        record: JSONL record with toolUseResult

    Returns:
        "create", "write", or "read"

    Mapping:
        "create" → "create"
        "update" → "write"
        "text"   → "read"
        unknown  → "read" (safe default)
    """
    ...


def extract_file_history_paths(record: Dict) -> List[str]:
    """Extract file paths from file-history-snapshot record.

    Args:
        record: JSONL record with type="file-history-snapshot"

    Returns:
        List of file path strings (keys of trackedFileBackups)
    """
    ...


# =============================================================================
# Test Fixtures (for TDD)
# =============================================================================

FIXTURE_USER_WITH_TOOL_USE_RESULT = {
    "type": "user",
    "uuid": "test-uuid-1",
    "timestamp": "2026-01-16T10:00:00.000Z",
    "sessionId": "test-session",
    "toolUseResult": {
        "type": "create",
        "filePath": "/path/to/created/file.md",
        "content": "# New File Content"
    }
}

FIXTURE_USER_WITH_NESTED_FILE_PATH = {
    "type": "user",
    "uuid": "test-uuid-2",
    "timestamp": "2026-01-16T10:01:00.000Z",
    "sessionId": "test-session",
    "toolUseResult": {
        "type": "update",
        "file": {
            "filePath": "/path/to/updated/file.py",
            "content": "# Updated content"
        }
    }
}

FIXTURE_FILE_HISTORY_SNAPSHOT = {
    "type": "file-history-snapshot",
    "messageId": "test-msg-id",
    "snapshot": {
        "messageId": "test-msg-id",
        "trackedFileBackups": {
            "/path/to/tracked/file1.py": {
                "backupFileName": "abc123@v3",
                "version": 3,
                "backupTime": "2026-01-16T09:00:00.000Z"
            },
            "/path/to/tracked/file2.md": {
                "backupFileName": None,
                "version": 1
            },
            "C:\\Windows\\path\\file3.ts": {
                "backupFileName": "def456@v2",
                "version": 2
            }
        },
        "timestamp": "2026-01-16T09:00:00.000Z"
    },
    "isSnapshotUpdate": False
}

FIXTURE_USER_WITHOUT_TOOL_USE_RESULT = {
    "type": "user",
    "uuid": "test-uuid-3",
    "timestamp": "2026-01-16T10:02:00.000Z",
    "sessionId": "test-session",
    "message": {
        "role": "user",
        "content": "Just a regular message with no tool result"
    }
}
