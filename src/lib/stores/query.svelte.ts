import { queryFlex } from '$lib/api/tauri';
import type { QueryFlexArgs, QueryResult, CommandError } from '$lib/types';

export function createQueryStore() {
  let loading = $state(false);
  let data = $state<QueryResult | null>(null);
  let error = $state<CommandError | null>(null);
  let lastQuery = $state<QueryFlexArgs | null>(null);

  async function fetch(args: QueryFlexArgs) {
    loading = true;
    error = null;
    lastQuery = args;

    try {
      data = await queryFlex(args);
    } catch (e) {
      error = e as CommandError;
      data = null;
    } finally {
      loading = false;
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
