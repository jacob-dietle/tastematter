import { describe, it, expect, vi } from "vitest";
import { evaluateRule, processAlertRules } from "../src/alerting.js";
import type {
  WatchRule,
  AlertStateRow,
  TriggerFn,
  EngagementRow,
} from "../src/types.js";

function makeRule(overrides?: Partial<WatchRule>): WatchRule {
  return {
    name: "new-intel",
    trigger: "content_change",
    schedule: "0 */4 * * *",
    config: { type: "content_change", paths: ["**/*.md"] },
    channels: ["email"],
    format: "instant",
    enabled: true,
    ...overrides,
  };
}

function makeAlertState(
  overrides?: Partial<AlertStateRow>
): AlertStateRow {
  return {
    rule_name: "new-intel",
    engagement_id: "pixee",
    last_checked_at: "2026-01-01T00:00:00Z",
    last_fired_at: null,
    last_corpus_sha: null,
    state_data: null,
    ...overrides,
  };
}

describe("evaluateRule", () => {
  it("fires content_change when enabled", () => {
    const result = evaluateRule(makeRule(), null);
    expect(result.shouldFire).toBe(true);
    expect(result.reason).toContain("content_change always fires");
  });

  it("does not fire when disabled", () => {
    const result = evaluateRule(makeRule({ enabled: false }), null);
    expect(result.shouldFire).toBe(false);
    expect(result.reason).toContain("disabled");
  });

  it("fires content_change even with prior state", () => {
    const result = evaluateRule(makeRule(), makeAlertState());
    expect(result.shouldFire).toBe(true);
  });

  it("fires schedule trigger", () => {
    const result = evaluateRule(
      makeRule({ trigger: "schedule", config: { type: "schedule" } }),
      null
    );
    expect(result.shouldFire).toBe(true);
    expect(result.reason).toContain("Scheduled rule");
  });

  it("does not fire unimplemented trigger types", () => {
    const result = evaluateRule(
      makeRule({
        trigger: "threshold",
        config: { type: "threshold", metric: "x", operator: ">", value: 1 },
      }),
      null
    );
    expect(result.shouldFire).toBe(false);
    expect(result.reason).toContain("not yet implemented");
  });
});

describe("processAlertRules", () => {
  function makeEngagement(
    overrides?: Partial<EngagementRow>
  ): EngagementRow {
    return {
      id: "pixee",
      owner_id: "founder",
      display_name: "Pixee AI",
      config_json: JSON.stringify({
        alerting: {
          provider: "knock",
          workflow_key: "new-intel-brief",
          recipients: [{ id: "user_123" }],
          rules: [
            {
              name: "new-intel",
              trigger: "content_change",
              schedule: "0 */4 * * *",
              config: { type: "content_change", paths: ["**/*.md"] },
              channels: ["email"],
              format: "instant",
              enabled: true,
            },
          ],
        },
      }),
      created_at: "2026-01-01",
      updated_at: "2026-01-01",
      ...overrides,
    };
  }

  function makeMockDB(engagements: EngagementRow[] = [makeEngagement()]) {
    return {
      getEngagementsByOwner: vi
        .fn()
        .mockResolvedValue({ success: true, data: engagements }),
      getAlertState: vi
        .fn()
        .mockResolvedValue({ success: true, data: null }),
      upsertAlertState: vi
        .fn()
        .mockResolvedValue({ success: true, data: undefined }),
      insertAlertHistory: vi
        .fn()
        .mockResolvedValue({ success: true, data: undefined }),
      insertActivityLog: vi
        .fn()
        .mockResolvedValue({ success: true, data: undefined }),
      // These methods exist on the DB type but aren't used in alerting
      upsertEngagement: vi.fn(),
      getAlertHistory: vi.fn(),
    };
  }

  function makeMockTriggerFn(): TriggerFn {
    return vi.fn().mockResolvedValue({
      success: true,
      data: { workflow_run_id: "run_abc" },
    });
  }

  it("processes rules and fires alerts", async () => {
    const db = makeMockDB();
    const triggerFn = makeMockTriggerFn();

    const result = await processAlertRules({
      db: db as any,
      ownerId: "founder",
      knockApiKey: "sk_test_key",
      triggerFn,
    });

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.checked).toBe(1);
      expect(result.data.fired).toBe(1);
      expect(result.data.errors).toHaveLength(0);
    }

    expect(triggerFn).toHaveBeenCalledOnce();
    expect(db.insertAlertHistory).toHaveBeenCalledOnce();
    expect(db.upsertAlertState).toHaveBeenCalledOnce();
    expect(db.insertActivityLog).toHaveBeenCalledOnce();
  });

  it("skips engagements without alerting config", async () => {
    const engagement = makeEngagement({
      config_json: JSON.stringify({ name: "personal" }),
    });
    const db = makeMockDB([engagement]);
    const triggerFn = makeMockTriggerFn();

    const result = await processAlertRules({
      db: db as any,
      ownerId: "founder",
      knockApiKey: "sk_test_key",
      triggerFn,
    });

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.checked).toBe(0);
      expect(result.data.fired).toBe(0);
    }
    expect(triggerFn).not.toHaveBeenCalled();
  });

  it("records errors when trigger fails", async () => {
    const db = makeMockDB();
    const triggerFn = vi
      .fn()
      .mockResolvedValue({ success: false, error: "Knock API 500" });

    const result = await processAlertRules({
      db: db as any,
      ownerId: "founder",
      knockApiKey: "sk_test_key",
      triggerFn: triggerFn as TriggerFn,
    });

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.fired).toBe(0);
      expect(result.data.errors).toHaveLength(1);
      expect(result.data.errors[0]).toContain("Knock API 500");
    }

    // Should log failed history
    expect(db.insertAlertHistory).toHaveBeenCalledWith(
      expect.objectContaining({
        success: 0,
        error_message: "Knock API 500",
      })
    );
  });

  it("returns error when engagements query fails", async () => {
    const db = makeMockDB();
    db.getEngagementsByOwner.mockResolvedValue({
      success: false,
      error: "D1 error",
    });
    const triggerFn = makeMockTriggerFn();

    const result = await processAlertRules({
      db: db as any,
      ownerId: "founder",
      knockApiKey: "sk_test_key",
      triggerFn,
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Failed to load engagements");
    }
  });

  it("records config parse errors and continues", async () => {
    const engagements = [
      makeEngagement({ id: "bad", config_json: "not json" }),
      makeEngagement({ id: "good" }),
    ];
    const db = makeMockDB(engagements);
    const triggerFn = makeMockTriggerFn();

    const result = await processAlertRules({
      db: db as any,
      ownerId: "founder",
      knockApiKey: "sk_test_key",
      triggerFn,
    });

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.errors).toHaveLength(1);
      expect(result.data.errors[0]).toContain("bad");
      expect(result.data.errors[0]).toContain("config parse error");
      // The good engagement should still fire
      expect(result.data.fired).toBe(1);
    }
  });

  it("handles disabled rules within an engagement", async () => {
    const engagement = makeEngagement({
      config_json: JSON.stringify({
        alerting: {
          provider: "knock",
          workflow_key: "test",
          recipients: [{ id: "user_1" }],
          rules: [
            {
              name: "disabled-rule",
              trigger: "content_change",
              schedule: "0 */4 * * *",
              config: { type: "content_change", paths: ["**/*.md"] },
              channels: ["email"],
              format: "instant",
              enabled: false,
            },
          ],
        },
      }),
    });
    const db = makeMockDB([engagement]);
    const triggerFn = makeMockTriggerFn();

    const result = await processAlertRules({
      db: db as any,
      ownerId: "founder",
      knockApiKey: "sk_test_key",
      triggerFn,
    });

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.checked).toBe(1);
      expect(result.data.fired).toBe(0);
    }
    expect(triggerFn).not.toHaveBeenCalled();
  });
});
