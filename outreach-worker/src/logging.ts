/**
 * Structured D1 Logging (from cf-worker-scaffold)
 *
 * Lifecycle: start() -> step() / warn() / error() -> complete() or fail()
 */

export interface FlowLogger {
  start(inputId?: string): Promise<string>;
  step(message: string, details?: Record<string, unknown>): Promise<void>;
  warn(message: string, details?: Record<string, unknown>): Promise<void>;
  error(message: string, details?: Record<string, unknown>): Promise<void>;
  complete(outputPath?: string): Promise<void>;
  fail(error: Error | string): Promise<void>;
}

export function createFlowLogger(db: D1Database, flowName: string): FlowLogger {
  let executionId: string | null = null;
  let startTime: number | null = null;
  let inputId: string | null = null;

  async function log(
    eventType: string,
    level: string,
    message: string,
    extra: Record<string, unknown> = {}
  ): Promise<void> {
    if (!executionId) throw new Error("Logger not started");
    await db
      .prepare(
        `INSERT INTO flow_logs (flow_name, execution_id, event_type, level, message, details, duration_ms, input_id, output_path, error_message, error_stack)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .bind(
        flowName,
        executionId,
        eventType,
        level,
        message,
        extra.details ? JSON.stringify(extra.details) : null,
        extra.duration_ms ?? null,
        inputId,
        extra.output_path ?? null,
        extra.error_message ?? null,
        extra.error_stack ?? null
      )
      .run();
  }

  return {
    async start(input?: string): Promise<string> {
      executionId = crypto.randomUUID();
      startTime = Date.now();
      inputId = input ?? null;
      await log("started", "info", "Execution started");
      return executionId;
    },

    async step(message: string, details?: Record<string, unknown>): Promise<void> {
      await log("step", "info", message, { details });
    },

    async warn(message: string, details?: Record<string, unknown>): Promise<void> {
      await log("step", "warn", message, { details });
    },

    async error(message: string, details?: Record<string, unknown>): Promise<void> {
      await log("step", "error", message, { details });
    },

    async complete(outputPath?: string): Promise<void> {
      if (!startTime) throw new Error("Logger not started");
      const durationMs = Date.now() - startTime;
      await log("completed", "info", "Execution completed", { duration_ms: durationMs, output_path: outputPath });

      await db
        .prepare(
          `INSERT INTO flow_health (flow_name, last_execution_id, last_run_at, last_status, last_duration_ms, total_executions, avg_duration_ms)
           VALUES (?, ?, datetime('now'), 'completed', ?, 1, ?)
           ON CONFLICT(flow_name) DO UPDATE SET
             last_execution_id = excluded.last_execution_id,
             last_run_at = excluded.last_run_at,
             last_status = 'completed',
             last_duration_ms = excluded.last_duration_ms,
             last_error = NULL,
             total_executions = total_executions + 1,
             avg_duration_ms = (avg_duration_ms * total_executions + excluded.last_duration_ms) / (total_executions + 1),
             updated_at = datetime('now')`
        )
        .bind(flowName, executionId, durationMs, durationMs)
        .run();
    },

    async fail(error: Error | string): Promise<void> {
      if (!startTime) throw new Error("Logger not started");
      const durationMs = Date.now() - startTime;
      const errorMessage = error instanceof Error ? error.message : error;
      const errorStack = error instanceof Error ? error.stack ?? null : null;
      await log("failed", "error", "Execution failed", {
        duration_ms: durationMs,
        error_message: errorMessage,
        error_stack: errorStack,
      });

      await db
        .prepare(
          `INSERT INTO flow_health (flow_name, last_execution_id, last_run_at, last_status, last_duration_ms, last_error, total_executions, total_failures)
           VALUES (?, ?, datetime('now'), 'failed', ?, ?, 1, 1)
           ON CONFLICT(flow_name) DO UPDATE SET
             last_execution_id = excluded.last_execution_id,
             last_run_at = excluded.last_run_at,
             last_status = 'failed',
             last_duration_ms = excluded.last_duration_ms,
             last_error = excluded.last_error,
             total_executions = total_executions + 1,
             total_failures = total_failures + 1,
             updated_at = datetime('now')`
        )
        .bind(flowName, executionId, durationMs, errorMessage)
        .run();
    },
  };
}
