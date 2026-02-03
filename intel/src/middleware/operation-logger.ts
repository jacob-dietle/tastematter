/**
 * Operation Logger Middleware
 *
 * Higher-order function that wraps endpoint handlers with consistent
 * structured logging for start, success, and error events.
 *
 * Implements Charity Majors' "wide structured events" pattern:
 * - One event per operation (not scattered printf statements)
 * - Correlation ID for cross-service tracing
 * - Duration measurement for performance visibility
 *
 * Usage:
 * ```typescript
 * const result = await withOperationLogging({
 *   operation: "name_chain",
 *   getInputMetrics: (body) => ({ chain_id: body.chain_id }),
 *   getOutputMetrics: (r) => ({ generated_name: r.generated_name }),
 * }, async () => nameChain(client, data))(ctx);
 * ```
 */

import { log } from "../services/logger";
import { classifyError } from "../index";

/**
 * Configuration for operation logging
 */
export interface OperationConfig {
  /** Operation name for logs (e.g., "name_chain", "analyze_commit") */
  operation: string;

  /** Extract metrics from request body for start log */
  getInputMetrics?: (body: unknown) => Record<string, unknown>;

  /** Extract metrics from result for success log */
  getOutputMetrics?: (result: unknown) => Record<string, unknown>;
}

/**
 * Context type expected by the middleware
 */
interface OperationContext {
  correlationId: string;
  body: unknown;
  set: { status: number };
}

/**
 * Error response structure
 */
interface ErrorResponse {
  error: string;
  code: string;
  message: string;
}

/**
 * Wraps an async handler with structured logging
 *
 * @param config - Operation configuration (name, metrics extractors)
 * @param handler - The async handler to wrap
 * @returns Wrapped handler that logs start/success/error events
 */
export function withOperationLogging<T>(
  config: OperationConfig,
  handler: (ctx: OperationContext) => Promise<T>
): (ctx: OperationContext) => Promise<T | ErrorResponse> {
  const { operation, getInputMetrics, getOutputMetrics } = config;

  return async (ctx: OperationContext): Promise<T | ErrorResponse> => {
    const { correlationId, body, set } = ctx;
    const startTime = Date.now();

    // Extract input metrics if provided
    const inputMetrics = getInputMetrics ? getInputMetrics(body) : {};

    // Log start event
    log.info({
      correlation_id: correlationId,
      operation,
      ...inputMetrics,
      message: `Starting ${operation}`,
    });

    try {
      // Execute the handler
      const result = await handler(ctx);

      // Extract output metrics if provided
      const outputMetrics = getOutputMetrics ? getOutputMetrics(result) : {};

      // Log success event
      log.info({
        correlation_id: correlationId,
        operation,
        duration_ms: Date.now() - startTime,
        success: true,
        ...outputMetrics,
        message: `${operation} completed`,
      });

      return result;
    } catch (error) {
      // Classify the error for appropriate HTTP status
      const { status, code } = classifyError(error);

      // Log error event
      log.error({
        correlation_id: correlationId,
        operation,
        duration_ms: Date.now() - startTime,
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
        error_code: code,
        message: `${operation} failed`,
      });

      // Set HTTP status
      set.status = status;

      // Return error response
      return {
        error: `${operation.replace(/_/g, " ")} failed`,
        code,
        message: error instanceof Error ? error.message : "Unknown error",
      };
    }
  };
}
