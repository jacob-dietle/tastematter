import type { Result, KnockTriggerPayload } from "./types.js";

/**
 * Triggers a Knock workflow via the Knock API.
 * Pure fetch — no SDK dependency.
 */
export async function triggerKnockWorkflow(
  apiKey: string,
  workflowKey: string,
  payload: KnockTriggerPayload
): Promise<Result<{ workflow_run_id: string }>> {
  try {
    const url = `https://api.knock.app/v1/workflows/${workflowKey}/trigger`;

    const resp = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!resp.ok) {
      const text = await resp.text();
      return {
        success: false,
        error: `Knock API ${resp.status}: ${text}`,
      };
    }

    const data = (await resp.json()) as { workflow_run_id?: string };
    return {
      success: true,
      data: { workflow_run_id: data.workflow_run_id ?? "unknown" },
    };
  } catch (err) {
    return { success: false, error: String(err) };
  }
}
