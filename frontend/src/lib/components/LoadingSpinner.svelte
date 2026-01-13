<div class="spinner" data-testid="loading-spinner">
  <span class="sr-only">Loading...</span>
</div>

<style>
  .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid var(--border-color);
    border-top: 3px solid var(--text-primary);
    border-radius: var(--radius-full);
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }
</style>
