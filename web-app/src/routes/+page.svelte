<script lang="ts">
  // MVP: Hardcoded engagement data
  const engagements = [
    { name: 'Nickel', lastAlert: '2 hours ago' },
  ];
</script>

<h1>Dashboard</h1>

<section class="engagements">
  <h2>Active Engagements</h2>
  {#each engagements as eng}
    <div class="engagement-card">
      <span class="name">{eng.name}</span>
      <span class="last-alert">Last alert: {eng.lastAlert}</span>
    </div>
  {/each}
</section>

<style>
  h1 {
    font-size: 24px;
    font-weight: 700;
    margin-bottom: 24px;
  }

  h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--fg-muted, #888);
    margin-bottom: 12px;
  }

  .engagements {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .engagement-card {
    background: var(--card-bg, #141414);
    border: 1px solid var(--border, #222);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .name {
    font-weight: 600;
    font-size: 15px;
  }

  .last-alert {
    color: var(--fg-muted, #888);
    font-size: 13px;
  }
</style>
