<script lang="ts">
  import { getKnock } from '$lib/knock';
  import { registerServiceWorker, requestPushPermission, registerPushToken } from '$lib/push';
  import { PUBLIC_KNOCK_FCM_CHANNEL_ID } from '$env/static/public';

  let pushEnabled = $state(false);
  let pushStatus = $state<string>('');
  let loading = $state(false);

  async function togglePush() {
    if (pushEnabled) {
      pushEnabled = false;
      pushStatus = 'Push notifications disabled.';
      return;
    }

    loading = true;
    pushStatus = 'Requesting permission...';

    try {
      const granted = await requestPushPermission();
      if (!granted) {
        pushStatus = 'Permission denied. Enable notifications in browser settings.';
        loading = false;
        return;
      }

      pushStatus = 'Registering service worker...';
      const registration = await registerServiceWorker();
      if (!registration) {
        pushStatus = 'Service workers not supported in this browser.';
        loading = false;
        return;
      }

      // In production, FCM token comes from Firebase messaging.
      // For MVP, we log the subscription and register a placeholder.
      const subscription = await registration.pushManager.subscribe({
        userActivated: true,
        applicationServerKey: undefined,
      } as any);

      const knock = getKnock();
      if (knock) {
        pushStatus = 'Registering with Knock...';
        await registerPushToken(
          knock,
          'default-user',
          PUBLIC_KNOCK_FCM_CHANNEL_ID,
          JSON.stringify(subscription)
        );
      }

      pushEnabled = true;
      pushStatus = 'Push notifications enabled.';
    } catch (e) {
      pushStatus = `Error: ${e instanceof Error ? e.message : 'Unknown error'}`;
    } finally {
      loading = false;
    }
  }
</script>

<h1>Settings</h1>

<section class="setting">
  <div class="setting-row">
    <div>
      <span class="setting-label">Push Notifications</span>
      <p class="setting-desc">Receive alerts when new downloads are detected.</p>
    </div>
    <button class="toggle" class:active={pushEnabled} onclick={togglePush} disabled={loading}>
      {pushEnabled ? 'ON' : 'OFF'}
    </button>
  </div>
  {#if pushStatus}
    <p class="status">{pushStatus}</p>
  {/if}
</section>

<style>
  h1 {
    font-size: 24px;
    font-weight: 700;
    margin-bottom: 24px;
  }

  .setting {
    background: var(--card-bg, #141414);
    border: 1px solid var(--border, #222);
    border-radius: 8px;
    padding: 16px;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .setting-label {
    font-weight: 600;
    font-size: 15px;
  }

  .setting-desc {
    color: var(--fg-muted, #888);
    font-size: 13px;
    margin: 4px 0 0;
  }

  .toggle {
    background: var(--border, #222);
    color: var(--fg-muted, #888);
    border: none;
    border-radius: 6px;
    padding: 6px 16px;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
  }

  .toggle.active {
    background: var(--accent, #3b82f6);
    color: white;
  }

  .toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status {
    color: var(--fg-muted, #888);
    font-size: 12px;
    margin-top: 12px;
  }
</style>
