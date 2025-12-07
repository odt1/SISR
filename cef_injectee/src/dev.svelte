<svelte:options css="injected" customElement="dev-element" />

<script module lang="ts">
  import { setupHttpInterceptor } from './dev/http-interceptor';

  document.body.replaceChildren(document.createElement('dev-element'));
  window.SISR_HOST = 'localhost:1337';
  setupHttpInterceptor();
</script>

<script lang="ts">
  const modules = import.meta.glob('./entrypoints/*.ts');
  const entrypoints = Object.keys(modules).map((path) =>
    path.replace('./entrypoints/', '').replace('.ts', ''),
  );

  async function loadEntry(name: string) {
    const modulePath = `./entrypoints/${name}.ts?t=${Date.now()}`;
    await import(modulePath);
    console.log(`Loaded ${name}.ts`);
  }
</script>

<main>
  <h1>THIS PAGE IS ONLY USED FOR DEV OUTSIDE OF STEAM!</h1>
  <div>
    <h2>Entrypoints:</h2>
    <div class="entrypoints">
      {#each entrypoints as entry}
        <span>{entry}</span>
        <button onclick={() => loadEntry(entry)}>Load</button>
      {/each}
    </div>
  </div>
</main>

<style lang="postcss">
  main {
    padding: 1rem;
    font-family: system-ui, sans-serif;
  }

  h1 {
    color: red;
  }

  button {
    padding: 0.5rem 1rem;
    font-size: 1rem;
    cursor: pointer;
  }

  .entrypoints {
    display: grid;
    grid-template-columns: min-content min-content;
    place-items: center;
    gap: 0.5rem 1rem;
    & > span {
      width: 100%;
    }
  }
</style>
