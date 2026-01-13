<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { createContextStore, setAppContext } from '$lib/stores/context.svelte';
  import { createFilesStore } from '$lib/stores/files.svelte';
  import { createTimelineStore } from '$lib/stores/timeline.svelte';
  import { logService } from '$lib/logging';
  import TimeRangeToggle from '$lib/components/TimeRangeToggle.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import QueryResults from '$lib/components/QueryResults.svelte';
  import GitPanel from '$lib/components/GitPanel.svelte';
  import TimelineView from '$lib/components/TimelineView.svelte';
  import WorkstreamView from '$lib/components/WorkstreamView.svelte';
  import ChainNav from '$lib/components/ChainNav.svelte';

  // Create and set global context
  const ctx = createContextStore();
  setAppContext(ctx);

  // Create view-specific stores with context
  const filesStore = createFilesStore(ctx);
  const timelineStore = createTimelineStore(ctx);

  let activeView = $state<'files' | 'timeline' | 'sessions'>('files');

  // Handle time range change - updates context, which triggers refetch
  function handleTimeChange(time: string) {
    ctx.setTimeRange(time as '7d' | '14d' | '30d');
  }

  // Fetch data on mount and when context changes
  onMount(() => {
    // Initialize correlation ID for this session
    logService.startRequest();

    ctx.refreshChains();
    filesStore.fetch();
  });

  // Refetch files when context changes
  $effect(() => {
    // Track these as dependencies (changes trigger refetch)
    const _ = ctx.timeRange;
    const __ = ctx.selectedChain;

    // Use untrack to check data without creating dependency (prevents infinite loop)
    // Note: Only check for data, not error - checking error causes refetch loop on failures
    const hasInitialData = untrack(() => filesStore.data !== null);
    if (hasInitialData) {
      filesStore.fetch();
    }
  });

  // Refetch timeline when switching to timeline view
  $effect(() => {
    if (activeView === 'timeline') {
      timelineStore.fetch();
    }
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
      <TimeRangeToggle
        selected={ctx.timeRange}
        options={['7d', '14d', '30d']}
        onchange={handleTimeChange}
      />
    </div>
  </header>

  <div class="layout">
    <section class="content">
      {#if activeView === 'sessions'}
        <WorkstreamView />
      {:else if activeView === 'timeline'}
        {#if timelineStore.loading}
          <div class="loading-container">
            <LoadingSpinner />
          </div>
        {:else if timelineStore.error}
          <ErrorDisplay
            error={timelineStore.error}
            onretry={() => timelineStore.fetch()}
          />
        {:else if timelineStore.data}
          <TimelineView store={timelineStore} />
        {:else}
          <p class="empty">No timeline data yet.</p>
        {/if}
      {:else}
        {#if filesStore.loading}
          <div class="loading-container">
            <LoadingSpinner />
          </div>
        {:else if filesStore.error}
          <ErrorDisplay
            error={filesStore.error}
            onretry={() => filesStore.fetch()}
          />
        {:else if filesStore.data}
          <QueryResults data={filesStore.data} />
        {:else}
          <p class="empty">No data yet. Select a time range.</p>
        {/if}
      {/if}
    </section>

    <aside class="sidebar">
      <GitPanel />
      <ChainNav />
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
