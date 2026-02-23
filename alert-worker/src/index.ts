import type { Env, WorkerStatusResponse } from "./types.js";
import { createDB } from "./db.js";
import { triggerKnockWorkflow } from "./knock.js";
import { processAlertRules } from "./alerting.js";
import { ContextDO } from "./context-do.js";
import { ContextMCP } from "./mcp-wrapper.js";
import { executeAgenticQuery } from "./query-handler.js";
import { logQuery, getQueryLogs } from "./query-logging.js";

export { ContextDO, ContextMCP };

const ALLOWED_ORIGINS = ['https://app.tastematter.dev'];

function corsHeaders(request: Request): Record<string, string> {
  const origin = request.headers.get('Origin') || '';
  if (!ALLOWED_ORIGINS.includes(origin)) return {};
  return {
    'Access-Control-Allow-Origin': origin,
    'Access-Control-Allow-Credentials': 'true',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  };
}

function withCors(response: Response, request: Request): Response {
  const headers = corsHeaders(request);
  if (Object.keys(headers).length === 0) return response;
  const newResp = new Response(response.body, response);
  for (const [k, v] of Object.entries(headers)) {
    newResp.headers.set(k, v);
  }
  return newResp;
}

async function handleRequest(url: URL, request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
  // --- MCP endpoints (Phase 2: Context Publishing) ---

  if (url.pathname === '/sse' || url.pathname === '/sse/message') {
    return ContextMCP.serveSSE('/sse').fetch(request, env, ctx);
  }

  if (url.pathname === '/mcp') {
    return ContextMCP.serve('/mcp').fetch(request, env, ctx);
  }

  // --- Rich status for control plane polling ---

  if (url.pathname === "/status" && request.method === "GET") {
    const db = createDB(env.ALERTS_DB);

    // Corpus info
    let corpus: WorkerStatusResponse['corpus'] = undefined;
    if (env.CONTEXT_DO) {
      try {
        const doId = env.CONTEXT_DO.idFromName('singleton');
        const stub = env.CONTEXT_DO.get(doId);
        const health = await stub.fetch(new Request('http://internal/health'));
        const body = await health.json() as any;
        corpus = {
          commit: body.commit ?? 'unknown',
          file_count: body.fileCount ?? 0,
          loaded_at: body.loadedAt ?? new Date().toISOString(),
          source_repo: 'gtm_operating_system',
        };
      } catch { /* corpus not available */ }
    }

    // Trail: last alert fired
    const lastAlert = await db.getAlertHistory(undefined, 1);
    let trail: WorkerStatusResponse['trail'] = undefined;
    if (lastAlert.success && lastAlert.data.length > 0) {
      const a = lastAlert.data[0];
      trail = {
        last_deposit: `alert_fired: ${a.rule_name}`,
        at: a.fired_at,
        type: a.trigger_type,
        detail: `${a.engagement_id} — ${a.success ? 'sent' : 'failed'}`,
      };
    }

    // D1 health: from alert_history counts
    const historyResult = await db.getAlertHistory(undefined, 100);
    let d1Health: WorkerStatusResponse['d1_health'] = undefined;
    if (historyResult.success) {
      const all = historyResult.data;
      const failures = all.filter(a => !a.success);
      d1Health = {
        total_executions: all.length,
        total_failures: failures.length,
        failure_rate: all.length > 0 ? `${((failures.length / all.length) * 100).toFixed(1)}%` : '0%',
        last_execution: all.length > 0 ? {
          status: all[0].success ? 'completed' : 'failed',
          duration_ms: 0,
          at: all[0].fired_at,
        } : undefined,
        last_failure: failures.length > 0 ? {
          error: failures[0].error_message ?? 'unknown',
          at: failures[0].fired_at,
        } : undefined,
      };
    }

    return Response.json({
      identity: {
        worker: 'tastematter-alert-worker',
        display_name: 'Tastematter Alert Worker',
        system_id: 'tastematter-platform',
        account_id: '4c8353a21e0bfc69a1e036e223cba4d8',
      },
      vitals: {
        status: 'ok',
        features: { alerting: true, publishing: !!env.CONTEXT_DO },
      },
      corpus,
      trail,
      d1_health: d1Health,
      schedule: {
        cron: '0 */4 * * *',
        last_run: trail?.at,
      },
    } satisfies WorkerStatusResponse);
  }

  // --- Health check (enhanced with publishing status) ---

  if (url.pathname === "/health" && request.method === "GET") {
    const features: Record<string, boolean> = { alerting: true, publishing: false };
    let corpus: any = null;

    if (env.CONTEXT_DO) {
      try {
        const doId = env.CONTEXT_DO.idFromName('singleton');
        const stub = env.CONTEXT_DO.get(doId);
        const health = await stub.fetch(new Request('http://internal/health'));
        corpus = await health.json();
        features.publishing = true;
      } catch (e) {
        // Publishing not available
      }
    }

    return Response.json({
      status: "ok",
      worker: "tastematter-alert-worker",
      features,
      corpus,
    });
  }

  // --- Publishing: Corpus reload ---

  if (url.pathname === '/reload' && request.method === 'POST') {
    if (!env.CONTEXT_DO) {
      return Response.json({ error: 'Publishing not configured' }, { status: 501 });
    }
    const doId = env.CONTEXT_DO.idFromName('singleton');
    const stub = env.CONTEXT_DO.get(doId);
    return stub.fetch(new Request('http://internal/reload'));
  }

  // --- Publishing: Direct query ---

  if (url.pathname === '/query' && request.method === 'GET') {
    if (!env.CONTEXT_DO || !env.ANTHROPIC_API_KEY) {
      return Response.json({ error: 'Publishing not configured' }, { status: 501 });
    }

    const query = url.searchParams.get('q') || 'What is in this knowledge base?';
    const debug = url.searchParams.get('debug') === 'true';

    try {
      const result = await executeAgenticQuery(query, env, {
        debug,
        onProgress: (msg) => console.log(msg)
      });

      ctx.waitUntil(
        logQuery(env.ALERTS_DB, {
          engagement_id: 'default',
          query,
          response_length: result.response.length,
          duration_ms: result.duration,
          tool_calls: result.totalTurns,
          success: 1,
        }).catch(err => console.error('Failed to log query:', err))
      );

      if (debug) {
        return Response.json({
          query,
          response: result.response,
          conversationHistory: result.conversationHistory,
          totalTurns: result.totalTurns,
          duration: result.duration,
          model: result.model,
        });
      }

      return new Response(result.response, {
        headers: { 'Content-Type': 'text/plain; charset=utf-8' }
      });
    } catch (error: any) {
      ctx.waitUntil(
        logQuery(env.ALERTS_DB, {
          engagement_id: 'default',
          query,
          success: 0,
          error_message: error.message,
        }).catch(() => {})
      );
      return Response.json({ error: error.message }, { status: 500 });
    }
  }

  // --- Publishing: Query logs ---

  if (url.pathname === '/query/logs' && request.method === 'GET') {
    const engagementId = url.searchParams.get('engagement_id') ?? undefined;
    const limit = url.searchParams.get('limit');
    const result = await getQueryLogs(env.ALERTS_DB, {
      engagementId,
      limit: limit ? parseInt(limit, 10) : undefined
    });

    if (!result.success) {
      return Response.json({ error: result.error }, { status: 500 });
    }
    return Response.json({ data: result.data });
  }

  // --- Alerting endpoints (Phase 1) ---

  if (url.pathname === "/alert/history" && request.method === "GET") {
    const engagementId = url.searchParams.get("engagement_id") ?? undefined;
    const limit = url.searchParams.get("limit");
    const db = createDB(env.ALERTS_DB);
    const result = await db.getAlertHistory(
      engagementId,
      limit ? parseInt(limit, 10) : undefined
    );

    if (!result.success) {
      return Response.json({ error: result.error }, { status: 500 });
    }
    return Response.json({ data: result.data });
  }

  if (url.pathname === "/alert/trigger" && request.method === "POST") {
    const db = createDB(env.ALERTS_DB);
    const result = await processAlertRules({
      db,
      ownerId: env.OWNER_ID,
      knockApiKey: env.KNOCK_API_KEY,
      triggerFn: triggerKnockWorkflow,
    });

    if (!result.success) {
      return Response.json({ error: result.error }, { status: 500 });
    }
    return Response.json({ data: result.data });
  }

  return Response.json({ error: "Not found" }, { status: 404 });
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // Handle CORS preflight
    if (request.method === 'OPTIONS') {
      return new Response(null, { status: 204, headers: corsHeaders(request) });
    }

    const response = await handleRequest(url, request, env, ctx);
    return withCors(response, request);
  },

  async scheduled(
    _event: ScheduledEvent,
    env: Env,
    ctx: ExecutionContext
  ) {
    const db = createDB(env.ALERTS_DB);

    // Fetch current corpus SHA for content_change detection
    let currentCorpusSha: string | undefined;
    if (env.CONTEXT_DO) {
      try {
        const doId = env.CONTEXT_DO.idFromName('singleton');
        const stub = env.CONTEXT_DO.get(doId);
        const health = await stub.fetch(new Request('http://internal/health'));
        const corpus = await health.json() as { commit?: string };
        currentCorpusSha = corpus.commit;
      } catch {
        // Corpus not available — content_change rules will skip
      }
    }

    ctx.waitUntil(
      processAlertRules({
        db,
        ownerId: env.OWNER_ID,
        knockApiKey: env.KNOCK_API_KEY,
        triggerFn: triggerKnockWorkflow,
        currentCorpusSha,
      }).then((result) => {
        if (result.success) {
          console.log(
            `Alert run: checked=${result.data.checked} fired=${result.data.fired} errors=${result.data.errors.length}`
          );
        } else {
          console.error(`Alert run failed: ${result.error}`);
        }
      })
    );
  },
};
