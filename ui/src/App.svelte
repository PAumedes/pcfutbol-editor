<script lang="ts">
  // Trivial hash router: "#/dev-components" -> the Storybook-style
  // component demo (Agent E's acceptance gate); everything else -> Agent
  // F's real screen router (ui/src/routes/Routes.svelte). No routing
  // library — this is intentionally minimal so it doesn't encroach on
  // ui/src/routes/** ownership; Routes.svelte does its own tab switching.
  import Routes from "./routes/Routes.svelte";
  import DemoPage from "./routes/dev-components/DemoPage.svelte";

  function currentHash(): string {
    return typeof window !== "undefined" ? window.location.hash : "";
  }

  let hash = currentHash();

  function onHashChange() {
    hash = currentHash();
  }
</script>

<svelte:window on:hashchange={onHashChange} />

{#if hash === "#/dev-components"}
  <DemoPage />
{:else}
  <Routes />
{/if}
