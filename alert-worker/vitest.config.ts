import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    server: {
      deps: {
        inline: [/cloudflare:/, /^agents/],
      },
    },
  },
  resolve: {
    alias: {
      "cloudflare:workers": path.resolve(__dirname, "tests/mocks/cloudflare-workers.ts"),
      "agents/mcp": path.resolve(__dirname, "tests/mocks/agents-mcp.ts"),
    },
  },
});
