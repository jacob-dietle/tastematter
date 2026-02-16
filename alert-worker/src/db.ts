import type {
  Result,
  EngagementRow,
  AlertHistoryRow,
  AlertStateRow,
  ActivityLogRow,
  InsertAlertHistoryInput,
  UpsertAlertStateInput,
  InsertActivityLogInput,
} from "./types.js";

/**
 * Creates a DB operations object wrapping a D1 database.
 * All methods return Result<T> for consistent error handling.
 * Follows the createDB closure pattern from conference_pr worker.
 */
export function createDB(d1: D1Database) {
  return {
    // --- Engagements ---

    async getEngagementsByOwner(
      ownerId: string
    ): Promise<Result<EngagementRow[]>> {
      try {
        const { results } = await d1
          .prepare("SELECT * FROM engagements WHERE owner_id = ?")
          .bind(ownerId)
          .all();
        return { success: true, data: results as unknown as EngagementRow[] };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },

    async upsertEngagement(
      engagement: EngagementRow
    ): Promise<Result<void>> {
      try {
        await d1
          .prepare(
            `INSERT INTO engagements (id, owner_id, display_name, config_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               owner_id = excluded.owner_id,
               display_name = excluded.display_name,
               config_json = excluded.config_json,
               updated_at = excluded.updated_at`
          )
          .bind(
            engagement.id,
            engagement.owner_id,
            engagement.display_name,
            engagement.config_json,
            engagement.created_at,
            engagement.updated_at
          )
          .run();
        return { success: true, data: undefined };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },

    // --- Alert History ---

    async insertAlertHistory(
      entry: InsertAlertHistoryInput
    ): Promise<Result<void>> {
      try {
        await d1
          .prepare(
            `INSERT INTO alert_history (engagement_id, rule_name, trigger_type, knock_workflow_run_id, payload, success, error_message)
             VALUES (?, ?, ?, ?, ?, ?, ?)`
          )
          .bind(
            entry.engagement_id,
            entry.rule_name,
            entry.trigger_type,
            entry.knock_workflow_run_id ?? null,
            entry.payload ?? null,
            entry.success ?? 1,
            entry.error_message ?? null
          )
          .run();
        return { success: true, data: undefined };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },

    async getAlertHistory(
      engagementId?: string,
      limit?: number
    ): Promise<Result<AlertHistoryRow[]>> {
      try {
        let query = "SELECT * FROM alert_history";
        const binds: unknown[] = [];

        if (engagementId) {
          query += " WHERE engagement_id = ?";
          binds.push(engagementId);
        }

        query += " ORDER BY fired_at DESC";

        if (limit) {
          query += " LIMIT ?";
          binds.push(limit);
        }

        let stmt = d1.prepare(query);
        if (binds.length > 0) {
          stmt = stmt.bind(...binds);
        }

        const { results } = await stmt.all();
        return {
          success: true,
          data: results as unknown as AlertHistoryRow[],
        };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },

    // --- Alert State ---

    async getAlertState(
      ruleName: string,
      engagementId: string
    ): Promise<Result<AlertStateRow | null>> {
      try {
        const row = await d1
          .prepare(
            "SELECT * FROM alert_state WHERE rule_name = ? AND engagement_id = ?"
          )
          .bind(ruleName, engagementId)
          .first();
        return {
          success: true,
          data: row ? (row as unknown as AlertStateRow) : null,
        };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },

    async upsertAlertState(
      state: UpsertAlertStateInput
    ): Promise<Result<void>> {
      try {
        await d1
          .prepare(
            `INSERT INTO alert_state (rule_name, engagement_id, last_checked_at, last_fired_at, last_corpus_sha, state_data)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(rule_name) DO UPDATE SET
               engagement_id = excluded.engagement_id,
               last_checked_at = excluded.last_checked_at,
               last_fired_at = excluded.last_fired_at,
               last_corpus_sha = excluded.last_corpus_sha,
               state_data = excluded.state_data`
          )
          .bind(
            state.rule_name,
            state.engagement_id,
            state.last_checked_at ?? null,
            state.last_fired_at ?? null,
            state.last_corpus_sha ?? null,
            state.state_data ?? null
          )
          .run();
        return { success: true, data: undefined };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },

    // --- Activity Log ---

    async insertActivityLog(
      entry: InsertActivityLogInput
    ): Promise<Result<void>> {
      try {
        await d1
          .prepare(
            `INSERT INTO activity_log (engagement_id, event_type, message, details)
             VALUES (?, ?, ?, ?)`
          )
          .bind(
            entry.engagement_id ?? null,
            entry.event_type,
            entry.message ?? null,
            entry.details ?? null
          )
          .run();
        return { success: true, data: undefined };
      } catch (err) {
        return { success: false, error: String(err) };
      }
    },
  };
}
