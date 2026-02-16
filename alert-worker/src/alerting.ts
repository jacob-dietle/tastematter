import type {
  Result,
  AlertStateRow,
  WatchRule,
  TriggerFn,
  KnockTriggerPayload,
} from "./types.js";
import type { createDB } from "./db.js";
import { parseEngagementConfig } from "./config.js";

interface EvaluateResult {
  shouldFire: boolean;
  reason: string;
}

/**
 * Evaluates a single watch rule against its prior state.
 * Phase 1 simplified: content_change always fires (no corpus SHA diff yet).
 * Disabled rules never fire.
 */
export function evaluateRule(
  rule: WatchRule,
  priorState: AlertStateRow | null
): EvaluateResult {
  if (!rule.enabled) {
    return { shouldFire: false, reason: "Rule is disabled" };
  }

  // Phase 1: content_change always fires (no corpus SHA comparison yet)
  if (rule.trigger === "content_change") {
    return { shouldFire: true, reason: "Phase 1: content_change always fires" };
  }

  // Phase 1: schedule always fires
  if (rule.trigger === "schedule") {
    return { shouldFire: true, reason: "Scheduled rule fires on cron" };
  }

  // Other trigger types not yet implemented
  return {
    shouldFire: false,
    reason: `Trigger type '${rule.trigger}' not yet implemented`,
  };
}

interface ProcessAlertRulesInput {
  db: ReturnType<typeof createDB>;
  ownerId: string;
  knockApiKey: string;
  triggerFn: TriggerFn;
}

interface ProcessAlertRulesOutput {
  fired: number;
  checked: number;
  errors: string[];
}

/**
 * Orchestrates alert evaluation across all engagements for an owner.
 * Reads engagements from DB, evaluates each rule, calls triggerFn, logs results.
 */
export async function processAlertRules(
  input: ProcessAlertRulesInput
): Promise<Result<ProcessAlertRulesOutput>> {
  const { db, ownerId, knockApiKey, triggerFn } = input;
  let fired = 0;
  let checked = 0;
  const errors: string[] = [];

  try {
    // Get all engagements for this owner
    const engagementsResult = await db.getEngagementsByOwner(ownerId);
    if (!engagementsResult.success) {
      return {
        success: false,
        error: `Failed to load engagements: ${engagementsResult.error}`,
      };
    }

    for (const engagement of engagementsResult.data) {
      // Parse alerting config
      const configResult = parseEngagementConfig(engagement.config_json);
      if (!configResult.success) {
        errors.push(
          `${engagement.id}: config parse error: ${configResult.error}`
        );
        continue;
      }

      const alertingConfig = configResult.data;
      if (!alertingConfig) {
        // No alerting config — skip
        continue;
      }

      // Evaluate each rule
      for (const rule of alertingConfig.rules) {
        checked++;

        // Get prior state
        const stateResult = await db.getAlertState(
          rule.name,
          engagement.id
        );
        const priorState = stateResult.success ? stateResult.data : null;

        const evalResult = evaluateRule(rule, priorState);

        // Update last_checked_at
        const now = new Date().toISOString();
        await db.upsertAlertState({
          rule_name: rule.name,
          engagement_id: engagement.id,
          last_checked_at: now,
          last_fired_at: evalResult.shouldFire
            ? now
            : priorState?.last_fired_at ?? undefined,
          last_corpus_sha: priorState?.last_corpus_sha ?? undefined,
        });

        if (!evalResult.shouldFire) {
          continue;
        }

        // Build payload
        const payload: KnockTriggerPayload = {
          recipients: alertingConfig.recipients.map((r) => r.id),
          data: {
            subject: `Alert: ${rule.name} (${engagement.display_name})`,
            body: evalResult.reason,
            trigger_type: rule.trigger,
          },
        };

        // Trigger notification
        const triggerResult = await triggerFn(
          knockApiKey,
          alertingConfig.workflow_key,
          payload
        );

        if (triggerResult.success) {
          fired++;
          await db.insertAlertHistory({
            engagement_id: engagement.id,
            rule_name: rule.name,
            trigger_type: rule.trigger,
            knock_workflow_run_id: triggerResult.data.workflow_run_id,
            payload: JSON.stringify(payload),
            success: 1,
          });
        } else {
          errors.push(
            `${engagement.id}/${rule.name}: trigger failed: ${triggerResult.error}`
          );
          await db.insertAlertHistory({
            engagement_id: engagement.id,
            rule_name: rule.name,
            trigger_type: rule.trigger,
            success: 0,
            error_message: triggerResult.error,
          });
        }
      }
    }

    // Log activity
    await db.insertActivityLog({
      event_type: "alert_run_complete",
      message: `Checked ${checked} rules, fired ${fired}, errors: ${errors.length}`,
      details: errors.length > 0 ? JSON.stringify(errors) : undefined,
    });

    return { success: true, data: { fired, checked, errors } };
  } catch (err) {
    return { success: false, error: String(err) };
  }
}
