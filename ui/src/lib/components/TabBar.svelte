<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Tab } from "./tabBar";

  export let tabs: Tab[] = [];
  export let activeId: string;

  const dispatch = createEventDispatcher<{ change: string }>();

  function select(id: string) {
    if (id === activeId) return;
    activeId = id;
    dispatch("change", id);
  }

  function onKeydown(event: KeyboardEvent, index: number) {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    event.preventDefault();
    const delta = event.key === "ArrowRight" ? 1 : -1;
    const next = (index + delta + tabs.length) % tabs.length;
    select(tabs[next].id);
  }
</script>

<div class="pcf-tabbar" role="tablist">
  {#each tabs as tab, i (tab.id)}
    <button
      class="pcf-tabbar__tab"
      class:pcf-tabbar__tab--active={tab.id === activeId}
      role="tab"
      aria-selected={tab.id === activeId}
      tabindex={tab.id === activeId ? 0 : -1}
      on:click={() => select(tab.id)}
      on:keydown={(e) => onKeydown(e, i)}
    >
      {tab.label}
    </button>
  {/each}
</div>

<style>
  .pcf-tabbar {
    display: flex;
    gap: 2px;
    font-family: var(--pcf-font-heading);
    font-size: var(--pcf-font-size-base);
  }

  .pcf-tabbar__tab {
    appearance: none;
    cursor: pointer;
    padding: var(--pcf-spacing-xs) var(--pcf-spacing-md);
    background: var(--pcf-color-panel-dark);
    color: var(--pcf-color-text-inverse);
    border: var(--pcf-bevel-width) solid var(--pcf-color-panel-light);
    border-bottom: none;
    border-right-color: var(--pcf-color-panel-darker);
    border-radius: var(--pcf-radius) var(--pcf-radius) 0 0;
    font: inherit;
    font-weight: var(--pcf-font-weight-regular);
    transition: background var(--pcf-motion-fast) ease-out,
      color var(--pcf-motion-fast) ease-out;
  }

  .pcf-tabbar__tab:hover {
    background: var(--pcf-color-panel-darker);
  }

  .pcf-tabbar__tab--active {
    background: var(--pcf-color-panel);
    color: var(--pcf-color-text);
    font-weight: var(--pcf-font-weight-bold);
  }

  .pcf-tabbar__tab:focus-visible {
    outline: 2px solid var(--pcf-color-accent);
    outline-offset: 1px;
  }
</style>
