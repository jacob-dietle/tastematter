/**
 * Operation Logger Middleware Tests (TDD)
 *
 * Tests for withOperationLogging higher-order function that wraps
 * endpoint handlers with consistent structured logging.
 *
 * Following Kent Beck's Red-Green-Refactor:
 * 1. Write these tests FIRST (they will fail)
 * 2. Implement middleware to make them pass
 * 3. Refactor if needed
 */

import { describe, test, expect, mock, beforeEach, afterEach } from "bun:test";
import { withOperationLogging, type OperationConfig } from "@/middleware/operation-logger";

// Mock the logger module
const mockLogInfo = mock(() => {});
const mockLogError = mock(() => {});

// We'll need to mock the logger - store original and restore after
let originalConsoleLog: typeof console.log;
let originalConsoleError: typeof console.error;
let capturedLogs: Array<{ level: string; event: Record<string, unknown> }> = [];

beforeEach(() => {
  capturedLogs = [];
  originalConsoleLog = console.log;
  originalConsoleError = console.error;

  // Capture structured logs
  console.log = (output: string) => {
    try {
      const parsed = JSON.parse(output);
      capturedLogs.push({ level: "info", event: parsed });
    } catch {
      // Not JSON, ignore
    }
  };
  console.error = (output: string) => {
    try {
      const parsed = JSON.parse(output);
      capturedLogs.push({ level: "error", event: parsed });
    } catch {
      // Not JSON, ignore
    }
  };
});

afterEach(() => {
  console.log = originalConsoleLog;
  console.error = originalConsoleError;
});

// Helper to create mock context
function createMockContext(correlationId: string, body: unknown = {}) {
  return {
    correlationId,
    body,
    set: { status: 200 },
  };
}

describe("Operation Logger Middleware", () => {
  describe("withOperationLogging", () => {
    test("logs start event with correlation_id", async () => {
      // Given: A wrapped handler
      const config: OperationConfig = {
        operation: "test_operation",
      };
      const handler = async () => ({ result: "success" });
      const wrapped = withOperationLogging(config, handler);

      // When: Handler is invoked
      const ctx = createMockContext("test-correlation-123");
      await wrapped(ctx);

      // Then: Start event logged with correlation_id
      const startLog = capturedLogs.find(
        (l) => l.event.message?.toString().includes("Starting")
      );
      expect(startLog).toBeDefined();
      expect(startLog?.event.correlation_id).toBe("test-correlation-123");
      expect(startLog?.event.operation).toBe("test_operation");
    });

    test("logs success with duration_ms", async () => {
      // Given: A wrapped handler that takes some time
      const config: OperationConfig = {
        operation: "timed_operation",
      };
      const handler = async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        return { result: "done" };
      };
      const wrapped = withOperationLogging(config, handler);

      // When: Handler completes successfully
      const ctx = createMockContext("timing-test-456");
      await wrapped(ctx);

      // Then: Success event logged with duration_ms
      const successLog = capturedLogs.find(
        (l) => l.event.success === true
      );
      expect(successLog).toBeDefined();
      expect(successLog?.event.duration_ms).toBeGreaterThanOrEqual(10);
      expect(successLog?.event.correlation_id).toBe("timing-test-456");
    });

    test("logs error with classified error_code", async () => {
      // Given: A wrapped handler that throws
      const config: OperationConfig = {
        operation: "failing_operation",
      };
      const error = new Error("Something went wrong");
      (error as any).name = "APIError";
      (error as any).status = 429;

      const handler = async () => {
        throw error;
      };
      const wrapped = withOperationLogging(config, handler);

      // When: Handler throws
      const ctx = createMockContext("error-test-789");
      await wrapped(ctx);

      // Then: Error event logged with error_code
      const errorLog = capturedLogs.find((l) => l.level === "error");
      expect(errorLog).toBeDefined();
      expect(errorLog?.event.correlation_id).toBe("error-test-789");
      expect(errorLog?.event.error_code).toBe("RATE_LIMIT_ERROR");
      expect(errorLog?.event.success).toBe(false);
    });

    test("passes through successful result", async () => {
      // Given: A wrapped handler that returns data
      const config: OperationConfig = {
        operation: "data_operation",
      };
      const expectedResult = {
        chain_id: "abc",
        generated_name: "Test Chain",
        confidence: 0.9,
      };
      const handler = async () => expectedResult;
      const wrapped = withOperationLogging(config, handler);

      // When: Handler returns
      const ctx = createMockContext("passthrough-test");
      const result = await wrapped(ctx);

      // Then: Result is passed through unchanged
      expect(result).toEqual(expectedResult);
    });

    test("returns error response on failure", async () => {
      // Given: A wrapped handler that throws
      const config: OperationConfig = {
        operation: "error_response_operation",
      };
      const handler = async () => {
        const err = new Error("Auth failed");
        (err as any).name = "AuthenticationError";
        (err as any).status = 401;
        throw err;
      };
      const wrapped = withOperationLogging(config, handler);

      // When: Handler throws
      const ctx = createMockContext("error-response-test");
      const result = await wrapped(ctx);

      // Then: Error response returned with correct structure
      expect(result).toHaveProperty("error");
      expect(result).toHaveProperty("code", "AUTHENTICATION_ERROR");
      expect(result).toHaveProperty("message");
      expect(ctx.set.status).toBe(401);
    });

    test("includes custom input metrics in start log", async () => {
      // Given: Config with getInputMetrics
      const config: OperationConfig = {
        operation: "metrics_operation",
        getInputMetrics: (body) => ({
          chain_id: (body as any).chain_id,
          files_count: (body as any).files?.length || 0,
        }),
      };
      const handler = async () => ({ result: "ok" });
      const wrapped = withOperationLogging(config, handler);

      // When: Handler invoked with body
      const ctx = createMockContext("metrics-test", {
        chain_id: "chain-xyz",
        files: ["a.ts", "b.ts", "c.ts"],
      });
      await wrapped(ctx);

      // Then: Start log includes input metrics
      const startLog = capturedLogs.find(
        (l) => l.event.message?.toString().includes("Starting")
      );
      expect(startLog?.event.chain_id).toBe("chain-xyz");
      expect(startLog?.event.files_count).toBe(3);
    });

    test("includes custom output metrics in success log", async () => {
      // Given: Config with getOutputMetrics
      const config: OperationConfig = {
        operation: "output_metrics_operation",
        getOutputMetrics: (result) => ({
          generated_name: (result as any).generated_name,
          confidence: (result as any).confidence,
        }),
      };
      const handler = async () => ({
        generated_name: "Authentication Flow",
        confidence: 0.85,
        category: "feature",
      });
      const wrapped = withOperationLogging(config, handler);

      // When: Handler completes
      const ctx = createMockContext("output-metrics-test");
      await wrapped(ctx);

      // Then: Success log includes output metrics
      const successLog = capturedLogs.find((l) => l.event.success === true);
      expect(successLog?.event.generated_name).toBe("Authentication Flow");
      expect(successLog?.event.confidence).toBe(0.85);
    });
  });
});
