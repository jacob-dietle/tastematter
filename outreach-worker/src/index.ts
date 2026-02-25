/**
 * Tastematter Outreach Worker - Entry Point
 *
 * Routes:
 *   GET  /health          - Health check (public)
 *   POST /webhook         - Kondo webhook receiver (webhook secret)
 *   GET  /dashboard       - Pipeline summary (CF Access auth)
 *   GET  /contacts        - List contacts with filters (CF Access auth)
 *   GET  /contacts/:id    - Contact detail + events (CF Access auth)
 *   POST /contacts        - Create single contact (CF Access auth)
 *   POST /contacts/batch  - Batch import (CF Access auth)
 *   PATCH /contacts/:id   - Manual status override (CF Access auth)
 *   GET  /events          - Recent events feed (CF Access auth)
 */

import type { Env } from "./types";
import { checkServiceToken, checkWebhookSecret } from "./auth";
import { handleWebhook } from "./webhook";
import { handleListContacts, handleGetContact, handleCreateContact, handleBatchImport, handleUpdateContact } from "./contacts";
import { handleDashboard } from "./dashboard";
import { handleEvents } from "./events";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;
    const method = request.method;

    try {
      // ----------------------------------------------------------------
      // Public routes
      // ----------------------------------------------------------------

      if (path === "/health") {
        return Response.json({
          status: "ok",
          worker: "tastematter-outreach",
          timestamp: new Date().toISOString(),
        });
      }

      // ----------------------------------------------------------------
      // Webhook route (authenticated via webhook secret)
      // ----------------------------------------------------------------

      if (path === "/webhook" && method === "POST") {
        const webhookAuthError = checkWebhookSecret(request, env);
        if (webhookAuthError) return webhookAuthError;
        return handleWebhook(request, env);
      }

      // ----------------------------------------------------------------
      // Protected routes (CF Access service token required)
      // ----------------------------------------------------------------

      const authError = checkServiceToken(request, env);
      if (authError) return authError;

      // Dashboard
      if (path === "/dashboard" && method === "GET") {
        return handleDashboard(env);
      }

      // Contacts - batch import (must come before /contacts/:id match)
      if (path === "/contacts/batch" && method === "POST") {
        return handleBatchImport(request, env);
      }

      // Contacts - list
      if (path === "/contacts" && method === "GET") {
        return handleListContacts(env, url);
      }

      // Contacts - create single
      if (path === "/contacts" && method === "POST") {
        return handleCreateContact(request, env);
      }

      // Contacts - get by ID or update by ID
      const contactMatch = path.match(/^\/contacts\/([a-f0-9]+)$/);
      if (contactMatch) {
        const contactId = contactMatch[1];
        if (method === "GET") {
          return handleGetContact(env, contactId);
        }
        if (method === "PATCH") {
          return handleUpdateContact(request, env, contactId);
        }
      }

      // Events feed
      if (path === "/events" && method === "GET") {
        return handleEvents(env, url);
      }

      // ----------------------------------------------------------------
      // Catch-all
      // ----------------------------------------------------------------
      return Response.json(
        {
          endpoints: {
            "GET /health": "Health check (public)",
            "POST /webhook": "Kondo webhook receiver (webhook secret)",
            "GET /dashboard": "Pipeline summary (auth required)",
            "GET /contacts": "List contacts (auth required)",
            "GET /contacts/:id": "Contact detail + events (auth required)",
            "POST /contacts": "Create contact (auth required)",
            "POST /contacts/batch": "Batch import (auth required)",
            "PATCH /contacts/:id": "Update contact (auth required)",
            "GET /events": "Events feed (auth required)",
          },
        },
        { status: 404 }
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error("Unhandled request error:", message);
      return Response.json({ error: message }, { status: 500 });
    }
  },
} satisfies ExportedHandler<Env>;
