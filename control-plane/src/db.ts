import type {
  WorkerRegistryRow, HealthLogRow, HealthCheckResult,
  WorkerWithStatus, SystemRegistryRow, SystemWithMembers,
  SystemStatus, SyncLogRow, SyncWebhookPayload,
} from "./types.js";

export function createDB(d1: D1Database) {
  return {
    // --- Workers ---

    async getEnabledWorkers(): Promise<WorkerRegistryRow[]> {
      const result = await d1
        .prepare("SELECT * FROM worker_registry WHERE enabled = 1")
        .all<WorkerRegistryRow>();
      return result.results ?? [];
    },

    async getWorker(id: string): Promise<WorkerRegistryRow | null> {
      return d1
        .prepare("SELECT * FROM worker_registry WHERE id = ?")
        .bind(id)
        .first<WorkerRegistryRow>();
    },

    async registerWorker(worker: {
      id: string;
      display_name: string;
      health_url: string;
      expected_cadence?: string | null;
      max_silence_hours?: number;
      auth_type?: string;
      tags?: string | null;
      enabled?: number;
      system_id?: string | null;
      account_id?: string | null;
      status_url?: string | null;
    }): Promise<void> {
      await d1
        .prepare(
          `INSERT INTO worker_registry (id, display_name, health_url, expected_cadence, max_silence_hours, auth_type, tags, enabled, system_id, account_id, status_url)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
        )
        .bind(
          worker.id,
          worker.display_name,
          worker.health_url,
          worker.expected_cadence ?? null,
          worker.max_silence_hours ?? 24,
          worker.auth_type ?? "none",
          worker.tags ?? null,
          worker.enabled ?? 1,
          worker.system_id ?? null,
          worker.account_id ?? null,
          worker.status_url ?? null,
        )
        .run();
    },

    async updateWorker(id: string, fields: Partial<{
      system_id: string | null;
      account_id: string | null;
      status_url: string | null;
      enabled: number;
      display_name: string;
      health_url: string;
      expected_cadence: string | null;
      max_silence_hours: number;
      auth_type: string;
    }>): Promise<void> {
      const sets: string[] = [];
      const values: unknown[] = [];
      for (const [key, val] of Object.entries(fields)) {
        if (val !== undefined) {
          sets.push(`${key} = ?`);
          values.push(val);
        }
      }
      if (sets.length === 0) return;
      sets.push("updated_at = datetime('now')");
      values.push(id);
      await d1
        .prepare(`UPDATE worker_registry SET ${sets.join(", ")} WHERE id = ?`)
        .bind(...values)
        .run();
    },

    async deleteWorker(id: string): Promise<void> {
      await d1.prepare("DELETE FROM worker_registry WHERE id = ?").bind(id).run();
    },

    // --- Health Log ---

    async logHealthCheck(result: HealthCheckResult): Promise<void> {
      await d1
        .prepare(
          `INSERT INTO health_log (worker_id, http_status, response_time_ms, status, last_activity, activity_type, raw_response, error_message)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
        )
        .bind(
          result.worker_id,
          result.http_status,
          result.response_time_ms,
          result.status,
          result.last_activity,
          result.activity_type,
          result.raw_response,
          result.error_message,
        )
        .run();
    },

    async getLatestHealthStatus(workerId: string): Promise<HealthLogRow | null> {
      return d1
        .prepare("SELECT * FROM health_log WHERE worker_id = ? ORDER BY checked_at DESC LIMIT 1")
        .bind(workerId)
        .first<HealthLogRow>();
    },

    async getWorkersWithStatus(): Promise<WorkerWithStatus[]> {
      const result = await d1
        .prepare(
          `SELECT
            wr.*,
            hl.status as current_status,
            hl.checked_at as last_checked,
            hl.last_activity,
            hl.response_time_ms as last_response_time_ms,
            hl.error_message,
            hl.raw_response
          FROM worker_registry wr
          LEFT JOIN (
            SELECT worker_id, status, checked_at, last_activity, response_time_ms, error_message, raw_response,
              ROW_NUMBER() OVER (PARTITION BY worker_id ORDER BY checked_at DESC) as rn
            FROM health_log
          ) hl ON wr.id = hl.worker_id AND hl.rn = 1
          ORDER BY wr.display_name`
        )
        .all<WorkerWithStatus>();
      return result.results ?? [];
    },

    async getHealthHistory(workerId: string, limit = 24): Promise<HealthLogRow[]> {
      const result = await d1
        .prepare("SELECT * FROM health_log WHERE worker_id = ? ORDER BY checked_at DESC LIMIT ?")
        .bind(workerId, limit)
        .all<HealthLogRow>();
      return result.results ?? [];
    },

    // --- Systems ---

    async getSystems(): Promise<SystemRegistryRow[]> {
      const result = await d1
        .prepare("SELECT * FROM system_registry ORDER BY display_name")
        .all<SystemRegistryRow>();
      return result.results ?? [];
    },

    async getSystem(id: string): Promise<SystemRegistryRow | null> {
      return d1
        .prepare("SELECT * FROM system_registry WHERE id = ?")
        .bind(id)
        .first<SystemRegistryRow>();
    },

    async registerSystem(system: {
      id: string;
      display_name: string;
      description?: string | null;
      health_rule?: string;
    }): Promise<void> {
      await d1
        .prepare(
          `INSERT INTO system_registry (id, display_name, description, health_rule)
           VALUES (?, ?, ?, ?)`
        )
        .bind(
          system.id,
          system.display_name,
          system.description ?? null,
          system.health_rule ?? "all",
        )
        .run();
    },

    async deleteSystem(id: string): Promise<void> {
      await d1.prepare("DELETE FROM system_registry WHERE id = ?").bind(id).run();
    },

    async updateSystemStatus(id: string, status: SystemStatus): Promise<void> {
      await d1
        .prepare("UPDATE system_registry SET current_status = ?, status_changed_at = datetime('now') WHERE id = ?")
        .bind(status, id)
        .run();
    },

    async getSystemsWithMembers(): Promise<SystemWithMembers[]> {
      const systems = await d1
        .prepare("SELECT * FROM system_registry ORDER BY display_name")
        .all<SystemRegistryRow>();
      const workers = await d1
        .prepare(
          `SELECT
            wr.*,
            hl.status as current_status,
            hl.checked_at as last_checked,
            hl.last_activity,
            hl.response_time_ms as last_response_time_ms,
            hl.error_message,
            hl.raw_response
          FROM worker_registry wr
          LEFT JOIN (
            SELECT worker_id, status, checked_at, last_activity, response_time_ms, error_message, raw_response,
              ROW_NUMBER() OVER (PARTITION BY worker_id ORDER BY checked_at DESC) as rn
            FROM health_log
          ) hl ON wr.id = hl.worker_id AND hl.rn = 1
          WHERE wr.system_id IS NOT NULL
          ORDER BY wr.display_name`
        )
        .all<WorkerWithStatus>();

      const workersBySystem = new Map<string, WorkerWithStatus[]>();
      for (const w of workers.results ?? []) {
        const list = workersBySystem.get(w.system_id!) ?? [];
        list.push(w);
        workersBySystem.set(w.system_id!, list);
      }

      return (systems.results ?? []).map((s) => ({
        ...s,
        members: workersBySystem.get(s.id) ?? [],
      }));
    },

    // --- Sync Log ---

    async logSync(entry: SyncWebhookPayload): Promise<void> {
      await d1
        .prepare(
          `INSERT INTO sync_log (worker_id, commit_sha, file_count, source_repo, action_run_url, success, error_message)
           VALUES (?, ?, ?, ?, ?, ?, ?)`
        )
        .bind(
          entry.worker_id,
          entry.commit_sha,
          entry.file_count ?? null,
          entry.source_repo ?? null,
          entry.action_run_url ?? null,
          entry.success !== false ? 1 : 0,
          entry.error_message ?? null,
        )
        .run();
    },

    async getLatestSync(workerId: string): Promise<SyncLogRow | null> {
      return d1
        .prepare("SELECT * FROM sync_log WHERE worker_id = ? ORDER BY synced_at DESC LIMIT 1")
        .bind(workerId)
        .first<SyncLogRow>();
    },

    async getSyncHistory(workerId: string, limit = 10): Promise<SyncLogRow[]> {
      const result = await d1
        .prepare("SELECT * FROM sync_log WHERE worker_id = ? ORDER BY synced_at DESC LIMIT ?")
        .bind(workerId, limit)
        .all<SyncLogRow>();
      return result.results ?? [];
    },
  };
}
