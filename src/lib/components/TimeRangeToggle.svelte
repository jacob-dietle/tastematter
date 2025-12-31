<script lang="ts">
  interface Props {
    selected?: string;
    options?: string[];
    onchange?: (time: string) => void;
    disabled?: boolean;
  }

  let {
    selected = '7d',
    options = ['7d', '14d', '30d'],
    onchange,
    disabled = false
  }: Props = $props();

  function handleClick(time: string) {
    onchange?.(time);
  }
</script>

<div class="time-selector" role="group" aria-label="Time range">
  {#each options as time}
    <button
      class:selected={selected === time}
      onclick={() => handleClick(time)}
      {disabled}
    >
      {time}
    </button>
  {/each}
</div>

<style>
  .time-selector {
    display: flex;
    gap: 0.5rem;
  }

  button {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-card);
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.2s;
  }

  button:hover {
    background: var(--bg-hover);
  }

  button.selected {
    background: var(--text-heading);
    color: var(--text-inverse);
    border-color: var(--text-heading);
  }
</style>
