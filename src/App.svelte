<script lang="ts">
  let greeting = $state('Hello Tastematter');
</script>

<main>
  <h1>{greeting}</h1>
  <p>Context OS Visibility Layer</p>
</main>

<style>
  main {
    font-family: system-ui, -apple-system, sans-serif;
    text-align: center;
    padding: 2rem;
  }

  h1 {
    color: #1a1a2e;
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  p {
    color: #666;
    font-size: 1.1rem;
  }
</style>
