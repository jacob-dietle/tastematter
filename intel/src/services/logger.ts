/**
 * Structured Logger Service
 *
 * Provides JSON-formatted logging with correlation IDs for request tracing.
 * All log events include timestamp and level.
 *
 * Logs are output to:
 * 1. Console (stdout/stderr) - for real-time visibility
 * 2. File (~/.tastematter/logs/intel-YYYY-MM-DD.jsonl) - for persistence
 *
 * Usage:
 * ```typescript
 * import { log } from "@/services/logger";
 *
 * log.info({
 *   correlation_id: id,
 *   operation: "analyze_commit",
 *   commit_hash: request.commit_hash,
 *   message: "Starting commit analysis",
 * });
 * ```
 */

import { fileLogger, type StructuredLogEvent } from "./file-logger";

export interface LogEvent {
  [key: string]: unknown;
  message?: string;
  correlation_id?: string;
  operation?: string;
}

interface StructuredLog extends LogEvent {
  level: "info" | "warn" | "error";
  timestamp: string;
}

function formatLog(level: StructuredLog["level"], event: LogEvent): StructuredLog {
  return {
    level,
    timestamp: new Date().toISOString(),
    ...event,
  };
}

/**
 * Structured logger with JSON output
 *
 * Outputs to both console and file for:
 * - Real-time visibility (console)
 * - Persistence and analysis (file)
 */
export const log = {
  /**
   * Log informational message
   */
  info: (event: LogEvent): void => {
    const structured = formatLog("info", event);
    console.log(JSON.stringify(structured));
    fileLogger.log(structured as StructuredLogEvent);
  },

  /**
   * Log warning message
   */
  warn: (event: LogEvent): void => {
    const structured = formatLog("warn", event);
    console.warn(JSON.stringify(structured));
    fileLogger.log(structured as StructuredLogEvent);
  },

  /**
   * Log error message
   */
  error: (event: LogEvent): void => {
    const structured = formatLog("error", event);
    console.error(JSON.stringify(structured));
    fileLogger.log(structured as StructuredLogEvent);
  },
};
