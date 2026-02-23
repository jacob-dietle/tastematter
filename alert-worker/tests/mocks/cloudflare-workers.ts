// Mock for cloudflare:workers module in test environment
export class DurableObject {
  ctx: any;
  env: any;

  constructor(ctx: any, env: any) {
    this.ctx = ctx;
    this.env = env;
  }

  async fetch(request: Request): Promise<Response> {
    return new Response("Not implemented", { status: 501 });
  }
}
