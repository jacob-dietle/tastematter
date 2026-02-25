/**
 * Kondo Webhook Handler
 *
 * Receives webhook payloads from Kondo on label/message/note changes.
 * Upserts contact by linkedin_url, detects label changes, creates events.
 *
 * Kondo payload structure (verified from production test):
 *   event.type: "general-update-test" | other event types
 *   data.contact_linkedin_url: LinkedIn profile URL
 *   data.contact_first_name / contact_last_name: Name
 *   data.contact_headline / contact_location: Profile info
 *   data.kondo_labels: [{kondo_label_id, kondo_label_name, kondo_labeled_at}]
 *   data.kondo_note: Note text
 *   data.kondo_url: Kondo conversation URL
 *   data.conversation_latest_content: Last message text
 *   data.conversation_latest_timestamp: Last message ISO timestamp
 *
 * Label → Status mapping:
 *   tm-wave2     → identified (+ wave = 'wave_2')
 *   tm-contacted → contacted  (+ first_contact_at if null)
 *   tm-installed → installed  (+ install_confirmed_at)
 *   tm-active    → active
 *   tm-feedback  → feedback_received (+ feedback_count++)
 */

import type { Env, KondoWebhookPayload, ContactStatus, EventType } from "./types";
import { createFlowLogger } from "./logging";

const LABEL_STATUS_MAP: Record<string, ContactStatus> = {
  "tm-wave2": "identified",
  "tm-contacted": "contacted",
  "tm-installed": "installed",
  "tm-active": "active",
  "tm-feedback": "feedback_received",
};

// Priority order: higher index = higher priority status
const STATUS_PRIORITY: ContactStatus[] = [
  "identified",
  "contacted",
  "replied",
  "installed",
  "active",
  "feedback_received",
];

export async function handleWebhook(request: Request, env: Env): Promise<Response> {
  const logger = createFlowLogger(env.DB, "kondo-webhook");

  try {
    const raw: KondoWebhookPayload = await request.json();
    const executionId = await logger.start();

    // 1. Log raw payload
    await env.DB
      .prepare("INSERT INTO webhook_log (payload) VALUES (?)")
      .bind(JSON.stringify(raw))
      .run();

    const data = raw.data;

    // 2. Extract linkedin_url from nested data
    const linkedinUrl = data?.contact_linkedin_url;
    if (!linkedinUrl) {
      await logger.warn("Webhook missing contact_linkedin_url", {
        event_type: raw.event?.type,
        has_data: !!data,
        data_keys: data ? Object.keys(data) : [],
      });
      await env.DB
        .prepare("UPDATE webhook_log SET processed = 1, error_message = ? WHERE id = (SELECT MAX(id) FROM webhook_log)")
        .bind("missing contact_linkedin_url")
        .run();
      return Response.json({ ok: true, skipped: "no contact_linkedin_url" });
    }

    // 3. Normalize Kondo payload into our working format
    const name = [data.contact_first_name, data.contact_last_name].filter(Boolean).join(" ") || null;
    const headline = data.contact_headline ?? null;
    const location = data.contact_location ?? null;
    const kondoUrl = data.kondo_url ?? null;
    const kondoNotes = data.kondo_note ?? null;
    const messageTimestamp = data.conversation_latest_timestamp ?? null;
    const messagePreview = data.conversation_latest_content?.slice(0, 500) ?? null;

    // Extract label names from Kondo's label objects
    const newLabels: string[] = (data.kondo_labels ?? []).map((l) => l.kondo_label_name);

    await logger.step("Processing webhook", {
      linkedin_url: linkedinUrl,
      event_type: raw.event?.type,
      labels: newLabels,
    });

    // 4. Get existing contact (if any) for label diff
    const existing = await env.DB
      .prepare("SELECT id, status, kondo_labels, feedback_count FROM contacts WHERE linkedin_url = ?")
      .bind(linkedinUrl)
      .first<{ id: string; status: ContactStatus; kondo_labels: string | null; feedback_count: number }>();

    const oldLabels: string[] = existing?.kondo_labels ? JSON.parse(existing.kondo_labels) : [];

    // 5. Determine highest-priority status from labels
    let targetStatus: ContactStatus = existing?.status ?? "identified";
    for (const label of newLabels) {
      const mappedStatus = LABEL_STATUS_MAP[label];
      if (mappedStatus) {
        const mappedPriority = STATUS_PRIORITY.indexOf(mappedStatus);
        const currentPriority = STATUS_PRIORITY.indexOf(targetStatus);
        if (mappedPriority > currentPriority) {
          targetStatus = mappedStatus;
        }
      }
    }

    // 6. Build field updates for special labels
    const now = new Date().toISOString();
    let firstContactAt: string | null = null;
    let installConfirmedAt: string | null = null;
    let feedbackCount = existing?.feedback_count ?? 0;

    if (newLabels.includes("tm-contacted") && !existing?.status?.match(/contacted|replied|installed|active|feedback_received/)) {
      firstContactAt = now;
    }
    if (newLabels.includes("tm-installed")) {
      installConfirmedAt = now;
    }
    if (newLabels.includes("tm-feedback") && !oldLabels.includes("tm-feedback")) {
      feedbackCount += 1;
    }

    // 7. Upsert contact
    if (existing) {
      await env.DB
        .prepare(
          `UPDATE contacts SET
            name = COALESCE(?, name),
            headline = COALESCE(?, headline),
            location = COALESCE(?, location),
            kondo_labels = ?,
            kondo_notes = COALESCE(?, kondo_notes),
            kondo_url = COALESCE(?, kondo_url),
            last_message_at = COALESCE(?, last_message_at),
            last_message_preview = COALESCE(?, last_message_preview),
            first_contact_at = COALESCE(?, first_contact_at),
            install_confirmed_at = COALESCE(?, install_confirmed_at),
            feedback_count = ?,
            status = ?,
            updated_at = datetime('now')
          WHERE id = ?`
        )
        .bind(
          name,
          headline,
          location,
          JSON.stringify(newLabels),
          kondoNotes,
          kondoUrl,
          messageTimestamp,
          messagePreview,
          firstContactAt,
          installConfirmedAt,
          feedbackCount,
          targetStatus,
          existing.id
        )
        .run();
    } else {
      const wave = newLabels.includes("tm-wave2") ? "wave_2" : "wave_1";

      await env.DB
        .prepare(
          `INSERT INTO contacts (linkedin_url, name, headline, location, source, wave, status, kondo_labels, kondo_notes, kondo_url, last_message_at, last_message_preview, first_contact_at, install_confirmed_at, feedback_count)
           VALUES (?, ?, ?, ?, 'kondo_webhook', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
        )
        .bind(
          linkedinUrl,
          name,
          headline,
          location,
          wave,
          targetStatus,
          JSON.stringify(newLabels),
          kondoNotes,
          kondoUrl,
          messageTimestamp,
          messagePreview,
          firstContactAt,
          installConfirmedAt,
          feedbackCount
        )
        .run();
    }

    // 8. Get contact ID for events
    const contact = await env.DB
      .prepare("SELECT id FROM contacts WHERE linkedin_url = ?")
      .bind(linkedinUrl)
      .first<{ id: string }>();

    if (!contact) {
      await logger.error("Contact not found after upsert", { linkedin_url: linkedinUrl });
      return Response.json({ ok: false, error: "upsert failed" }, { status: 500 });
    }

    // 9. Create events for detected label changes
    const addedLabels = newLabels.filter((l) => !oldLabels.includes(l));
    const removedLabels = oldLabels.filter((l) => !newLabels.includes(l));

    for (const label of addedLabels) {
      await env.DB
        .prepare("INSERT INTO outreach_events (contact_id, event_type, event_data, source) VALUES (?, ?, ?, 'kondo_webhook')")
        .bind(contact.id, "label_changed" as EventType, JSON.stringify({ label_added: label }))
        .run();
    }

    for (const label of removedLabels) {
      await env.DB
        .prepare("INSERT INTO outreach_events (contact_id, event_type, event_data, source) VALUES (?, ?, ?, 'kondo_webhook')")
        .bind(contact.id, "label_changed" as EventType, JSON.stringify({ label_removed: label }))
        .run();
    }

    // If status changed, log it
    if (existing && existing.status !== targetStatus) {
      const eventType: EventType =
        targetStatus === "installed" ? "install_confirmed" :
        targetStatus === "feedback_received" ? "feedback_received" : "label_changed";
      await env.DB
        .prepare("INSERT INTO outreach_events (contact_id, event_type, event_data, source) VALUES (?, ?, ?, 'kondo_webhook')")
        .bind(contact.id, eventType, JSON.stringify({ old_status: existing.status, new_status: targetStatus }))
        .run();
    }

    // Mark webhook as processed
    await env.DB
      .prepare("UPDATE webhook_log SET processed = 1 WHERE id = (SELECT MAX(id) FROM webhook_log)")
      .run();

    await logger.step("Webhook processed", {
      contact_id: contact.id,
      status: targetStatus,
      labels_added: addedLabels.length,
      labels_removed: removedLabels.length,
    });
    await logger.complete();

    return Response.json({
      ok: true,
      contact_id: contact.id,
      status: targetStatus,
      events_created: addedLabels.length + removedLabels.length + (existing && existing.status !== targetStatus ? 1 : 0),
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("Webhook handler error:", message);
    await logger.fail(err instanceof Error ? err : message);
    return Response.json({ error: message }, { status: 500 });
  }
}
