import type { Env } from "./types.js";
import { createDB } from "./db.js";
import { triggerKnockWorkflow } from "./knock.js";
import { processAlertRules } from "./alerting.js";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    // GET /health
    if (url.pathname === "/health" && request.method === "GET") {
      return Response.json({
        status: "ok",
        worker: "tastematter-alert-worker",
      });
    }

    // GET /alert/history
    if (url.pathname === "/alert/history" && request.method === "GET") {
      const engagementId =
        url.searchParams.get("engagement_id") ?? undefined;
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

    // POST /alert/trigger
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

    // 404 fallback
    return Response.json(
      { error: "Not found" },
      { status: 404 }
    );
  },

  async scheduled(
    _event: ScheduledEvent,
    env: Env,
    ctx: ExecutionContext
  ) {
    const db = createDB(env.ALERTS_DB);

    ctx.waitUntil(
      processAlertRules({
        db,
        ownerId: env.OWNER_ID,
        knockApiKey: env.KNOCK_API_KEY,
        triggerFn: triggerKnockWorkflow,
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
