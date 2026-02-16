import { describe, it, expect } from "vitest";
import type {
  Result,
  Env,
  EngagementRow,
  AlertHistoryRow,
  AlertStateRow,
  ActivityLogRow,
  KnockRecipient,
  KnockTriggerPayload,
  AlertingConfig,
  WatchRule,
  TriggerType,
  TriggerConfig,
  InsertAlertHistoryInput,
  UpsertAlertStateInput,
  InsertActivityLogInput,
  TriggerFn,
} from "../src/types.js";

describe("types", () => {
  it("Result<T> success shape", () => {
    const result: Result<string> = { success: true, data: "hello" };
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data).toBe("hello");
    }
  });

  it("Result<T> failure shape", () => {
    const result: Result<string> = { success: false, error: "bad" };
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toBe("bad");
    }
  });

  it("EngagementRow shape", () => {
    const row: EngagementRow = {
      id: "pixee",
      owner_id: "founder",
      display_name: "Pixee AI",
      config_json: "{}",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    };
    expect(row.id).toBe("pixee");
    expect(row.owner_id).toBe("founder");
  });

  it("AlertHistoryRow shape", () => {
    const row: AlertHistoryRow = {
      id: 1,
      engagement_id: "pixee",
      rule_name: "new-intel",
      trigger_type: "content_change",
      fired_at: "2026-01-01T00:00:00Z",
      knock_workflow_run_id: "run_123",
      payload: null,
      success: 1,
      error_message: null,
    };
    expect(row.id).toBe(1);
    expect(row.success).toBe(1);
  });

  it("AlertStateRow shape", () => {
    const row: AlertStateRow = {
      rule_name: "new-intel",
      engagement_id: "pixee",
      last_checked_at: null,
      last_fired_at: null,
      last_corpus_sha: null,
      state_data: null,
    };
    expect(row.rule_name).toBe("new-intel");
  });

  it("ActivityLogRow shape", () => {
    const row: ActivityLogRow = {
      id: 1,
      engagement_id: "pixee",
      timestamp: "2026-01-01T00:00:00Z",
      event_type: "alert_fired",
      message: "Fired new-intel rule",
      details: null,
    };
    expect(row.event_type).toBe("alert_fired");
  });

  it("KnockRecipient shape", () => {
    const recipient: KnockRecipient = {
      id: "user_123",
      email: "test@example.com",
      name: "Test User",
    };
    expect(recipient.id).toBe("user_123");
  });

  it("KnockTriggerPayload shape", () => {
    const payload: KnockTriggerPayload = {
      recipients: ["user_123"],
      data: {
        subject: "New intel brief",
        body: "Content changed",
        trigger_type: "content_change",
      },
    };
    expect(payload.recipients).toHaveLength(1);
    expect(payload.data.trigger_type).toBe("content_change");
  });

  it("WatchRule shape", () => {
    const rule: WatchRule = {
      name: "new-intel",
      trigger: "content_change",
      schedule: "0 */4 * * *",
      config: { type: "content_change", paths: ["knowledge_base/**/*.md"] },
      channels: ["email"],
      format: "instant",
      enabled: true,
    };
    expect(rule.name).toBe("new-intel");
    expect(rule.enabled).toBe(true);
  });

  it("AlertingConfig shape", () => {
    const config: AlertingConfig = {
      provider: "knock",
      workflow_key: "new-intel-brief",
      recipients: [{ id: "user_123" }],
      rules: [],
    };
    expect(config.provider).toBe("knock");
    expect(config.workflow_key).toBe("new-intel-brief");
  });

  it("TriggerConfig discriminated union", () => {
    const configs: TriggerConfig[] = [
      { type: "content_change", paths: ["**/*.md"] },
      { type: "pattern_match", pattern: "ERROR" },
      { type: "threshold", metric: "tokens", operator: ">", value: 1000 },
      { type: "schedule" },
      { type: "corpus_drift", max_commits_behind: 5 },
    ];
    expect(configs).toHaveLength(5);
    expect(configs[0].type).toBe("content_change");
  });

  it("TriggerType covers all variants", () => {
    const types: TriggerType[] = [
      "content_change",
      "pattern_match",
      "threshold",
      "schedule",
      "corpus_drift",
    ];
    expect(types).toHaveLength(5);
  });

  it("InsertAlertHistoryInput shape", () => {
    const input: InsertAlertHistoryInput = {
      engagement_id: "pixee",
      rule_name: "new-intel",
      trigger_type: "content_change",
    };
    expect(input.engagement_id).toBe("pixee");
  });

  it("UpsertAlertStateInput shape", () => {
    const input: UpsertAlertStateInput = {
      rule_name: "new-intel",
      engagement_id: "pixee",
      last_checked_at: "2026-01-01T00:00:00Z",
    };
    expect(input.rule_name).toBe("new-intel");
  });

  it("InsertActivityLogInput shape", () => {
    const input: InsertActivityLogInput = {
      event_type: "cron_start",
      message: "Cron triggered",
    };
    expect(input.event_type).toBe("cron_start");
  });

  it("TriggerFn signature", () => {
    const fn: TriggerFn = async (_apiKey, _workflowKey, _payload) => ({
      success: true,
      data: { workflow_run_id: "run_abc" },
    });
    expect(typeof fn).toBe("function");
  });
});
