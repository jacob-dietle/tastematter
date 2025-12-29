<script lang="ts">
  import { createQueryStore } from '$lib/stores/query.svelte';
  import TimeSelector from '$lib/components/TimeSelector.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import QueryResults from '$lib/components/QueryResults.svelte';

  const query = createQueryStore();

  let selectedTime = $state('7d');

  function handleTimeChange(time: string) {
    selectedTime = time;
    query.fetch({ time, agg: ['count', 'recency'], limit: 50 });
  }

  // Fetch on mount
  $effect(() => {
    query.fetch({ time: selectedTime, agg: ['count', 'recency'], limit: 50 });
  });
</script>

<main>
  <header>
    <h1>Tastematter</h1>
    <TimeSelector selected={selectedTime} onchange={handleTimeChange} />
  </header>

  <section class="content">
    {#if query.loading}
      <div class="loading-container">
        <LoadingSpinner />
      </div>
    {:else if query.error}
      <ErrorDisplay
        error={query.error}
        onretry={() => handleTimeChange(selectedTime)}
      />
    {:else if query.data}
      <QueryResults data={query.data} />
    {:else}
      <p class="empty">No data yet. Select a time range.</p>
    {/if}
  </section>
</main>

<style>
  main {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
    font-family: system-ui, -apple-system, sans-serif;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #eee;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    color: #1a1a2e;
  }

  .content {
    min-height: 400px;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 4rem 0;
  }

  .empty {
    text-align: center;
    color: #666;
    padding: 4rem 0;
  }
</style>
