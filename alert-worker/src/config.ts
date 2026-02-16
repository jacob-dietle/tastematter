import type { Result, AlertingConfig } from "./types.js";

/**
 * Parses the config_json from an engagement row and extracts the alerting section.
 * Returns null data if the engagement has no alerting config (e.g., publishing-only).
 */
export function parseEngagementConfig(
  configJson: string
): Result<AlertingConfig | null> {
  try {
    const parsed = JSON.parse(configJson);

    if (!parsed.alerting) {
      return { success: true, data: null };
    }

    const alerting = parsed.alerting;

    if (!alerting.provider || alerting.provider !== "knock") {
      return {
        success: false,
        error: `Unsupported alerting provider: ${alerting.provider}`,
      };
    }

    if (!alerting.workflow_key) {
      return {
        success: false,
        error: "Missing alerting.workflow_key",
      };
    }

    if (!Array.isArray(alerting.recipients) || alerting.recipients.length === 0) {
      return {
        success: false,
        error: "Missing or empty alerting.recipients",
      };
    }

    if (!Array.isArray(alerting.rules)) {
      return {
        success: false,
        error: "Missing alerting.rules array",
      };
    }

    return {
      success: true,
      data: alerting as AlertingConfig,
    };
  } catch (err) {
    return {
      success: false,
      error: `Invalid JSON: ${String(err)}`,
    };
  }
}
