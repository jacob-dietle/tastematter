import type { PageServerLoad, Actions } from './$types';

const CONTROL_PLANE = 'https://control.tastematter.dev';
const ALERT_WORKER = 'https://api.tastematter.dev';

function getAuthHeaders(env: any): Record<string, string> {
  const clientId = String(env?.CF_ACCESS_CLIENT_ID ?? '');
  const clientSecret = String(env?.CF_ACCESS_CLIENT_SECRET ?? '');
  if (!clientId || !clientSecret) return {};
  return {
    'CF-Access-Client-Id': clientId,
    'CF-Access-Client-Secret': clientSecret,
  };
}

async function fetchJson(url: string, headers: Record<string, string>): Promise<{ ok: boolean; data: any; error: string | null }> {
  try {
    const resp = await fetch(url, { headers });
    if (!resp.ok) return { ok: false, data: null, error: `HTTP ${resp.status}` };
    const body = await resp.json() as any;
    return { ok: true, data: body.data ?? body, error: null };
  } catch (e: any) {
    return { ok: false, data: null, error: e.message };
  }
}

export const load: PageServerLoad = async ({ platform }) => {
  const auth = getAuthHeaders(platform?.env);

  const [systemsRes, workersRes, alertsRes] = await Promise.all([
    fetchJson(`${CONTROL_PLANE}/systems`, auth),
    fetchJson(`${CONTROL_PLANE}/workers`, auth),
    fetchJson(`${ALERT_WORKER}/alert/history?limit=50`, auth),
  ]);

  return {
    systems: systemsRes.data ?? [],
    workers: workersRes.data ?? [],
    alerts: alertsRes.data ?? [],
    systemsError: systemsRes.error,
    workersError: workersRes.error,
    alertsError: alertsRes.error,
  };
};

export const actions: Actions = {
  forceCheck: async ({ request, platform }) => {
    const auth = getAuthHeaders(platform?.env);
    const data = await request.formData();
    const workerId = data.get('worker_id') as string;
    if (!workerId) return { success: false, error: 'Missing worker_id' };
    const result = await fetchJson(`${CONTROL_PLANE}/workers/${encodeURIComponent(workerId)}/check`, {
      ...auth,
      'Content-Type': 'application/json',
    });
    // POST needs method override since fetchJson defaults to GET
    try {
      const resp = await fetch(`${CONTROL_PLANE}/workers/${encodeURIComponent(workerId)}/check`, {
        method: 'POST',
        headers: auth,
      });
      return { success: resp.ok };
    } catch {
      return { success: false };
    }
  },

  reloadCorpus: async ({ request, platform }) => {
    const auth = getAuthHeaders(platform?.env);
    const data = await request.formData();
    const workerId = data.get('worker_id') as string;
    if (!workerId) return { success: false };
    try {
      const resp = await fetch(`${CONTROL_PLANE}/workers/${encodeURIComponent(workerId)}/reload`, {
        method: 'POST',
        headers: auth,
      });
      return { success: resp.ok };
    } catch {
      return { success: false };
    }
  },

  toggleWorker: async ({ request, platform }) => {
    const auth = getAuthHeaders(platform?.env);
    const data = await request.formData();
    const workerId = data.get('worker_id') as string;
    const currentEnabled = data.get('enabled') as string;
    const newEnabled = currentEnabled === '1' ? 0 : 1;
    try {
      const resp = await fetch(`${CONTROL_PLANE}/workers/${encodeURIComponent(workerId)}`, {
        method: 'PATCH',
        headers: { ...auth, 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: newEnabled }),
      });
      return { success: resp.ok };
    } catch {
      return { success: false };
    }
  },
};
