<script lang="ts">
  import type { QueryResult } from '$lib/types';

  interface Props {
    data: QueryResult;
  }

  let { data }: Props = $props();
</script>

<div class="results">
  <p class="count" data-testid="result-count">
    {data.result_count} files
  </p>

  <table data-testid="results-table">
    <thead>
      <tr>
        <th>File</th>
        <th>Accesses</th>
        <th>Last Access</th>
      </tr>
    </thead>
    <tbody>
      {#each data.results as result}
        <tr>
          <td class="file-path">{result.file_path}</td>
          <td class="access-count">{result.access_count}</td>
          <td class="last-access">{result.last_access ?? 'Never'}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .results {
    width: 100%;
  }

  .count {
    font-size: 0.875rem;
    color: #666;
    margin-bottom: 1rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  th, td {
    padding: 0.75rem;
    text-align: left;
    border-bottom: 1px solid #eee;
  }

  th {
    font-weight: 600;
    color: #333;
    background: #f9f9f9;
  }

  .file-path {
    font-family: monospace;
    font-size: 0.8125rem;
  }

  .access-count {
    text-align: center;
    font-weight: 500;
  }

  .last-access {
    color: #666;
  }

  tr:hover {
    background: #f5f5f5;
  }
</style>
