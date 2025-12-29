import { invoke } from '@tauri-apps/api/core';
import type { QueryFlexArgs, QueryResult, CommandError } from '$lib/types';

export async function queryFlex(args: QueryFlexArgs): Promise<QueryResult> {
  try {
    return await invoke<QueryResult>('query_flex', {
      files: args.files,
      time: args.time,
      chain: args.chain,
      session: args.session,
      agg: args.agg,
      limit: args.limit,
      sort: args.sort,
    });
  } catch (error) {
    // Tauri returns errors as strings or objects, normalize to CommandError
    if (typeof error === 'string') {
      throw { code: 'INVOKE_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}
