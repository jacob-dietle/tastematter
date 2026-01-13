import { queryFlex } from '$lib/api';
import type { QueryFlexArgs, QueryResult, CommandError } from '$lib/types';

export function createQueryStore() {
  let loading = $state(false);
  let data = $state<QueryResult | null>(null);
  let error = $state<CommandError | null>(null);
  let lastQuery = $state<QueryFlexArgs | null>(null);

  // Request deduplication: ignore stale responses from superseded requests
  let currentRequestId = 0;

  async function fetch(args: QueryFlexArgs) {
    const requestId = ++currentRequestId;
    loading = true;
    error = null;
    lastQuery = args;

    try {
      const result = await queryFlex(args);
      // Only update state if this is still the current request
      if (requestId === currentRequestId) {
        data = result;
      }
    } catch (e) {
      if (requestId === currentRequestId) {
        error = e as CommandError;
        data = null;
      }
    } finally {
      if (requestId === currentRequestId) {
        loading = false;
      }
    }
  }

  function reset() {
    loading = false;
    data = null;
    error = null;
    lastQuery = null;
  }

  return {
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get lastQuery() { return lastQuery; },
    fetch,
    reset
  };
}
