import { describe, test, expect, beforeEach } from "bun:test";

/**
 * RED Tests for Cost Guard Middleware
 *
 * TDD: Write tests FIRST, then implement middleware to make them GREEN
 *
 * Cost guard tracks API costs and prevents exceeding daily budget.
 * In-memory only for MVP (no persistence).
 */

describe("Cost Guard Middleware", () => {
  describe("CostGuard class", () => {
    test("allows request when under daily budget", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
      expect(guard.canProceed("haiku", 0.00025)).toBe(true);
    });

    test("blocks request when would exceed daily budget", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 0.001 });
      guard.recordCost("haiku", 0.001);
      expect(guard.canProceed("haiku", 0.00025)).toBe(false);
    });

    test("tracks costs by model", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
      guard.recordCost("haiku", 0.001);
      guard.recordCost("sonnet", 0.003);
      expect(guard.getTodaySpend()).toBeCloseTo(0.004, 5);
    });

    test("getTodaySpend returns 0 when no costs recorded", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
      expect(guard.getTodaySpend()).toBe(0);
    });

    test("multiple recordCost calls accumulate", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
      guard.recordCost("haiku", 0.001);
      guard.recordCost("haiku", 0.001);
      guard.recordCost("haiku", 0.001);
      expect(guard.getTodaySpend()).toBeCloseTo(0.003, 5);
    });

    test("canProceed considers existing spend plus estimated cost", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 0.005 });
      guard.recordCost("haiku", 0.003);
      // 0.003 + 0.003 = 0.006 > 0.005 budget
      expect(guard.canProceed("sonnet", 0.003)).toBe(false);
      // 0.003 + 0.001 = 0.004 < 0.005 budget
      expect(guard.canProceed("haiku", 0.001)).toBe(true);
    });

    test("allows request when exactly at budget", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 0.002 });
      guard.recordCost("haiku", 0.001);
      // 0.001 + 0.001 = 0.002 = budget (should be allowed)
      expect(guard.canProceed("haiku", 0.001)).toBe(true);
    });
  });

  describe("estimateCost helper", () => {
    test("estimateCost returns estimate for haiku", async () => {
      const { estimateCost } = await import("@/middleware/cost-guard");
      const cost = estimateCost("haiku", 256);
      expect(cost).toBeGreaterThan(0);
      expect(cost).toBeLessThan(0.001); // Haiku is cheap
    });

    test("estimateCost returns higher estimate for sonnet", async () => {
      const { estimateCost } = await import("@/middleware/cost-guard");
      const haikuCost = estimateCost("haiku", 256);
      const sonnetCost = estimateCost("sonnet", 256);
      expect(sonnetCost).toBeGreaterThan(haikuCost);
    });

    test("estimateCost scales with token count", async () => {
      const { estimateCost } = await import("@/middleware/cost-guard");
      const smallCost = estimateCost("haiku", 100);
      const largeCost = estimateCost("haiku", 1000);
      expect(largeCost).toBeGreaterThan(smallCost);
    });
  });

  describe("getStats method", () => {
    test("getStats returns current statistics", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
      guard.recordCost("haiku", 0.001);
      guard.recordCost("sonnet", 0.003);

      const stats = guard.getStats();
      expect(stats.today_spend_usd).toBeCloseTo(0.004, 5);
      expect(stats.daily_budget_usd).toBe(1.0);
      expect(stats.remaining_usd).toBeCloseTo(0.996, 5);
      expect(stats.request_count).toBe(2);
    });

    test("getStats shows correct remaining when over budget", async () => {
      const { CostGuard } = await import("@/middleware/cost-guard");
      const guard = new CostGuard({ dailyBudgetUsd: 0.001 });
      guard.recordCost("sonnet", 0.003);

      const stats = guard.getStats();
      expect(stats.remaining_usd).toBe(0); // Never negative
    });
  });
});
