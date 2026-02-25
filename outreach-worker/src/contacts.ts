/**
 * Contact CRUD Operations
 *
 * Endpoints:
 *   GET  /contacts        - List with filters (?status=, ?wave=, ?source=)
 *   GET  /contacts/:id    - Single contact with events
 *   POST /contacts        - Create single contact
 *   POST /contacts/batch  - Batch import (for commenter list)
 *   PATCH /contacts/:id   - Manual status override
 */

import type { Env, Contact, BatchImportRequest, ContactUpdateRequest, OutreachEvent } from "./types";

export async function handleListContacts(env: Env, url: URL): Promise<Response> {
  const status = url.searchParams.get("status");
  const wave = url.searchParams.get("wave");
  const source = url.searchParams.get("source");
  const limit = parseInt(url.searchParams.get("limit") || "100");

  let query = "SELECT * FROM contacts WHERE 1=1";
  const params: unknown[] = [];

  if (status) {
    query += " AND status = ?";
    params.push(status);
  }
  if (wave) {
    query += " AND wave = ?";
    params.push(wave);
  }
  if (source) {
    query += " AND source = ?";
    params.push(source);
  }

  query += " ORDER BY updated_at DESC LIMIT ?";
  params.push(limit);

  const result = await env.DB.prepare(query).bind(...params).all<Contact>();
  return Response.json({
    contacts: result.results ?? [],
    total: result.results?.length ?? 0,
  });
}

export async function handleGetContact(env: Env, contactId: string): Promise<Response> {
  const contact = await env.DB
    .prepare("SELECT * FROM contacts WHERE id = ?")
    .bind(contactId)
    .first<Contact>();

  if (!contact) {
    return Response.json({ error: "Contact not found" }, { status: 404 });
  }

  const events = await env.DB
    .prepare("SELECT * FROM outreach_events WHERE contact_id = ? ORDER BY created_at DESC LIMIT 50")
    .bind(contactId)
    .all<OutreachEvent>();

  return Response.json({
    contact,
    events: events.results ?? [],
  });
}

export async function handleCreateContact(request: Request, env: Env): Promise<Response> {
  const body = await request.json<{
    linkedin_url: string;
    name?: string;
    headline?: string;
    location?: string;
    source?: string;
    wave?: string;
  }>();

  if (!body.linkedin_url) {
    return Response.json({ error: "linkedin_url is required" }, { status: 400 });
  }

  // Check for existing
  const existing = await env.DB
    .prepare("SELECT id FROM contacts WHERE linkedin_url = ?")
    .bind(body.linkedin_url)
    .first();

  if (existing) {
    return Response.json({ error: "Contact already exists", contact_id: (existing as { id: string }).id }, { status: 409 });
  }

  await env.DB
    .prepare(
      `INSERT INTO contacts (linkedin_url, name, headline, location, source, wave)
       VALUES (?, ?, ?, ?, ?, ?)`
    )
    .bind(
      body.linkedin_url,
      body.name ?? null,
      body.headline ?? null,
      body.location ?? null,
      body.source ?? "manual",
      body.wave ?? "wave_2"
    )
    .run();

  const contact = await env.DB
    .prepare("SELECT * FROM contacts WHERE linkedin_url = ?")
    .bind(body.linkedin_url)
    .first<Contact>();

  // Create identified event
  if (contact) {
    await env.DB
      .prepare("INSERT INTO outreach_events (contact_id, event_type, event_data, source) VALUES (?, 'identified', ?, 'manual')")
      .bind(contact.id, JSON.stringify({ source: body.source ?? "manual" }))
      .run();
  }

  return Response.json({ ok: true, contact }, { status: 201 });
}

export async function handleBatchImport(request: Request, env: Env): Promise<Response> {
  const body = await request.json<BatchImportRequest>();

  if (!body.contacts || !Array.isArray(body.contacts) || body.contacts.length === 0) {
    return Response.json({ error: "contacts array is required and must not be empty" }, { status: 400 });
  }

  if (body.contacts.length > 200) {
    return Response.json({ error: "Maximum 200 contacts per batch" }, { status: 400 });
  }

  let imported = 0;
  let skipped = 0;
  const errors: Array<{ linkedin_url: string; error: string }> = [];

  for (const c of body.contacts) {
    if (!c.linkedin_url) {
      errors.push({ linkedin_url: "(missing)", error: "linkedin_url required" });
      continue;
    }

    try {
      // Check existing
      const existing = await env.DB
        .prepare("SELECT id FROM contacts WHERE linkedin_url = ?")
        .bind(c.linkedin_url)
        .first();

      if (existing) {
        skipped++;
        continue;
      }

      await env.DB
        .prepare(
          `INSERT INTO contacts (linkedin_url, name, headline, location, source, wave)
           VALUES (?, ?, ?, ?, ?, ?)`
        )
        .bind(
          c.linkedin_url,
          c.name ?? null,
          c.headline ?? null,
          c.location ?? null,
          c.source ?? "batch_import",
          c.wave ?? "wave_2"
        )
        .run();

      // Create event
      const contact = await env.DB
        .prepare("SELECT id FROM contacts WHERE linkedin_url = ?")
        .bind(c.linkedin_url)
        .first<{ id: string }>();

      if (contact) {
        await env.DB
          .prepare("INSERT INTO outreach_events (contact_id, event_type, event_data, source) VALUES (?, 'identified', ?, 'batch_import')")
          .bind(contact.id, JSON.stringify({ source: c.source ?? "batch_import", wave: c.wave ?? "wave_2" }))
          .run();
      }

      imported++;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      errors.push({ linkedin_url: c.linkedin_url, error: msg });
    }
  }

  return Response.json({
    ok: true,
    imported,
    skipped,
    errors: errors.length > 0 ? errors : undefined,
    total_submitted: body.contacts.length,
  });
}

export async function handleUpdateContact(request: Request, env: Env, contactId: string): Promise<Response> {
  const body = await request.json<ContactUpdateRequest>();

  // Verify contact exists
  const existing = await env.DB
    .prepare("SELECT * FROM contacts WHERE id = ?")
    .bind(contactId)
    .first<Contact>();

  if (!existing) {
    return Response.json({ error: "Contact not found" }, { status: 404 });
  }

  const updates: string[] = [];
  const params: unknown[] = [];

  if (body.status) {
    updates.push("status = ?");
    params.push(body.status);
  }
  if (body.wave) {
    updates.push("wave = ?");
    params.push(body.wave);
  }
  if (body.name) {
    updates.push("name = ?");
    params.push(body.name);
  }
  if (body.headline) {
    updates.push("headline = ?");
    params.push(body.headline);
  }

  if (updates.length === 0) {
    return Response.json({ error: "No fields to update" }, { status: 400 });
  }

  updates.push("updated_at = datetime('now')");
  params.push(contactId);

  await env.DB
    .prepare(`UPDATE contacts SET ${updates.join(", ")} WHERE id = ?`)
    .bind(...params)
    .run();

  // Create override event if status changed
  if (body.status && body.status !== existing.status) {
    await env.DB
      .prepare("INSERT INTO outreach_events (contact_id, event_type, event_data, source) VALUES (?, 'status_override', ?, 'manual')")
      .bind(contactId, JSON.stringify({ old_status: existing.status, new_status: body.status }))
      .run();
  }

  const updated = await env.DB
    .prepare("SELECT * FROM contacts WHERE id = ?")
    .bind(contactId)
    .first<Contact>();

  return Response.json({ ok: true, contact: updated });
}
