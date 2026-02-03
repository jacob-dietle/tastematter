/**
 * Error Classification Tests (TDD)
 *
 * Tests that Anthropic SDK errors are mapped to correct HTTP status codes.
 * Uses duck-typed error objects (name + status) for ESM compatibility.
 */

import { describe, test, expect } from "bun:test";
import { classifyError } from "@/index";

// Helper to create SDK-like error objects
function createAPIError(status: number, name = "APIError") {
  const error = new Error(`API Error ${status}`);
  (error as any).name = name;
  (error as any).status = status;
  return error;
}

function createConnectionError(name = "APIConnectionError") {
  const error = new Error("Connection failed");
  (error as any).name = name;
  return error;
}

describe("Error Classification", () => {
  describe("classifyError function", () => {
    test("401 status returns AUTHENTICATION_ERROR", () => {
      const error = createAPIError(401, "AuthenticationError");
      const result = classifyError(error);
      expect(result.status).toBe(401);
      expect(result.code).toBe("AUTHENTICATION_ERROR");
    });

    test("429 status returns RATE_LIMIT_ERROR", () => {
      const error = createAPIError(429, "RateLimitError");
      const result = classifyError(error);
      expect(result.status).toBe(429);
      expect(result.code).toBe("RATE_LIMIT_ERROR");
    });

    test("APIConnectionError returns SERVICE_UNAVAILABLE", () => {
      const error = createConnectionError("APIConnectionError");
      const result = classifyError(error);
      expect(result.status).toBe(503);
      expect(result.code).toBe("SERVICE_UNAVAILABLE");
    });

    test("APIConnectionTimeoutError returns SERVICE_UNAVAILABLE", () => {
      const error = createConnectionError("APIConnectionTimeoutError");
      const result = classifyError(error);
      expect(result.status).toBe(503);
      expect(result.code).toBe("SERVICE_UNAVAILABLE");
    });

    test("400 status returns BAD_REQUEST", () => {
      const error = createAPIError(400, "BadRequestError");
      const result = classifyError(error);
      expect(result.status).toBe(400);
      expect(result.code).toBe("BAD_REQUEST");
    });

    test("500 status returns UPSTREAM_ERROR", () => {
      const error = createAPIError(500, "InternalServerError");
      const result = classifyError(error);
      expect(result.status).toBe(502);
      expect(result.code).toBe("UPSTREAM_ERROR");
    });

    test("529 status (overloaded) returns UPSTREAM_ERROR", () => {
      const error = createAPIError(529, "OverloadedError");
      const result = classifyError(error);
      expect(result.status).toBe(502);
      expect(result.code).toBe("UPSTREAM_ERROR");
    });

    test("Unknown Error returns INTERNAL_ERROR", () => {
      const error = new Error("Something unexpected");
      const result = classifyError(error);
      expect(result.status).toBe(500);
      expect(result.code).toBe("INTERNAL_ERROR");
    });

    test("Non-Error value returns INTERNAL_ERROR", () => {
      const result = classifyError("string error");
      expect(result.status).toBe(500);
      expect(result.code).toBe("INTERNAL_ERROR");
    });

    test("null returns INTERNAL_ERROR", () => {
      const result = classifyError(null);
      expect(result.status).toBe(500);
      expect(result.code).toBe("INTERNAL_ERROR");
    });
  });
});
