<script lang="ts">
  interface Props {
    unreadCount: number;
  }

  let { unreadCount }: Props = $props();
</script>

<a href="/notifications" class="bell" aria-label="Notifications">
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
    <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
  </svg>
  {#if unreadCount > 0}
    <span class="badge" data-testid="badge">{unreadCount}</span>
  {/if}
</a>

<style>
  .bell {
    position: relative;
    display: inline-flex;
    align-items: center;
    color: var(--fg, #e0e0e0);
    text-decoration: none;
  }

  .bell:hover {
    color: var(--accent, #3b82f6);
    text-decoration: none;
  }

  .badge {
    position: absolute;
    top: -4px;
    right: -6px;
    background: var(--danger, #ef4444);
    color: white;
    font-size: 11px;
    font-weight: 600;
    min-width: 18px;
    height: 18px;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
  }
</style>
