import { describe, it, expect, vi, beforeEach } from "vitest";
import { triggerKnockWorkflow } from "../src/knock.js";
import type { KnockTriggerPayload } from "../src/types.js";

const TEST_API_KEY = "sk_test_abc123";
const TEST_WORKFLOW_KEY = "new-intel-brief";
const TEST_PAYLOAD: KnockTriggerPayload = {
  recipients: ["user_123"],
  data: {
    subject: "New intel brief",
    body: "Content has changed",
    trigger_type: "content_change",
  },
};

describe("triggerKnockWorkflow", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("sends correct request to Knock API", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ workflow_run_id: "run_abc" }),
    });
    vi.stubGlobal("fetch", mockFetch);

    const result = await triggerKnockWorkflow(
      TEST_API_KEY,
      TEST_WORKFLOW_KEY,
      TEST_PAYLOAD
    );

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.workflow_run_id).toBe("run_abc");
    }

    expect(mockFetch).toHaveBeenCalledOnce();
    const [url, options] = mockFetch.mock.calls[0];
    expect(url).toBe(
      `https://api.knock.app/v1/workflows/${TEST_WORKFLOW_KEY}/trigger`
    );
    expect(options.method).toBe("POST");
    expect(options.headers.Authorization).toBe(`Bearer ${TEST_API_KEY}`);
    expect(options.headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(options.body)).toEqual(TEST_PAYLOAD);
  });

  it("returns error on 401 unauthorized", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        text: async () => "Unauthorized",
      })
    );

    const result = await triggerKnockWorkflow(
      "bad_key",
      TEST_WORKFLOW_KEY,
      TEST_PAYLOAD
    );

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("401");
      expect(result.error).toContain("Unauthorized");
    }
  });

  it("returns error on network failure", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("Network error"))
    );

    const result = await triggerKnockWorkflow(
      TEST_API_KEY,
      TEST_WORKFLOW_KEY,
      TEST_PAYLOAD
    );

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Network error");
    }
  });

  it("handles missing workflow_run_id in response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({}),
      })
    );

    const result = await triggerKnockWorkflow(
      TEST_API_KEY,
      TEST_WORKFLOW_KEY,
      TEST_PAYLOAD
    );

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.workflow_run_id).toBe("unknown");
    }
  });
});
