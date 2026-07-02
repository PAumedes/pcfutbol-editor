<script lang="ts">
  // Top-level screen router for the editor. This is the one file in
  // ui/src/routes that composes the individual screens with Agent E's
  // TabBar — everything else (App.svelte, main.ts) is E's app-entry/router
  // shell. To wire this in, E's App.svelte just needs:
  //
  //   import Routes from "./routes/Routes.svelte";
  //   ... <Routes />
  //
  // No SvelteKit/file-based routing is set up (none of Agent E's scaffold
  // pulled it in), so this is a plain tab switcher over local state rather
  // than URL-driven routes. Swap it for real routing later without
  // touching the individual screens below.
  import TabBar from "../lib/components/TabBar.svelte";
  import type { Tab } from "../lib/components/tabBar";
  import FirstRunScreen from "./FirstRunScreen.svelte";
  import TeamScreen from "./TeamScreen.svelte";
  import SquadScreen from "./SquadScreen.svelte";
  import PlayerScreen from "./PlayerScreen.svelte";
  import CoachTacticsScreen from "./CoachTacticsScreen.svelte";
  import CrestsPhotosScreen from "./CrestsPhotosScreen.svelte";
  import ExportScreen from "./ExportScreen.svelte";
  import { onMount } from "svelte";
  import { redo, redoAvailable, restoreAutosave, undo, undoAvailable } from "./lib/appStore";

  const tabs: Tab[] = [
    { id: "team", label: "Team" },
    { id: "squad", label: "Squad" },
    { id: "coach", label: "Coach & tactics" },
    { id: "crests-photos", label: "Crests & photos" },
    { id: "export", label: "Export" },
  ];

  let activeId = "team";
  let ready = false;
  let selectedPlayerIndex: number | null = null;

  onMount(() => {
    // Resume mid-edit session if the user closed/reloaded without exporting.
    // There is no backend persistence yet (PLAN.md M1), so this is the only
    // safety net against losing work.
    restoreAutosave();
  });

  function onTabChange(event: CustomEvent<string>) {
    activeId = event.detail;
    if (activeId !== "squad") selectedPlayerIndex = null;
  }

  function onSelectPlayer(event: CustomEvent<number>) {
    selectedPlayerIndex = event.detail;
  }

  function onKeydown(e: KeyboardEvent) {
    const meta = e.ctrlKey || e.metaKey;
    if (!meta) return;
    if (e.key === "z" && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (e.key === "y" || (e.key === "z" && e.shiftKey)) {
      e.preventDefault();
      redo();
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

{#if !ready}
  <FirstRunScreen on:ready={() => (ready = true)} />
{:else}
  <div class="pcf-toolbar">
    <button disabled={!$undoAvailable} on:click={undo}>Undo</button>
    <button disabled={!$redoAvailable} on:click={redo}>Redo</button>
  </div>

  <TabBar {tabs} {activeId} on:change={onTabChange} />

  <div class="pcf-screen">
    {#if activeId === "team"}
      <TeamScreen />
    {:else if activeId === "squad"}
      {#if selectedPlayerIndex !== null}
        <button on:click={() => (selectedPlayerIndex = null)}>&larr; Back to squad</button>
        <PlayerScreen playerIndex={selectedPlayerIndex} />
      {:else}
        <SquadScreen on:select={onSelectPlayer} />
      {/if}
    {:else if activeId === "coach"}
      <CoachTacticsScreen />
    {:else if activeId === "crests-photos"}
      <CrestsPhotosScreen />
    {:else if activeId === "export"}
      <ExportScreen />
    {/if}
  </div>
{/if}

<style>
  .pcf-toolbar {
    display: flex;
    gap: var(--pcf-spacing-sm);
    margin-bottom: var(--pcf-spacing-sm);
  }
  .pcf-screen {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-md);
  }
</style>
