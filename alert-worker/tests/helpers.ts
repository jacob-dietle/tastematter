/**
 * Test helpers: Mock D1Database and utilities.
 */

interface MockStatement {
  _query: string;
  _binds: unknown[];
  bind(...values: unknown[]): MockStatement;
  first(): Promise<Record<string, unknown> | null>;
  all(): Promise<{ results: Record<string, unknown>[] }>;
  run(): Promise<{ meta: { changes: number } }>;
}

interface MockD1Call {
  query: string;
  binds: unknown[];
}

export interface MockD1 {
  prepare(query: string): MockStatement;
  batch(stmts: MockStatement[]): Promise<unknown[]>;
  exec(query: string): Promise<void>;
  _calls: MockD1Call[];
  _firstResult: Record<string, unknown> | null;
  _allResults: Record<string, unknown>[];
}

export function createMockD1(options?: {
  firstResult?: Record<string, unknown> | null;
  allResults?: Record<string, unknown>[];
  shouldThrow?: string;
}): MockD1 {
  const calls: MockD1Call[] = [];
  const firstResult = options?.firstResult ?? null;
  const allResults = options?.allResults ?? [];
  const shouldThrow = options?.shouldThrow;

  function createStatement(query: string): MockStatement {
    const stmt: MockStatement = {
      _query: query,
      _binds: [],
      bind(...values: unknown[]) {
        stmt._binds = values;
        calls.push({ query, binds: values });
        return stmt;
      },
      async first() {
        if (shouldThrow) throw new Error(shouldThrow);
        if (stmt._binds.length === 0) {
          calls.push({ query, binds: [] });
        }
        return firstResult;
      },
      async all() {
        if (shouldThrow) throw new Error(shouldThrow);
        if (stmt._binds.length === 0) {
          calls.push({ query, binds: [] });
        }
        return { results: allResults };
      },
      async run() {
        if (shouldThrow) throw new Error(shouldThrow);
        if (stmt._binds.length === 0) {
          calls.push({ query, binds: [] });
        }
        return { meta: { changes: 1 } };
      },
    };
    return stmt;
  }

  return {
    prepare(query: string) {
      return createStatement(query);
    },
    async batch(stmts: MockStatement[]) {
      if (shouldThrow) throw new Error(shouldThrow);
      return stmts.map(() => ({ results: allResults }));
    },
    async exec(query: string) {
      if (shouldThrow) throw new Error(shouldThrow);
      calls.push({ query, binds: [] });
    },
    _calls: calls,
    _firstResult: firstResult,
    _allResults: allResults,
  };
}
