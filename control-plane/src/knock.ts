import type { Result, SystemRegistryRow, WorkerWithStatus, SystemStatus } from "./types.js";

interface SystemAlertPayload {
  recipients: string[];
  data: {
    system_id: string;
    system_name: string;
    previous_status: string;
    current_status: string;
    changed_at: string;
    affected_workers: Array<{ name: string; status: string; error?: string }>;
    summary: string;
  };
}

export async function triggerSystemAlert(
  apiKey: string,
  workflowKey: string,
  payload: SystemAlertPayload,
): Promise<Result<{ workflow_run_id: string }>> {
  try {
    const resp = await fetch(`https://api.knock.app/v1/workflows/${workflowKey}/trigger`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify(payload),
    });
    if (!resp.ok) {
      const text = await resp.text();
      return { success: false, error: `Knock API ${resp.status}: ${text}` };
    }
    const body = (await resp.json()) as { workflow_run_id?: string };
    return { success: true, data: { workflow_run_id: body.workflow_run_id ?? "unknown" } };
  } catch (err) {
    return { success: false, error: String(err) };
  }
}

export function buildAlertSummary(
  system: SystemRegistryRow,
  status: SystemStatus,
  members: WorkerWithStatus[],
): string {
  if (status === "broken") {
    const downWorkers = members.filter(
      (m) => m.current_status === "down" || m.current_status === "timeout",
    );
    return `${system.display_name} is BROKEN. ${downWorkers.map((w) => w.display_name).join(", ")} down.`;
  }
  return `${system.display_name} recovered to ${status}.`;
}
