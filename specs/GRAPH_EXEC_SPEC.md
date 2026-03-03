# `tastematter graph-exec` — Specification

**Status:** Implementation-ready
**Date:** 2026-03-03
**Source:** Context Package 03, Boa spike validation, epistemic grounding session

## Overview

Add a `graph-exec` subcommand to tastematter that executes LLM-generated JavaScript
against an in-memory knowledge graph and optional temporal data, using the Boa JS engine
(pure Rust, no Node/bun dependency).

```
tastematter graph-exec --graph <path> '<js-code>'
tastematter graph-exec --graph <path> --db <db-path> '<js-code>'
```

## Architecture

```
tastematter graph-exec --graph 04_knowledge_base '<code>'
    │
    All Rust, single binary
    ├── 1. Load graph (walkdir + serde_yaml + regex)     — new: graph/mod.rs
    ├── 2. Load temporal data (SQLite, optional)          — existing: query.rs
    ├── 3. Build Boa JS context                           — new: graph/executor.rs
    │   ├── codemode.graph_search(input)                  — Rust fn → JS
    │   ├── codemode.graph_traverse(input)                — Rust fn → JS
    │   ├── codemode.graph_read(input)                    — Rust fn → JS
    │   └── codemode.graph_query(input)                   — Rust fn → JS
    ├── 4. Execute JS code                                — context.eval()
    └── 5. Print JSON result to stdout
```

## Phase Breakdown

### Phase 1: Graph Loader (`src/graph/mod.rs`) — ~150 lines

Port `apps/codemode-graph/src/graph-loader.ts` to Rust.

**Data structures (mirroring TypeScript types.ts):**

```rust
pub struct CorpusFile {
    pub path: String,
    pub content: String,
    pub frontmatter: serde_yaml::Value,
}

pub struct LinkEntry {
    pub outbound: Vec<String>,
    pub inbound: Vec<String>,
}

pub struct GraphIndex {
    pub links: HashMap<String, LinkEntry>,
    pub tree: HashMap<String, Vec<String>>,
    pub tags: HashMap<String, Vec<String>>,
    pub domains: HashMap<String, Vec<String>>,
    pub statuses: HashMap<String, Vec<String>>,
}

pub struct CorpusSnapshot {
    pub root: String,
    pub files: HashMap<String, CorpusFile>,
    pub file_count: usize,
    pub loaded_at: String,
    pub index: GraphIndex,
}
```

**Logic (from graph-loader.ts:41-130):**
1. `walkdir` to find all `.md` files, skip `node_modules/.git/.claude/dist/.next/.svelte-kit`
2. For each file: read contents, split YAML frontmatter via `serde_yaml`, extract `[[wiki-links]]` via regex
3. Build indexes: links (outbound/inbound), tree (directory→files), tags, domains, statuses
4. Node name = filename stem (e.g., `technical/context-engineering.md` → `context-engineering`)

**Success criteria:** Loads test fixture (5 files) correctly. Loads real graph (183 files) in <1s.

### Phase 2: Graph Tools as Boa Functions (`src/graph/executor.rs`) — ~200 lines

Register 4 graph tools as synchronous JS functions in Boa context.

**Functions (matching graph-tools.ts:92-288 signatures):**

| Function | Input | Output |
|----------|-------|--------|
| `codemode.graph_search({pattern, scope?, maxResults?})` | Regex search content/frontmatter | `[{path, matches: [{line, content}], score, frontmatter}]` |
| `codemode.graph_traverse({start, direction?, maxDepth?, filter?})` | BFS from node | `{nodes: [{path, name, depth, frontmatter}], edges: [{from, to}]}` |
| `codemode.graph_read({path, section?, maxLines?})` | Read single file | `{path, content, frontmatter, outbound_links, inbound_links}` |
| `codemode.graph_query({filter, sort?, limit?})` | Filter by metadata | `{nodes: [{path, name, status, domain, tags, last_updated, link_count}], total}` |

**Key patterns (from boa-spike main.rs):**
- `NativeFunction::from_copy_closure` for each function
- `JsObject::default()` for the `codemode` namespace
- `ctx.register_global_property()` to expose globally
- Extract JS input objects via `args.get_or_undefined(0).as_object()` + `.get()`
- Return results as nested `JsObject`s

**Key constraint:** All functions are synchronous. Graph data is in-memory. No async needed.

### Phase 3: CLI Subcommand (`src/main.rs`) — ~30 lines

Add `GraphExec` variant to `Commands` enum:

```rust
/// Execute JavaScript against a knowledge graph
GraphExec {
    /// Path to knowledge graph directory
    #[arg(long)]
    graph: String,

    /// JavaScript code to execute
    code: String,
},
```

Handler:
1. Load graph from `--graph` path
2. Build Boa context with `codemode.*` functions
3. `context.eval(code)`
4. Print result as JSON to stdout

### Phase 4 (future): Temporal Functions — deferred

Register `tm.*` functions for temporal queries. **Deferred because:**
- The `codemode.*` functions alone are immediately valuable
- Temporal functions need async→sync bridge (pre-query pattern)
- Ship graph-exec with graph tools first, add temporal later

## Dependencies

**New:** `boa_engine = "0.20"` (only new crate)

**Existing (already in Cargo.toml):**
- `walkdir = "2.4"` (line 31)
- `serde_yaml = "0.9"` (line 33)
- `serde_json = "1.0"` (line 19)
- `chrono = "0.4"` (line 21) — for `loaded_at` timestamp

**Regex:** Need `regex` crate for `[[wiki-link]]` extraction. Check if already a dep.

## Test Plan (TDD — RED then GREEN)

### Phase 1 Tests: Graph Loader

```
test_load_empty_directory           — 0 files, empty snapshot
test_load_single_file               — 1 .md with frontmatter, correct parsing
test_load_skips_non_md              — .txt files ignored
test_load_skips_dotdirs             — .git, .claude directories skipped
test_extract_wiki_links             — "[[foo]] and [[bar]]" → ["foo", "bar"]
test_extract_wiki_links_dedup       — "[[foo]] text [[foo]]" → ["foo"]
test_build_outbound_links           — file with [[links]] → correct outbound
test_build_inbound_links            — target of [[link]] gets inbound entry
test_build_tree_index               — directory structure indexed
test_build_tag_index                — frontmatter tags indexed
test_build_domain_index             — frontmatter domain indexed
test_build_status_index             — frontmatter status indexed
test_node_name_from_path            — "technical/context-engineering.md" → "context-engineering"
test_malformed_frontmatter          — bad YAML → empty frontmatter, content preserved
test_fixture_graph                  — load test-graph fixture, verify counts match TS (5 files)
```

### Phase 2 Tests: Boa Executor

```
test_graph_search_basic             — search "context" returns matches
test_graph_search_frontmatter       — scope: frontmatter searches YAML
test_graph_search_max_results       — respects maxResults limit
test_graph_traverse_outbound        — BFS follows outbound links
test_graph_traverse_inbound         — BFS follows inbound links
test_graph_traverse_depth_limit     — respects maxDepth
test_graph_traverse_filter          — filter by domain/status/tags
test_graph_read_basic               — reads file content + links
test_graph_read_section             — extracts named section
test_graph_read_not_found           — missing path throws JS error
test_graph_query_by_domain          — filters by domain
test_graph_query_by_status          — filters by status
test_graph_query_by_tags            — filters by tags (all required)
test_graph_query_sort               — sort by name/status
test_graph_query_limit              — respects limit
test_composition                    — search → read → process in single JS block
test_error_handling                 — bad JS syntax → error result, no crash
test_real_llm_pattern               — IIFE pattern from boa-spike test 6
```

### Phase 3 Tests: CLI Integration

```
test_cli_graph_exec_basic           — runs command, gets JSON output
test_cli_graph_exec_missing_graph   — error message for bad path
test_cli_graph_exec_bad_js          — error result for syntax error
```

## File Plan

| File | Purpose | Lines (est) | Phase |
|------|---------|-------------|-------|
| `src/graph/mod.rs` | Graph loader: walk, parse, index | ~150 | 1 |
| `src/graph/executor.rs` | Boa sandbox: register tools, eval | ~200 | 2 |
| `src/main.rs` | Add `GraphExec` variant + handler | ~30 | 3 |
| `src/lib.rs` | Add `pub mod graph;` | 1 | 1 |
| `Cargo.toml` | Add `boa_engine = "0.20"` | 1 | 2 |
| Test fixtures | Copy from codemode-graph or create minimal | ~20 | 1 |

## Anti-Patterns (DO NOT)

- **Do NOT hardcode specific queries as Rust algorithms** — preserves Code Mode composability
- **Do NOT use async in Boa** — all data is in-memory, sync functions are correct
- **Do NOT parse imports for code files** — behavioral co-access is sufficient for v1
- **Do NOT add temporal functions yet** — ship graph tools first, add `tm.*` in Phase 4
- **Do NOT mirror TypeScript test structure** — write idiomatic Rust tests

## Reference Files

| File | Purpose |
|------|---------|
| `apps/codemode-graph/src/types.ts` | Type definitions to mirror |
| `apps/codemode-graph/src/graph-loader.ts` | Loading logic to port |
| `apps/codemode-graph/src/graph-tools.ts` | Function signatures to match |
| `apps/boa-spike/src/main.rs` | Boa API patterns that work |
| `apps/codemode-graph/test/fixtures/test-graph/` | Test data |
