/**
 * Correlation ID Middleware for request tracing
 *
 * Propagates X-Correlation-ID header through the request/response cycle.
 * If no correlation ID is provided, generates a new UUID v4.
 *
 * Usage:
 * ```typescript
 * const app = new Elysia()
 *   .use(correlationMiddleware())
 *   .get("/", ({ correlationId }) => {
 *     // correlationId is available directly in context
 *   });
 * ```
 */

import { Elysia } from "elysia";

const CORRELATION_HEADER = "X-Correlation-ID";

/**
 * Generate a UUID v4 for correlation ID
 */
function generateCorrelationId(): string {
  return crypto.randomUUID();
}

/**
 * Elysia plugin for correlation ID propagation
 */
export function correlationMiddleware() {
  return new Elysia({ name: "correlation" })
    .derive({ as: "scoped" }, ({ request }) => {
      // Check for existing correlation ID (case-insensitive header lookup)
      const existingId =
        request.headers.get(CORRELATION_HEADER) ||
        request.headers.get(CORRELATION_HEADER.toLowerCase());

      // Use existing or generate new
      const correlationId = existingId || generateCorrelationId();

      // Return correlation ID as a derived context property
      return { correlationId };
    })
    .onAfterHandle({ as: "scoped" }, ({ correlationId, set }) => {
      // Add correlation ID to response headers
      if (correlationId) {
        set.headers[CORRELATION_HEADER] = correlationId;
      }
    });
}

/**
 * Helper to extract correlation ID from store or context
 * Works with both legacy store pattern and new derive pattern
 *
 * @param store - The Elysia request store or context
 * @returns The correlation ID or undefined
 */
export function getCorrelationId(store: unknown): string | undefined {
  if (store && typeof store === "object" && "correlationId" in store) {
    return (store as { correlationId: string }).correlationId;
  }
  return undefined;
}
