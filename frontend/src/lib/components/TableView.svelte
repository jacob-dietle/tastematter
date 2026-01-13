<script lang="ts">
  import type { FileResult } from '$lib/types';

  interface Props {
    results: FileResult[];
  }

  let { results }: Props = $props();
</script>

<div class="table-container">
  <table data-testid="results-table">
    <thead>
      <tr>
        <th>File</th>
        <th>Accesses</th>
        <th>Last Access</th>
      </tr>
    </thead>
    <tbody>
      {#each results as result}
        <tr>
          <td class="file-path" title={result.file_path}>{result.file_path}</td>
          <td class="access-count">{result.access_count}</td>
          <td class="last-access">{result.last_access ?? 'Never'}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .table-container {
    width: 100%;
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
    table-layout: fixed;
  }

  th, td {
    padding: 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--border-color);
  }

  th {
    font-weight: 600;
    color: var(--text-primary);
    background: var(--bg-secondary);
    position: sticky;
    top: 0;
  }

  /* Column widths */
  th:nth-child(1), td:nth-child(1) { width: 60%; }
  th:nth-child(2), td:nth-child(2) { width: 20%; }
  th:nth-child(3), td:nth-child(3) { width: 20%; }

  .file-path {
    font-family: monospace;
    font-size: 0.8125rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 0; /* Enables text-overflow with table-layout: fixed */
  }

  .access-count {
    text-align: center;
    font-weight: 500;
  }

  .last-access {
    color: var(--text-muted);
    white-space: nowrap;
  }

  tr:hover {
    background: var(--bg-hover);
  }
</style>
