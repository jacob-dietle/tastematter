// src/mcp-wrapper.ts
import { McpAgent } from 'agents/mcp';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import type { Env } from './types.js';
import { executeAgenticQueryStreaming } from './query-handler.js';

export class ContextMCP extends McpAgent {
  server = new McpServer({
    name: 'tastematter-context',
    version: '1.0.0'
  });

  async init() {
    this.server.tool(
      'query',
      { question: z.string().describe('The question to answer from the knowledge base') },
      async ({ question }) => {
        try {
          console.log(`[MCP Client ${this.ctx.id}] Received query: "${question}"`);

          const { stream, finalResultPromise } = await executeAgenticQueryStreaming(
            question,
            this.env as Env,
            {
              debug: false,
              onProgress: (msg) => console.log(`[MCP Client ${this.ctx.id}] ${msg}`),
              streaming: true
            }
          );

          // Buffer the stream (streaming solved timeout at DO level)
          const reader = stream.getReader();
          while (true) {
            const { done } = await reader.read();
            if (done) break;
          }

          const result = await finalResultPromise;
          console.log(`[MCP Client ${this.ctx.id}] Query completed in ${result.duration}ms`);

          return {
            content: [{
              type: 'text' as const,
              text: result.response
            }]
          };

        } catch (error: any) {
          console.error(`[MCP Client ${this.ctx.id}] Query error:`, error);

          return {
            content: [{
              type: 'text' as const,
              text: `Error executing query: ${error.message}`
            }],
            isError: true
          };
        }
      }
    );

    console.log(`[MCP] ContextMCP initialized for client ${this.ctx.id}`);
  }
}
