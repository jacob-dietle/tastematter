"""Comparison utilities for parity testing."""
from dataclasses import dataclass
from typing import Any, Dict, Set


@dataclass
class ParityResult:
    """Result of a parity comparison."""
    python_value: Any
    rust_value: Any
    matches: bool
    diff_pct: float = 0.0
    details: str = ""


def compare_counts(
    py_count: int,
    rs_count: int,
    tolerance_pct: float = 0.0,
    label: str = "count"
) -> ParityResult:
    """Compare two counts with optional tolerance.

    Args:
        py_count: Python implementation count
        rs_count: Rust implementation count
        tolerance_pct: Allowed percentage difference (0 = exact match)
        label: Description for the comparison

    Returns:
        ParityResult with match status and diff percentage
    """
    if py_count == 0 and rs_count == 0:
        return ParityResult(py_count, rs_count, True, 0.0)

    diff_pct = abs(py_count - rs_count) / max(py_count, rs_count, 1) * 100

    if tolerance_pct == 0:
        matches = py_count == rs_count
    else:
        matches = diff_pct <= tolerance_pct

    details = f"{label}: Python={py_count}, Rust={rs_count}, diff={diff_pct:.2f}%"
    return ParityResult(py_count, rs_count, matches, diff_pct, details)


def compare_sets(
    py_set: Set[str],
    rs_set: Set[str],
    label: str = "set"
) -> ParityResult:
    """Compare two sets for equality.

    Args:
        py_set: Python implementation set
        rs_set: Rust implementation set
        label: Description for the comparison

    Returns:
        ParityResult with match status
    """
    matches = py_set == rs_set

    if matches:
        details = f"{label}: Both have {len(py_set)} items"
    else:
        only_py = py_set - rs_set
        only_rs = rs_set - py_set
        details = f"{label}: Python has {len(only_py)} unique, Rust has {len(only_rs)} unique"
        if only_py:
            details += f"\n  Only in Python (first 5): {list(only_py)[:5]}"
        if only_rs:
            details += f"\n  Only in Rust (first 5): {list(only_rs)[:5]}"

    return ParityResult(len(py_set), len(rs_set), matches, details=details)


def compare_dicts(
    py_dict: Dict[str, int],
    rs_dict: Dict[str, int],
    label: str = "dict"
) -> ParityResult:
    """Compare two dictionaries with integer values.

    Args:
        py_dict: Python implementation dict
        rs_dict: Rust implementation dict
        label: Description for the comparison

    Returns:
        ParityResult with match status
    """
    # Get all keys
    all_keys = set(py_dict.keys()) | set(rs_dict.keys())

    mismatches = []
    for key in sorted(all_keys):
        py_val = py_dict.get(key, 0)
        rs_val = rs_dict.get(key, 0)
        if py_val != rs_val:
            mismatches.append(f"{key}: Python={py_val}, Rust={rs_val}")

    matches = len(mismatches) == 0

    if matches:
        details = f"{label}: All {len(all_keys)} keys match"
    else:
        details = f"{label}: {len(mismatches)} mismatches:\n  " + "\n  ".join(mismatches[:10])
        if len(mismatches) > 10:
            details += f"\n  ... and {len(mismatches) - 10} more"

    return ParityResult(py_dict, rs_dict, matches, details=details)


def assert_parity(result: ParityResult, msg: str = ""):
    """Assert that a parity result matches.

    Args:
        result: ParityResult to check
        msg: Optional message prefix

    Raises:
        AssertionError: If parity check fails
    """
    if not result.matches:
        error = f"{msg}\n{result.details}" if msg else result.details
        raise AssertionError(error)
