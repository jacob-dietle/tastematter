/**
 * CF Access Service Token Authentication
 *
 * Pattern from cf-worker-scaffold: validates CF-Access-Client-Id and
 * CF-Access-Client-Secret headers against environment variables.
 */

import type { Env } from "./types";

export function checkServiceToken(request: Request, env: Env): Response | null {
  const clientId = request.headers.get("CF-Access-Client-Id");
  const clientSecret = request.headers.get("CF-Access-Client-Secret");

  if (!clientId || !clientSecret) {
    return Response.json(
      { error: "Missing service token headers (CF-Access-Client-Id, CF-Access-Client-Secret)" },
      { status: 401 }
    );
  }

  if (clientId !== env.CF_ACCESS_CLIENT_ID || clientSecret !== env.CF_ACCESS_CLIENT_SECRET) {
    return Response.json({ error: "Invalid service token" }, { status: 403 });
  }

  return null;
}

/**
 * Verify webhook secret from Kondo.
 * Returns null if valid, Response if invalid.
 */
export function checkWebhookSecret(request: Request, env: Env): Response | null {
  // Kondo sends API key via x-api-key header
  const secret = request.headers.get("x-api-key") || request.headers.get("X-Webhook-Secret");
  if (!secret || secret !== env.WEBHOOK_SECRET) {
    return Response.json({ error: "Invalid webhook secret" }, { status: 401 });
  }
  return null;
}
