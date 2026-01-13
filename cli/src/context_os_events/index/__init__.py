"""Index layer for Context OS Intelligence.

This module provides deterministic index structures built from raw data
(JSONL, git, file system) without LLM involvement.

Index Structures:
- chain_graph: Session chains from leafUuid linking
- inverted_index: File -> sessions mapping
- file_tree: Annotated directory hierarchy
- co_access: Files frequently accessed together (game trails)
- temporal: Weekly/daily buckets
- bloom: Fast membership checks
- context_index: Unified interface

Architecture:
    ┌─────────────────────────────────────────┐
    │  AGENT LAYER (Judgment)                 │
    │  • Workstream classification            │
    │  • Natural language queries             │
    └─────────────────────────────────────────┘
                        │
                        ▼
    ┌─────────────────────────────────────────┐
    │  INDEX LAYER (This Module)              │
    │  • Chain graph (leafUuid-based)         │
    │  • File tree index (annotated)          │
    │  • Co-access matrix (game trails)       │
    │  • Temporal buckets (weekly)            │
    │  • NO LLM - pure parsing                │
    └─────────────────────────────────────────┘
"""

from .chain_graph import (
    ChainNode,
    Chain,
    build_chain_graph,
    extract_leaf_uuids,
    extract_message_uuids,
    persist_chains,
    load_chains,
    get_chain_for_session,
    get_session_context,
    get_session_chain,
    get_session_parent,
    get_chain_depth,
)

from .inverted_index import (
    FileAccess,
    extract_file_accesses,
    build_inverted_index,
    get_sessions_for_file,
    get_files_for_session,
    persist_inverted_index,
    load_inverted_index,
)

from .file_tree import (
    FileTreeNode,
    build_file_tree,
    bubble_up_stats,
    get_node_by_path,
    get_children,
    get_directory_stats,
    normalize_path,
    persist_file_tree,
    load_file_tree,
)

from .co_access import (
    CoAccessEntry,
    build_co_access_matrix,
    get_co_accessed,
    persist_co_access,
    load_co_access,
)

from .bloom import (
    BloomFilter,
)

from .temporal import (
    TemporalBucket,
    build_temporal_buckets,
    get_week_bucket,
    get_buckets_in_range,
    file_touched_in_week,
    persist_temporal_buckets,
    load_temporal_buckets,
)

from .context_index import (
    ContextIndex,
    LoadedChain,
    LoadedChainNode,
)

__all__ = [
    # Chain Graph
    "ChainNode",
    "Chain",
    "build_chain_graph",
    "extract_leaf_uuids",
    "extract_message_uuids",
    "persist_chains",
    "load_chains",
    "get_chain_for_session",
    "get_session_context",
    "get_session_chain",
    "get_session_parent",
    "get_chain_depth",
    # Inverted Index
    "FileAccess",
    "extract_file_accesses",
    "build_inverted_index",
    "get_sessions_for_file",
    "get_files_for_session",
    "persist_inverted_index",
    "load_inverted_index",
    # File Tree
    "FileTreeNode",
    "build_file_tree",
    "bubble_up_stats",
    "get_node_by_path",
    "get_children",
    "get_directory_stats",
    "normalize_path",
    "persist_file_tree",
    "load_file_tree",
    # Co-Access Matrix
    "CoAccessEntry",
    "build_co_access_matrix",
    "get_co_accessed",
    "persist_co_access",
    "load_co_access",
    # Bloom Filter
    "BloomFilter",
    # Temporal Buckets
    "TemporalBucket",
    "build_temporal_buckets",
    "get_week_bucket",
    "get_buckets_in_range",
    "file_touched_in_week",
    "persist_temporal_buckets",
    "load_temporal_buckets",
    # Unified ContextIndex
    "ContextIndex",
    "LoadedChain",
    "LoadedChainNode",
]
