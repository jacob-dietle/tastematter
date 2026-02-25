/**
 * Dashboard - Pipeline aggregation queries
 *
 * GET /dashboard returns pipeline counts, wave breakdown, source breakdown,
 * and recent events for a quick overview of outreach state.
 */

import type { Env, DashboardResponse, ContactStatus, OutreachEvent } from "./types";

const ALL_STATUSES: ContactStatus[] = [
  "identified",
  "contacted",
  "replied",
  "installed",
  "active",
  "feedback_received",
  "churned",
];

export async function handleDashboard(env: Env): Promise<Response> {
  // Pipeline counts by status
  const statusCounts = await env.DB
    .prepare("SELECT status, COUNT(*) as count FROM contacts GROUP BY status")
    .all<{ status: ContactStatus; count: number }>();

  const pipeline: Record<string, number> = {};
  for (const s of ALL_STATUSES) {
    pipeline[s] = 0;
  }
  for (const row of statusCounts.results ?? []) {
    pipeline[row.status] = row.count;
  }

  // Counts by wave
  const waveCounts = await env.DB
    .prepare(
      `SELECT wave, COUNT(*) as total,
              SUM(CASE WHEN status = 'installed' THEN 1 ELSE 0 END) as installed
       FROM contacts GROUP BY wave`
    )
    .all<{ wave: string; total: number; installed: number }>();

  const byWave: Record<string, { total: number; installed: number }> = {};
  for (const row of waveCounts.results ?? []) {
    byWave[row.wave] = { total: row.total, installed: row.installed };
  }

  // Counts by source
  const sourceCounts = await env.DB
    .prepare("SELECT source, COUNT(*) as count FROM contacts GROUP BY source")
    .all<{ source: string; count: number }>();

  const bySource: Record<string, number> = {};
  for (const row of sourceCounts.results ?? []) {
    bySource[row.source] = row.count;
  }

  // Total contacts
  const totalResult = await env.DB
    .prepare("SELECT COUNT(*) as total FROM contacts")
    .first<{ total: number }>();

  // Recent events
  const recentEvents = await env.DB
    .prepare("SELECT * FROM outreach_events ORDER BY created_at DESC LIMIT 20")
    .all<OutreachEvent>();

  const response: DashboardResponse = {
    pipeline: pipeline as Record<ContactStatus, number>,
    by_wave: byWave,
    by_source: bySource,
    recent_events: recentEvents.results ?? [],
    total_contacts: totalResult?.total ?? 0,
    last_updated: new Date().toISOString(),
  };

  return Response.json(response);
}
