---
title: "Context Alerting & Publishing - Product Architecture"
type: canonical-spec
created: 2026-02-13
last_updated: 2026-02-14
status: draft-v4
principle: "SUPER LIGHTWEIGHT AND EASY — ZERO AUTH CODE"
foundation:
  - "[[canonical/00_VISION.md]]"
  - "[[canonical/02_ROADMAP.md]]"
  - "[[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]"
proven_patterns:
  - "[[apps/intelligence_pipeline/]]"
  - "[[apps/cv_agentic_knowledge/app/deployments/corporate-visions/]]"
  - "[[apps/clients/nickel/conference_pr/worker/]]"
  - "[[apps/tastematter/download-alert-worker/]]"
  - "[[_system/patterns/cloudflare-access-workers.md]]"
notification_infrastructure:
  - provider: knock
  - docs: "https://docs.knock.app"
  - mcp_server: "@knocklabs/agent-toolkit"
tags:
  - tastematter
  - alerting
  - publishing
  - context-worker
  - knock
  - canonical
---

# Context Alerting & Publishing

## Executive Summary

Two new paid features for tastematter that solve the two biggest pain points discovered building five production context systems:

1. **Context Alerting** -- No easy way to get notified when context changes matter
2. **Context Publishing** -- No easy way to share/publish context for others to use

Both features build on a shared primitive: the **Context Worker** -- a lightweight Cloudflare Worker that watches, processes, and serves context.

**Notification infrastructure:** [Knock](https://knock.app) as the unified notification layer. One API call from the worker triggers Knock, which routes to email, Web Push (desktop + mobile), Slack, and in-app feed. Knock's MCP server (`@knocklabs/agent-toolkit`) enables workflow configuration directly from Claude Code. No separate Resend/ntfy.sh/Slack integrations to manage.

**Publishing management:** Web app at `app.tastematter.dev` (Svelte) serves as the management UI for publishing + the notification center (Knock in-app feed) + Web Push registration endpoint. Same web app works on desktop browsers and mobile (PWA for iOS/Android push).

**Design philosophy:** Super lightweight. Cloudflare-native. Easy to set up. Extract proven patterns from production systems into generic, configurable modules.

### Go-to-Market Progression

```
ME (dogfood) → CLIENTS (productized service) → MARKET (product)
```

**Phase: Me** — Solve own painpoints. First alerts: "email me when a new intelligence brief is ready." First publishes: intel briefs as pages, GTM knowledge base as MCP, client deliverables as shareable pages, content portfolio as queryable source.

**Phase: Clients** — Deliver these as part of consulting engagements. Tastematter becomes the delivery vehicle. Each client gets an engagement with published artifacts + alerting. This is a productized service, not a self-serve product yet.

**Phase: Market** — Once patterns stabilize across 3-5 client engagements, open self-serve. Developer/team market: "publish your context in one command, get alerts when it matters."

**Pricing reality (2026-02):** Not enough real users to price. Design for flexibility. Free for personal use, bundled into consulting engagements for clients, subscription TBD for market phase.

### First Use Cases (from founder interview, 2026-02-14)

| Use Case | Feature | Priority |
|----------|---------|----------|
| Email + push when new intel brief is generated with high-relevance articles | Alerting (Knock workflow → email + Web Push + in-app) | **First alert to build** |
| Publish intelligence pipeline briefs as shareable static pages | Publishing (pages) | High |
| Publish GTM knowledge base as queryable MCP endpoint | Publishing (MCP) | High |
| Publish client deliverables (Nickel PR packages, Pixee reports) | Publishing (pages) | High |
| Publish content portfolio as queryable knowledge source | Publishing (MCP) | High |

---

## Part 1: The Context Worker Primitive

### What Is It

Every system built so far follows the same five-stage pattern:

```
INGEST --> PROCESS --> STORE --> SERVE --> NOTIFY
```

| Stage    | Intelligence Pipeline        | CVI Knowledge Graph       | Nickel Conference PR        | Download Alerts        |
|----------|------------------------------|---------------------------|-----------------------------|------------------------|
| Ingest   | Readwise, Parallel, Twitter  | Git repo -> corpus JSON   | Press list JSON import      | CF GraphQL API         |
| Process  | Claude classification        | Agentic search (Claude)   | Web research + scoring      | Regex filter           |
| Store    | D1 + R2                      | R2 -> Durable Object      | D1 + R2                     | (none)                 |
| Serve    | HTML dashboards, JSON API    | MCP protocol, HTTP query  | JSON API, export endpoint   | (none)                 |
| Notify   | Slack webhook                | (none)                    | (none)                      | ntfy.sh push (legacy)  |

The Context Worker is the productized, generic version of this pattern. Users configure it through tastematter's desktop UI; the worker runs on Cloudflare.

### Architecture

```
+------------------------------------------------------------------+
|  WEB APP: app.tastematter.dev (Svelte)                            |
|  Serves as: publishing management + notification center + PWA     |
|                                                                   |
|  +-------------------+  +-------------------+  +---------------+  |
|  | Alert Manager     |  | Publish Manager   |  | Notification  |  |
|  | - Watch rules     |  | - Path selector   |  |   Center      |  |
|  | - Knock workflows |  | - Access policy   |  | - Knock feed  |  |
|  | - History         |  | - Query logs      |  | - Web Push    |  |
|  +-------------------+  +-------------------+  +---------------+  |
|                               |                                   |
+-------------------------------|-----------------------------------+
                                | CLI / Cloudflare API
                                v
+------------------------------------------------------------------+
|  CLOUDFLARE (per context worker)                                  |
|                                                                   |
|  Worker: tm-{name}                                                |
|  +------------------------------------------------------------+  |
|  |  Router                                                     |  |
|  |  /health          GET    Health check                       |  |
|  |  /alert/trigger   POST   Manual alert trigger               |  |
|  |  /mcp             POST   MCP Streamable HTTP (Publishing)   |  |
|  |  /sse             GET    MCP SSE transport (Publishing)     |  |
|  |  /pages/*         GET    Static rendered pages (Publishing) |  |
|  |  /query           GET    Direct query endpoint              |  |
|  |  /logs            GET    Activity logs                      |  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  Durable Object: ContextDO (singleton)                            |
|  +------------------------------------------------------------+  |
|  |  - Holds corpus snapshot in memory (lazy load from R2)      |  |
|  |  - /grep, /read, /list tool endpoints                       |  |
|  |  - /reload for corpus refresh                               |  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  Storage:                                                         |
|  +----------+  +----------+  +----------+                         |
|  | D1       |  | R2       |  | KV       |                         |
|  | Logs     |  | Corpus   |  | Config   |                         |
|  | Alerts   |  | Briefs   |  | Config   |                         |
|  | State    |  | Pages    |  |          |                         |
|  +----------+  +----------+  +----------+                         |
|                                                                   |
|  Cron Triggers (configurable):                                    |
|  - Alert checks (e.g., every 4h, daily, custom)                  |
|  - Corpus refresh (e.g., on push, daily)                          |
+------------------------------------------------------------------+
                                |
                                | Single fetch() to trigger workflow
                                v
+------------------------------------------------------------------+
|  KNOCK (notification infrastructure)                              |
|                                                                   |
|  Workflow: "new-intel-brief"                                      |
|  +------------------------------------------------------------+  |
|  |  Step 1: Email (Resend/SendGrid provider)                   |  |
|  |  Step 2: Web Push (FCM → desktop + mobile browsers)         |  |
|  |  Step 3: In-app feed (bell icon in web app)                 |  |
|  |  Step 4: Slack (webhook, optional)                          |  |
|  |  Batch/delay/digest logic configured per workflow            |  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  MCP Server: @knocklabs/agent-toolkit                             |
|  - Configure workflows from Claude Code via natural language      |
|  - Create/update steps, channels, templates                       |
+------------------------------------------------------------------+
```

### The Context Worker Configuration Model

Extracted from the intelligence pipeline's YAML-driven topic config pattern. The key insight: **topic is configuration, not code**.

```yaml
# ~/.tastematter/workers/{name}.yaml
name: my-context
display_name: "My Knowledge Base"

# What this worker does (flags)
features:
  alerting: true
  publishing: true

# Source configuration
source:
  type: corpus           # corpus | feed | hybrid
  paths:                 # For corpus type
    - "knowledge_base/**/*.md"
    - "00_foundation/**/*.md"
  exclude:
    - "node_modules/**"
    - "*.log"
  repo: ~/projects/my-repo   # Git repo root (optional)

# Alerting configuration (if enabled)
alerting:
  provider: knock                 # Unified notification infrastructure
  workflow: new-intel-brief       # Knock workflow key (configured via MCP or dashboard)
  recipients:
    - id: dietl                   # Knock user ID
  rules:
    - name: "New content alert"
      trigger: content_change
      schedule: "0 */4 * * *"    # Check every 4 hours
    - name: "Daily digest"
      trigger: schedule
      schedule: "0 7 * * *"     # Daily at 7am UTC
      format: digest
  # Channel routing (email, push, slack, in-app) configured in Knock,
  # NOT in this file. Use Knock MCP server or dashboard to manage.

# Publishing configuration (if enabled)
publishing:
  auth:
    enabled: true
    keys: []               # Managed via CLI/UI
  mcp:
    enabled: true
    tool_name: query       # MCP tool name
  pages:
    enabled: true
    template: dashboard    # dashboard | brief | custom

# Deployment
deployment:
  region: us-east-1
  worker_name: tm-my-context    # Auto-generated
```

### What Already Exists vs What Needs Building

| Component | Exists (proven) | Needs Building |
|-----------|----------------|----------------|
| Corpus generation (git -> JSON) | `generate-corpus.ts` (133 lines) | Port to tastematter CLI |
| Durable Object corpus holder | `knowledge-graph-do.ts` (85 lines) | Generalize, add config |
| MCP wrapper (query tool) | `mcp-wrapper.ts` (100 lines) | Generalize per-worker |
| Agentic query handler | `query-handler.ts` (250 lines) | Reuse as-is |
| grep/read/list tools | 3 files (~200 lines total) | Reuse as-is |
| Cron-triggered checks | `download-alert-worker` (100 lines) | Generalize trigger system |
| ntfy.sh push notifications | `download-alert-worker` (100 lines) | Legacy — replaced by Knock Web Push |
| Slack notifications | `notifications.ts` (160 lines) | Legacy — Knock handles channel routing |
| YAML config loader | `config.ts` (163 lines) | Adapt for worker config |
| HTML page rendering | `index.ts` briefs/logs/stats (800 lines) | Extract as templates |
| D1 flow logging | Intelligence pipeline (schema + queries) | Reuse pattern |
| CF Access auth | Nickel worker (15 lines) | Edge auth (zero worker code) + optional defense in depth |
| Pipeline stages | Nickel import/enrich/score/generate | Extract as composable stages |
| Worker deployment | Manual wrangler | Automated from CLI/UI |
| Web app (management + notifications) | (none) | Build in Svelte (app.tastematter.dev) |
| Knock notification workflows | (none) | Configure via MCP server or dashboard |
| Web Push registration | (none) | FCM token registration in web app |

**Estimated new code:** ~35% new, ~55% extracted/adapted from existing systems, ~10% eliminated by Knock (no custom notification routing/formatting code).

---

## Part 2: Context Alerting

### Problem

Every system built so far has bolted on notifications as an afterthought: Slack webhooks hard-coded into worker code. The intelligence pipeline sends Slack messages. The download alert worker sends Slack messages. There is no easy, user-configurable way to say "tell me when X happens" and get notified by email or push.

### Design

Alerting is composed of three concepts:

1. **Watch Rules** -- What to watch for (triggers)
2. **Channels** -- How to notify (email, push, Slack)
3. **Schedule** -- When to check (cron-based)

#### Watch Rules (Triggers)

Extracted from the patterns observed across systems:

| Trigger Type | Example | Proven In |
|-------------|---------|-----------|
| `content_change` | New files added to watched paths | Download alert worker |
| `pattern_match` | Content matching a regex/keyword | Intelligence pipeline classification |
| `threshold` | Metric exceeds a value | Intelligence pipeline article count threshold |
| `schedule` | Time-based digest | Intelligence pipeline daily brief |
| `corpus_drift` | Published corpus diverges from source | CVI corpus snapshot pattern |

```typescript
interface WatchRule {
  name: string;
  trigger: 'content_change' | 'pattern_match' | 'threshold' | 'schedule' | 'corpus_drift';
  schedule: string;          // Cron expression
  config: TriggerConfig;     // Trigger-specific configuration
  channels: string[];        // Which channels to notify
  format: 'instant' | 'digest' | 'brief';
  enabled: boolean;
}

// Trigger-specific configs
type TriggerConfig =
  | { type: 'content_change'; paths: string[]; min_changes?: number }
  | { type: 'pattern_match'; pattern: string; case_insensitive?: boolean }
  | { type: 'threshold'; metric: string; operator: '>' | '<' | '='; value: number }
  | { type: 'schedule' }     // Pure time-based
  | { type: 'corpus_drift'; max_commits_behind?: number };
```

#### Notification Infrastructure: Knock

All notification routing handled by [Knock](https://knock.app) — a unified notification infrastructure platform. One API call from the worker triggers a Knock workflow, which routes to all configured channels. No custom notification code in the worker.

**Why Knock (not DIY Resend + ntfy.sh + Slack):**
- **One API call** replaces 3 separate integrations (email, push, Slack)
- **MCP server** (`@knocklabs/agent-toolkit`) — configure workflows from Claude Code via natural language
- **Web Push via FCM** — native OS notifications on desktop AND mobile browsers (no native app needed)
- **In-app feed** — pre-built notification center component (bell icon, cards, history)
- **Workflow engine** — batching, delays, digests handled by Knock, not your code
- **Delivery observability** — logs, debugging, delivery status built in
- **Free tier** — 10K notifications/month (sufficient for personal + early client use)

**Worker implementation (single fetch call):**

```typescript
async function triggerAlert(env: Env, alert: Alert): Promise<void> {
  await fetch(`https://api.knock.app/v1/workflows/${alert.workflow_key}/trigger`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${env.KNOCK_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      recipients: alert.recipients,
      data: {
        subject: alert.subject,
        body: alert.body,
        html: alert.html,
        url: alert.metadata.url,
        trigger_type: alert.trigger_type,
        changes: alert.metadata.changes,
      },
    }),
  });
}
```

**Knock workflow (configured via MCP or dashboard, NOT in worker code):**

```
Workflow: "new-intel-brief"
  Step 1: Email          → Provider: Resend/SendGrid, HTML template in Knock
  Step 2: Web Push       → Provider: FCM → desktop + mobile browser notifications
  Step 3: In-app feed    → Bell icon in web app (Knock JS SDK)
  Step 4: Slack          → Webhook (optional, for teams)
  + Batch/delay/digest logic configurable per step
```

**Knock MCP Server for workflow configuration:**

```json
{
  "mcpServers": {
    "knock": {
      "command": "npx",
      "args": ["-y", "@knocklabs/agent-toolkit", "-p", "local-mcp"],
      "env": { "KNOCK_SERVICE_TOKEN": "YOUR-SERVICE-TOKEN" }
    }
  }
}
```

MCP tools: `createWorkflow`, `triggerWorkflow`, `createOrUpdateEmailStepInWorkflow`, `createOrUpdatePushStepInWorkflow`, `createOrUpdateChatStepInWorkflow`, `createOrUpdateBatchStepInWorkflow`, `createOrUpdateInAppFeedStepInWorkflow`, `createOrUpdateDelayStepInWorkflow`, `listWorkflows`, `getWorkflow`.

**Channels managed in Knock (not in worker config):**

| Channel | Provider | Desktop | Mobile | How |
|---------|----------|---------|--------|-----|
| Email | Resend/SendGrid via Knock | Mail app | Mail app | Knock dashboard: add email provider |
| Web Push | FCM via Knock | Native OS notification | Native OS notification | Web app registers FCM token with Knock |
| In-app feed | Knock built-in | Bell icon in web app | Bell icon in PWA | Knock JS SDK in Svelte web app |
| Slack | Knock built-in | Slack desktop | Slack mobile | Knock dashboard: add webhook |

**Web Push registration flow (handles desktop + mobile):**

```
User visits app.tastematter.dev
    → Browser: "Allow notifications?" → User accepts
    → FCM SDK generates device token
    → Web app calls: knock.users.setChannelData(userId, 'fcm', { tokens: [token] })
    → Done. Native push on this device.

Desktop Chrome/Firefox/Safari: Works immediately.
Android Chrome: Works immediately.
iOS Safari: User taps "Add to Home Screen" first (PWA), then same flow.
```

**What Knock eliminates from worker code:**
- ~~Resend API integration~~ → Knock email step
- ~~ntfy.sh integration~~ → Knock Web Push step (FCM)
- ~~Slack webhook integration~~ → Knock chat step
- ~~Email HTML template rendering~~ → Knock template editor
- ~~Multi-channel routing logic~~ → Knock workflow engine
- ~~Digest/batching logic~~ → Knock batch step
- All replaced by a single `fetch()` to `api.knock.app`

#### Alert Formats

| Format | Description | Proven In |
|--------|-------------|-----------|
| `instant` | Single event notification (subject + body) | Download alert worker |
| `digest` | Batched summary over time period | Intelligence pipeline brief notification |
| `brief` | Rich HTML report with analysis | Intelligence pipeline briefs dashboard |

#### Alert Processing Flow

```
Cron trigger fires
    |
    v
Load watch rules from KV config
    |
    v
For each enabled rule:
    |
    +-- content_change: Compare corpus snapshot vs source
    +-- pattern_match: Grep corpus for pattern
    +-- threshold: Query D1 for metric
    +-- schedule: Always fires (time-based)
    +-- corpus_drift: Check git SHA vs deployed SHA
    |
    v
If trigger condition met:
    |
    v
Build alert payload (subject, body, metadata)
    |
    v
Single fetch() → Knock API: trigger workflow
    |  Knock handles: email + push + in-app + slack routing
    |  Knock handles: batching, delays, digest formatting
    |  Knock handles: template rendering, delivery retry
    v
Log to D1 (alert_history table)
```

#### D1 Schema for Alerting

```sql
CREATE TABLE alert_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  rule_name TEXT NOT NULL,
  trigger_type TEXT NOT NULL,
  fired_at TEXT NOT NULL DEFAULT (datetime('now')),
  channels_notified TEXT,       -- JSON array of channel types
  payload TEXT,                 -- JSON alert content
  success INTEGER DEFAULT 1,
  error_message TEXT
);

CREATE TABLE alert_state (
  rule_name TEXT PRIMARY KEY,
  last_checked_at TEXT,
  last_fired_at TEXT,
  last_corpus_sha TEXT,        -- For corpus_drift tracking
  state_data TEXT              -- JSON for trigger-specific state
);
```

### Alerting in the Web App

```
+------------------------------------------------------------------+
| ALERTS                                              [+ New Rule]  |
+------------------------------------------------------------------+
|                                                                   |
| WATCH RULES                                                       |
| +---------------------------------------------------------------+|
| | [on]  New content alert         every 4h       Knock workflow ||
| |       Watching: knowledge_base/**/*.md                         ||
| |       Last checked: 2 min ago | Last fired: yesterday 3pm     ||
| +---------------------------------------------------------------+|
| | [on]  Daily digest              daily 7am UTC  Knock workflow ||
| |       Format: digest | All watched paths                       ||
| |       Last fired: today 7:00am                                 ||
| +---------------------------------------------------------------+|
| | [off] Corpus drift warning      every 12h      Knock workflow ||
| |       Published corpus 3 commits behind source                 ||
| +---------------------------------------------------------------+|
|                                                                   |
| NOTIFICATION CHANNELS (managed in Knock)                          |
| +------------------+ +------------------+ +------------------+    |
| | Email            | | Web Push         | | Slack            |    |
| | via Knock+Resend | | via Knock+FCM    | | via Knock        |    |
| | [Knock Dashboard]| | [Enable Push]    | | [Knock Dashboard]|    |
| +------------------+ +------------------+ +------------------+    |
| | In-app Feed      |                                              |
| | via Knock SDK    |                                              |
| | [Active]         |                                              |
| +------------------+                                              |
|                                                                   |
| RECENT ALERTS (last 7 days)                         [Knock Logs]  |
| +---------------------------------------------------------------+|
| | Feb 13 3pm  New content alert   3 new files   all channels   ||
| | Feb 13 7am  Daily digest        12 changes    email + feed   ||
| | Feb 12 3pm  New content alert   1 new file    all channels   ||
| +---------------------------------------------------------------+|
+------------------------------------------------------------------+
```

---

## Part 3: Context Publishing

### Problem

Publishing context for others to use currently requires: cloning a repo, understanding the CVI deployment pattern, manually running generate-corpus.ts, manually deploying with wrangler, and hand-editing wrangler.toml. There is no way for a tastematter user to say "publish this folder as a queryable MCP source" and have it work in under 5 minutes.

### Design

Publishing has two equally important modes:

1. **Static Pages** -- Rendered HTML dashboards (like the intelligence pipeline's briefs pages)
2. **Queryable MCP Sources** -- Agentic search over published context (like the CVI deployment)

Both run on the same Context Worker.

#### Static Page Publishing

Extracted from the intelligence pipeline's HTML rendering pattern. The worker serves pre-rendered or on-demand HTML pages from context data.

**Page Templates:**

| Template | Description | Proven In |
|----------|-------------|-----------|
| `dashboard` | Overview with stats, recent activity, key metrics | Intel pipeline `/stats` |
| `brief` | Rich content brief with sections, evidence chains | Intel pipeline `/briefs/:id` |
| `catalog` | Browsable file/topic listing with search | Intel pipeline `/briefs` list |
| `log` | Activity/query log viewer | Intel pipeline `/logs` |
| `custom` | User-provided HTML template | (new) |

**How it works:**

```
Corpus snapshot in R2
    |
    v
Worker receives GET /pages/*
    |
    v
Load template + corpus data
    |
    v
Render HTML (server-side, no JS framework needed)
    |
    v
Return styled page (dark theme, responsive)
```

Pages are served at `https://tm-{name}.{user-domain}/pages/` or on a Cloudflare workers.dev subdomain.

The rendering approach is intentionally simple: template strings with CSS, exactly like the intelligence pipeline does it. No React, no build step, no hydration. The intelligence pipeline proves this works well for dashboards and briefs.

#### Queryable MCP Publishing

Directly ported from the CVI knowledge graph pattern. The proven flow:

```
Local files (git repo, directory)
    |  generate-corpus
    v
corpus-snapshot.json (JSON, ~MB range)
    |  upload to R2
    v
R2 Bucket
    |  lazy load on first request
    v
Durable Object (holds corpus in memory)
    |  grep/read/list tools
    v
Worker MCP endpoints (/mcp, /sse)
    |  agentic query handler (Claude Haiku)
    v
MCP protocol response
```

**MCP Tool Exposure:**

Single high-level `query` tool (proven in CVI), not raw grep/read/list. The agent handles tool orchestration internally. This is the key insight from the CVI deployment: callers should not need to understand the corpus structure.

```typescript
// Exposed to MCP clients
server.tool(
  'query',
  { question: z.string().describe('Question to answer from the knowledge base') },
  async ({ question }) => {
    const result = await executeAgenticQuery(question, env);
    return { content: [{ type: 'text', text: result.response }] };
  }
);
```

**Authentication: Cloudflare Zero Trust (not custom API keys)**

Auth is handled at the Cloudflare edge via Zero Trust Access -- zero auth code in the worker. This is the proven pattern from the intelligence pipeline (`intel.tastematter.dev`) and Nickel workers.
[VERIFIED: `_system/patterns/cloudflare-access-workers.md`, `20_CONTEXT_PACKAGE_CUSTOM_DOMAIN_ACCESS.md`]

```
MCP Client sends request
    |  CF-Access-Client-Id: <service_token_client_id>
    |  CF-Access-Client-Secret: <service_token_client_secret>
    v
Cloudflare Access (edge, before worker code runs)
    |  Validates service token or browser session
    v
Authenticated? --> Worker receives request (no auth code needed)
Not authenticated? --> 302 redirect to login (browser) or 403 (API)
```

**Three auth tiers (all managed by Cloudflare, zero worker code):**

| Tier | Mechanism | Use Case |
|------|-----------|----------|
| Browser users | CF Access email policy | Client viewing dashboards/pages |
| CLI / automation | CF Access service token | `tastematter` CLI → Worker |
| MCP clients | CF Access service token | Claude Desktop → published MCP |

**Why not custom API keys in KV:**
- Intel pipeline proves edge auth works with zero code [VERIFIED: `src/index.ts` has no auth middleware]
- CF Access provides session management, audit logging, key rotation -- for free
- Service tokens use the same `CF-Access-Client-Id`/`CF-Access-Client-Secret` headers that Nickel already uses
- One fewer KV namespace, one fewer middleware layer, one fewer attack surface
- Reusable Access policies across Workers (CF shipped Dec 2025)

#### Web App: app.tastematter.dev

The publishing management UI and notification center are delivered as a web app, not desktop-only. This is a deliberate architectural choice:

**Why web app (not Tauri-only):**
- **Web Push registration** requires a web origin — the web app IS the push registration surface
- **Mobile access** — same app works as PWA on iOS/Android (Add to Home Screen)
- **Client access** — clients manage their engagement from a browser, no install needed
- **Notification center** — Knock in-app feed component renders in the web app
- **Future Tauri integration** — Tauri can embed the same Svelte components (shared codebase)

**Web app serves three roles:**

| Role | Description | Key Component |
|------|-------------|---------------|
| Publishing management | Engagement config, access control, corpus status, query logs | Svelte management views |
| Notification center | Bell icon, notification cards, history, preferences | Knock JS SDK + in-app feed |
| Push registration | FCM token registration for Web Push on desktop + mobile | FCM SDK + Knock channel data API |

**Technology stack:**
- **Svelte** — same component framework as Tauri frontend (shared components)
- **Cloudflare Pages** — static hosting with edge functions if needed
- **Knock JS SDK** (`@knocklabs/react` or vanilla JS) — notification feed + push registration
- **Firebase JS SDK** (FCM only) — Web Push token generation

**PWA configuration (for iOS mobile push):**

```json
// manifest.json
{
  "name": "Tastematter",
  "short_name": "Tastematter",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#0a0a0a",
  "theme_color": "#0a0a0a",
  "icons": [{ "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" }]
}
```

Plus a minimal service worker for push event handling:

```typescript
// sw.js — minimal service worker for Web Push
self.addEventListener('push', (event) => {
  const data = event.data?.json() ?? {};
  event.waitUntil(
    self.registration.showNotification(data.title ?? 'Tastematter', {
      body: data.body,
      icon: '/icon-192.png',
      data: { url: data.url },
    })
  );
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const url = event.notification.data?.url ?? '/';
  event.waitUntil(clients.openWindow(url));
});
```

**Web app architecture:**

```
app.tastematter.dev (Svelte + CF Pages)
├── /                          Landing + engagement list
├── /e/{engagement}/           Engagement dashboard
│   ├── /publishing            Artifacts, corpus status, query logs
│   ├── /alerts                Watch rules, trigger history
│   ├── /access                CF Zero Trust management
│   └── /notifications         Full notification history (Knock feed)
├── /settings                  Profile, push preferences, Knock prefs
└── manifest.json + sw.js      PWA + Web Push support
```

#### Publishing in the Web App

```
+------------------------------------------------------------------+
| PUBLISHING                                          [+ New]       |
+------------------------------------------------------------------+
|                                                                   |
| ENGAGEMENTS                                                       |
| +---------------------------------------------------------------+|
| | [live]  pixee                           us-east-1              ||
| |         linkedin-intel (brief) + knowledge (mcp)               ||
| |         2 service tokens | 127 queries | CF Access: 3 emails   ||
| |         Last updated: 2 hours ago                  [Manage]    ||
| +---------------------------------------------------------------+|
| | [live]  personal                        us-east-1              ||
| |         knowledge (mcp) + dashboard (pages)                    ||
| |         1 service token | 45 queries  | CF Access: owner only  ||
| |         Last updated: 1 day ago                    [Manage]    ||
| +---------------------------------------------------------------+|
| | [off]   team-docs                       eu-west-1              ||
| |         catalog (pages)                                        ||
| |         0 tokens | 0 queries          | CF Access: paused      ||
| |         Last updated: 5 days ago                   [Resume]    ||
| +---------------------------------------------------------------+|
|                                                                   |
| QUICK STATS (7 days)                                              |
| +---------------+ +---------------+ +---------------+             |
| | 847           | | 6 tokens      | | 98.2%         |             |
| | Total Queries | | 8 emails      | | Success Rate  |             |
| +---------------+ +---------------+ +---------------+             |
|                                                                   |
+------------------------------------------------------------------+
```

**Manage View (per engagement):**

```
+------------------------------------------------------------------+
| pixee                                           [Pause] [Delete]  |
+------------------------------------------------------------------+
|                                                                   |
| PUBLISHED ARTIFACTS                                               |
| +---------------------------------------------------------------+|
| | linkedin-intel (brief)                                         ||
| |   https://pixee.tastematter.dev/intel/                         ||
| |   Source: D1 topic=linkedin_pixee | 12 briefs                  ||
| +---------------------------------------------------------------+|
| | knowledge (mcp)                                                ||
| |   https://pixee.tastematter.dev/mcp                            ||
| |   Source: 03_gtm_engagements/03_active_client/pixee_ai_gtm/** ||
| |   Corpus: 247 files, 1.2 MB | SHA: a1b2c3d                    ||
| +---------------------------------------------------------------+|
|                                    [+ Add Artifact] [Refresh All] |
|                                                                   |
| ACCESS (Cloudflare Zero Trust)              [Manage in CF Dashboard]|
| +---------------------------------------------------------------+|
| | CF Access App: pixee-tastematter (application_id: abc123)      ||
| | Policy: Allow Pixee team                                       ||
| |                                                                ||
| | Emails:                                     [+ Grant Access]   ||
| |   jake@pixee.dev          last login: 2h ago                  ||
| |   team@pixee.dev          last login: 1d ago                  ||
| |                                                                ||
| | Service Tokens:                             [+ Create Token]   ||
| |   claude-desktop          created: Feb 10    [Revoke]          ||
| |   ci-pipeline             created: Feb 1     [Revoke]          ||
| +---------------------------------------------------------------+|
|                                                                   |
| RECENT QUERIES                                                    |
| +---------------------------------------------------------------+|
| | 2 min ago  | "What is context engineering?"  | 1.2s | 3 tools ||
| | 15 min ago | "List all active clients"       | 0.8s | 2 tools ||
| | 1 hr ago   | "Summarize positioning docs"    | 2.1s | 5 tools ||
| +---------------------------------------------------------------+|
+------------------------------------------------------------------+
```

---

## Part 4: Shared Infrastructure

### Worker Template

A single deployable worker template that supports both alerting and publishing, enabled by feature flags in config. This avoids the current problem of having 4+ separate workers with duplicated patterns.

```typescript
// Template: tm-worker/src/index.ts
//
// AUTH: Cloudflare Access at the edge. Zero auth code here.
// NOTIFICATIONS: Single fetch() to Knock API. Zero channel routing code here.
// Cron triggers bypass Access (they run inside the Worker, not via HTTP).

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);
    const config = await loadConfig(env);

    // Health (always available — Access protects it at the edge)
    if (url.pathname === '/health') return json({ status: 'ok', features: config.features });

    // Alerting endpoints (if enabled)
    if (config.features.alerting) {
      if (url.pathname === '/alert/trigger') return handleAlertTrigger(env, request);
      if (url.pathname === '/alert/history') return handleAlertHistory(env, url);
    }

    // Publishing endpoints (if enabled)
    // No auth check here — Cloudflare Access validates at edge before request reaches worker
    if (config.features.publishing) {
      if (url.pathname === '/mcp') return McpHandler.serve('/mcp').fetch(request, env, ctx);
      if (url.pathname === '/sse' || url.pathname === '/sse/message')
        return McpHandler.serveSSE('/sse').fetch(request, env, ctx);
      if (url.pathname.startsWith('/pages/')) return renderPage(env, url);
      if (url.pathname === '/query') return handleDirectQuery(env, url);
    }

    // Logs (always available)
    if (url.pathname === '/logs') return handleLogs(env, url);

    return json({ error: 'Not found' }, 404);
  },

  async scheduled(event: ScheduledEvent, env: Env, ctx: ExecutionContext): Promise<void> {
    const config = await loadConfig(env);

    if (config.features.alerting) {
      ctx.waitUntil(processAlertRules(env, config));
    }

    if (config.features.publishing && config.publishing?.auto_refresh) {
      ctx.waitUntil(refreshCorpus(env, config));
    }
  },
};
```

### D1 Schema (shared)

```sql
-- Engagement config (D1-backed, not local YAML — see Design Decision #8)
CREATE TABLE IF NOT EXISTS engagements (
  id TEXT PRIMARY KEY,                    -- Slug: "pixee", "personal"
  owner_id TEXT NOT NULL,                 -- User ID (variable, not hardcoded)
  display_name TEXT NOT NULL,
  config_json TEXT NOT NULL,              -- Full EngagementConfig as JSON
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_engagements_owner ON engagements(owner_id);

-- Alert tracking
CREATE TABLE IF NOT EXISTS alert_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  engagement_id TEXT NOT NULL,            -- Scoped to engagement
  rule_name TEXT NOT NULL,
  trigger_type TEXT NOT NULL,
  fired_at TEXT NOT NULL DEFAULT (datetime('now')),
  knock_workflow_run_id TEXT,             -- Knock delivery tracking
  payload TEXT,
  success INTEGER DEFAULT 1,
  error_message TEXT
);

CREATE TABLE IF NOT EXISTS alert_state (
  rule_name TEXT PRIMARY KEY,
  engagement_id TEXT NOT NULL,
  last_checked_at TEXT,
  last_fired_at TEXT,
  last_corpus_sha TEXT,
  state_data TEXT
);

-- Query logging (publishing)
CREATE TABLE IF NOT EXISTS query_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  engagement_id TEXT NOT NULL,            -- Scoped to engagement
  timestamp TEXT NOT NULL DEFAULT (datetime('now')),
  query TEXT NOT NULL,
  response_length INTEGER,
  duration_ms INTEGER,
  tool_calls INTEGER,
  cf_access_client_id TEXT,              -- Service token ID (from CF Access JWT)
  corpus_commit TEXT,
  success INTEGER DEFAULT 1,
  error_message TEXT
);

-- Activity log (shared)
CREATE TABLE IF NOT EXISTS activity_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  engagement_id TEXT,                     -- NULL for system-level events
  timestamp TEXT NOT NULL DEFAULT (datetime('now')),
  event_type TEXT NOT NULL,
  message TEXT,
  details TEXT
);
```

### CLI Commands

**Design philosophy:** Engagement is the top-level noun. Everything published belongs to an engagement. The 80% use case: `tastematter publish pixee-linkedin-intel --template brief --link-to pixee-knowledge`.

```bash
# --- Engagement lifecycle ---
tastematter engagement init pixee                          # Interactive: name, emails, region
tastematter engagement list                                # Show all engagements + status
tastematter engagement status pixee                        # Health, artifacts, access summary

# --- Publishing (per engagement) ---
# Publish static pages (intel pipeline pattern)
tastematter publish pages \
  --engagement pixee \
  --name linkedin-intel \
  --template brief \
  --source-topic linkedin_pixee

# Publish queryable MCP source (CVI pattern)
tastematter publish context \
  --engagement pixee \
  --name knowledge \
  --paths "03_gtm_engagements/03_active_client/pixee_ai_gtm/**"

# Link artifacts within an engagement
tastematter publish link \
  --engagement pixee \
  --artifacts linkedin-intel,knowledge

# List everything published for an engagement
tastematter publish list --engagement pixee

# --- Corpus management ---
tastematter corpus generate \
  --paths "knowledge_base/**" \
  --output ./corpus-snapshot.json
tastematter corpus upload --engagement pixee --name knowledge
tastematter corpus refresh --engagement pixee --name knowledge   # Generate + upload

# --- Access management (wraps Cloudflare Zero Trust API) ---
tastematter access grant --engagement pixee --email jake@pixee.dev
tastematter access revoke --engagement pixee --email jake@pixee.dev
tastematter access token create --engagement pixee --name "claude-desktop"
tastematter access token list --engagement pixee
tastematter access token revoke --engagement pixee --name "claude-desktop"
tastematter access list --engagement pixee                  # Show all emails + tokens

# --- Alert management ---
tastematter alert list --engagement pixee
tastematter alert add --engagement pixee \
  --trigger content_change \
  --schedule "0 */4 * * *" \
  --channel email
tastematter alert test --engagement pixee --rule "New content alert"
tastematter alert history --engagement pixee

# --- Worker lifecycle (lower-level, rarely used directly) ---
tastematter worker deploy --engagement pixee               # Deploy all artifacts
tastematter worker logs --engagement pixee                  # View activity logs
tastematter worker pause --engagement pixee                 # Pause worker
tastematter worker destroy --engagement pixee               # Remove worker + CF Access app
```

**CLI → Cloudflare API mapping (access commands):**

| CLI Command | Cloudflare API Call |
|-------------|-------------------|
| `engagement init` | `POST /access/apps` + `POST /access/apps/{id}/policies` |
| `access grant --email` | `PUT /access/apps/{id}/policies/{id}` (add email to include) |
| `access token create` | `POST /access/service_tokens` |
| `access token revoke` | `DELETE /access/service_tokens/{id}` |
| `worker deploy` | `wrangler deploy` + verify Access app exists |
| `worker destroy` | `wrangler delete` + `DELETE /access/apps/{id}` |

---

## Part 5: Implementation Path

### Phase 1: Alert Worker MVP + Knock Setup (smallest useful version)

**Goal:** Email + Web Push + in-app alerts when the intelligence pipeline generates a new brief. This is the **first alert the founder would set up today**.

**Concrete first use case:** "Email me, push-notify me on desktop AND phone, and show it in my notification feed when a new intelligence brief is generated with high-relevance articles."

**What to build:**
1. **Knock account + workflow** — Create "new-intel-brief" workflow via MCP server or dashboard
   - Email step (connect Resend as email provider in Knock)
   - Web Push step (connect FCM as push provider in Knock)
   - In-app feed step
2. **Worker template** with `/health`, cron handler, single Knock `triggerWorkflow` call
3. **Web app MVP** at `app.tastematter.dev` (Svelte + CF Pages):
   - Knock notification feed (bell icon + notification cards)
   - Web Push registration (FCM token → Knock)
   - PWA manifest + service worker for iOS push
   - Protected by CF Access (your email = admin)
4. Single trigger type: `content_change` (poll D1 for new briefs)
5. D1 schema: `engagements` + `alert_history` + `alert_state` tables (see Design Decision #8)
6. `OWNER_ID` env var in wrangler.toml — used in Knock recipients, D1 queries (not hardcoded)
7. `tastematter worker init` and `tastematter worker deploy` CLI commands

**What to reuse:**
- Cron handler pattern: `download-alert-worker/src/index.ts`
- Config loading: `config.ts` pattern from intelligence pipeline
- D1 logging: flow_logs pattern from intelligence pipeline

**What Knock handles (zero custom code):**
- Email delivery (via Resend provider integration)
- Web Push delivery (via FCM provider integration)
- In-app feed (via Knock JS SDK)
- Template rendering, delivery retry, batching

**Estimated effort:** ~250 lines worker TypeScript (simpler — no notification routing code) + ~200 lines web app (Svelte notification center + push registration) + Knock config via MCP.

**Success criteria:**
- Founder receives email when new intel brief is generated
- Native push notification appears on desktop browser AND phone (PWA)
- Bell icon in web app shows notification with brief summary
- Alert fires within 4 hours of a new brief
- Knock delivery logs show successful multi-channel delivery

### Phase 2: Publishing MVP

**Goal:** Publish a directory as a queryable MCP source with CF Zero Trust auth.

**What to build:**
1. Add MCP endpoints to worker template (`/mcp`, `/sse`)
2. Add Durable Object (ContextDO) for corpus holding
3. Add MCP wrapper with `query` tool
4. `tastematter engagement init` + `tastematter access` CLI commands (CF API integration)
5. D1 query_log table
6. R2 corpus upload from CLI
7. CF Access Application + Policy creation via Cloudflare API
8. **Web app: publishing management views** (engagement dashboard, corpus status, query logs)

**What to reuse:**
- KnowledgeGraphDO: 85 lines, direct port
- MCP wrapper: 100 lines, direct port
- Query handler: 250 lines, direct port
- grep/read/list tools: 200 lines, direct port
- CF Access pattern: proven in intel pipeline + Nickel (zero worker auth code)

**Estimated effort:** ~200 lines new code + ~635 lines ported from CVI + ~150 lines CF API in CLI + ~400 lines web app views.

**Success criteria:**
- User can publish a directory as MCP source in under 5 minutes
- Claude Desktop can connect via CF service token and query the published context
- Cloudflare Zero Trust prevents unauthorized access (zero auth code in worker)
- Query logs visible via web app AND CLI

### Phase 3: Static Pages

**Goal:** Published contexts also serve rendered HTML dashboards.

**What to build:**
1. Page templates (dashboard, catalog, brief)
2. `/pages/*` route handler
3. Template rendering engine (string templates + CSS)
4. Web app: page template selection + preview

**What to reuse:**
- HTML rendering patterns from intelligence pipeline (800+ lines of proven templates)
- Dark theme CSS from briefs dashboard
- Stats/metrics patterns from `/stats` endpoint

**Estimated effort:** ~400 lines (mostly HTML/CSS templates).

### Phase 4: Web App Polish + Advanced Triggers

**Goal:** Full management experience and all trigger types.

**What to build:**
1. Complete web app: Alert Manager, Publish Manager, Access Manager views
2. All trigger types (pattern_match, threshold, schedule, corpus_drift)
3. Knock notification preferences UI (per-workflow channel opt-in/opt-out)
4. Alert history viewer with Knock delivery status

**Estimated effort:** ~1200 lines (Svelte web app views + Knock preferences integration).

**Future: Tauri desktop app** — If/when tastematter ships a desktop app, the same Svelte components from the web app can be embedded in Tauri. The Knock JS SDK works in both contexts. Web Push continues to work for push notifications, or Tauri can fire native OS notifications directly.

### Phase 5: Advanced Features

**Goal:** Multi-channel digest, analytics, team features.

**What to build:**
1. Digest format (batched alerts over time period)
2. Brief format (LLM-generated summary of changes, like intelligence pipeline)
3. Usage analytics dashboard in desktop app
4. Slack channel support (backward compatibility)
5. Auto-refresh on git push (webhook)

---

## Part 6: Cost Model

### Per-Worker Cloudflare Costs

Based on production data from existing systems:

| Resource | Free Tier | Estimated Usage | Monthly Cost |
|----------|-----------|-----------------|-------------|
| Workers requests | 100K/day | ~1K/day (alerts + queries) | $0 |
| D1 reads | 5M/day | ~10K/day | $0 |
| D1 writes | 100K/day | ~500/day | $0 |
| D1 storage | 5 GB | ~50 MB | $0 |
| R2 storage | 10 GB | ~50 MB (corpus + briefs) | $0 |
| R2 operations | 1M Class A, 10M Class B | ~5K/month | $0 |
| KV reads | 100K/day | ~1K/day | $0 |
| KV writes | 1K/day | ~50/day | $0 |
| Durable Objects | Included with Workers Paid ($5/mo) | 1 DO per worker | ~$0.05 |

**CVI production reference:** ~$1.52/month serving 1,585 files (36MB corpus).

**Knock notification costs:**
- Free tier: 10K notifications/month (sufficient for personal + early client use)
- Paid: $250/month for 50K notifications (when scaling to market phase)
- Includes: email routing, Web Push (FCM), in-app feed, Slack, delivery logs, workflow engine
- Replaces: separate Resend ($0-20/mo) + ntfy.sh ($0) + custom routing code

**Total estimated cost per worker:** $0-2/month on Workers Free, ~$5/month on Workers Paid (needed for Durable Objects). Knock free tier adds $0 for up to 10K notifications/month.

### Tastematter Pricing Model (suggestion)

| Tier | Workers | Features | Price |
|------|---------|----------|-------|
| Free | 1 engagement, alerting only | Email alerts, basic triggers, CF Access | $0 |
| Pro | 3 engagements, alerting + publishing | All triggers, MCP, pages, service tokens | $19/mo |
| Team | 10 engagements, everything | Shared access policies, analytics, custom domains | $49/mo |

Users bring their own Cloudflare account (free tier sufficient for most). Tastematter handles deployment, configuration, and monitoring.

---

## Part 7: Key Design Decisions

### 1. One Worker Per Context (not multi-tenant)

Each published context or alert configuration gets its own Cloudflare Worker. This matches the existing pattern (intelligence pipeline, CVI, Nickel are all separate workers) and keeps things simple:
- Isolation: one worker failure does not affect others
- Billing clarity: per-worker cost is transparent
- Simplicity: no routing layer, no tenant ID in every query

### 2. Knock as Unified Notification Layer (not DIY Resend + ntfy.sh + Slack)

**Rejected approach:** Three separate notification integrations (Resend for email, ntfy.sh for push, Slack webhooks) with custom routing/formatting/retry logic in worker code.

**Chosen approach:** [Knock](https://knock.app) handles all notification routing. Worker makes one API call. Knock delivers to email (via Resend/SendGrid provider), Web Push (via FCM), in-app feed, and Slack.

**Why Knock over DIY:**
- One `fetch()` call replaces three separate integrations
- Workflow engine handles batching, delays, digests — logic we'd otherwise build ourselves
- MCP server (`@knocklabs/agent-toolkit`) enables workflow configuration from Claude Code
- In-app feed component gives us a notification center for free
- Web Push via FCM gives native OS notifications on desktop AND mobile (no native app needed)
- Delivery logs, retry, observability — all built in
- Free tier (10K/month) sufficient for personal + early client use

**Why Web Push (FCM via Knock) over ntfy.sh:**
- Native OS notifications on desktop browsers + Android + iOS (as PWA)
- No third-party app install required (ntfy.sh requires its own phone app)
- Professional UX — notifications look like they come from tastematter, not "ntfy"
- Web app (`app.tastematter.dev`) serves as both management UI and push registration surface
- Future Tauri desktop app can reuse same FCM integration or fire native notifications directly

**Evolution:** ntfy.sh was the right v3 choice (zero infrastructure, immediate results). Knock is the right v4 choice (product-grade notification experience, scales to clients and market).

### 3. Corpus Snapshot Pattern (not live filesystem access)

The CVI pattern of snapshotting files into a JSON blob stored in R2 is proven to work well:
- Fast queries (entire corpus in Durable Object memory)
- No filesystem dependencies at runtime
- Versioned via git SHA
- Works with any source (git repos, local directories, arbitrary files)

The tradeoff is staleness: the corpus is a point-in-time snapshot. This is solved by configurable refresh schedules (cron) or webhook-triggered refreshes (git push).

### 4. Single `query` MCP Tool (not raw grep/read/list)

The CVI deployment proves that exposing a single high-level `query` tool is better than raw tools:
- Callers do not need to understand corpus structure
- The internal agent handles tool orchestration
- Better responses via agentic reasoning over raw tool output
- Simpler MCP integration for consumers

### 5. Templates Not Frameworks for Pages

HTML page rendering uses template strings + CSS, not React/Vue/Svelte. This is proven by the intelligence pipeline's 1000+ lines of HTML rendering that work reliably. Benefits:
- Zero build step for pages
- No JS framework dependency in workers
- Fast server-side rendering
- Easy to customize

### 6. Cloudflare Zero Trust for Auth (not custom API keys)

**Rejected approach:** API keys stored in KV, validated by middleware code in worker.
**Chosen approach:** Cloudflare Access at the edge, zero auth code in worker.

**Evidence from production systems:**
- Intelligence pipeline: CF Access protects `intel.tastematter.dev` with zero code changes
  [VERIFIED: `src/index.ts` has no auth middleware; `20_CONTEXT_PACKAGE_CUSTOM_DOMAIN_ACCESS.md`]
- Nickel workers: CF Access service tokens for machine-to-machine auth
  [VERIFIED: `cloudflare-access-auth-pattern.md` documents the pattern]
- CF shipped one-click Access for Workers (Oct 2025) and reusable policies (Dec 2025)

**Why this is better:**
- Zero auth code to write, test, or maintain
- Session management, audit logging, key rotation -- all handled by Cloudflare
- Service tokens (`CF-Access-Client-Id`/`CF-Access-Client-Secret`) work for MCP clients
- Reusable policies across all Workers in an engagement
- Defense in depth: edge validation + optional worker-side `checkServiceToken()` (from Nickel pattern)
- Free on CF Zero Trust free tier (up to 50 users)

**What this eliminates from the spec:**
- ~~KV namespace for API keys~~
- ~~`checkApiKey()` middleware function~~
- ~~Key generation/rotation logic in CLI~~
- ~~`ApiKey` type contract~~ (replaced by CF service token management)
- ~~Rate limit implementation~~ (CF handles this)

### 7. Engagement as Top-Level Noun (not "worker" or "context")

All publishing, access, and alerting is scoped to an engagement (e.g., "pixee", "personal"). This matches the GTM operating system's engagement model (`03_gtm_engagements/`) and enables:
- Linking related artifacts within an engagement (intel pages + MCP knowledge + PR results)
- Shared access policies per engagement (one CF Access app per engagement)
- CLI UX: `tastematter publish pages --engagement pixee` reads naturally
- Landing page generation: `https://pixee.tastematter.dev/` shows all Pixee artifacts

### 8. Clean Seams for Multi-Tenancy (not multi-tenant, not hardcoded)

**Decision:** Build for single user (me) but with clean architectural seams that make multi-tenancy a low-risk retrofit when needed. Don't build multi-tenancy infrastructure. Don't hardcode yourself either.

**Go-to-market reality:**
```
ME (now)       → Single admin, no self-serve needed
CLIENTS (next) → You set up engagements manually, clients consume via CF Access
MARKET (later) → Self-serve signup, billing, per-user isolation
```

**Do now (~1 day extra in Phase 1):**

1. **User ID variable everywhere** — `env.OWNER_ID` (not hardcoded `"dietl"`). In Knock recipient calls, D1 queries, web app context. Zero architectural cost, just discipline.

2. **D1 config storage (not local YAML)** — Engagement configs stored in D1, not `~/.tastematter/engagements/*.yaml`. The YAML structure stays the same, just persisted in D1. This avoids the hardest migration later (local files → database).

```sql
-- Config table (added to shared D1 schema)
CREATE TABLE IF NOT EXISTS engagements (
  id TEXT PRIMARY KEY,                    -- Slug: "pixee", "personal"
  owner_id TEXT NOT NULL,                 -- User ID: "dietl" (future: per-user)
  display_name TEXT NOT NULL,
  config_json TEXT NOT NULL,              -- Full EngagementConfig as JSON
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_engagements_owner ON engagements(owner_id);
```

3. **CF Access on web app** — `app.tastematter.dev` protected by CF Access from day one. Your email = admin. Client emails added per engagement. This IS the auth layer for "me" and "clients" phases — zero custom auth code.

4. **Engagement-scoped data everywhere** — All D1 tables include `engagement_id`. All R2 keys prefixed by engagement. Already naturally isolated by one-worker-per-engagement, but the seam is explicit.

**Explicitly defer to market phase:**
- ~~User registration / self-serve signup~~
- ~~Billing (Stripe integration)~~
- ~~Usage metering / plan limits~~
- ~~Per-user Cloudflare account management~~
- ~~Multi-user admin dashboards~~
- ~~Onboarding wizard~~

**When to build multi-tenancy:** When you have 3-5 paying clients and spend more time on manual setup than on the product. You'll know exactly which parts to automate because you'll have done them manually 3-5 times.

**Switching cost when the time comes (~7-10 days):**
- Add auth to web app (replace CF Access with proper user sessions): ~2-3 days
- Add per-user scoping to D1 queries (owner_id index already exists): ~1-2 days
- Billing integration (Stripe): ~3-5 days
- The D1 config + user ID variable + engagement-scoped data means zero data migration.

---

## Part 8: Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cloudflare API changes | Low | High | Pin wrangler version, integration tests |
| Corpus too large for DO memory | Medium | Medium | Size limits in UI (warn >50MB), chunking for large repos |
| Knock free tier exhaustion (10K/mo) | Low | Low | Upgrade to paid ($250/mo) when nearing limit; monitor in Knock dashboard |
| Worker cold start latency (MCP) | Low | Medium | DO warm-up on first request, edge caching |
| CF Access policy misconfiguration | Low | High | CLI validates policy after creation; `tastematter access list` for audit |
| Service token leak | Medium | Medium | Tokens scoped per engagement; revoke via CLI or dashboard; CF audit log |
| User does not have CF account | Medium | Medium | Clear onboarding, link to free tier signup |
| CF Zero Trust free tier limits | Low | Low | 50 users free; Pro tier for larger teams ($7/user/mo) |
| Multi-tenancy too early | Medium | Medium | Clean seams (Decision #8) cost ~1 day; full build costs 10-17 days. Defer until 3-5 clients. |
| iOS PWA push quirks | Low | Medium | Test on real iOS device early; Add to Home Screen requirement documented in onboarding |

---

## Part 9: Consolidated Type Contracts

All TypeScript interfaces for the Context Worker system. Designed for the **300ms comprehension rule** -- read any contract and immediately understand what goes in, what comes out, what transforms.

### Engagement Configuration (Top-Level)

```typescript
// Stored in D1 `engagements` table as config_json (see Design Decision #8)
// NOT local YAML — enables future multi-tenancy without data migration.
// The top-level organizing unit. Everything published belongs to an engagement.
// Progressive disclosure: Level 1 = name + artifacts, Level 2 = access, Level 3 = alerting

interface EngagementConfig {
  name: string;                          // Slug (e.g., "pixee") — D1 primary key
  owner_id: string;                      // From env.OWNER_ID (not hardcoded)
  display_name: string;                  // Human-readable (e.g., "Pixee AI")

  // What's published under this engagement
  artifacts: PublishedArtifact[];

  // Cloudflare Zero Trust access (managed via CLI, stored here for reference)
  access: AccessConfig;

  // Deployment
  deployment: DeploymentConfig;

  // Alerting (optional)
  alerting?: AlertingConfig;
}

// A single published artifact within an engagement
// Two types: "pages" (static HTML) or "context" (queryable MCP)
type PublishedArtifact =
  | PageArtifact
  | ContextArtifact;

interface PageArtifact {
  type: 'pages';
  name: string;                          // e.g., "linkedin-intel"
  template: PageTemplate;                // dashboard | brief | catalog | log | custom
  source: PageDataSource;
  url_path: string;                      // e.g., "/intel/" (relative to engagement domain)
}

interface ContextArtifact {
  type: 'context';
  name: string;                          // e.g., "knowledge"
  source: CorpusSource;
  mcp_tool_name?: string;                // Default: "query"
  url_path: string;                      // e.g., "/mcp" (relative to engagement domain)
}
```

### Access Configuration (Cloudflare Zero Trust)

```typescript
// Wraps Cloudflare Zero Trust API resources.
// CLI creates/manages these via CF API; this config tracks the IDs.
// [VERIFIED: _system/patterns/cloudflare-access-workers.md — proven pattern]

interface AccessConfig {
  // CF Access Application (created via POST /access/apps)
  application_id?: string;               // Set after first `engagement init`
  application_name?: string;             // e.g., "pixee-tastematter"

  // Who can access (managed via CLI, reflected here)
  emails: string[];                      // e.g., ["jake@pixee.dev"]

  // Service tokens for programmatic access (CLI, MCP clients)
  service_tokens: ServiceTokenRef[];
}

// Reference to a CF Access service token (secret never stored locally)
interface ServiceTokenRef {
  name: string;                          // User-provided label (e.g., "claude-desktop")
  token_id: string;                      // CF API resource ID
  client_id: string;                     // CF-Access-Client-Id value
  created_at: string;                    // ISO timestamp
  // client_secret is NEVER stored — shown once at creation time
}
```

### Worker & Source Configuration

```typescript
interface DeploymentConfig {
  region: CloudflareRegion;
  worker_name: string;                   // Auto-generated: tm-{engagement_name}
  domain: string;                        // e.g., "pixee.tastematter.dev"
}

interface CorpusSource {
  type: 'corpus' | 'feed' | 'hybrid';
  paths: string[];                       // Glob patterns (e.g., "knowledge_base/**/*.md")
  exclude?: string[];                    // Glob exclusions
  repo?: string;                         // Git repo root (optional)
}

// See "Notification Configuration (Knock)" section for full AlertingConfig
// Channels are managed in Knock (not in this config)

type CloudflareRegion =
  | 'us-east-1' | 'us-west-1' | 'eu-west-1'
  | 'eu-central-1' | 'ap-northeast-1' | 'ap-southeast-1';
```

### Alert Types

```typescript
// Watch rule: what to watch, when to check, how to notify
interface WatchRule {
  name: string;
  trigger: TriggerType;
  schedule: string;                      // Cron expression
  config: TriggerConfig;
  channels: string[];                    // Channel names from AlertingConfig.channels
  format: AlertFormat;
  enabled: boolean;
}

type TriggerType = 'content_change' | 'pattern_match' | 'threshold' | 'schedule' | 'corpus_drift';
type AlertFormat = 'instant' | 'digest' | 'brief';

// Trigger-specific configuration (discriminated union)
type TriggerConfig =
  | { type: 'content_change'; paths: string[]; min_changes?: number }
  | { type: 'pattern_match'; pattern: string; case_insensitive?: boolean }
  | { type: 'threshold'; metric: string; operator: '>' | '<' | '='; value: number }
  | { type: 'schedule' }
  | { type: 'corpus_drift'; max_commits_behind?: number };

// Alert payload: the data that flows through channels
interface Alert {
  id: string;                            // UUID
  rule_name: string;
  trigger_type: TriggerType;
  fired_at: string;                      // ISO timestamp
  subject: string;                       // Short summary line
  body: string;                          // Detailed content (plain text)
  html?: string;                         // Rich HTML content (for email)
  metadata: {
    worker_name: string;
    changes?: string[];                  // Changed file paths (for content_change)
    matched_files?: string[];            // Matched files (for pattern_match)
    metric_value?: number;               // Current value (for threshold)
    corpus_sha?: string;                 // Current corpus SHA
    source_sha?: string;                 // Latest source SHA (for corpus_drift)
  };
}
```

### Notification Configuration (Knock)

```typescript
// Notification routing is handled entirely by Knock.
// No per-channel configuration in worker code or worker config.
// Channels (email, push, slack, in-app) are configured as workflow steps in Knock
// via the Knock dashboard or MCP server (@knocklabs/agent-toolkit).
//
// The worker only needs:
//   1. KNOCK_API_KEY (secret)
//   2. Workflow key (string, e.g., "new-intel-brief")
//   3. Recipient IDs (Knock user IDs)

interface AlertingConfig {
  provider: 'knock';
  workflow_key: string;                    // Knock workflow key
  recipients: KnockRecipient[];            // Who to notify
  rules: WatchRule[];                      // When to notify (trigger conditions)
}

interface KnockRecipient {
  id: string;                              // Knock user ID
  email?: string;                          // For user creation/sync
  name?: string;                           // Display name in Knock
}

// Knock workflow trigger payload (sent from worker → Knock API)
interface KnockTriggerPayload {
  recipients: string[];                    // Knock user IDs
  data: {
    subject: string;                       // Alert subject line
    body: string;                          // Plain text body
    html?: string;                         // Rich HTML (for email template)
    url?: string;                          // Click-through URL
    trigger_type: TriggerType;
    changes?: string[];                    // Changed file paths
    matched_files?: string[];              // Pattern match results
    metric_value?: number;                 // Threshold value
    corpus_sha?: string;                   // Current corpus version
  };
}
```

**Knock workflow steps (configured in Knock, NOT in code):**

| Step | Knock Step Type | Provider | What User Sees |
|------|----------------|----------|----------------|
| Email | `createOrUpdateEmailStepInWorkflow` | Resend (or SendGrid) | Email in inbox |
| Web Push | `createOrUpdatePushStepInWorkflow` | FCM | Native OS notification (desktop + mobile) |
| In-app feed | `createOrUpdateInAppFeedStepInWorkflow` | Knock built-in | Bell icon badge in web app |
| Slack | `createOrUpdateChatStepInWorkflow` | Slack webhook | Slack message |
| Batch | `createOrUpdateBatchStepInWorkflow` | N/A | Groups alerts over time window |
| Delay | `createOrUpdateDelayStepInWorkflow` | N/A | Waits before next step |

### Publishing Types

#### Corpus Snapshot

```typescript
// Direct port from CVI: generate-corpus.ts
// [VERIFIED: apps/cv_agentic_knowledge/.../scripts/generate-corpus.ts:7-22]

interface CorpusSnapshot {
  version: string;                       // Schema version (currently "1.0")
  commit: string;                        // Git SHA for versioning
  fileCount: number;
  totalSize: number;                     // Total bytes
  generatedAt: string;                   // ISO timestamp
  files: Record<string, FileEntry>;      // Path -> content map
  allPaths: string[];                    // Pre-computed path index (files + directories)
}

interface FileEntry {
  path: string;
  content: string;
  size: number;
  frontmatter?: Record<string, unknown>; // Parsed YAML frontmatter
}
```

#### Page Publisher

```typescript
// Configuration for static page rendering
// Proven patterns extracted from intelligence pipeline HTML dashboards
// [VERIFIED: apps/intelligence_pipeline/src/index.ts — renderBriefView, renderStats, etc.]

interface PagePublisher {
  name: string;                          // Page route name (e.g., "dashboard")
  template: PageTemplate;                // Which template to render
  source: PageDataSource;                // Where data comes from
  custom_css?: string;                   // Optional CSS overrides
  title?: string;                        // Page title override
  // Access: handled by Cloudflare Zero Trust at the edge (not in code)
  // All pages inherit the engagement's CF Access policy
}

type PageTemplate = 'dashboard' | 'brief' | 'catalog' | 'log' | 'custom';

type PageDataSource =
  | { type: 'corpus'; filter?: string }  // Render from corpus snapshot
  | { type: 'd1'; query: string }        // Render from D1 query results
  | { type: 'r2'; prefix: string };      // Render from R2 objects
```

#### Cloudflare API Request Types

```typescript
// Types for CLI → Cloudflare Zero Trust API integration.
// The CLI creates/manages these resources; the worker never touches them.
// [VERIFIED: Context7 Cloudflare docs — Zero Trust Access API]

// POST /accounts/{account_id}/access/apps
interface CreateAccessAppRequest {
  name: string;                          // e.g., "pixee-tastematter"
  domain: string;                        // e.g., "pixee.tastematter.dev"
  type: 'self_hosted';
  session_duration?: string;             // Default: "24h"
  auto_redirect_to_identity?: boolean;   // Default: false
}

// POST /accounts/{account_id}/access/apps/{app_id}/policies
interface CreateAccessPolicyRequest {
  name: string;                          // e.g., "Allow Pixee team"
  decision: 'allow';
  precedence: number;
  include: AccessPolicyRule[];
}

type AccessPolicyRule =
  | { email: { email: string } }                    // Specific email
  | { email_domain: { domain: string } }            // Email domain
  | { service_token: Record<string, never> }         // Any service token
  | { service_token: { token_id: string } };         // Specific service token

// POST /accounts/{account_id}/access/service_tokens
interface CreateServiceTokenRequest {
  name: string;                          // e.g., "claude-desktop"
}

// Response (client_secret shown ONCE — CLI must display immediately)
interface CreateServiceTokenResponse {
  client_id: string;                     // CF-Access-Client-Id header value
  client_secret: string;                 // CF-Access-Client-Secret header value (one-time)
  id: string;                            // Token resource ID (for revocation)
  name: string;
  created_at: string;
}
```

#### Query Log

```typescript
// Tracks every MCP query for analytics and debugging

interface QueryLog {
  id: number;                            // Auto-increment (D1)
  timestamp: string;                     // ISO timestamp
  query: string;                         // User's question
  response_length: number;               // Response size in chars
  duration_ms: number;                   // End-to-end latency
  tool_calls: number;                    // How many grep/read/list calls
  cf_access_client_id?: string;          // Service token ID (from CF-Access-Jwt-Assertion)
  corpus_commit: string;                 // Which corpus version was queried
  success: boolean;
  error_message?: string;
}
```

### Environment Bindings

```typescript
// Cloudflare Worker environment (wrangler.toml bindings)
//
// AUTH NOTE: No auth-related bindings. Cloudflare Access validates at the edge
// before requests reach the worker. The worker trusts all incoming requests.
// CF injects CF-Access-Jwt-Assertion header with identity claims.

interface Env {
  // Storage
  DB: D1Database;                        // Alert state, query logs, activity logs
  CORPUS_BUCKET: R2Bucket;               // Corpus snapshots, generated briefs
  CONFIG: KVNamespace;                   // Worker config (NO API keys — CF Access handles auth)

  // Durable Objects
  CONTEXT_DO: DurableObjectNamespace;    // Corpus-in-memory holder (KnowledgeGraphDO)
  MCP_OBJECT: DurableObjectNamespace;    // MCP protocol handler (McpAgent)

  // Secrets
  ANTHROPIC_API_KEY: string;             // For agentic query handler
  KNOCK_API_KEY: string;                 // Knock API key (triggers workflows)

  // Identity (see Design Decision #8: Clean Seams)
  OWNER_ID: string;                      // User ID — NOT hardcoded. Set in wrangler.toml [vars].
                                         // Used in: Knock recipients, D1 owner_id, web app context.
                                         // Future multi-tenancy: replaced by auth-derived user ID.
}

// CLI-side config (NOT in worker — used by tastematter CLI to call CF API)
interface CliConfig {
  cloudflare_account_id: string;         // CF account ID
  cloudflare_api_token: string;          // CF API token (scoped to Access + Workers)
}
```

---

## Part 10: Generalization Guide

How to extract proven code from existing systems into the generic Context Worker template. Each subsection maps a source file to its generalized form.

### Corpus Generation (CVI -> tastematter CLI)

**Source:** `apps/cv_agentic_knowledge/.../scripts/generate-corpus.ts` (133 lines)
**Target:** `apps/tastematter/core/src/corpus.rs` (Rust port) + `apps/tastematter/worker-template/scripts/generate-corpus.ts` (TS fallback)

**What to generalize:**
- Replace hardcoded `"*.md" "*.yaml"` with configurable `source.paths` from WorkerConfig
- Add `source.exclude` support via glob negation
- Add `--output` flag (currently writes to stdout)
- Add `--dry-run` flag to preview file selection
- Port `buildPathIndex()` as-is (generic utility)
- Port frontmatter parsing as-is (works with any markdown)

**What stays the same:**
- `git ls-files` for source discovery (proven, respects .gitignore)
- `CorpusSnapshot` JSON format (universal)
- `FileEntry` structure (path, content, size, frontmatter)

### Durable Object (CVI -> Worker Template)

**Source:** `apps/cv_agentic_knowledge/.../src/durable-objects/knowledge-graph-do.ts` (85 lines)
**Target:** `apps/tastematter/worker-template/src/durable-objects/context-do.ts`

**What to generalize:**
- Rename `KnowledgeGraphDO` -> `ContextDO`
- Rename `KNOWLEDGE_GRAPH_DO` binding -> `CONTEXT_DO`
- Add config loading from KV (currently hardcoded bucket name)
- Add corpus version tracking (expose `corpus.commit` via `/health`)

**What stays the same:**
- Lazy loading pattern from R2 (exact same code)
- `/grep`, `/read`, `/list` internal endpoints (exact same code)
- `/reload` and `/health` endpoints
- In-memory corpus holding (no changes)

### MCP Wrapper (CVI -> Worker Template)

**Source:** `apps/cv_agentic_knowledge/.../src/mcp-wrapper.ts` (100 lines)
**Target:** `apps/tastematter/worker-template/src/mcp-wrapper.ts`

**What to generalize:**
- Rename `KnowledgeGraphMCP` -> `ContextMCP`
- Make `tool_name` configurable from `publishing.mcp.tool_name` (default: "query")
- Make tool description configurable (describe what this context contains)
- Load Anthropic API key from env (already does this)

**What stays the same:**
- `McpAgent` class extension pattern
- Single `query` tool architecture
- `executeAgenticQueryStreaming()` call

### Query Handler (CVI -> Worker Template)

**Source:** `apps/cv_agentic_knowledge/.../src/query-handler.ts` (600 lines)
**Target:** `apps/tastematter/worker-template/src/query-handler.ts`

**What to generalize:**
- Make system prompt configurable (currently CVI-specific language about "knowledge graph")
- Default system prompt should be generic: "You are a knowledge base assistant for {display_name}"
- Make model configurable (currently hardcoded `claude-haiku-4-5-20251001`)
- Add query logging to D1 (currently no logging)

**What stays the same:**
- Both `executeAgenticQuery()` and `executeAgenticQueryStreaming()` functions
- Tool definitions (grep, read, list) -- generic by design
- `MAX_TOOL_RESULT_CHARS` truncation (50,000 chars)
- `betaTool` SDK integration pattern
- DO stub communication pattern (`http://internal/grep`, etc.)

### Notification Delivery (Knock replaces custom notification code)

**Source (legacy):** `apps/intelligence_pipeline/src/generation/notifications.ts` (160 lines)
**Source (legacy):** `apps/tastematter/download-alert-worker/src/index.ts` (ntfy.sh pattern)
**Target:** Single `triggerAlert()` function that calls Knock API

**What's eliminated by Knock:**
- ~~Custom email rendering~~ → Knock template editor
- ~~Custom push formatting~~ → Knock push step
- ~~Slack Block Kit building~~ → Knock chat step
- ~~Multi-channel routing logic~~ → Knock workflow engine
- ~~Delivery retry logic~~ → Knock built-in retry
- ~~Digest/batch accumulation~~ → Knock batch step

**What remains in worker code:**
- Alert payload construction (subject, body, metadata from trigger evaluation)
- Single `fetch()` to `api.knock.app/v1/workflows/{key}/trigger`
- D1 logging of alert_history (local record)

**Reference:** Intel pipeline `notifications.ts` patterns (urgency hierarchy, breaking alert detection) may inform Knock template design but are NOT ported as code.

### Auth: Cloudflare Zero Trust (no worker code needed)

**Source:** `_system/patterns/cloudflare-access-workers.md` (proven pattern)
**Source:** `apps/intelligence_pipeline/20_CONTEXT_PACKAGE_CUSTOM_DOMAIN_ACCESS.md` (production evidence)
**Target:** Cloudflare dashboard + CLI automation (NOT worker code)

**What happens at deploy time (CLI handles this):**
1. `tastematter engagement init pixee` creates CF Access Application via API
2. `tastematter access grant --email jake@pixee.dev` adds email to Access policy
3. `tastematter access token create --name claude-desktop` creates CF service token

**What happens at request time (zero worker code):**
1. Request arrives at Cloudflare edge
2. CF Access validates: browser session (email) OR service token headers
3. If valid: request forwarded to worker with `CF-Access-Jwt-Assertion` header
4. If invalid: 302 redirect (browser) or 403 (API)
5. Worker trusts all requests (edge already validated)

**Optional defense in depth (from Nickel pattern):**
The Nickel `checkServiceToken()` function can be added as a secondary gate, but is NOT required when CF Access is enforcing at the edge. Include only for high-security engagements.

```typescript
// Optional: defense in depth (NOT the primary auth gate)
// Port from: apps/clients/nickel/conference_pr/worker/src/index.ts:242-255
function checkServiceToken(request: Request, env: Env): Response | null { ... }
```

### Cron Handler (Download Alert Worker -> Worker Template)

**Source:** `apps/tastematter/download-alert-worker/src/index.ts` (100 lines)
**Target:** `apps/tastematter/worker-template/src/scheduled.ts`

**What to generalize:**
- Replace hardcoded GraphQL query with generic trigger evaluation
- Load `WatchRule[]` from KV config instead of hardcoded logic
- Iterate rules, evaluate conditions, fire alerts
- Add `alert_state` D1 updates (last_checked_at, last_fired_at)
- Add `alert_history` D1 logging

**What stays the same:**
- `scheduled()` handler pattern (cron entry point)
- Filter-then-notify flow (evaluate trigger → build payload → call Knock)

---

## Part 11: Verification Criteria

Since this is a design spec (no code yet), verification = review against these criteria:

### Pattern Coverage

| System | Key Pattern | Covered In Spec |
|--------|------------|-----------------|
| Intelligence Pipeline | Cron-triggered classification + brief generation | Part 2: Watch Rules + Alert Formats |
| Intelligence Pipeline | Notification formatting patterns | Part 2: Knock workflows (templates in Knock, not code) |
| Intelligence Pipeline | HTML dashboard rendering (1000+ lines) | Part 3: Static Page Publishing |
| Intelligence Pipeline | YAML topic-as-config | Part 1: Worker Config Model |
| CVI Knowledge Graph | Corpus snapshot generation | Part 3: Queryable MCP + Part 9: CorpusSnapshot type |
| CVI Knowledge Graph | DO corpus-in-memory pattern | Part 1: Architecture (ContextDO) |
| CVI Knowledge Graph | MCP wrapper with single query tool | Part 3: MCP Tool Exposure |
| CVI Knowledge Graph | Agentic query handler with streaming | Part 10: Query Handler generalization |
| Nickel Conference PR | CF Access service token auth | Part 10: Auth Middleware generalization |
| Nickel Conference PR | D1 flow logging | Part 4: D1 Schema (activity_log table) |
| Nickel Conference PR | Stage-based pipeline pattern | Part 2: Alert Processing Flow |
| Download Alert Worker | Cron -> query -> filter -> notify | Part 2: Alert Processing Flow + Part 10 |

### Feasibility Checks

- [x] Every component runs in a CF Worker (all patterns already proven in Workers)
- [x] Knock API is a single `fetch()` call from CF Worker (replaces 3 separate integrations)
- [x] Knock free tier: 10K notifications/month (sufficient for personal + early client use)
- [x] FCM Web Push works on desktop browsers + Android + iOS (as PWA) — no native app needed
- [x] D1 handles all state storage needs (alert_history, query_log, activity_log)
- [x] R2 handles all blob storage (corpus snapshots, generated briefs, pages)
- [x] KV handles worker config; auth handled by CF Access at edge (zero KV for keys)
- [x] Durable Objects handle stateful corpus (proven in CVI at 36MB / 1,585 files)
- [x] Svelte web app on CF Pages serves management UI + Knock notification center + PWA push registration

### Cost Verification

- Knock free tier: 10K notifications/month (sufficient for MVP and early clients)
- Knock paid: $250/month for 50K (when scaling to market phase)
- CF Workers free tier: 100K requests/day (sufficient for most users)
- CF Pages: Free for static sites (web app hosting)
- DO: Requires Workers Paid ($5/month) -- call this out in onboarding
- CVI production cost reference: ~$1.52/month for 1,585 files, 36MB corpus

### Spec Alignment

- Extends spec 10 (MCP Publishing Architecture) with alerting + static pages + engagement model
- **Supersedes** spec 10's API key auth with Cloudflare Zero Trust (edge auth, zero code)
- **Supersedes** v3's ntfy.sh + Resend with Knock as unified notification layer
- Inherits type contracts: `QueryLog`, `CorpusSnapshot` from spec 10
- **Replaces** `ApiKey` and `PublisherConfig` with `AccessConfig` and `EngagementConfig`
- **Replaces** `EmailChannel`, `NtfyChannel`, `SlackChannel` with `KnockTriggerPayload` + Knock-managed workflows
- Adds new types: `WatchRule`, `Alert`, `AlertingConfig`, `KnockRecipient`, `PagePublisher`, `PublishedArtifact`
- Adds new surface: web app (`app.tastematter.dev`) for publishing management + notifications + PWA push
- CLI scoped to engagement: `tastematter engagement`/`publish`/`access`/`alert`/`corpus`
- Auth proven across: intelligence pipeline (edge), Nickel (edge + defense in depth)

---

## Appendix A: Existing Code Inventory

### Intelligence Pipeline (`apps/intelligence_pipeline/`)

| File | Lines | Reusable For |
|------|-------|-------------|
| `src/index.ts` | 1291 | HTML page rendering templates, router pattern |
| `src/shared/types.ts` | 667 | Type contract patterns (TopicConfig, logging) |
| `src/shared/config.ts` | 163 | YAML config loading, validation pattern |
| `src/generation/notifications.ts` | 160 | Notification formatting (adapt for email) |
| `src/generation/worker.ts` | ~200 | Cron-triggered generation pattern |
| `wrangler.toml` | 50 | D1 + R2 + cron binding pattern |
| `configs/context_engineering.yaml` | 186 | Topic-as-config pattern |

### CVI Knowledge Graph (`apps/cv_agentic_knowledge/app/deployments/corporate-visions/`)

| File | Lines | Reusable For |
|------|-------|-------------|
| `src/index.ts` | 209 | MCP + HTTP endpoint routing |
| `src/query-handler.ts` | 600 | Agentic query with tools (blocking + streaming) |
| `src/mcp-wrapper.ts` | 100 | MCP protocol integration |
| `src/durable-objects/knowledge-graph-do.ts` | 85 | Corpus-in-memory DO pattern |
| `src/tools/grep.ts` | ~80 | Grep tool implementation |
| `src/tools/read.ts` | ~50 | Read tool implementation |
| `src/tools/list.ts` | ~70 | List tool implementation |
| `src/query-logging.ts` | ~100 | R2-based query logging |
| `scripts/generate-corpus.ts` | 133 | Corpus generation (direct port) |

### Nickel Conference PR Worker (`apps/clients/nickel/conference_pr/worker/`)

| File | Lines | Reusable For |
|------|-------|-------------|
| `src/index.ts` | 263 | CF Access auth pattern, pipeline routing |
| `src/config.ts` | 30 | YAML config loading with caching |
| `src/pipeline/` | ~400 | Stage-based pipeline pattern |
| `src/corpus/corpus-do.ts` | ~80 | Client corpus DO variant |

### Download Alert Worker (`apps/tastematter/download-alert-worker/`)

| File | Lines | Reusable For |
|------|-------|-------------|
| `src/index.ts` | 100 | Cron + filter + notify pattern (ntfy.sh legacy; cron handler pattern still useful) |

### MCP Publishing Spec (`apps/tastematter/specs/canonical/10_MCP_PUBLISHING_ARCHITECTURE.md`)

| Section | Reusable For |
|---------|-------------|
| Type contracts (PublisherConfig, ApiKey, QueryLog) | Direct adoption |
| CLI command design | Direct adoption with namespace change |
| UI wireframes | Design reference |
| Auth flow diagram | Implementation reference |

---

## Appendix B: File Placement

This spec: `apps/tastematter/specs/canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md`

Worker template (to be created): `apps/tastematter/worker-template/`

Related context packages: `apps/tastematter/specs/context_packages/06_alerting_publishing/`

---

**Last Updated:** 2026-02-15
**Status:** Draft v4 -- Knock + web app + clean multi-tenancy seams. Ready for Phase 1 implementation.
**Revision History:**
- v4 (2026-02-15): Replaced ntfy.sh + Resend + Slack with Knock as unified notification infrastructure. Added web app (`app.tastematter.dev`) for publishing management + Knock notification center + Web Push registration (PWA for iOS/Android). Knock MCP server enables workflow configuration from Claude Code. Simplified worker code (one fetch to Knock replaces three integrations). Added Web Push via FCM for native desktop + mobile browser notifications without native app. Added Design Decision #8: clean seams for multi-tenancy (D1 config, OWNER_ID variable, engagement-scoped data, CF Access on web app) — ~1 day extra, prevents expensive retrofit. D1 schema updated with `engagements` table + `engagement_id` columns. Updated all type contracts, architecture diagrams, implementation phases, cost model, risk assessment.
- v3 (2026-02-14): Replaced Web Push with ntfy.sh (proven in download-alert-worker). Added go-to-market progression (me → clients → market). Added first use cases from founder interview.
- v2 (2026-02-13): Replaced API key auth with Cloudflare Zero Trust. Added engagement as top-level noun. Added CF API type contracts.
- v1 (2026-02-13): Initial draft.
**Next Action:** Phase 1 implementation — Alert Worker MVP + Knock setup + web app MVP with notification center + Web Push registration
