<script lang="ts">
  import type { FileResult } from '$lib/types';

  interface Props {
    results: FileResult[];
  }

  let { results }: Props = $props();
</script>

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
        <td class="file-path">{result.file_path}</td>
        <td class="access-count">{result.access_count}</td>
        <td class="last-access">{result.last_access ?? 'Never'}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
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
