/**
 * File Logger Service Tests (TDD)
 *
 * Tests for FileLogService that persists structured logs to
 * ~/.tastematter/logs/intel-YYYY-MM-DD.jsonl
 *
 * Following Kent Beck's Red-Green-Refactor:
 * 1. Write these tests FIRST (they will fail)
 * 2. Implement file-logger.ts to make them pass
 * 3. Refactor if needed
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import { existsSync, mkdirSync, rmSync, readFileSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";

// We'll mock the home directory for testing
const TEST_LOG_DIR = join(tmpdir(), ".tastematter-test-logs");

// Clean up before and after tests
beforeEach(() => {
  if (existsSync(TEST_LOG_DIR)) {
    rmSync(TEST_LOG_DIR, { recursive: true });
  }
});

afterEach(() => {
  if (existsSync(TEST_LOG_DIR)) {
    rmSync(TEST_LOG_DIR, { recursive: true });
  }
});

describe("FileLogService", () => {
  test("creates log directory if not exists", async () => {
    // Given: Log directory does not exist
    expect(existsSync(TEST_LOG_DIR)).toBe(false);

    // When: FileLogService is instantiated with custom directory
    const { FileLogService } = await import("@/services/file-logger");
    const service = new FileLogService(TEST_LOG_DIR);

    // Then: Directory is created
    expect(existsSync(TEST_LOG_DIR)).toBe(true);
  });

  test("writes structured JSON to daily log file", async () => {
    // Given: A FileLogService instance
    const { FileLogService } = await import("@/services/file-logger");
    const service = new FileLogService(TEST_LOG_DIR);

    // When: We log an event
    const event = {
      level: "info" as const,
      timestamp: "2026-01-26T12:00:00.000Z",
      correlation_id: "test-123",
      operation: "test_operation",
      message: "Test message",
    };
    service.log(event);

    // Then: Log file exists with correct content
    const today = new Date().toISOString().split("T")[0];
    const logPath = join(TEST_LOG_DIR, `intel-${today}.jsonl`);
    expect(existsSync(logPath)).toBe(true);

    const content = readFileSync(logPath, "utf8");
    const lines = content.trim().split("\n");
    expect(lines.length).toBe(1);

    const parsed = JSON.parse(lines[0]);
    expect(parsed.correlation_id).toBe("test-123");
    expect(parsed.operation).toBe("test_operation");
    expect(parsed.level).toBe("info");
  });

  test("appends to existing log file", async () => {
    // Given: A FileLogService with one event already logged
    const { FileLogService } = await import("@/services/file-logger");
    const service = new FileLogService(TEST_LOG_DIR);

    service.log({
      level: "info" as const,
      timestamp: "2026-01-26T12:00:00.000Z",
      correlation_id: "first-event",
      message: "First",
    });

    // When: We log another event
    service.log({
      level: "error" as const,
      timestamp: "2026-01-26T12:01:00.000Z",
      correlation_id: "second-event",
      message: "Second",
    });

    // Then: Both events are in the file
    const today = new Date().toISOString().split("T")[0];
    const logPath = join(TEST_LOG_DIR, `intel-${today}.jsonl`);
    const content = readFileSync(logPath, "utf8");
    const lines = content.trim().split("\n");

    expect(lines.length).toBe(2);
    expect(JSON.parse(lines[0]).correlation_id).toBe("first-event");
    expect(JSON.parse(lines[1]).correlation_id).toBe("second-event");
  });

  test("produces valid JSONL format (one JSON per line)", async () => {
    // Given: Multiple events logged
    const { FileLogService } = await import("@/services/file-logger");
    const service = new FileLogService(TEST_LOG_DIR);

    for (let i = 0; i < 5; i++) {
      service.log({
        level: "info" as const,
        timestamp: new Date().toISOString(),
        correlation_id: `event-${i}`,
        message: `Event ${i}`,
      });
    }

    // When: We read the file
    const today = new Date().toISOString().split("T")[0];
    const logPath = join(TEST_LOG_DIR, `intel-${today}.jsonl`);
    const content = readFileSync(logPath, "utf8");
    const lines = content.trim().split("\n");

    // Then: Each line is valid JSON
    expect(lines.length).toBe(5);
    lines.forEach((line, i) => {
      expect(() => JSON.parse(line)).not.toThrow();
      expect(JSON.parse(line).correlation_id).toBe(`event-${i}`);
    });
  });

  test("handles errors gracefully without throwing", async () => {
    // Given: A FileLogService
    const { FileLogService } = await import("@/services/file-logger");
    const service = new FileLogService(TEST_LOG_DIR);

    // When: We log after directory is removed (simulating error)
    rmSync(TEST_LOG_DIR, { recursive: true });

    // Then: Should not throw (graceful degradation)
    expect(() =>
      service.log({
        level: "info" as const,
        timestamp: new Date().toISOString(),
        message: "This should not crash",
      })
    ).not.toThrow();
  });
});
