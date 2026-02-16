<script lang="ts">
  import { onMount } from 'svelte';
  import { initKnock, getKnock } from '$lib/knock';
  import NotificationBell from '$lib/components/NotificationBell.svelte';
  import '../app.css';
  import { PUBLIC_KNOCK_PUBLIC_API_KEY } from '$env/static/public';

  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let unreadCount = $state(0);

  onMount(() => {
    const userId = 'default-user'; // MVP: hardcoded user
    const knock = initKnock(PUBLIC_KNOCK_PUBLIC_API_KEY, userId);

    try {
      const feed = knock.feeds.initialize('in-app-feed');
      feed.fetch().then(() => {
        const state = feed.getState();
        unreadCount = state.metadata?.unread_count ?? 0;
      });

      feed.on('items.received.realtime', () => {
        const state = feed.getState();
        unreadCount = state.metadata?.unread_count ?? 0;
      });
    } catch (e) {
      console.error('Knock feed init failed:', e);
    }
  });
</script>

<div class="app">
  <nav class="navbar">
    <a href="/" class="logo">Tastematter</a>
    <div class="nav-actions">
      <NotificationBell {unreadCount} />
      <a href="/settings" class="nav-link">Settings</a>
    </div>
  </nav>

  <main class="content">
    {@render children()}
  </main>
</div>

<style>
  .app {
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
  }

  .navbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border, #222);
  }

  .logo {
    font-size: 18px;
    font-weight: 700;
    color: var(--fg, #e0e0e0);
    text-decoration: none;
  }

  .nav-actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .nav-link {
    color: var(--fg-muted, #888);
    font-size: 14px;
  }

  .nav-link:hover {
    color: var(--fg, #e0e0e0);
    text-decoration: none;
  }

  .content {
    flex: 1;
    padding: 20px;
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
  }
</style>
