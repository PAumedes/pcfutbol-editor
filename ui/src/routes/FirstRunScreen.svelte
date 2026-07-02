<script lang="ts">
  // First-run flow: "point me at your game folder." Wraps the pure state
  // machine in ./lib/firstRun.ts. Runs against ipc.ts, which itself falls
  // back to ui/src/lib/mocks until Agent D's backend lands — nothing here
  // needs to change when that swap happens.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import Advisor from "../lib/components/Advisor.svelte";
  import { detectGameFolder, firstRunState, loadGameFolder } from "./lib/appStore";
  import { createEventDispatcher, onMount } from "svelte";

  const dispatch = createEventDispatcher<{ ready: void }>();

  let manualPath = "";

  onMount(() => {
    void detectGameFolder();
  });

  function onBrowseSubmit() {
    if (manualPath.trim().length === 0) return;
    void loadGameFolder(manualPath.trim());
  }

  $: if ($firstRunState.step === "loaded") {
    dispatch("ready");
  }
</script>

<BeveledPanel title="Welcome">
  {#if $firstRunState.step === "idle" || $firstRunState.step === "detecting"}
    <p>Looking for your PC Apertura 98/99 install…</p>
  {:else if $firstRunState.step === "detected"}
    <p>Found a game folder:</p>
    <p class="pcf-mono">{$firstRunState.gameDir}</p>
    <button on:click={() => loadGameFolder($firstRunState.step === "detected" ? $firstRunState.gameDir : "")}>
      Use this folder
    </button>
    <button on:click={() => (manualPath = "")}>Pick a different folder instead</button>
  {:else if $firstRunState.step === "not-detected"}
    <Advisor heading="Couldn't auto-detect" dismissible={false}>
      We couldn't find your PC Apertura 98/99 install automatically. Point us at the folder
      that contains <code>EQ003003.PKF</code>.
    </Advisor>
    <label>
      Game folder
      <input type="text" bind:value={manualPath} placeholder="C:\Games\PC Apertura 98-99" />
    </label>
    <button on:click={onBrowseSubmit}>Continue</button>
  {:else if $firstRunState.step === "loading"}
    <p>Reading team index from {$firstRunState.gameDir}…</p>
  {:else if $firstRunState.step === "loaded"}
    <p>Loaded {$firstRunState.teamIndex.length} team(s) from {$firstRunState.gameDir}.</p>
  {:else if $firstRunState.step === "error"}
    <Advisor heading="Something went wrong" dismissible={false}>
      {$firstRunState.message}
    </Advisor>
    <label>
      Try a different folder
      <input type="text" bind:value={manualPath} placeholder="C:\Games\PC Apertura 98-99" />
    </label>
    <button on:click={onBrowseSubmit}>Retry</button>
  {/if}
</BeveledPanel>

<style>
  .pcf-mono {
    font-family: var(--pcf-font-heading);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-xs);
    margin: var(--pcf-spacing-sm) 0;
  }
</style>
