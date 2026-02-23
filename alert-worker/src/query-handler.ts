// src/query-handler.ts
import type { Env, QueryResult, StreamingQueryResult, QueryOptions } from './types.js';

export async function executeAgenticQuery(
  query: string,
  env: Env,
  options?: QueryOptions
): Promise<QueryResult> {
  const startTime = Date.now();
  options?.onProgress?.(`Query received: "${query}"`);

  // Get DO stub
  const doId = env.CONTEXT_DO.idFromName('singleton');
  const stub = env.CONTEXT_DO.get(doId);

  // Dynamic imports (REQUIRED for CF Workers)
  const Anthropic = (await import('@anthropic-ai/sdk')).default;
  const { betaTool } = await import('@anthropic-ai/sdk/helpers/beta/json-schema');

  const MAX_TOOL_RESULT_CHARS = 50000;
  function truncateToolResult(result: string): string {
    if (result.length <= MAX_TOOL_RESULT_CHARS) return result;
    const truncated = result.substring(0, MAX_TOOL_RESULT_CHARS);
    return truncated + `\n\n[TRUNCATED: Result exceeded ${MAX_TOOL_RESULT_CHARS} chars.]`;
  }

  const grepTool = betaTool({
    name: 'grep',
    description: 'Search for patterns in the knowledge base. Returns ranked list of files matching the pattern.',
    inputSchema: {
      type: 'object',
      properties: {
        pattern: { type: 'string', description: 'Regex pattern to search for' },
        caseInsensitive: { type: 'boolean', description: 'Whether to ignore case (default: false)' },
        maxResults: { type: 'number', description: 'Maximum results (default: 50)' }
      },
      required: ['pattern']
    } as const,
    run: async (input) => {
      options?.onProgress?.(`GREP: "${input.pattern}"`);
      const response = await stub.fetch(new Request('http://internal/grep', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          pattern: input.pattern,
          options: { caseInsensitive: input.caseInsensitive ?? false, maxResults: input.maxResults ?? 50 }
        })
      }));
      const results = await response.json() as any[];
      options?.onProgress?.(`GREP found ${results.length} results`);
      return truncateToolResult(JSON.stringify(results, null, 2));
    }
  });

  const readTool = betaTool({
    name: 'read',
    description: 'Read the full content of a specific file from the knowledge base.',
    inputSchema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'File path to read' }
      },
      required: ['path']
    } as const,
    run: async (input) => {
      options?.onProgress?.(`READ: "${input.path}"`);
      const response = await stub.fetch(new Request('http://internal/read', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: input.path })
      }));
      const data = await response.json<{ content: string }>();
      options?.onProgress?.(`READ returned ${data.content.length} characters`);
      return truncateToolResult(data.content);
    }
  });

  const listTool = betaTool({
    name: 'list',
    description: 'List files/directories matching a glob pattern. Use to discover filesystem structure.',
    inputSchema: {
      type: 'object',
      properties: {
        pattern: { type: 'string', description: 'Glob pattern' },
        directories: { type: 'boolean', description: 'Include directories (default: true)' },
        files: { type: 'boolean', description: 'Include files (default: true)' }
      },
      required: ['pattern']
    } as const,
    run: async (input) => {
      options?.onProgress?.(`LIST: "${input.pattern}"`);
      const response = await stub.fetch(new Request('http://internal/list', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          pattern: input.pattern,
          options: { directories: input.directories ?? true, files: input.files ?? true, maxResults: 100 }
        })
      }));
      const results = await response.json() as any[];
      options?.onProgress?.(`LIST found ${results.length} matches`);
      return truncateToolResult(JSON.stringify(results, null, 2));
    }
  });

  const anthropic = new Anthropic({ apiKey: env.ANTHROPIC_API_KEY });

  const systemPrompt = `You are a knowledge base assistant for a distributed knowledge graph. Filesystem structure = semantic information.

# Core Principles
1. **Explore, don't assume** - Use tools to discover
2. **Show your work** - State what you checked and what you found
3. **Cite sources** - Every claim needs attribution

# Evidence Levels
- **[VERIFIED: ...]** - Direct source confirms
- **[INFERRED: ...]** - Deduced from evidence
- **[UNVERIFIABLE]** - Cannot confirm

# Tools & When to Use

**list(pattern)** - Structure discovery via glob
- Use for: "Who/what exists?", "List all X"

**grep(pattern)** - Content search via regex
- Use for: "Which files mention X?", "Where is X defined?"

**read(path)** - Get full file content
- Use after list/grep identifies relevant files

# Key Insights
- Directory names = entity names
- Directory hierarchy = categorization
- Use list() for STRUCTURE, grep() for CONTENT
- Always cite: [SOURCE: path] or [VERIFIED: tool(args)]
- State limitations: [UNVERIFIABLE] when you can't confirm

**Remember:** Show your work. Cite sources. State what you checked.`;

  options?.onProgress?.('Starting toolRunner...');
  const runner = anthropic.beta.messages.toolRunner({
    model: 'claude-haiku-4-5-20251001',
    max_tokens: 4096,
    system: systemPrompt,
    messages: [{ role: 'user', content: query }],
    tools: [grepTool, readTool, listTool]
  });

  const result = await runner;

  const responseText = result.content
    .filter((block: any) => block.type === 'text')
    .map((block: any) => block.text)
    .join('\n');

  const duration = Date.now() - startTime;
  options?.onProgress?.(`Query completed in ${duration}ms`);

  return {
    response: responseText,
    conversationHistory: runner.params.messages,
    totalTurns: runner.params.messages.length,
    duration,
    model: 'claude-haiku-4-5-20251001'
  };
}

export async function executeAgenticQueryStreaming(
  query: string,
  env: Env,
  options?: QueryOptions
): Promise<StreamingQueryResult> {
  const startTime = Date.now();
  options?.onProgress?.(`Query received: "${query}"`);

  const doId = env.CONTEXT_DO.idFromName('singleton');
  const stub = env.CONTEXT_DO.get(doId);

  const Anthropic = (await import('@anthropic-ai/sdk')).default;
  const { betaTool } = await import('@anthropic-ai/sdk/helpers/beta/json-schema');

  const MAX_TOOL_RESULT_CHARS = 50000;
  function truncateToolResult(result: string): string {
    if (result.length <= MAX_TOOL_RESULT_CHARS) return result;
    return result.substring(0, MAX_TOOL_RESULT_CHARS) + `\n\n[TRUNCATED]`;
  }

  const grepTool = betaTool({
    name: 'grep',
    description: 'Search for patterns in the knowledge base.',
    inputSchema: {
      type: 'object',
      properties: {
        pattern: { type: 'string', description: 'Regex pattern' },
        caseInsensitive: { type: 'boolean', description: 'Ignore case' },
        maxResults: { type: 'number', description: 'Max results' }
      },
      required: ['pattern']
    } as const,
    run: async (input) => {
      const response = await stub.fetch(new Request('http://internal/grep', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pattern: input.pattern, options: { caseInsensitive: input.caseInsensitive ?? false, maxResults: input.maxResults ?? 50 } })
      }));
      return truncateToolResult(JSON.stringify(await response.json(), null, 2));
    }
  });

  const readTool = betaTool({
    name: 'read',
    description: 'Read a file from the knowledge base.',
    inputSchema: {
      type: 'object',
      properties: { path: { type: 'string', description: 'File path' } },
      required: ['path']
    } as const,
    run: async (input) => {
      const response = await stub.fetch(new Request('http://internal/read', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: input.path })
      }));
      const data = await response.json<{ content: string }>();
      return truncateToolResult(data.content);
    }
  });

  const listTool = betaTool({
    name: 'list',
    description: 'List files/directories matching a glob pattern.',
    inputSchema: {
      type: 'object',
      properties: {
        pattern: { type: 'string', description: 'Glob pattern' },
        directories: { type: 'boolean', description: 'Include directories' },
        files: { type: 'boolean', description: 'Include files' }
      },
      required: ['pattern']
    } as const,
    run: async (input) => {
      const response = await stub.fetch(new Request('http://internal/list', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pattern: input.pattern, options: { directories: input.directories ?? true, files: input.files ?? true, maxResults: 100 } })
      }));
      return truncateToolResult(JSON.stringify(await response.json(), null, 2));
    }
  });

  const anthropic = new Anthropic({ apiKey: env.ANTHROPIC_API_KEY });

  const systemPrompt = `You are a knowledge base assistant. Use tools to explore, cite sources, show your work.

Tools: list(pattern) for structure, grep(pattern) for content search, read(path) for file content.
Evidence: [VERIFIED: ...], [INFERRED: ...], [UNVERIFIABLE]`;

  const runner = anthropic.beta.messages.toolRunner({
    model: 'claude-haiku-4-5-20251001',
    max_tokens: 4096,
    system: systemPrompt,
    messages: [{ role: 'user', content: query }],
    tools: [grepTool, readTool, listTool],
    stream: true
  });

  let fullResponse = '';
  let eventCount = 0;

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      try {
        for await (const messageStream of runner) {
          for await (const event of messageStream) {
            eventCount++;
            const eventData = JSON.stringify(event) + '\n';
            controller.enqueue(new TextEncoder().encode(eventData));

            if (event.type === 'content_block_delta') {
              if ((event as any).delta.type === 'text_delta') {
                fullResponse += (event as any).delta.text;
              }
            }
          }
        }
        controller.close();
      } catch (error: any) {
        console.error('[STREAM ERROR]', error.message);
        controller.error(error);
        throw error;
      }
    }
  });

  const finalResultPromise = runner.done().then(result => {
    const duration = Date.now() - startTime;
    const responseText = result.content
      .filter((block: any) => block.type === 'text')
      .map((block: any) => block.text)
      .join('\n');

    return {
      response: responseText || fullResponse,
      conversationHistory: runner.params.messages,
      totalTurns: runner.params.messages.length,
      duration,
      model: 'claude-haiku-4-5-20251001'
    };
  });

  return { stream, finalResultPromise };
}
