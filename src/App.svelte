<script lang="ts">
  import { createQueryStore } from '$lib/stores/query.svelte';
  import TimeRangeToggle from '$lib/components/TimeRangeToggle.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import QueryResults from '$lib/components/QueryResults.svelte';
  import GitPanel from '$lib/components/GitPanel.svelte';
  import TimelineView from '$lib/components/TimelineView.svelte';
  import SessionView from '$lib/components/SessionView.svelte';
  import ChainNavigator from '$lib/components/ChainNavigator.svelte';

  const query = createQueryStore();

  let selectedTime = $state('7d');
  let activeView = $state<'files' | 'timeline' | 'sessions'>('files');
  let selectedChain = $state<string | null>(null);

  function handleChainSelect(chainId: string | null) {
    selectedChain = chainId;
  }

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
    <div class="header-controls">
      <div class="view-toggle">
        <button
          class:active={activeView === 'files'}
          onclick={() => activeView = 'files'}>Files</button>
        <button
          class:active={activeView === 'timeline'}
          onclick={() => activeView = 'timeline'}>Timeline</button>
        <button
          class:active={activeView === 'sessions'}
          onclick={() => activeView = 'sessions'}>Sessions</button>
      </div>
      {#if activeView === 'files'}
        <TimeRangeToggle selected={selectedTime} options={['7d', '30d', '90d']} onchange={handleTimeChange} />
      {/if}
    </div>
  </header>

  <div class="layout">
    <section class="content">
      {#if activeView === 'sessions'}
        <SessionView chainFilter={selectedChain} />
      {:else if activeView === 'timeline'}
        <TimelineView />
      {:else}
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
      {/if}
    </section>

    <aside class="sidebar">
      <GitPanel />
      {#if activeView === 'sessions'}
        <ChainNavigator onSelect={handleChainSelect} />
      {/if}
    </aside>
  </div>
</main>

<style>
  main {
    max-width: 1400px;
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
    border-bottom: 1px solid var(--border-color);
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    color: var(--text-heading);
  }

  .header-controls {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .view-toggle {
    display: flex;
    gap: 0;
  }

  .view-toggle button {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color);
    background: var(--bg-card);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.875rem;
  }

  .view-toggle button:first-child {
    border-radius: 4px 0 0 4px;
  }

  .view-toggle button:not(:first-child) {
    border-left: none;
  }

  .view-toggle button:last-child {
    border-radius: 0 4px 4px 0;
  }

  .view-toggle button.active {
    background: var(--text-heading);
    color: var(--text-inverse);
    border-color: var(--text-heading);
  }

  .layout {
    display: grid;
    grid-template-columns: 1fr 320px;
    gap: 2rem;
  }

  .content {
    min-height: 400px;
  }

  .sidebar {
    position: sticky;
    top: 2rem;
    height: fit-content;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 4rem 0;
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: 4rem 0;
  }

  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }

    .sidebar {
      order: -1;
    }
  }
</style>
