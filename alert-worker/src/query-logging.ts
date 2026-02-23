// src/query-logging.ts
// D1-based query logging (adapted from CVI's R2-based logging)
import type { Result, InsertQueryLogInput } from './types.js';

/**
 * Log a query to D1 query_log table
 */
export async function logQuery(
  db: D1Database,
  input: InsertQueryLogInput
): Promise<Result<void>> {
  try {
    await db
      .prepare(
        `INSERT INTO query_log (engagement_id, query, response_length, duration_ms, tool_calls, corpus_commit, success, error_message)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .bind(
        input.engagement_id,
        input.query,
        input.response_length ?? null,
        input.duration_ms ?? null,
        input.tool_calls ?? null,
        input.corpus_commit ?? null,
        input.success ?? 1,
        input.error_message ?? null
      )
      .run();
    return { success: true, data: undefined };
  } catch (err) {
    return { success: false, error: String(err) };
  }
}

/**
 * Get recent query logs
 */
export async function getQueryLogs(
  db: D1Database,
  options: { engagementId?: string; limit?: number } = {}
): Promise<Result<any[]>> {
  try {
    const { engagementId, limit = 50 } = options;
    let query = 'SELECT * FROM query_log';
    const binds: unknown[] = [];

    if (engagementId) {
      query += ' WHERE engagement_id = ?';
      binds.push(engagementId);
    }

    query += ' ORDER BY timestamp DESC LIMIT ?';
    binds.push(limit);

    let stmt = db.prepare(query);
    if (binds.length > 0) {
      stmt = stmt.bind(...binds);
    }

    const { results } = await stmt.all();
    return { success: true, data: results as any[] };
  } catch (err) {
    return { success: false, error: String(err) };
  }
}
