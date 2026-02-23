import { describe, it, expect, vi, beforeEach } from "vitest";
import { createDB } from "../src/db.js";

function mockD1() {
  const results: any[] = [];
  const boundParams: any[] = [];

  const stmt = {
    bind: (...args: any[]) => {
      boundParams.push(...args);
      return stmt;
    },
    all: async <T>() => ({ results: results as T[] }),
    first: async <T>() => (results[0] as T) ?? null,
    run: async () => ({ success: true }),
  };

  const d1 = {
    prepare: vi.fn(() => stmt),
    _setResults: (r: any[]) => {
      results.length = 0;
      results.push(...r);
    },
    _getBoundParams: () => boundParams,
    _clearParams: () => { boundParams.length = 0; },
  };

  return d1 as unknown as D1Database & {
    _setResults: (r: any[]) => void;
    _getBoundParams: () => any[];
    _clearParams: () => void;
  };
}

describe("createDB", () => {
  let d1: ReturnType<typeof mockD1>;
  let db: ReturnType<typeof createDB>;

  beforeEach(() => {
    d1 = mockD1();
    db = createDB(d1);
  });

  // --- Workers ---

  describe("getEnabledWorkers", () => {
    it("queries worker_registry for enabled workers", async () => {
      d1._setResults([{ id: "test", display_name: "Test", enabled: 1 }]);
      const workers = await db.getEnabledWorkers();
      expect(workers).toHaveLength(1);
      expect(workers[0].id).toBe("test");
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("enabled = 1"));
    });
  });

  describe("registerWorker", () => {
    it("inserts into worker_registry with new fields", async () => {
      await db.registerWorker({
        id: "new-worker",
        display_name: "New Worker",
        health_url: "https://new.workers.dev/health",
        expected_cadence: "4h",
        max_silence_hours: 8,
        auth_type: "none",
        system_id: "intel-pipeline",
        account_id: "4c8353a2",
        status_url: "https://new.workers.dev/status",
      });
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("INSERT INTO worker_registry"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("system_id"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("account_id"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("status_url"));
    });
  });

  describe("updateWorker", () => {
    it("updates specified fields", async () => {
      await db.updateWorker("test", { system_id: "platform", enabled: 0 });
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("UPDATE worker_registry"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("system_id = ?"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("enabled = ?"));
    });

    it("skips update when no fields provided", async () => {
      await db.updateWorker("test", {});
      // prepare should not be called for UPDATE (only initial calls)
      const updateCalls = (d1.prepare as any).mock.calls.filter(
        (c: any[]) => c[0]?.includes?.("UPDATE")
      );
      expect(updateCalls).toHaveLength(0);
    });
  });

  describe("logHealthCheck", () => {
    it("inserts into health_log", async () => {
      await db.logHealthCheck({
        worker_id: "test",
        http_status: 200,
        response_time_ms: 150,
        status: "healthy",
        last_activity: "2026-02-19T00:00:00Z",
        activity_type: "cron",
        raw_response: '{"status":"ok"}',
        error_message: null,
      });
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("INSERT INTO health_log"));
    });
  });

  describe("getWorkersWithStatus", () => {
    it("joins registry with latest health log including error and raw_response", async () => {
      d1._setResults([
        { id: "test", display_name: "Test", current_status: "healthy", last_checked: "2026-02-19T00:00:00Z", error_message: null, raw_response: null },
      ]);
      const workers = await db.getWorkersWithStatus();
      expect(workers).toHaveLength(1);
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("error_message"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("raw_response"));
    });
  });

  describe("getHealthHistory", () => {
    it("returns health log entries for a worker", async () => {
      d1._setResults([
        { id: 1, worker_id: "test", status: "healthy" },
        { id: 2, worker_id: "test", status: "stale" },
      ]);
      const history = await db.getHealthHistory("test", 10);
      expect(history).toHaveLength(2);
    });
  });

  describe("deleteWorker", () => {
    it("deletes from worker_registry", async () => {
      await db.deleteWorker("test");
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("DELETE FROM worker_registry"));
    });
  });

  // --- Systems ---

  describe("registerSystem", () => {
    it("inserts into system_registry", async () => {
      await db.registerSystem({
        id: "new-system",
        display_name: "New System",
        description: "A test system",
        health_rule: "all",
      });
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("INSERT INTO system_registry"));
    });
  });

  describe("getSystems", () => {
    it("returns all systems", async () => {
      d1._setResults([
        { id: "intel-pipeline", display_name: "Intel Pipeline", health_rule: "all" },
      ]);
      const systems = await db.getSystems();
      expect(systems).toHaveLength(1);
      expect(systems[0].id).toBe("intel-pipeline");
    });
  });

  describe("deleteSystem", () => {
    it("deletes from system_registry", async () => {
      await db.deleteSystem("intel-pipeline");
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("DELETE FROM system_registry"));
    });
  });

  describe("updateSystemStatus", () => {
    it("updates status and timestamp", async () => {
      await db.updateSystemStatus("intel-pipeline", "broken");
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("UPDATE system_registry"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("current_status = ?"));
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("status_changed_at"));
    });
  });

  // --- Sync ---

  describe("logSync", () => {
    it("inserts into sync_log", async () => {
      await db.logSync({
        worker_id: "alert-worker",
        commit_sha: "abc123",
        file_count: 34,
        source_repo: "gtm_operating_system",
      });
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("INSERT INTO sync_log"));
    });

    it("handles failure sync", async () => {
      await db.logSync({
        worker_id: "alert-worker",
        commit_sha: "abc123",
        success: false,
        error_message: "R2 upload failed",
      });
      expect(d1.prepare).toHaveBeenCalledWith(expect.stringContaining("INSERT INTO sync_log"));
    });
  });

  describe("getSyncHistory", () => {
    it("returns sync log entries", async () => {
      d1._setResults([
        { id: 1, worker_id: "test", commit_sha: "abc", synced_at: "2026-02-20" },
      ]);
      const history = await db.getSyncHistory("test");
      expect(history).toHaveLength(1);
      expect(history[0].commit_sha).toBe("abc");
    });
  });

  describe("getLatestSync", () => {
    it("returns most recent sync", async () => {
      d1._setResults([{ id: 1, worker_id: "test", commit_sha: "latest" }]);
      const sync = await db.getLatestSync("test");
      expect(sync).not.toBeNull();
      expect(sync!.commit_sha).toBe("latest");
    });

    it("returns null when no syncs", async () => {
      d1._setResults([]);
      const sync = await db.getLatestSync("test");
      expect(sync).toBeNull();
    });
  });
});
