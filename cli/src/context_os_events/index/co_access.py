"""Co-access matrix for Context OS Intelligence.

Tracks files frequently accessed together ("game trails").
Enables: "You're working on X, you probably need Y"

Algorithm:
1. Build session sets per file from inverted index
2. Compute PMI (Pointwise Mutual Information): log(P(A,B) / (P(A) * P(B)))
3. Filter by minimum co-occurrence count (default 3) to avoid noise
4. PMI > 0 means files co-occur more than random chance

Why PMI over Jaccard:
- Jaccard produces 1.0 for any pair touched in same single session (noise)
- PMI measures whether co-occurrence is SURPRISING given baseline popularity
- Spotify, Google, Netflix use PMI variants for recommendations
"""

import logging
import math
from dataclasses import dataclass
from typing import Dict, List, Optional, Set, Tuple

from .inverted_index import FileAccess

logger = logging.getLogger(__name__)


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class CoAccessEntry:
    """Entry representing co-access relationship between two files.

    Tracks how often two files are accessed in the same sessions,
    measured by PMI (Pointwise Mutual Information).
    """
    file_a: str
    file_b: str
    pmi_score: float              # log(P(A,B) / (P(A) * P(B)))
    co_occurrence_count: int      # Sessions that touched both
    total_sessions: int           # Total sessions in corpus


# ============================================================================
# Helper Functions
# ============================================================================

def _build_session_sets(
    inverted_index: Dict[str, List[FileAccess]],
    min_sessions: int = 2
) -> Dict[str, Set[str]]:
    """Extract unique session IDs per file from inverted index.

    Args:
        inverted_index: Dict mapping file_path -> List[FileAccess]
        min_sessions: Minimum number of sessions to include a file

    Returns:
        Dict mapping file_path -> Set[session_id]
    """
    session_sets: Dict[str, Set[str]] = {}

    for file_path, accesses in inverted_index.items():
        sessions = {access.session_id for access in accesses}
        if len(sessions) >= min_sessions:
            session_sets[file_path] = sessions

    return session_sets


def _compute_jaccard(sessions_a: Set[str], sessions_b: Set[str]) -> float:
    """Compute Jaccard similarity between two session sets.

    Jaccard = |A ∩ B| / |A ∪ B|

    Note: Kept for backwards compatibility. Prefer _compute_pmi() for
    production use as it handles sample size issues better.

    Args:
        sessions_a: Session IDs for file A
        sessions_b: Session IDs for file B

    Returns:
        Jaccard similarity score (0.0 to 1.0)
    """
    if not sessions_a or not sessions_b:
        return 0.0

    intersection = len(sessions_a & sessions_b)
    union = len(sessions_a | sessions_b)

    if union == 0:
        return 0.0

    return intersection / union


def _compute_pmi(
    sessions_a: Set[str],
    sessions_b: Set[str],
    total_sessions: int,
    min_co_occurrence: int = 3
) -> Optional[float]:
    """Compute PMI (Pointwise Mutual Information) between two session sets.

    PMI = log(P(A,B) / (P(A) * P(B)))

    Measures how much MORE likely A and B co-occur than random chance.
    - PMI > 0: Co-occur more than expected (signal)
    - PMI = 0: Independent (no relationship)
    - PMI < 0: Co-occur less than expected (anti-correlated)

    Args:
        sessions_a: Session IDs for file A
        sessions_b: Session IDs for file B
        total_sessions: Total number of sessions in corpus
        min_co_occurrence: Minimum co-occurrences to compute (filters noise)

    Returns:
        PMI score or None if below min_co_occurrence threshold
    """
    if not sessions_a or not sessions_b or total_sessions == 0:
        return None

    intersection = sessions_a & sessions_b
    co_occurrence_count = len(intersection)

    # Filter out low sample sizes (this is the key noise reduction)
    if co_occurrence_count < min_co_occurrence:
        return None

    # Compute probabilities
    p_a = len(sessions_a) / total_sessions
    p_b = len(sessions_b) / total_sessions
    p_both = co_occurrence_count / total_sessions

    # Avoid log(0) - shouldn't happen given min_co_occurrence check
    if p_both == 0 or p_a == 0 or p_b == 0:
        return None

    # PMI formula
    return math.log(p_both / (p_a * p_b))


# ============================================================================
# Matrix Building
# ============================================================================

def build_co_access_matrix(
    inverted_index: Dict[str, List[FileAccess]],
    min_sessions: int = 2,
    min_co_occurrence: int = 3,
    use_pmi: bool = True
) -> Dict[str, List[Tuple[str, float]]]:
    """Build co-access matrix from inverted file index.

    Computes PMI (Pointwise Mutual Information) between all file pairs.
    PMI measures whether co-occurrence is surprising given baseline popularity.

    Args:
        inverted_index: Dict mapping file_path -> List[FileAccess]
        min_sessions: Minimum sessions to include a file (default 2)
        min_co_occurrence: Minimum co-occurrences to include pair (default 3)
        use_pmi: If True, use PMI scoring. If False, use legacy Jaccard.

    Returns:
        Dict mapping file_path -> List[(co_accessed_file, score)]
        Sorted by score descending
    """
    if not inverted_index:
        return {}

    # Build session sets per file
    session_sets = _build_session_sets(inverted_index, min_sessions)

    if len(session_sets) < 2:
        return {}

    # Count total unique sessions for PMI normalization
    all_sessions: Set[str] = set()
    for sessions in session_sets.values():
        all_sessions.update(sessions)
    total_sessions = len(all_sessions)

    # Compute pairwise scores
    co_access: Dict[str, List[Tuple[str, float]]] = {}
    files = list(session_sets.keys())

    for i, file_a in enumerate(files):
        sessions_a = session_sets[file_a]

        for j, file_b in enumerate(files[i + 1:], start=i + 1):
            sessions_b = session_sets[file_b]

            if use_pmi:
                score = _compute_pmi(
                    sessions_a, sessions_b, total_sessions, min_co_occurrence
                )
                # PMI returns None if below threshold
                if score is None:
                    continue
            else:
                # Legacy Jaccard mode
                score = _compute_jaccard(sessions_a, sessions_b)
                if score < 0.3:  # Old threshold
                    continue

            # Add both directions
            co_access.setdefault(file_a, []).append((file_b, score))
            co_access.setdefault(file_b, []).append((file_a, score))

    # Sort each file's list by score descending
    for file_path in co_access:
        co_access[file_path].sort(key=lambda x: x[1], reverse=True)

    return co_access


# ============================================================================
# Query Functions
# ============================================================================

def get_co_accessed(
    matrix: Dict[str, List[Tuple[str, float]]],
    file_path: str,
    limit: int = 5
) -> List[Tuple[str, float]]:
    """Get top N co-accessed files for a given file.

    Args:
        matrix: Co-access matrix from build_co_access_matrix()
        file_path: File to query
        limit: Maximum number of results

    Returns:
        List of (co_accessed_file, jaccard_score) tuples
    """
    if not matrix or file_path not in matrix:
        return []

    return matrix[file_path][:limit]


# ============================================================================
# Database Persistence
# ============================================================================

def persist_co_access(
    db,
    matrix: Dict[str, List[Tuple[str, float]]]
) -> Dict[str, int]:
    """Persist co-access matrix to database.

    Writes to co_access table.

    Args:
        db: SQLite connection
        matrix: Co-access matrix from build_co_access_matrix()

    Returns:
        Stats dict: {"pairs_stored": N}
    """
    pairs_stored = 0
    seen_pairs: Set[Tuple[str, str]] = set()

    for file_a, co_accessed_list in matrix.items():
        for file_b, jaccard_score in co_accessed_list:
            # Normalize pair to avoid duplicates (a,b) and (b,a)
            pair_key = (min(file_a, file_b), max(file_a, file_b))
            if pair_key in seen_pairs:
                continue
            seen_pairs.add(pair_key)

            db.execute("""
                INSERT OR REPLACE INTO co_access
                (file_a, file_b, jaccard_score, co_occurrence_count, total_sessions)
                VALUES (?, ?, ?, ?, ?)
            """, (
                pair_key[0],
                pair_key[1],
                jaccard_score,
                None,  # Could compute if needed
                None,  # Could compute if needed
            ))
            pairs_stored += 1

    db.commit()

    return {"pairs_stored": pairs_stored}


def load_co_access(db) -> Dict[str, List[Tuple[str, float]]]:
    """Load co-access matrix from database.

    Args:
        db: SQLite connection

    Returns:
        Co-access matrix: Dict[file_path -> List[(co_accessed_file, score)]]
    """
    cursor = db.execute("""
        SELECT file_a, file_b, jaccard_score
        FROM co_access
        ORDER BY jaccard_score DESC
    """)

    matrix: Dict[str, List[Tuple[str, float]]] = {}

    for row in cursor.fetchall():
        file_a = row[0]
        file_b = row[1]
        jaccard_score = row[2]

        # Add both directions
        matrix.setdefault(file_a, []).append((file_b, jaccard_score))
        matrix.setdefault(file_b, []).append((file_a, jaccard_score))

    # Sort each file's list by score descending
    for file_path in matrix:
        matrix[file_path].sort(key=lambda x: x[1], reverse=True)

    return matrix
