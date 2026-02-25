/**
 * Events Feed
 *
 * GET /events returns recent outreach events with optional filtering.
 * Supports ?contact_id=, ?event_type=, ?limit= query params.
 */

import type { Env, OutreachEvent } from "./types";

export async function handleEvents(env: Env, url: URL): Promise<Response> {
  const contactId = url.searchParams.get("contact_id");
  const eventType = url.searchParams.get("event_type");
  const limit = parseInt(url.searchParams.get("limit") || "50");

  let query = "SELECT * FROM outreach_events WHERE 1=1";
  const params: unknown[] = [];

  if (contactId) {
    query += " AND contact_id = ?";
    params.push(contactId);
  }
  if (eventType) {
    query += " AND event_type = ?";
    params.push(eventType);
  }

  query += " ORDER BY created_at DESC LIMIT ?";
  params.push(limit);

  const result = await env.DB.prepare(query).bind(...params).all<OutreachEvent>();

  return Response.json({
    events: result.results ?? [],
    total: result.results?.length ?? 0,
  });
}
