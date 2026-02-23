import { describe, it, expect } from "vitest";
import { logQuery, getQueryLogs } from "../src/query-logging.js";
import { createMockD1 } from "./helpers.js";

describe("logQuery", () => {
  it("inserts a query log entry", async () => {
    const mock = createMockD1();
    const result = await logQuery(mock as unknown as D1Database, {
      engagement_id: "nickel",
      query: "What is this knowledge base?",
      response_length: 500,
      duration_ms: 1200,
      tool_calls: 3,
      corpus_commit: "abc123",
    });

    expect(result.success).toBe(true);
    expect(mock._calls[0].query).toContain("INSERT INTO query_log");
    expect(mock._calls[0].binds[0]).toBe("nickel");
    expect(mock._calls[0].binds[1]).toBe("What is this knowledge base?");
  });

  it("handles optional fields as null", async () => {
    const mock = createMockD1();
    const result = await logQuery(mock as unknown as D1Database, {
      engagement_id: "default",
      query: "test query",
    });

    expect(result.success).toBe(true);
    expect(mock._calls[0].binds[2]).toBeNull(); // response_length
    expect(mock._calls[0].binds[3]).toBeNull(); // duration_ms
    expect(mock._calls[0].binds[4]).toBeNull(); // tool_calls
  });

  it("returns error on D1 failure", async () => {
    const mock = createMockD1({ shouldThrow: "D1 error" });
    const result = await logQuery(mock as unknown as D1Database, {
      engagement_id: "test",
      query: "test",
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("D1 error");
    }
  });
});

describe("getQueryLogs", () => {
  it("returns query logs", async () => {
    const mockRow = {
      id: 1,
      engagement_id: "nickel",
      timestamp: "2026-01-01",
      query: "test query",
      response_length: 500,
      duration_ms: 1200,
      tool_calls: 3,
      corpus_commit: "abc123",
      success: 1,
      error_message: null,
    };
    const mock = createMockD1({ allResults: [mockRow] });
    const result = await getQueryLogs(mock as unknown as D1Database);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data).toHaveLength(1);
    }
  });

  it("filters by engagement_id", async () => {
    const mock = createMockD1({ allResults: [] });
    await getQueryLogs(mock as unknown as D1Database, {
      engagementId: "nickel",
    });

    expect(mock._calls[0].query).toContain("WHERE engagement_id = ?");
    expect(mock._calls[0].binds[0]).toBe("nickel");
  });

  it("applies limit", async () => {
    const mock = createMockD1({ allResults: [] });
    await getQueryLogs(mock as unknown as D1Database, { limit: 10 });

    expect(mock._calls[0].query).toContain("LIMIT ?");
  });
});
