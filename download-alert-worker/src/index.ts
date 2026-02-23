interface Env {
	CF_ACCOUNT_ID: string;
	CF_API_TOKEN: string;
	NTFY_TOPIC: string;
}

const BUCKET = "tastematter-releases";
const BINARY_PATTERN = /^(releases\/v[^/]+|staging\/latest)\/tastematter-/;

const QUERY = `
query R2Downloads($accountTag: String!, $start: Time!, $end: Time!) {
  viewer {
    accounts(filter: { accountTag: $accountTag }) {
      r2OperationsAdaptiveGroups(
        limit: 100
        filter: {
          bucketName: "${BUCKET}"
          actionType: "GetObject"
          datetime_geq: $start
          datetime_leq: $end
        }
      ) {
        sum { requests }
        dimensions { objectName datetime }
      }
    }
  }
}`;

export default {
	async scheduled(event: ScheduledEvent, env: Env, ctx: ExecutionContext) {
		const end = new Date(event.scheduledTime);
		const start = new Date(end.getTime() - 20 * 60 * 1000);

		const resp = await fetch("https://api.cloudflare.com/client/v4/graphql", {
			method: "POST",
			headers: {
				Authorization: `Bearer ${env.CF_API_TOKEN}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				query: QUERY,
				variables: {
					accountTag: env.CF_ACCOUNT_ID,
					start: start.toISOString(),
					end: end.toISOString(),
				},
			}),
		});

		if (!resp.ok) {
			console.error(`GraphQL API error: ${resp.status}`);
			return;
		}

		const json = (await resp.json()) as any;
		const groups =
			json.data?.viewer?.accounts?.[0]?.r2OperationsAdaptiveGroups ?? [];

		const downloads = groups.filter((g: any) =>
			BINARY_PATTERN.test(g.dimensions.objectName),
		);

		if (downloads.length === 0) return;

		const lines = downloads.map((d: any) => {
			const name: string = d.dimensions.objectName;
			const count: number = d.sum.requests;
			const match =
				name.match(/releases\/(v[^/]+)/) ||
				name.match(/(staging\/latest)/);
			const version = match ? match[1] : "unknown";
			const platform = name.split("/").pop() ?? "unknown";
			return `${platform} (${version}) - ${count} download${count > 1 ? "s" : ""}`;
		});

		const total = downloads.reduce(
			(s: number, d: any) => s + d.sum.requests,
			0,
		);
		const title = `${total} tastematter download${total > 1 ? "s" : ""}`;
		const body = lines.join("\n");

		await fetch(`https://ntfy.sh/${env.NTFY_TOPIC}`, {
			method: "POST",
			headers: {
				Title: title,
				Tags: "package",
			},
			body,
		});
	},

	async fetch(_request: Request, _env: Env): Promise<Response> {
		return new Response(
			JSON.stringify({
				status: "ok",
				worker: "tastematter-download-alerts",
			}),
			{ headers: { "Content-Type": "application/json" } },
		);
	},
};
