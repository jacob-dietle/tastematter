// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		interface Platform {
			env?: {
				CF_ACCESS_CLIENT_ID: string;
				CF_ACCESS_CLIENT_SECRET: string;
			};
		}
	}
}

export {};
