import { describe, it, expect } from "vitest";
import { createDB } from "../src/db.js";
import { createMockD1 } from "./helpers.js";
import type { EngagementRow } from "../src/types.js";

describe("createDB", () => {
  describe("getEngagementsByOwner", () => {
    it("returns engagements for owner", async () => {
      const mockRow = {
        id: "pixee",
        owner_id: "founder",
        display_name: "Pixee AI",
        config_json: "{}",
        created_at: "2026-01-01",
        updated_at: "2026-01-01",
      };
      const mock = createMockD1({ allResults: [mockRow] });
      const db = createDB(mock as unknown as D1Database);

      const result = await db.getEngagementsByOwner("founder");

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data).toHaveLength(1);
        expect(result.data[0].id).toBe("pixee");
      }
      expect(mock._calls[0].binds).toEqual(["founder"]);
    });

    it("returns error on D1 failure", async () => {
      const mock = createMockD1({ shouldThrow: "D1 connection error" });
      const db = createDB(mock as unknown as D1Database);

      const result = await db.getEngagementsByOwner("founder");

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toContain("D1 connection error");
      }
    });
  });

  describe("upsertEngagement", () => {
    it("inserts or updates engagement", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      const engagement: EngagementRow = {
        id: "pixee",
        owner_id: "founder",
        display_name: "Pixee AI",
        config_json: '{"alerting":{}}',
        created_at: "2026-01-01",
        updated_at: "2026-01-01",
      };

      const result = await db.upsertEngagement(engagement);

      expect(result.success).toBe(true);
      expect(mock._calls[0].query).toContain("INSERT INTO engagements");
      expect(mock._calls[0].query).toContain("ON CONFLICT");
      expect(mock._calls[0].binds[0]).toBe("pixee");
    });

    it("returns error on failure", async () => {
      const mock = createMockD1({ shouldThrow: "constraint error" });
      const db = createDB(mock as unknown as D1Database);

      const result = await db.upsertEngagement({
        id: "x",
        owner_id: "x",
        display_name: "x",
        config_json: "{}",
        created_at: "",
        updated_at: "",
      });

      expect(result.success).toBe(false);
    });
  });

  describe("insertAlertHistory", () => {
    it("inserts alert history entry", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      const result = await db.insertAlertHistory({
        engagement_id: "pixee",
        rule_name: "new-intel",
        trigger_type: "content_change",
        knock_workflow_run_id: "run_123",
      });

      expect(result.success).toBe(true);
      expect(mock._calls[0].query).toContain("INSERT INTO alert_history");
      expect(mock._calls[0].binds[0]).toBe("pixee");
      expect(mock._calls[0].binds[1]).toBe("new-intel");
    });

    it("uses defaults for optional fields", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      const result = await db.insertAlertHistory({
        engagement_id: "pixee",
        rule_name: "test",
        trigger_type: "schedule",
      });

      expect(result.success).toBe(true);
      // knock_workflow_run_id, payload, success default, error_message
      expect(mock._calls[0].binds[3]).toBeNull(); // knock_workflow_run_id
      expect(mock._calls[0].binds[4]).toBeNull(); // payload
      expect(mock._calls[0].binds[5]).toBe(1); // success default
      expect(mock._calls[0].binds[6]).toBeNull(); // error_message
    });
  });

  describe("getAlertHistory", () => {
    it("returns all history when no engagement filter", async () => {
      const mockRow = {
        id: 1,
        engagement_id: "pixee",
        rule_name: "new-intel",
        trigger_type: "content_change",
        fired_at: "2026-01-01",
        knock_workflow_run_id: null,
        payload: null,
        success: 1,
        error_message: null,
      };
      const mock = createMockD1({ allResults: [mockRow] });
      const db = createDB(mock as unknown as D1Database);

      const result = await db.getAlertHistory();

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data).toHaveLength(1);
      }
    });

    it("filters by engagement_id", async () => {
      const mock = createMockD1({ allResults: [] });
      const db = createDB(mock as unknown as D1Database);

      await db.getAlertHistory("pixee");

      expect(mock._calls[0].query).toContain("WHERE engagement_id = ?");
      expect(mock._calls[0].binds[0]).toBe("pixee");
    });

    it("applies limit", async () => {
      const mock = createMockD1({ allResults: [] });
      const db = createDB(mock as unknown as D1Database);

      await db.getAlertHistory(undefined, 10);

      expect(mock._calls[0].query).toContain("LIMIT ?");
      expect(mock._calls[0].binds[0]).toBe(10);
    });

    it("applies both engagement_id and limit", async () => {
      const mock = createMockD1({ allResults: [] });
      const db = createDB(mock as unknown as D1Database);

      await db.getAlertHistory("pixee", 5);

      expect(mock._calls[0].query).toContain("WHERE engagement_id = ?");
      expect(mock._calls[0].query).toContain("LIMIT ?");
      expect(mock._calls[0].binds).toEqual(["pixee", 5]);
    });
  });

  describe("getAlertState", () => {
    it("returns state for rule", async () => {
      const mockRow = {
        rule_name: "new-intel",
        engagement_id: "pixee",
        last_checked_at: "2026-01-01",
        last_fired_at: null,
        last_corpus_sha: null,
        state_data: null,
      };
      const mock = createMockD1({ firstResult: mockRow });
      const db = createDB(mock as unknown as D1Database);

      const result = await db.getAlertState("new-intel", "pixee");

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.rule_name).toBe("new-intel");
      }
    });

    it("returns null when no state exists", async () => {
      const mock = createMockD1({ firstResult: null });
      const db = createDB(mock as unknown as D1Database);

      const result = await db.getAlertState("nonexistent", "pixee");

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data).toBeNull();
      }
    });
  });

  describe("upsertAlertState", () => {
    it("upserts alert state", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      const result = await db.upsertAlertState({
        rule_name: "new-intel",
        engagement_id: "pixee",
        last_checked_at: "2026-01-01",
        last_fired_at: "2026-01-01",
      });

      expect(result.success).toBe(true);
      expect(mock._calls[0].query).toContain("INSERT INTO alert_state");
      expect(mock._calls[0].query).toContain("ON CONFLICT");
    });

    it("handles optional fields as null", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      await db.upsertAlertState({
        rule_name: "test",
        engagement_id: "pixee",
      });

      expect(mock._calls[0].binds[2]).toBeNull(); // last_checked_at
      expect(mock._calls[0].binds[3]).toBeNull(); // last_fired_at
      expect(mock._calls[0].binds[4]).toBeNull(); // last_corpus_sha
      expect(mock._calls[0].binds[5]).toBeNull(); // state_data
    });
  });

  describe("insertActivityLog", () => {
    it("inserts activity log entry", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      const result = await db.insertActivityLog({
        engagement_id: "pixee",
        event_type: "alert_fired",
        message: "New intel brief generated",
      });

      expect(result.success).toBe(true);
      expect(mock._calls[0].query).toContain("INSERT INTO activity_log");
      expect(mock._calls[0].binds[0]).toBe("pixee");
      expect(mock._calls[0].binds[1]).toBe("alert_fired");
    });

    it("allows null engagement_id for system events", async () => {
      const mock = createMockD1();
      const db = createDB(mock as unknown as D1Database);

      const result = await db.insertActivityLog({
        event_type: "cron_start",
        message: "Scheduled run started",
      });

      expect(result.success).toBe(true);
      expect(mock._calls[0].binds[0]).toBeNull(); // engagement_id
    });
  });
});
