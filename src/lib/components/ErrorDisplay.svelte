<script lang="ts">
  import type { CommandError } from '$lib/types';

  interface Props {
    error: CommandError;
    onretry?: () => void;
  }

  let { error, onretry }: Props = $props();
</script>

<div class="error" data-testid="error-display" role="alert">
  <h3>Error: {error.code}</h3>
  <p>{error.message}</p>
  {#if error.details}
    <details>
      <summary>Details</summary>
      <pre>{error.details}</pre>
    </details>
  {/if}
  {#if onretry}
    <button onclick={onretry}>Retry</button>
  {/if}
</div>

<style>
  .error {
    padding: 1rem;
    background: #fff5f5;
    border: 1px solid #feb2b2;
    border-radius: 8px;
    color: #c53030;
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 1rem;
  }

  p {
    margin: 0 0 0.5rem;
  }

  details {
    margin-top: 0.5rem;
  }

  pre {
    font-size: 0.75rem;
    overflow-x: auto;
    background: #fff;
    padding: 0.5rem;
    border-radius: 4px;
  }

  button {
    margin-top: 0.5rem;
    padding: 0.5rem 1rem;
    background: #c53030;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
