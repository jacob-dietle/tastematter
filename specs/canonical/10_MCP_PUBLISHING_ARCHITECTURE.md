---
title: "MCP Publishing Architecture"
type: canonical-spec
created: 2026-01-17
last_updated: 2026-01-17
status: draft
phase: 5
principle: "INVESTMENT NOT RENT"
foundation:
  - "[[canonical/02_ROADMAP.md]]"
  - "[[canonical/00_VISION.md]]"
proven_patterns:
  - "[[apps/cv_agentic_knowledge/app/deployments/corporate-visions/]]"
  - "[[03_gtm_engagements/03_active_client/pixee_ai_gtm/docs/04_continuation/jan_2026/00_CONTEXT_METAWORKER_SPEC.md]]"
related:
  - "[[canonical/03_CORE_ARCHITECTURE.md]]"
  - "[[canonical/04_TRANSPORT_ARCHITECTURE.md]]"
tags:
  - tastematter
  - mcp
  - context-streaming
  - canonical
---

# MCP Publishing Architecture

## Executive Summary

Enable Tastematter users to publish their context as MCP servers, transforming local knowledge into queryable, authenticated, optionally monetized context services.

**Core Value Proposition:** Your context becomes a product. You control access. You capture value.

**Proven By:**
- CVI (Corporate Visions) deployment: Working MCP server with grep/read/list tools [VERIFIED: `apps/cv_agentic_knowledge/app/deployments/corporate-visions/`]
- Context7 precedent: Upstash's most popular product is their MCP server, not their database [VERIFIED: `00_CONTEXT_METAWORKER_SPEC.md:447`]
- Pixee engagement: Applied pattern for client with internal + external layers [VERIFIED: `00_CONTEXT_METAWORKER_SPEC.md:18-32`]

---

## Architecture Overview

### Level 3 Definition

From `canonical/00_VISION.md:48-51`:
```
Level 3: Inter-OS Protocols (FUTURE)
        Context as a service, MCP publishing, pay-walling
```

### Two-Layer Model

**Layer 1: Internal Publishing** (This Spec - Phase 5A)
- Publish context for personal/team use
- Local or cloud deployment
- Authentication via API keys

**Layer 2: External Publishing** (Future - Phase 5B)
- Public-facing context services
- Pay-walling integration
- Usage analytics and monetization

---

## Proven Patterns (CVI Reference Implementation)

### Architecture Stack

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    CVI PROVEN ARCHITECTURE                               │
│                                                                          │
│  LOCAL                           CLOUDFLARE                              │
│  ─────                           ──────────                              │
│  ┌─────────────────┐            ┌──────────────────────────────────────┐│
│  │ generate-corpus │            │ Worker (index.ts)                    ││
│  │ (scripts/)      │──push────▶ │ ├── /mcp    (Streamable HTTP)       ││
│  │                 │            │ ├── /sse    (Server-Sent Events)    ││
│  │ Input: git repo │            │ ├── /health                         ││
│  │ Output: JSON    │            │ ├── /reload                         ││
│  └─────────────────┘            │ └── /queries (logging)              ││
│                                 │                                      ││
│  ┌─────────────────┐            │ Durable Object (KnowledgeGraphDO)   ││
│  │ wrangler deploy │            │ ├── Holds corpus in memory          ││
│  │                 │──deploy──▶ │ ├── /grep, /read, /list endpoints   ││
│  │                 │            │ └── Auto-reload from R2              ││
│  └─────────────────┘            │                                      ││
│                                 │ MCP Wrapper (KnowledgeGraphMCP)     ││
│                                 │ ├── Exposes `query` tool             ││
│                                 │ ├── Uses @modelcontextprotocol/sdk  ││
│                                 │ └── Streaming to prevent timeout     ││
│                                 │                                      ││
│                                 │ R2 Bucket                            ││
│                                 │ └── corpus-snapshot.json             ││
│                                 └──────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────┘
```

[VERIFIED: `corporate-visions/src/index.ts`, `knowledge-graph-do.ts`, `mcp-wrapper.ts`]

### Key Components (Proven)

#### 1. Corpus Generation (`generate-corpus.ts`)

**Purpose:** Convert git-tracked files to queryable corpus snapshot

**Pattern:**
```typescript
interface CorpusSnapshot {
  version: string;
  commit: string;           // Git SHA for versioning
  fileCount: number;
  totalSize: number;
  generatedAt: string;
  files: Record<string, FileEntry>;
  allPaths: string[];       // Pre-computed path index
}

interface FileEntry {
  path: string;
  content: string;
  size: number;
  frontmatter?: any;        // Parsed YAML frontmatter
}
```

**Implementation:**
- Uses `git ls-files` to respect `.gitignore`
- Parses frontmatter with `gray-matter`
- Builds path index for list operations
- Output: JSON to stdout (pipe to file or upload)

[VERIFIED: `corporate-visions/scripts/generate-corpus.ts:47-111`]

#### 2. Worker Entry Point (`index.ts`)

**Endpoints:**
| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/mcp` | POST | Streamable HTTP MCP transport |
| `/sse` | GET | Server-Sent Events MCP transport |
| `/health` | GET | Check corpus load status |
| `/reload` | POST | Force corpus reload from R2 |
| `/queries` | GET | List query logs |
| `/queries/:id` | GET | Fetch specific query log |
| `/?q=` | GET | Direct query (debug mode available) |

[VERIFIED: `corporate-visions/src/index.ts:10-208`]

#### 3. Durable Object (`knowledge-graph-do.ts`)

**Purpose:** Stateful corpus holder with tool endpoints

**Internal Endpoints:**
- `/grep` - Pattern search with ranking
- `/read` - File content retrieval
- `/list` - Glob pattern matching
- `/reload` - Refresh from R2
- `/health` - Status check

**Pattern:**
```typescript
export class KnowledgeGraphDO extends DurableObject<Env> {
  private corpus: CorpusSnapshot | null = null;
  private loadPromise: Promise<void> | null = null;

  async fetch(request: Request): Promise<Response> {
    // Lazy load corpus from R2 on first request
    if (!this.corpus && !this.loadPromise) {
      this.loadPromise = this.loadCorpusFromR2();
    }
    // ... handle endpoints
  }
}
```

[VERIFIED: `corporate-visions/src/durable-objects/knowledge-graph-do.ts:9-84`]

#### 4. MCP Wrapper (`mcp-wrapper.ts`)

**Purpose:** Expose agentic query tool via MCP protocol

**Pattern:**
```typescript
export class KnowledgeGraphMCP extends McpAgent {
  server = new McpServer({
    name: 'cv-knowledge-graph',
    version: '1.0.0'
  });

  async init() {
    this.server.tool(
      'query',
      { question: z.string().describe('The question to answer') },
      async ({ question }) => {
        const result = await executeAgenticQueryStreaming(question, this.env);
        return { content: [{ type: 'text', text: result.response }] };
      }
    );
  }
}
```

**Key Insight:** Single high-level `query` tool, not raw grep/read/list. The agent handles tool orchestration internally.

[VERIFIED: `corporate-visions/src/mcp-wrapper.ts:8-99`]

#### 5. Query Handler (`query-handler.ts`)

**Purpose:** Agentic loop using Anthropic SDK with tools

**Tools Defined:**
```typescript
const grepTool = betaTool({
  name: 'grep',
  description: 'Search for patterns in the knowledge base',
  inputSchema: { pattern: string, caseInsensitive?: boolean, maxResults?: number }
});

const readTool = betaTool({
  name: 'read',
  description: 'Read the full content of a specific file',
  inputSchema: { path: string }
});

const listTool = betaTool({
  name: 'list',
  description: 'List files/directories matching a glob pattern',
  inputSchema: { pattern: string, directories?: boolean, files?: boolean }
});
```

**Safety:** Tool results truncated to 50,000 chars to prevent token bloat.

[VERIFIED: `corporate-visions/src/query-handler.ts:25-170`]

#### 6. Wrangler Configuration

**Required Bindings:**
```toml
# Durable Object for corpus
[[durable_objects.bindings]]
name = "KNOWLEDGE_GRAPH_DO"
class_name = "KnowledgeGraphDO"

# MCP Durable Object (name MUST be MCP_OBJECT for McpAgent framework)
[[durable_objects.bindings]]
name = "MCP_OBJECT"
class_name = "KnowledgeGraphMCP"

# R2 bucket for corpus storage
[[r2_buckets]]
binding = "CORPUS_BUCKET"
bucket_name = "your-corpus-bucket"
```

[VERIFIED: `corporate-visions/wrangler.toml:14-35`]

---

## Tastematter Integration Architecture

### Design Goal

Enable users to:
1. Select paths/repos to publish from Tastematter UI
2. Generate corpus from selected paths
3. Deploy MCP server to Cloudflare (or run locally)
4. Manage API keys and access
5. View query logs and usage

### Component Mapping

| CVI Component | Tastematter Equivalent | Status |
|---------------|------------------------|--------|
| `generate-corpus.ts` | `tastematter publish corpus` CLI | To Build |
| Manual wrangler deploy | Automated deploy from UI | To Build |
| Static wrangler.toml | Generated from UI config | To Build |
| No auth | API key verification in Worker | To Build |
| Console logs | Query log viewer in Tastematter | To Build |

### Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    TASTEMATTER MCP PUBLISHING                            │
│                                                                          │
│  TASTEMATTER DESKTOP APP                                                 │
│  ───────────────────────                                                 │
│  ┌──────────────────────────────────────────────────────────────────────┐│
│  │ Publishing Manager View                                              ││
│  │ ┌────────────────┐  ┌────────────────┐  ┌────────────────┐           ││
│  │ │ Path Selector  │  │ Deploy Config  │  │ API Key Mgmt   │           ││
│  │ │ - Repos        │  │ - Name         │  │ - Generate     │           ││
│  │ │ - Directories  │  │ - Region       │  │ - Revoke       │           ││
│  │ │ - Patterns     │  │ - Auth toggle  │  │ - Usage stats  │           ││
│  │ └────────────────┘  └────────────────┘  └────────────────┘           ││
│  │                                                                       ││
│  │ ┌────────────────────────────────────────────────────────────────┐   ││
│  │ │ Published Contexts                                              │   ││
│  │ │ ┌─────────────────────────────────────────────────────────────┐│   ││
│  │ │ │ personal-knowledge  │ Running │ 3 API keys │ 127 queries   ││   ││
│  │ │ │ pixee-context-os    │ Running │ 1 API key  │ 45 queries    ││   ││
│  │ │ │ team-playbook       │ Paused  │ 0 API keys │ 0 queries     ││   ││
│  │ │ └─────────────────────────────────────────────────────────────┘│   ││
│  │ └────────────────────────────────────────────────────────────────┘   ││
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  TASTEMATTER CLI                                                         │
│  ──────────────                                                          │
│  ┌──────────────────────────────────────────────────────────────────────┐│
│  │ $ tastematter publish corpus --paths "knowledge_base/**" --output .  ││
│  │ $ tastematter publish deploy --name my-context --region us-east-1   ││
│  │ $ tastematter publish keys create --name "claude-desktop"           ││
│  │ $ tastematter publish logs --name my-context --limit 50             ││
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  CLOUDFLARE (per published context)                                      │
│  ─────────────────────────────────                                       │
│  ┌──────────────────────────────────────────────────────────────────────┐│
│  │ Worker: tastematter-{context-name}                                   ││
│  │ ├── Auth middleware (API key verification)                          ││
│  │ ├── /mcp, /sse (MCP transport)                                       ││
│  │ ├── /health, /reload                                                 ││
│  │ └── Query logging to R2                                              ││
│  │                                                                       ││
│  │ Durable Objects: KnowledgeGraphDO, KnowledgeGraphMCP                 ││
│  │ R2 Bucket: tastematter-{context-name}-corpus                         ││
│  │ KV Namespace: tastematter-{context-name}-keys (API key store)        ││
│  └──────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Type Contracts

### Publisher Configuration

```typescript
// ~/.context-os/publishers.yaml schema
interface PublisherConfig {
  name: string;                    // Unique identifier (slug)
  displayName: string;             // Human-readable name

  // Content selection
  paths: PathSelector[];           // What to include
  excludePatterns?: string[];      // What to exclude

  // Deployment
  deployment: {
    provider: 'cloudflare' | 'local';
    region?: CloudflareRegion;
    workerName?: string;           // Auto-generated if not provided
  };

  // Authentication
  auth: {
    enabled: boolean;
    apiKeys?: ApiKeyRef[];         // Reference to stored keys
  };

  // Corpus
  corpus: {
    lastGenerated?: string;        // ISO timestamp
    lastDeployed?: string;
    commitSha?: string;
  };

  // Status
  status: 'running' | 'paused' | 'error' | 'not-deployed';
}

interface PathSelector {
  type: 'repo' | 'directory' | 'glob';
  path: string;
  recursive?: boolean;
}

type CloudflareRegion =
  | 'us-east-1' | 'us-west-1' | 'eu-west-1'
  | 'ap-northeast-1' | 'ap-southeast-1';
```

### API Key Management

```typescript
interface ApiKey {
  id: string;                      // UUID
  name: string;                    // User-provided name (e.g., "claude-desktop")
  keyHash: string;                 // SHA-256 hash of actual key
  prefix: string;                  // First 8 chars for identification
  createdAt: string;
  lastUsed?: string;
  expiresAt?: string;
  usageCount: number;
  rateLimit?: {
    requestsPerMinute: number;
    requestsPerDay: number;
  };
}

// Key format: tm_{context_name}_{random_32_chars}
// Example: tm_personal_knowledge_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

### Query Logging

```typescript
interface QueryLog {
  id: string;                      // UUID
  timestamp: string;
  query: string;
  response: string;

  // Performance
  durationMs: number;
  toolCalls: number;
  tokensUsed?: number;

  // Context
  apiKeyPrefix?: string;           // Which key was used
  corpusCommit: string;

  // For analytics
  success: boolean;
  errorMessage?: string;
}
```

---

## CLI Commands (Phase 5A)

### Corpus Management

```bash
# Generate corpus from paths
tastematter publish corpus \
  --paths "knowledge_base/**" "00_foundation/**" \
  --exclude "*.log" "node_modules/**" \
  --output ./corpus-snapshot.json

# Dry run (show what would be included)
tastematter publish corpus --paths "**/*.md" --dry-run

# Generate with frontmatter parsing
tastematter publish corpus --paths "**/*.md" --parse-frontmatter
```

### Deployment

```bash
# Initialize new publisher (interactive)
tastematter publish init

# Deploy to Cloudflare
tastematter publish deploy \
  --name my-context \
  --region us-east-1 \
  --auth-enabled

# Update existing deployment
tastematter publish update --name my-context

# Pause/resume
tastematter publish pause --name my-context
tastematter publish resume --name my-context

# Delete deployment
tastematter publish delete --name my-context --confirm
```

### API Key Management

```bash
# Create API key
tastematter publish keys create \
  --publisher my-context \
  --name "claude-desktop" \
  --expires 90d

# List keys
tastematter publish keys list --publisher my-context

# Revoke key
tastematter publish keys revoke --publisher my-context --id abc123

# Rotate key (create new, revoke old)
tastematter publish keys rotate --publisher my-context --id abc123
```

### Monitoring

```bash
# View recent queries
tastematter publish logs --name my-context --limit 50

# Query usage stats
tastematter publish stats --name my-context --period 7d

# Health check
tastematter publish health --name my-context
```

---

## UI Components (Phase 5A)

### Publishing Manager View

**Location:** New top-level view in Tastematter sidebar

**Sections:**
1. **Path Selector** - Tree view of available repos/directories with checkboxes
2. **Publisher List** - Table of configured publishers with status
3. **Deploy Dialog** - Modal for configuring new deployment
4. **Key Manager** - Panel for API key CRUD
5. **Query Logs** - Searchable log viewer with filters

### Wireframes

```
┌─────────────────────────────────────────────────────────────────────────┐
│ PUBLISHING                                                    [+ New]   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│ YOUR PUBLISHED CONTEXTS                                                  │
│ ┌─────────────────────────────────────────────────────────────────────┐ │
│ │ ● personal-knowledge                                                │ │
│ │   us-east-1 • 3 API keys • 127 queries today                       │ │
│ │   Last updated: 2 hours ago                                [Manage] │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ ● pixee-context-os                                                  │ │
│ │   us-east-1 • 1 API key • 45 queries today                         │ │
│ │   Last updated: 1 day ago                                  [Manage] │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ ○ team-playbook (paused)                                            │ │
│ │   eu-west-1 • 0 API keys • 0 queries                               │ │
│ │   Last updated: 5 days ago                                 [Resume] │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ QUICK STATS (Last 7 Days)                                               │
│ ┌───────────────┐ ┌───────────────┐ ┌───────────────┐                   │
│ │ 847           │ │ 12            │ │ 98.2%         │                   │
│ │ Total Queries │ │ API Keys      │ │ Success Rate  │                   │
│ └───────────────┘ └───────────────┘ └───────────────┘                   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Authentication Architecture

### API Key Flow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           API KEY AUTHENTICATION                          │
│                                                                           │
│  CLIENT                          WORKER                          KV      │
│  ──────                          ──────                          ──      │
│                                                                           │
│  ┌─────────────┐                                                         │
│  │ MCP Request │                                                         │
│  │ Header:     │                                                         │
│  │ X-API-Key:  │──────────────▶ ┌─────────────────────┐                  │
│  │ tm_xxx_...  │                │ Auth Middleware     │                  │
│  └─────────────┘                │                     │                  │
│                                 │ 1. Extract key      │                  │
│                                 │ 2. Hash key         │──────▶ ┌───────┐ │
│                                 │ 3. Lookup in KV     │◀────── │ Keys  │ │
│                                 │ 4. Check expiry     │        │ Store │ │
│                                 │ 5. Check rate limit │        └───────┘ │
│                                 │ 6. Update lastUsed  │                  │
│                                 └─────────────────────┘                  │
│                                          │                               │
│                                          ▼                               │
│                                 ┌─────────────────────┐                  │
│                                 │ Valid? → Forward    │                  │
│                                 │ Invalid? → 401      │                  │
│                                 │ Rate limited? → 429 │                  │
│                                 └─────────────────────┘                  │
│                                                                           │
└──────────────────────────────────────────────────────────────────────────┘
```

### Key Storage

- **KV Namespace:** `tastematter-{context-name}-keys`
- **Key format:** `key:{hash}` → `ApiKey` JSON
- **Index:** `keys:index` → list of all key IDs

---

## Implementation Phases

### Phase 5A: Internal Publishing (MVP)

**Goal:** Publish context for personal use with basic auth

**Tasks:**

1. **CLI: Corpus Generation** (Port from CVI)
   - [ ] Implement `tastematter publish corpus` command
   - [ ] Add path selection with glob patterns
   - [ ] Add frontmatter parsing
   - [ ] Output corpus-snapshot.json

2. **CLI: Deployment**
   - [ ] Implement `tastematter publish init` (interactive setup)
   - [ ] Implement `tastematter publish deploy`
   - [ ] Generate wrangler.toml from config
   - [ ] Create R2 bucket automatically
   - [ ] Upload corpus to R2
   - [ ] Deploy worker via Wrangler API

3. **Worker Template**
   - [ ] Create deployable worker template in Tastematter
   - [ ] Add auth middleware
   - [ ] Add KV namespace for API keys
   - [ ] Implement query logging to R2

4. **CLI: Key Management**
   - [ ] Implement `tastematter publish keys create/list/revoke`
   - [ ] Store keys in KV namespace
   - [ ] Generate secure random keys

5. **Config Storage**
   - [ ] Define `~/.context-os/publishers.yaml` schema
   - [ ] Implement config CRUD in CLI

6. **Tastematter UI: Basic**
   - [ ] Add "Publishing" view to sidebar
   - [ ] Show list of publishers with status
   - [ ] Add "New Publisher" dialog

**Success Criteria:**
- [ ] Can publish local paths as MCP server in <5 min
- [ ] API keys prevent unauthorized access
- [ ] Query logs visible in CLI

### Phase 5B: Advanced Features

**Goal:** Pay-walling, usage analytics, team features

**Tasks:**

1. **Pay-walling Integration**
   - [ ] Stripe integration for usage billing
   - [ ] Free tier configuration
   - [ ] Usage metering

2. **Analytics Dashboard**
   - [ ] Query patterns visualization
   - [ ] Usage trends over time
   - [ ] Popular files/topics

3. **Team Features**
   - [ ] Shared publishers
   - [ ] Role-based access
   - [ ] Audit logging

4. **Tastematter UI: Full**
   - [ ] Revenue dashboard
   - [ ] Advanced key management
   - [ ] Query log explorer

---

## Known Unknowns

### Technical

1. **Cloudflare API Authentication**
   - How does Tastematter authenticate to deploy workers?
   - Options: OAuth flow, API token input, Wrangler CLI delegation
   - **Decision needed:** Which auth method to implement first?

2. **Worker Template Bundling**
   - Should worker code be bundled in Tastematter binary?
   - Or fetched from GitHub on deploy?
   - **Decision needed:** Bundling vs fetch strategy

3. **Local MCP Server Option**
   - Some users may not want Cloudflare
   - Can we run MCP server locally via Tastematter?
   - **Decision needed:** Local mode priority

4. **Corpus Size Limits**
   - CVI corpus is ~9MB JSON
   - Durable Object memory limits?
   - R2 free tier limits?
   - **Research needed:** Practical limits for corpus size

### Product

1. **Pricing Model**
   - Per-query billing? Monthly subscription? Free tier limits?
   - **Decision needed:** Before Phase 5B

2. **Multi-Tenant Architecture**
   - Each publisher = separate worker, or shared infrastructure?
   - Cost implications for many publishers
   - **Decision needed:** Architecture for scale

3. **Context Updates**
   - Auto-sync with git changes? Manual trigger?
   - Webhook from GitHub/GitLab?
   - **Decision needed:** Update mechanism

---

## Dependencies

### External

- **Cloudflare Account:** Required for deployment
- **Wrangler CLI or API:** For worker deployment
- **Anthropic API Key:** For query agent (in worker)

### Internal

- **Tastematter CLI:** Must be installed and configured
- **Phase 0-4 Complete:** Performance, git integration, multi-repo (recommended but not required)

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cloudflare API changes | Low | High | Pin wrangler version, integration tests |
| Corpus too large | Medium | Medium | Size limits in UI, chunking for large repos |
| API key compromise | Medium | High | Key rotation, expiry, per-key rate limits |
| Worker cold start latency | Low | Medium | Durable Object warm-up, edge caching |
| Cost overruns | Medium | Medium | Usage quotas, alerts, free tier limits |

---

## Success Metrics

**Phase 5A Complete When:**
- [ ] User can publish context as MCP server from Tastematter
- [ ] Time from decision to live MCP: <5 minutes
- [ ] Authentication prevents unauthorized access
- [ ] Query logs are retrievable
- [ ] At least 1 real user (yourself) using it daily

**Phase 5B Complete When:**
- [ ] Pay-walling generates revenue
- [ ] Usage analytics inform decisions
- [ ] 3+ external users consuming your context

---

## Context Sources

**Proven Patterns:**
- `apps/cv_agentic_knowledge/app/deployments/corporate-visions/` - Full working implementation
- `03_gtm_engagements/.../00_CONTEXT_METAWORKER_SPEC.md` - Applied to client engagement

**Vision Documents:**
- `apps/tastematter/specs/canonical/00_VISION.md` - Level 3 definition
- `apps/tastematter/specs/canonical/02_ROADMAP.md` - Phase 5 placement
- December 2025 voice memo - Original architectural vision

**Related Specs:**
- `apps/tastematter/specs/canonical/03_CORE_ARCHITECTURE.md` - How data flows
- `apps/tastematter/specs/canonical/04_TRANSPORT_ARCHITECTURE.md` - HTTP/IPC patterns

---

## Appendix: CVI File Reference

| File | Purpose | Lines |
|------|---------|-------|
| `src/index.ts` | Worker entry, endpoint routing | 209 |
| `src/query-handler.ts` | Agentic loop with tools | ~250 |
| `src/mcp-wrapper.ts` | MCP SDK integration | 100 |
| `src/durable-objects/knowledge-graph-do.ts` | Corpus holder | 85 |
| `scripts/generate-corpus.ts` | Corpus generation | 133 |
| `wrangler.toml` | Cloudflare config | 36 |

**Total proven code to port:** ~800 lines TypeScript

---

**Last Updated:** 2026-01-17
**Status:** Draft - Ready for iteration
**Next Action:** User review and decision on known unknowns
