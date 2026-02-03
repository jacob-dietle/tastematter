/**
 * File-based Structured Logger for Intel Service
 *
 * Writes JSON logs to ~/.tastematter/logs/intel-YYYY-MM-DD.jsonl
 * following the pattern from frontend/src-tauri/src/logging/service.rs
 *
 * Features:
 * - Daily log rotation (one file per day)
 * - JSONL format (one JSON object per line, greppable, jq-parseable)
 * - Graceful degradation (doesn't crash on write failures)
 * - Configurable directory for testing
 */

import { existsSync, mkdirSync, appendFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";
import type { LogEvent } from "./logger";

/**
 * Structured log event with required level and timestamp
 */
export interface StructuredLogEvent extends LogEvent {
  level: "info" | "warn" | "error";
  timestamp: string;
}

/**
 * File-based log service that persists structured events to JSONL files
 */
export class FileLogService {
  private logDir: string;
  private currentDate: string | null = null;
  private currentPath: string | null = null;

  /**
   * Create a new FileLogService
   * @param customLogDir - Optional custom directory (for testing). Defaults to ~/.tastematter/logs
   */
  constructor(customLogDir?: string) {
    this.logDir = customLogDir ?? join(homedir(), ".tastematter", "logs");
    this.ensureLogDir();
  }

  /**
   * Ensure log directory exists
   */
  private ensureLogDir(): void {
    try {
      if (!existsSync(this.logDir)) {
        mkdirSync(this.logDir, { recursive: true });
      }
    } catch {
      // Graceful degradation - directory creation failed, logs won't persist
      // but service continues running
    }
  }

  /**
   * Get the current log file path based on today's date
   * Implements daily rotation by including date in filename
   */
  private getLogPath(): string {
    const today = new Date().toISOString().split("T")[0];
    if (this.currentDate !== today) {
      this.currentDate = today;
      this.currentPath = join(this.logDir, `intel-${today}.jsonl`);
    }
    return this.currentPath!;
  }

  /**
   * Write a structured log event to the daily log file
   *
   * @param event - Structured log event with level and timestamp
   */
  log(event: StructuredLogEvent): void {
    try {
      const logPath = this.getLogPath();
      appendFileSync(logPath, JSON.stringify(event) + "\n", "utf8");
    } catch {
      // Graceful degradation - write failed, but don't crash the service
      // Console output from logger.ts will still work
    }
  }
}

/**
 * Default file logger instance using ~/.tastematter/logs
 */
export const fileLogger = new FileLogService();
