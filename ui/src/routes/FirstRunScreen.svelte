<script lang="ts">
  // First-run flow: "point me at your game folder." Wraps the pure state
  // machine in ./lib/firstRun.ts. Runs against ipc.ts, which itself falls
  // back to ui/src/lib/mocks until Agent D's backend lands — nothing here
  // needs to change when that swap happens.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import Advisor from "../lib/components/Advisor.svelte";
  import { detectGameFolder, firstRunState, loadGameFolder, selectTeam } from "./lib/appStore";
  import { pickFolder } from "../lib/ipc";
  import { createEventDispatcher, onMount } from "svelte";

  const dispatch = createEventDispatcher<{ ready: void }>();

  let manualPath = "";
  let selectingPointer: number | null = null;
  // Overrides the "detected" branch below: lets the user reject a
  // successfully auto-detected folder and pick a different one instead,
  // which the plain firstRunState machine has no state for on its own.
  let pickingManually = false;

  onMount(() => {
    void detectGameFolder();
  });

  function onBrowseSubmit() {
    if (manualPath.trim().length === 0) return;
    pickingManually = false;
    void loadGameFolder(manualPath.trim());
  }

  /** Native folder-choose dialog (falls back to no-op outside Tauri). */
  async function onBrowseClick() {
    const dir = await pickFolder();
    if (dir) {
      manualPath = dir;
      pickingManually = false;
      void loadGameFolder(dir);
    }
  }

  async function onPickTeam(pointer: number) {
    selectingPointer = pointer;
    await selectTeam(pointer);
    selectingPointer = null;
  }

  $: if ($firstRunState.step === "team-loaded") {
    dispatch("ready");
  }
</script>

<BeveledPanel title="Welcome">
  {#if $firstRunState.step === "idle" || $firstRunState.step === "detecting"}
    <p>Looking for your PC Apertura 98/99 install…</p>
  {:else if $firstRunState.step === "detected" && !pickingManually}
    <p>Found a game folder:</p>
    <p class="pcf-mono">{$firstRunState.gameDir}</p>
    <button on:click={() => loadGameFolder($firstRunState.step === "detected" ? $firstRunState.gameDir : "")}>
      Use this folder
    </button>
    <button on:click={() => (pickingManually = true)}>Pick a different folder instead</button>
  {:else if $firstRunState.step === "not-detected" || pickingManually}
    <Advisor heading="Couldn't auto-detect" dismissible={false}>
      We couldn't find your PC Apertura 98/99 install automatically. Point us at the folder
      that contains <code>DBDAT\EQ003003.PKF</code>.
    </Advisor>
    <button on:click={onBrowseClick}>Browse for folder…</button>
    <label>
      Or type the path
      <input type="text" bind:value={manualPath} placeholder="C:\Games\PC Apertura 98-99" />
    </label>
    <button on:click={onBrowseSubmit}>Continue</button>
  {:else if $firstRunState.step === "loading"}
    <p>Reading team index from {$firstRunState.gameDir}…</p>
  {:else if $firstRunState.step === "loaded"}
    <p>Loaded {$firstRunState.teamIndex.length} team(s) from {$firstRunState.gameDir}. Pick one to edit:</p>
    <ul class="pcf-team-list">
      {#each $firstRunState.teamIndex as entry (entry.pointer)}
        <li>
          <button disabled={selectingPointer !== null} on:click={() => onPickTeam(entry.pointer)}>
            {entry.shortName}
            {#if selectingPointer === entry.pointer}(loading…){/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else if $firstRunState.step === "error"}
    <Advisor heading="Something went wrong" dismissible={false}>
      {$firstRunState.message}
    </Advisor>
    <button on:click={onBrowseClick}>Browse for folder…</button>
    <label>
      Or type a different folder
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
  .pcf-team-list {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--pcf-spacing-sm);
    padding: 0;
  }
</style>
