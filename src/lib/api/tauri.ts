import { invoke } from '@tauri-apps/api/core';
import type { QueryFlexArgs, QueryResult, CommandError, GitStatus, GitOpResult } from '$lib/types';

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

export async function gitStatus(): Promise<GitStatus> {
  try {
    return await invoke<GitStatus>('git_status');
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'GIT_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function gitPull(): Promise<GitOpResult> {
  try {
    return await invoke<GitOpResult>('git_pull');
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'GIT_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function gitPush(): Promise<GitOpResult> {
  try {
    return await invoke<GitOpResult>('git_push');
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'GIT_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}
