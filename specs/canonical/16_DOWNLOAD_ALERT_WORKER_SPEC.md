# 16: Download Alert Worker

## Problem

Tastematter distributes binaries via Cloudflare R2 custom domain (`install.tastematter.dev`). There is no visibility into when someone downloads a binary. User wants an alert on each download.

## Epistemic Grounding

| Assumption | Status | Evidence |
|------------|--------|----------|
| R2 has native download events | DISPROVEN | R2 Event Notifications only support `object-create` and `object-delete` |
| R2 has access logs | DISPROVEN | R2 Audit Logs explicitly exclude data access operations (GetObject) |
| R2 has analytics for GetObject | VERIFIED | `r2OperationsAdaptiveGroups` GraphQL dataset tracks GetObject with `objectName` dimension |
| GraphQL Analytics API is free | VERIFIED | Available on all plans including Free, 31-day retention |
| No Worker infrastructure exists for tastematter | VERIFIED | Zero `wrangler.toml` files in tastematter directory |

## Architecture

```
                    ┌──────────────────────┐
                    │  Cron Trigger (15m)   │
                    └──────────┬───────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────┐
│  download-alert-worker                               │
│                                                      │
│  1. Query CF GraphQL API:                            │
│     r2OperationsAdaptiveGroups                       │
│     filter: bucketName="tastematter-releases"        │
│            actionType="GetObject"                    │
│            datetime_geq=(now - 20min)                │
│                                                      │
│  2. Filter results: objectName matches               │
│     releases/v*/tastematter-* OR                     │
│     staging/latest/tastematter-*                     │
│                                                      │
│  3. If matches > 0 → POST to Slack webhook           │
└──────────────────────────────────────────────────────┘

Existing infra (UNCHANGED):
  install.sh → curl → install.tastematter.dev → R2 bucket
```

Zero changes to download path. Worker only reads analytics, never touches R2 directly.

## Components

**1 Worker, 0 databases, 0 queues.**

| Component | Purpose |
|-----------|---------|
| `download-alert-worker` | Cron-triggered Worker that polls R2 analytics and sends Slack alerts |

No KV, no D1, no Durable Objects. The 15-min cron + 20-min query window (with overlap) eliminates the need for state tracking. Duplicate alerts for the same download across windows are acceptable at this volume.

## Type Contracts

```typescript
// --- GraphQL Request ---

interface R2AnalyticsVariables {
  accountTag: string;
  startDate: string;   // ISO 8601
  endDate: string;     // ISO 8601
  bucketName: string;  // "tastematter-releases"
}

// --- GraphQL Response ---

interface R2OperationsGroup {
  sum: { requests: number };
  dimensions: {
    objectName: string;   // e.g. "releases/v0.1.0-alpha.15/tastematter-linux-x86_64"
    datetime: string;
  };
}

interface R2AnalyticsResponse {
  data: {
    viewer: {
      accounts: Array<{
        r2OperationsAdaptiveGroups: R2OperationsGroup[];
      }>;
    };
  };
  errors?: Array<{ message: string }>;
}

// --- Parsed Download ---

interface BinaryDownload {
  objectName: string;
  platform: string;     // "linux-x86_64" | "darwin-aarch64" | "windows-x86_64" | "darwin-x86_64"
  version: string;      // "v0.1.0-alpha.15" | "staging"
  channel: string;      // "production" | "staging"
  requests: number;
}

// --- Slack Payload ---

interface SlackMessage {
  text: string;
  blocks?: Array<{
    type: string;
    text?: { type: string; text: string };
  }>;
}

// --- Worker Env ---

interface Env {
  CF_ACCOUNT_ID: string;       // Cloudflare account tag
  CF_API_TOKEN: string;        // API token with Account Analytics:Read
  SLACK_WEBHOOK_URL: string;   // Slack incoming webhook URL
}
```

## Implementation

### File Structure

```
apps/tastematter/download-alert-worker/
├── wrangler.toml       # Cron trigger config
├── src/
│   └── index.ts        # Single file — query + filter + alert
├── package.json        # Minimal (wrangler only)
└── tsconfig.json       # Standard CF Worker config
```

### wrangler.toml

```toml
name = "tastematter-download-alerts"
main = "src/index.ts"
compatibility_date = "2024-12-01"
account_id = "<TASTEMATTER_CF_ACCOUNT_ID>"

[triggers]
crons = ["*/15 * * * *"]    # Every 15 minutes

[observability]
enabled = true

# Secrets (set via `printf "value" | wrangler secret put NAME`):
# CF_ACCOUNT_ID       - Cloudflare account tag (for GraphQL API)
# CF_API_TOKEN         - API token with Account Analytics:Read scope
# SLACK_WEBHOOK_URL    - Slack incoming webhook URL
```

### src/index.ts — Full Implementation

```typescript
interface Env {
  CF_ACCOUNT_ID: string;
  CF_API_TOKEN: string;
  SLACK_WEBHOOK_URL: string;
}

const BUCKET = "tastematter-releases";
const BINARY_PATTERN = /^(releases\/v[^/]+|staging\/latest)\/tastematter-/;

const QUERY = `
query R2Downloads($accountTag: String!, $start: Time!, $end: Time!) {
  viewer {
    accounts(filter: { accountTag: $accountTag }) {
      r2OperationsAdaptiveGroups(
        limit: 100
        filter: {
          bucketName: "${BUCKET}"
          actionType: "GetObject"
          datetime_geq: $start
          datetime_leq: $end
        }
      ) {
        sum { requests }
        dimensions { objectName datetime }
      }
    }
  }
}`;

export default {
  async scheduled(event: ScheduledEvent, env: Env, ctx: ExecutionContext) {
    const end = new Date(event.scheduledTime);
    const start = new Date(end.getTime() - 20 * 60 * 1000); // 20 min window

    // 1. Query R2 analytics
    const resp = await fetch("https://api.cloudflare.com/client/v4/graphql", {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${env.CF_API_TOKEN}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        query: QUERY,
        variables: {
          accountTag: env.CF_ACCOUNT_ID,
          start: start.toISOString(),
          end: end.toISOString(),
        },
      }),
    });

    if (!resp.ok) {
      console.error(`GraphQL API error: ${resp.status}`);
      return;
    }

    const json = await resp.json() as any;
    const groups = json.data?.viewer?.accounts?.[0]?.r2OperationsAdaptiveGroups ?? [];

    // 2. Filter for binary downloads
    const downloads = groups.filter((g: any) =>
      BINARY_PATTERN.test(g.dimensions.objectName)
    );

    if (downloads.length === 0) return; // No binary downloads — done

    // 3. Format and send Slack alert
    const lines = downloads.map((d: any) => {
      const name = d.dimensions.objectName;
      const count = d.sum.requests;
      const match = name.match(/releases\/(v[^/]+)/) || name.match(/(staging\/latest)/);
      const version = match ? match[1] : "unknown";
      const platform = name.split("/").pop() ?? "unknown";
      return `• *${platform}* (${version}) — ${count} download${count > 1 ? "s" : ""}`;
    });

    const total = downloads.reduce((s: number, d: any) => s + d.sum.requests, 0);
    const text = `📦 *${total} tastematter download${total > 1 ? "s" : ""}* in the last 20 min\n${lines.join("\n")}`;

    await fetch(env.SLACK_WEBHOOK_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    });
  },

  async fetch(request: Request, env: Env): Promise<Response> {
    // Health check endpoint (optional, for manual verification)
    return new Response(JSON.stringify({ status: "ok", worker: "tastematter-download-alerts" }), {
      headers: { "Content-Type": "application/json" },
    });
  },
};
```

## Secrets Setup

Three secrets, set via `printf` (not `echo` — avoids trailing newline):

```bash
cd apps/tastematter/download-alert-worker

# 1. CF Account ID (from Cloudflare dashboard → Account Home → right sidebar)
printf "YOUR_ACCOUNT_TAG" | wrangler secret put CF_ACCOUNT_ID

# 2. CF API Token (create at dash.cloudflare.com/profile/api-tokens)
#    Permissions needed: Account Analytics:Read
printf "YOUR_API_TOKEN" | wrangler secret put CF_API_TOKEN

# 3. Slack Incoming Webhook URL (from Slack app settings)
printf "https://hooks.slack.com/services/T.../B.../xxx" | wrangler secret put SLACK_WEBHOOK_URL
```

**IMPORTANT:** `account_id` in wrangler.toml must match the account that owns the `tastematter-releases` R2 bucket. The `R2_ENDPOINT` secret in GitHub Actions contains this — format is `https://<ACCOUNT_ID>.r2.cloudflarestorage.com`.

## Deployment

```bash
cd apps/tastematter/download-alert-worker
npm install
wrangler deploy
```

That's it. Cron starts firing immediately after deploy.

## Verification

```bash
# 1. Check Worker is deployed
wrangler deployments list

# 2. Check cron is registered (Cloudflare dashboard → Workers → tastematter-download-alerts → Triggers)

# 3. Trigger manually to test
wrangler dev  # local dev
curl http://localhost:8787  # health check

# 4. Trigger a real download and wait for next cron window
curl -O https://install.tastematter.dev/releases/v0.1.0-alpha.15/tastematter-linux-x86_64
# Wait up to 20 minutes → Slack message should appear

# 5. Check Worker logs
wrangler tail
```

## What This Does NOT Do

- Does NOT proxy or intercept downloads (R2 custom domain serves directly)
- Does NOT track individual IP addresses or user agents (GraphQL analytics is aggregated)
- Does NOT persist download history (stateless — just alerts)
- Does NOT require DNS changes
- Does NOT use KV, D1, or Durable Objects

## Cost

- Worker invocations: 96/day (every 15 min × 24h) — well within free tier (100K/day)
- GraphQL API: Free on all plans
- Slack webhook: Free
- **Total: $0/month**

## Success Criteria

1. [ ] `wrangler deploy` succeeds
2. [ ] Cron fires every 15 minutes (visible in Worker logs)
3. [ ] Downloading a binary triggers Slack message within 20 minutes
4. [ ] No impact on existing download infrastructure (install scripts work unchanged)
5. [ ] Worker health endpoint returns 200
