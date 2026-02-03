/**
 * Cost Guard Middleware
 *
 * Tracks API costs and prevents exceeding daily budget.
 * In-memory only for MVP (no persistence between restarts).
 *
 * Usage:
 * ```typescript
 * import { CostGuard, estimateCost } from "@/middleware/cost-guard";
 *
 * const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
 *
 * const estimated = estimateCost("haiku", 256);
 * if (guard.canProceed("haiku", estimated)) {
 *   // Make API call
 *   guard.recordCost("haiku", actualCost);
 * }
 * ```
 */

/**
 * Configuration for cost guard
 */
export interface CostGuardConfig {
  dailyBudgetUsd: number;
}

/**
 * Record of a single API cost
 */
interface CostRecord {
  timestamp: Date;
  model: string;
  cost_usd: number;
}

/**
 * Statistics about current spending
 */
export interface CostStats {
  today_spend_usd: number;
  daily_budget_usd: number;
  remaining_usd: number;
  request_count: number;
}

/**
 * Approximate cost per 1K tokens for each model
 * Based on Anthropic pricing (input + output combined estimate)
 */
const COST_PER_1K_TOKENS: Record<string, number> = {
  haiku: 0.00025 + 0.00125, // $0.25/1M input + $1.25/1M output
  sonnet: 0.003 + 0.015, // $3/1M input + $15/1M output
};

/**
 * Estimate cost for a request
 *
 * @param model - Model identifier ("haiku" or "sonnet")
 * @param estimatedTokens - Estimated total tokens (input + output)
 * @returns Estimated cost in USD
 */
export function estimateCost(model: string, estimatedTokens: number): number {
  const rate = COST_PER_1K_TOKENS[model] ?? COST_PER_1K_TOKENS.sonnet;
  return (estimatedTokens / 1000) * rate;
}

/**
 * Cost guard for API budget management
 */
export class CostGuard {
  private config: CostGuardConfig;
  private records: CostRecord[] = [];

  constructor(config: CostGuardConfig) {
    this.config = config;
  }

  /**
   * Check if a request can proceed without exceeding budget
   *
   * @param model - Model identifier
   * @param estimatedCost - Estimated cost for this request in USD
   * @returns true if request is within budget
   */
  canProceed(model: string, estimatedCost: number): boolean {
    const todaySpend = this.getTodaySpend();
    return todaySpend + estimatedCost <= this.config.dailyBudgetUsd;
  }

  /**
   * Record a cost after API call completes
   *
   * @param model - Model identifier
   * @param cost_usd - Actual cost in USD
   */
  recordCost(model: string, cost_usd: number): void {
    this.records.push({
      timestamp: new Date(),
      model,
      cost_usd,
    });
  }

  /**
   * Get total spend for today (UTC)
   *
   * @returns Total spend in USD
   */
  getTodaySpend(): number {
    const today = new Date();
    today.setUTCHours(0, 0, 0, 0);

    return this.records
      .filter((r) => r.timestamp >= today)
      .reduce((sum, r) => sum + r.cost_usd, 0);
  }

  /**
   * Get current statistics
   *
   * @returns Cost statistics
   */
  getStats(): CostStats {
    const todaySpend = this.getTodaySpend();
    const remaining = Math.max(0, this.config.dailyBudgetUsd - todaySpend);

    const today = new Date();
    today.setUTCHours(0, 0, 0, 0);
    const requestCount = this.records.filter((r) => r.timestamp >= today).length;

    return {
      today_spend_usd: todaySpend,
      daily_budget_usd: this.config.dailyBudgetUsd,
      remaining_usd: remaining,
      request_count: requestCount,
    };
  }
}
