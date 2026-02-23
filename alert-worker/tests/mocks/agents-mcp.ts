// Mock for agents/mcp module in test environment
export class McpAgent {
  server: any;
  ctx: any;
  env: any;

  constructor(ctx: any, env: any) {
    this.ctx = ctx;
    this.env = env;
  }

  async init() {}

  static serveSSE(path: string) {
    return {
      fetch: async () => new Response("Not implemented", { status: 501 }),
    };
  }

  static serve(path: string) {
    return {
      fetch: async () => new Response("Not implemented", { status: 501 }),
    };
  }
}
