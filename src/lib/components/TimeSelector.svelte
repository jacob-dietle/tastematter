<script lang="ts">
  interface Props {
    selected?: string;
    onchange?: (time: string) => void;
  }

  let { selected = '7d', onchange }: Props = $props();

  const options = ['7d', '30d', '90d'];

  function handleClick(time: string) {
    onchange?.(time);
  }
</script>

<div class="time-selector" role="group" aria-label="Time range">
  {#each options as time}
    <button
      class:selected={selected === time}
      onclick={() => handleClick(time)}
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
    border: 1px solid #ccc;
    border-radius: 4px;
    background: white;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:hover {
    background: #f5f5f5;
  }

  button.selected {
    background: #1a1a2e;
    color: white;
    border-color: #1a1a2e;
  }
</style>
