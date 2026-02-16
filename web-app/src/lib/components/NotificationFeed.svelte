<script lang="ts">
  import { onMount } from 'svelte';
  import { getKnock } from '$lib/knock';

  interface FeedItem {
    id: string;
    title?: string;
    body?: string;
    inserted_at: string;
    read_at?: string | null;
  }

  let items: FeedItem[] = $state([]);
  let loading = $state(true);
  let feedInstance: any = null;

  onMount(async () => {
    const knock = getKnock();
    if (!knock) {
      loading = false;
      return;
    }

    try {
      feedInstance = knock.feeds.initialize('in-app-feed');
      await feedInstance.fetch();

      const store = feedInstance.getState();
      items = (store.items ?? []).map((item: any) => ({
        id: item.id,
        title: item.blocks?.[0]?.rendered ?? item.data?.title ?? 'Notification',
        body: item.blocks?.[1]?.rendered ?? item.data?.body ?? '',
        inserted_at: item.inserted_at,
        read_at: item.read_at,
      }));

      // Mark all as read on view
      const unread = store.items?.filter((i: any) => !i.read_at) ?? [];
      if (unread.length > 0) {
        await feedInstance.markAsRead(unread);
      }
    } catch (e) {
      console.error('Failed to fetch notifications:', e);
    } finally {
      loading = false;
    }
  });

  function formatTime(iso: string): string {
    const date = new Date(iso);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }
</script>

<div class="feed">
  {#if loading}
    <p class="muted">Loading notifications...</p>
  {:else if items.length === 0}
    <p class="muted">No notifications yet.</p>
  {:else}
    {#each items as item (item.id)}
      <div class="card" class:unread={!item.read_at}>
        <div class="card-header">
          <span class="title">{item.title}</span>
          <span class="time">{formatTime(item.inserted_at)}</span>
        </div>
        {#if item.body}
          <p class="body">{item.body}</p>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .feed {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .muted {
    color: var(--fg-muted, #888);
  }

  .card {
    background: var(--card-bg, #141414);
    border: 1px solid var(--border, #222);
    border-radius: 8px;
    padding: 12px 16px;
  }

  .card.unread {
    border-left: 3px solid var(--accent, #3b82f6);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .title {
    font-weight: 600;
    font-size: 14px;
  }

  .time {
    color: var(--fg-muted, #888);
    font-size: 12px;
  }

  .body {
    color: var(--fg-muted, #888);
    font-size: 13px;
    margin: 0;
  }
</style>
