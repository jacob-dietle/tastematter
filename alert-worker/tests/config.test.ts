import { describe, it, expect } from "vitest";
import { parseEngagementConfig } from "../src/config.js";

describe("parseEngagementConfig", () => {
  it("parses valid alerting config", () => {
    const config = {
      name: "pixee",
      alerting: {
        provider: "knock",
        workflow_key: "new-intel-brief",
        recipients: [{ id: "user_123", email: "test@example.com" }],
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
    };

    const result = parseEngagementConfig(JSON.stringify(config));

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data).not.toBeNull();
      expect(result.data!.provider).toBe("knock");
      expect(result.data!.workflow_key).toBe("new-intel-brief");
      expect(result.data!.recipients).toHaveLength(1);
      expect(result.data!.rules).toHaveLength(1);
    }
  });

  it("returns null for publishing-only engagement (no alerting)", () => {
    const config = {
      name: "personal",
      artifacts: [],
    };

    const result = parseEngagementConfig(JSON.stringify(config));

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data).toBeNull();
    }
  });

  it("returns error for invalid JSON", () => {
    const result = parseEngagementConfig("not json {{{");

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Invalid JSON");
    }
  });

  it("returns error for unsupported provider", () => {
    const config = {
      alerting: {
        provider: "twilio",
        workflow_key: "test",
        recipients: [{ id: "x" }],
        rules: [],
      },
    };

    const result = parseEngagementConfig(JSON.stringify(config));

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Unsupported alerting provider");
    }
  });

  it("returns error for missing workflow_key", () => {
    const config = {
      alerting: {
        provider: "knock",
        recipients: [{ id: "x" }],
        rules: [],
      },
    };

    const result = parseEngagementConfig(JSON.stringify(config));

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Missing alerting.workflow_key");
    }
  });

  it("returns error for empty recipients", () => {
    const config = {
      alerting: {
        provider: "knock",
        workflow_key: "test",
        recipients: [],
        rules: [],
      },
    };

    const result = parseEngagementConfig(JSON.stringify(config));

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Missing or empty alerting.recipients");
    }
  });

  it("returns error for missing rules array", () => {
    const config = {
      alerting: {
        provider: "knock",
        workflow_key: "test",
        recipients: [{ id: "x" }],
      },
    };

    const result = parseEngagementConfig(JSON.stringify(config));

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Missing alerting.rules array");
    }
  });
});
