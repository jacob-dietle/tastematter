---
name: context-query
description: >
  This skill should be used when users ask about their work context,
  what they're working on, recent activity, or file relationships.
  Uses strategic search patterns with the context-os CLI hypercube query system.
  Every claim MUST cite a receipt ID for user verification.
---

# Context Query Skill

**Core Principle:** "Visibility is the cost of agentic leverage. Every claim must be verifiable."

## Purpose

Extract insights about work context using strategic search patterns.
Not just running queries - understanding WHAT to search for and HOW to interpret results.

---

## Quick Reference

**CLI command (registered globally):**
```bash
tastematter query <command> [options]
```

**Canonical database location:** `~/.context-os/context_os_events.db`

**Troubleshooting - "No such table" or empty results:**
```bash
# Verify database exists at canonical location
ls ~/.context-os/context_os_events.db

# Check tastematter can find it
tastematter --version

# If database is missing or empty, initialize:
tastematter parse-sessions --project "C:\Users\<you>\.claude\projects"
tastematter build-chains
tastematter index-files

# On Windows, use backslashes for paths:
tastematter parse-sessions --project "C:\Users\dietl\.claude\projects"
```

**Default query pattern (always use for synthesis):**
```bash
tastematter query flex --files "*pattern*" --agg count,recency,sessions,chains --format json
```

**Key commands:**
| Command | Purpose |
|---------|---------|
| `query flex` | Flexible hypercube slicing (primary) |
| `query co-access` | Find related files |
| `query session` | All files in a session |
| `query chains` | List conversation chains |
| `query verify` | Verify a receipt |

---

## Strategy Selection Guide

**Start here.** Match the question type to a search strategy:

| Question Type | Strategy | First Query |
|--------------|----------|-------------|
| "What am I working on?" | Pilot Drilling | `--time 30d --agg count,recency,sessions,chains --limit 50` |
| "What's related to X?" | Known Anchor | `query co-access <file>` then expand |
| "What happened on [day]?" | Temporal Bracketing | `--time 7d` then filter by date |
| "Find work on [partial keywords]" | Triangulation for Discovery | 2-3 parallel searches → find intersection |
| "What did I abandon?" | Negative Space | `--time 30d` sort by oldest last_access |
| "Is [claim] accurate?" | Triangulation for Validation | Multiple independent queries |
| "What's the full story of X?" | Chain Walking | `query chains` then `query session` |
| "What themes span my work?" | Cluster Discovery | `query co-access` from multiple seeds |
| "Where is my attention going?" | Recency Gradient | `--time 14d --sort recency` |

---

## Search Strategies

### 1. Pilot Drilling (Breadth → Depth)

**When to use:** Starting fresh, "what am I working on?", orientation

**Pattern:** Cast wide net → find hot spots → drill into clusters

**Steps:**
```bash
# Step 1: Broad sweep (find everything)
query flex --time 30d --agg count,recency,sessions,chains --limit 50 --format json

# Step 2: Identify clusters by path pattern (mental grouping)
# Look for: gtm_engagements/pixee/*, knowledge_base/*, apps/*

# Step 3: Drill into hot cluster
query flex --files "*pixee*" --agg count,recency,sessions,chains --limit 20 --format json

# Step 4: Deep dive into most active session
query session <session_id_from_step_3> --format json
```

**Insight pattern:** Work areas emerge from file path clusters. Most-accessed files = current focus.

---

### 2. Known Anchor Expansion

**When to use:** "What's related to this file?", understanding context around a known point

**Pattern:** Start from known file → expand via co-access → follow session trails

**Steps:**
```bash
# Step 1: Get file history
query file <known_file> --format json

# Step 2: Find co-accessed files (relationship ring)
query co-access <known_file> --limit 20 --format json

# Step 3: Pick most co-accessed file, get its session context
query session <session_from_step_1> --format json

# Step 4: (Optional) Expand to second-degree relationships
query co-access <surprising_file_from_step_2> --limit 10 --format json
```

**Insight pattern:** Files that always appear together = work unit. Surprising co-access = hidden dependency.

---

### 3. Temporal Bracketing

**When to use:** "What did I do Tuesday?", finding work in specific time window

**Pattern:** Start wide time window → narrow progressively → land on specific sessions

**Steps:**
```bash
# Step 1: Week view
query flex --time 7d --agg count,recency,sessions,chains --limit 30 --format json

# Step 2: Filter results by date (look at last_access timestamps)
# Find sessions from target day

# Step 3: Get full context for target session
query session <session_from_target_day> --format json

# Step 4: Reconstruct sequence (if multiple sessions that day)
# Sessions ordered by timestamp = work sequence
```

**Insight pattern:** Session boundaries = context switches. Multiple sessions same day = interrupted work.

---

### 4. Triangulation for Discovery

**When to use:** Ambiguous searches where you have partial keywords but no specific file anchor. "Find work on [partial topic]", "What was I doing with [vague term]?"

**Pattern:** Run 2-3 small parallel searches from different angles → cross-reference results → find overlapping files/sessions → use intersection to establish anchor → proceed with Known Anchor or Chain Walking

**Why it works:** Complex phrase searches like "hubspot middle of funnel" often fail because:
- File paths don't contain full phrases
- Work spans multiple naming conventions
- No single keyword captures the full scope

Breaking into smaller searches and finding the intersection reveals the actual work cluster.

**Steps:**
```bash
# Step 1: Run 2-3 small parallel searches from different angles
# Use different keywords that SHOULD overlap if the work exists

# Search A: By project/tool name
tastematter query search "hubspot" --limit 10 --format json

# Search B: By workflow/feature name
tastematter query search "pipedream" --limit 10 --format json

# Search C: By version/iteration marker
tastematter query search "V2" --limit 10 --format json

# Step 2: Cross-reference results
# Look for files/sessions appearing in MULTIPLE search results
# Example: A file appearing in both "hubspot" and "pipedream" results
# is more likely to be the target than one appearing in only one

# Step 3: Identify intersection candidates
# Files appearing in 2+ searches = strong anchor candidates
# Sessions appearing in 2+ searches = likely the work session

# Step 4: Establish anchor from intersection
# Pick the most-overlapping file as your anchor
tastematter query file <intersection_file> --format json
# Extract session_id and chain_id

# Step 5: Switch to Known Anchor or Chain Walking
tastematter query session <session_id> --format json
# Now you have full context of that work session

# Step 6: (Optional) Verify with co-access
tastematter query co-access <anchor_file> --limit 10 --format json
# Confirms the work cluster around your discovered anchor
```

**Insight patterns:**
- High overlap across searches (3/3 searches contain file) = strong anchor candidate
- Low overlap (1/3 searches) = weak signal, try different search terms
- Zero overlap = either work doesn't exist or search terms are wrong
- Session overlap more valuable than file overlap (sessions = coherent work units)

**Anti-pattern:** Don't run a single complex phrase search expecting direct hits. Break it down.

**Comparison to Triangulation for Validation:**
- **Discovery**: Start with NO anchor, use search intersection to FIND one
- **Validation**: Have a claim, use multiple queries to VERIFY it

---

### 5. Chain Walking

**When to use:** "What was that thread of work?", reconstructing narratives

**Pattern:** Find chain → get all files → follow forward/backward in time

**Steps:**
```bash
# Step 1: List recent chains
query chains --limit 20 --format json

# Step 2: Get files for interesting chain
query flex --chain <chain_id> --agg count,sessions --format json

# Step 3: Get session details for that chain
query session <session_in_chain> --format json

# Step 4: Look for continuation
# Check if same files appear in later chains (work continued)
```

**Insight pattern:**
- Long chains (many files) = major work sessions
- Short chains = quick tasks or interruptions
- Same files across chains = sustained project

---

### 6. Negative Space Search

**When to use:** "What did I abandon?", finding dropped threads

**Pattern:** Find old files → check if work completed or orphaned

**Steps:**
```bash
# Step 1: Get all files, sorted by oldest last_access
query flex --time 30d --agg count,recency --limit 50 --format json
# Look at files with oldest last_access dates

# Step 2: For old files, check their chain status
query file <old_file> --format json
# Get the chain ID

# Step 3: Check chain - did it continue or stop?
query flex --chain <chain_id> --agg recency --format json

# Step 4: Cross-reference - were these files replaced?
query flex --files "*similar_name*" --agg recency --format json
```

**Insight pattern:**
- Old file, completed chain = finished work
- Old file, orphan chain = abandoned work
- Old file, replaced by newer file = evolved work

---

### 7. Triangulation for Validation

**When to use:** Validating claims, high-confidence answers

**Pattern:** Multiple independent queries → cross-reference → confirm or refute

**Steps:**
```bash
# Query A: By file pattern
query flex --files "*feature*" --agg count,sessions --format json

# Query B: By time
query flex --time 7d --agg count,sessions --format json

# Query C: By chain
query chains --limit 20 --format json

# Cross-reference: Do the same files/sessions appear in all three?
# High overlap = strong signal
# Low overlap = weak signal, investigate further
```

**Insight pattern:** Claims supported by multiple independent queries = high confidence.

---

### 8. Cluster Discovery

**When to use:** Finding natural project boundaries, understanding work themes

**Pattern:** Seed files → co-access expansion → identify clusters → name them

**Steps:**
```bash
# Step 1: Pick a seed file (any active file)
query flex --time 14d --limit 1 --format json

# Step 2: Expand via co-access
query co-access <seed_file> --limit 20 --format json

# Step 3: These files = Cluster A
# Pick a file NOT in Cluster A

# Step 4: Repeat co-access for second seed
query co-access <file_not_in_cluster_a> --limit 20 --format json

# Step 5: These files = Cluster B
# Continue until major clusters identified
```

**Insight pattern:** Clusters = implicit projects. Files bridging clusters = shared utilities or integration points.

---

### 9. Recency Gradient

**When to use:** "Where is my attention going?", trend detection

**Pattern:** Sort by time → observe direction of attention

**Steps:**
```bash
# Step 1: Recent activity with recency
query flex --time 14d --agg count,recency --sort recency --limit 30 --format json

# Step 2: Compare oldest vs newest in results
# What was I doing 2 weeks ago?
# What am I doing now?

# Step 3: For shift detection, compare time windows
query flex --time 7d --agg count --format json    # This week
query flex --time 14d --agg count --format json   # Last 2 weeks
# Diff the results - what's new? What dropped off?
```

**Insight pattern:** Attention shifts visible in recency order. Files falling off = deprioritized. New files appearing = new initiatives.

---

## Result Interpretation

### Access/Session Pattern Signatures

| Pattern | Meaning | Example |
|---------|---------|---------|
| High access_count, low session_count | Reference doc, read repeatedly | CLAUDE.md, architecture guides |
| Low access_count, high session_count | Touched lightly across contexts | Config files, imports |
| High access_count, high session_count | Active development focus | Main implementation files |
| Single access, single session | One-off reference or mistake | Random exploration |

### Session Type Patterns

| Session Format | Meaning |
|----------------|---------|
| UUID (e.g., `e550ae12-ce7d-...`) | Human-initiated session |
| `agent-*` prefix (e.g., `agent-cb3e753f`) | Delegated agent work |

**Insight:** High ratio of agent sessions = heavy delegation. Human-only files = can't delegate that work.

### Chain Signatures

| Chain Pattern | Meaning |
|---------------|---------|
| 2-5 files | Focused task (bug fix, small feature) |
| 10-20 files | Medium feature or investigation |
| 20+ files | Major work session (refactor, new feature) |
| Single session | One-off task, might be abandoned |
| Multi-session | Continued conversation, sustained work |

### Temporal Patterns

| Pattern | Meaning |
|---------|---------|
| Files clustered by first_access | Created together = same initiative |
| Gap then activity | Work paused and resumed |
| Decreasing access over time | Winding down / completing |
| Increasing access over time | Ramping up / new priority |

---

## Multi-Query Workflows

### Workflow: "What am I working on for [Client]?"

```bash
# 1. Get all client files with full aggregations
query flex --files "*pixee*" --agg count,recency,sessions,chains --limit 20 --format json

# 2. From results, identify:
#    - Top 3 file clusters by path pattern
#    - Most recent files (check last_access)
#    - Most active sessions

# 3. For the most active session, get full context:
query session <most_active_session_id> --format json

# 4. For surprising files (unexpected in results), check relationships:
query co-access <surprising_file> --format json

# 5. Synthesize with citations:
#    "Working on 138 Pixee files [q_abc123]. Main areas:
#     - Social media CLI (45 files)
#     - HubSpot integration (23 files)
#    To verify: tastematter query verify q_abc123"
```

### Workflow: "What did I abandon?"

```bash
# 1. Get all files, focus on recency
query flex --time 30d --agg count,recency,sessions,chains --limit 50 --format json

# 2. Filter to files with last_access > 14 days ago (manual scan)

# 3. For each old file, check its chain:
query file <old_file> --format json
#    Get chain_id from result

# 4. Check if chain continued:
query flex --chain <chain_id> --agg recency --format json
#    If no recent files = abandoned
#    If recent files exist = work evolved

# 5. Synthesize:
#    "Found 3 potentially abandoned threads [q_xyz789]:
#     - auth-refactor (last touched Dec 1)
#     - newsletter-v2 (last touched Nov 28)
#    These chains show no continuation."
```

### Workflow: "Reconstruct what happened last week"

```bash
# 1. Get week's activity
query flex --time 7d --agg count,recency,sessions,chains --format json

# 2. Get chain overview
query chains --limit 20 --format json

# 3. For each major chain, get session details:
query session <session_1> --format json
query session <session_2> --format json
# ... repeat for active sessions

# 4. Order by timestamp to see narrative:
#    Monday: Started X (session abc)
#    Tuesday: Continued X, started Y (sessions def, ghi)
#    Wednesday: Deep on Y (session jkl)

# 5. Synthesize chronological narrative with citations
```

### Progressive Disclosure Pattern

```bash
# Level 1: Quick count (fast, low detail)
query flex --files "*pattern*" --agg count --format json

# Level 2: Add recency (when was this active?)
query flex --files "*pattern*" --agg count,recency --format json

# Level 3: Add sessions (how spread out?)
query flex --files "*pattern*" --agg count,recency,sessions --format json

# Level 4: Full detail (complete picture)
query flex --files "*pattern*" --agg count,recency,sessions,chains --format json
```

**Rule:** Start at Level 4 for synthesis. Only use lower levels for quick checks.

---

## Content Integration

The hypercube provides **metadata patterns** (what files, when, how often, with what).
For **content-grounded narrative**, you need to READ what's actually IN the files.

### When to Read File Contents

**Always read** when:
- File appears in top 5 by access count (hot files = current focus)
- File shows unusual pattern (high access, single session = key reference)
- User asks "what" or "why" questions (not just "where" or "when")
- Synthesizing narrative that requires understanding intent

**Skip reading** when:
- Just counting or listing files
- File is config/boilerplate (pyproject.toml, package.json)
- User only asked about patterns, not content

### Content Sampling Strategy

After pattern analysis, sample content from hot files:

```
1. Get top 5-10 files from query
   query flex --time 30d --agg count,recency,sessions,chains --limit 10 --format json

2. For each hot file, read strategically:
   - Markdown: Headers + first 50 lines (orientation)
   - Code: First 100 lines or main class/function definitions
   - Specs: Full read (they're context-dense)

3. Identify structure:
   - What's the purpose of this file?
   - What are the main abstractions?
   - What does it connect to?

4. Combine with pattern data:
   - Pattern says: "High access, single session"
   - Content says: "This is a spec document that defines..."
   - Synthesis: "Reference spec, accessed repeatedly for implementation"
```

### Grep Integration

When you need to find specific code patterns in hot files:

```bash
# Find function/class definitions in a hot file
Grep: pattern="def |class |function " path=<hot_file>

# Find imports to understand dependencies
Grep: pattern="import |from |require" path=<hot_file>

# Find TODO/FIXME for incomplete work
Grep: pattern="TODO|FIXME|HACK" path=<directory>

# Find specific concepts across files
Grep: pattern="receipt|verification" glob="*.py"
```

**Pattern:** Use Grep to find WHAT code does. Use CLI to find WHICH files matter.

### Git Integration

When understanding changes over time:

```bash
# Recent commits touching a hot file
git log --oneline -10 -- <file_path>

# What changed in a file
git diff HEAD~5 -- <file_path>

# Who worked on this (useful for agent vs human work)
git log --format="%an" -10 -- <file_path>

# Correlate with session timestamps
# If session started 2025-12-17, find commits from that day
git log --since="2025-12-17" --until="2025-12-18" --oneline
```

**Pattern:** Git shows WHAT changed. CLI shows WHEN it was accessed by Claude.

---

## Content-Grounded Workflow

Complete workflow combining patterns + content:

```
### Step 1: Pattern Layer (metadata)
query flex --time 30d --agg count,recency,sessions,chains --limit 10 --format json
→ Receipt: [q_abc123]
→ Identify: Top 5 files by access count

### Step 2: Content Sampling (understanding)
Read: <hot_file_1> (first 100 lines)
→ Purpose: Main implementation of X
→ Structure: 3 classes, 15 methods

Read: <hot_file_2> (full, it's a spec)
→ Purpose: Defines architecture for Y
→ Key decisions: Uses pattern Z

### Step 3: Code Understanding (specifics)
Grep: pattern="class " path=<hot_file_1>
→ Classes: QueryEngine, QuerySpec, QueryReceipt

Grep: pattern="def " path=<hot_file_1>
→ Methods: execute(), aggregate(), render()

### Step 4: Git Context (evolution)
git log --oneline -5 -- <hot_file_1>
→ Recent: "Add verification layer", "Fix receipt format"

### Step 5: Synthesize (content-grounded narrative)
Combine:
- Pattern: 45 accesses, 8 sessions, 3 chains [q_abc123]
- Content: Query engine with 3 core classes
- Evolution: Recently added verification layer
- Insight: This is the heart of the hypercube, under active development

Citation: All claims traceable to [q_abc123] + specific file reads
```

### Pattern-to-Content Questions

When does a pattern REQUIRE content to interpret?

| Pattern | Content Needed? | Why |
|---------|-----------------|-----|
| High access, low session | Yes | What makes this a reference doc? |
| Agent-heavy sessions | Yes | What was the agent building? |
| Abandoned chain | Yes | What was the work that stopped? |
| File cluster | Maybe | What's the shared purpose? |
| High recency spike | Maybe | What just changed? |
| Low access, multi-session | No | Just touched lightly, context unclear anyway |

---

## Index Understanding

### What's Captured

The hypercube index captures:

| Dimension | Source | Data |
|-----------|--------|------|
| FILES | Tool calls | Every file path from Read, Write, Edit, Glob |
| SESSIONS | Claude Code | Session UUID, start/end time |
| TIME | Timestamps | UTC timestamp per access |
| CHAINS | Conversation | leafUuid linking continued conversations |
| ACCESS_TYPE | Tool type | read (Read), write (Write/Edit), create (new files) |

### What's NOT Captured

The index does NOT contain:

| Missing | Implication |
|---------|-------------|
| File content | Can't search by what's IN files |
| Edit diffs | Can't see what changed |
| User intent | Can't know WHY a file was accessed |
| Conversation text | Can't search by what was discussed |
| External access | Files opened outside Claude Code are invisible |
| Deleted files | Path exists in index even if file is gone |

### Limitations & Gotchas

| Gotcha | Workaround |
|--------|------------|
| Long paths truncated in table output | Always use `--format json` |
| Receipts expire after 30 days | Re-run queries for old claims |
| Verification re-executes query | May take a few seconds |
| Agent sessions count separately | Include when analyzing work |
| Co-access is bidirectional | A co-accessed with B = B co-accessed with A |

---

## CLI Reference

### query flex (Primary Command)

```bash
query flex [OPTIONS]

OPTIONS:
  --files TEXT     File pattern (glob): "*pixee*", "*.py"
  --time TEXT      Time window: "7d", "2w", "30d"
  --chain TEXT     Chain ID or "active"
  --session TEXT   Session ID prefix
  --access TEXT    Access type: "r", "w", "c", "rw"
  --agg TEXT       Aggregations: count,recency,sessions,chains
  --format TEXT    Output: "json" (for agents) or "table" (for humans)
  --limit INT      Max results (default: 20)
  --sort TEXT      Order: "count", "recency", "alpha"
```

### query verify (Verification)

```bash
query verify <receipt_id> [OPTIONS]

OPTIONS:
  --verbose     Show detailed diff
  --format      Output format
```

**Statuses:** MATCH (unchanged), DRIFT (changed), NOT_FOUND (expired/invalid)

### query receipts (Audit Trail)

```bash
query receipts [OPTIONS]

OPTIONS:
  --limit INT     Number to show (default: 20)
  --format TEXT   Output format
```

### Legacy Commands

| Command | Purpose |
|---------|---------|
| `query search <term>` | Keyword search in file paths |
| `query file <path>` | History for specific file |
| `query session <id>` | All files in a session |
| `query chains` | List conversation chains |
| `query co-access <path>` | Files accessed with target |
| `query recent` | Recent activity summary |

All support `--format json`.

---

## Citation Requirements

**CRITICAL: Every claim based on query results MUST include a receipt ID.**

### Format

Include `[receipt_id]` immediately after the claim.

### Examples

```markdown
# Correct - verifiable:
Found 147 Pixee files [q_7f3a2b]
Active chains: 3 [q_8c4d1e]
Last week: 12 sessions [q_9d5e2f]

# Wrong - unverifiable:
Found 147 Pixee files
Active chains: 3
```

### Response Pattern

```markdown
Based on query results [q_7f3a2b]:

**Summary:** 147 Pixee files, 12 sessions, 4 chains [q_7f3a2b]

Top areas:
1. Social media CLI (45 files)
2. HubSpot integration (23 files)

To verify: `tastematter query verify q_7f3a2b`
```

---

## Verification Workflow

1. **Run query with `--format json`**
2. **Extract `receipt_id` from response**
3. **Include `[receipt_id]` in synthesis**
4. **Tell user how to verify**

### Handling Drift

If user reports DRIFT:
1. Acknowledge data changed
2. Re-run query
3. Update synthesis with new receipt
4. Note: "Previous [old_id] superseded by [new_id]"

---

## Success Criteria

### Healthy Responses

- Every claim cites `[receipt_id]`
- Strategy matches question type
- Multi-query workflows for complex questions
- Result patterns interpreted, not just reported
- User told how to verify

### Warning Signs

- Claims without receipt citations
- Single query for complex questions
- Raw data dumping without interpretation
- Using `--agg count` alone (missing context)
- Table format when JSON needed

---

**Last Updated:** 2026-01-13
**Version:** 2.2 (Renamed CLI from context-os to tastematter, added database troubleshooting)
