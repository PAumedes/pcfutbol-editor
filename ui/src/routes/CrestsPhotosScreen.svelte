<script lang="ts">
  // Crests & Photos screen.
  //
  // *** PLAN.md §9 risk #6 ***: inserting a player mid-squad renumbers the
  // team's player-pointer block under "Auto" pointer mode, which silently
  // desyncs any photo already imported under the old pointer (pointer 7
  // used to be Palermo, now it's whoever slid into slot 7). The reference
  // editor's guidance — finish the squad before importing photos — is
  // surfaced unconditionally below, and again as a specific warning if
  // ./lib/pointerReshuffle.ts detects an actual reshuffle since the squad
  // was last "settled".
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import Advisor from "../lib/components/Advisor.svelte";
  import { currentDbc } from "./lib/appStore";
  import { PHOTO_IMPORT_ORDER_WARNING } from "./lib/pointerReshuffle";
  import * as ipc from "../lib/ipc";

  let crestOutcome: string | null = null;
  let photoOutcome: string | null = null;
  let outDir = "";

  async function importCrest() {
    const result = await ipc.importCrest("chosen-crest.png", $currentDbc.team.country, outDir);
    crestOutcome = `Imported ${result.filename} (${result.width}x${result.height}).`;
  }

  async function importPhoto(playerPointer: number) {
    const result = await ipc.importPhoto("chosen-photo.png", playerPointer, outDir);
    photoOutcome = `Imported ${result.filename} (${result.width}x${result.height}) for pointer ${playerPointer}.`;
  }
</script>

<Advisor heading="Import photos last" dismissible={false}>
  {PHOTO_IMPORT_ORDER_WARNING}
</Advisor>

<BeveledPanel title="Crests">
  <label>
    Output folder
    <input type="text" bind:value={outDir} placeholder="DBDAT\MINIESC" />
  </label>
  <button on:click={importCrest}>Import crest…</button>
  {#if crestOutcome}
    <p class="pcf-outcome">{crestOutcome}</p>
  {/if}
</BeveledPanel>

<BeveledPanel title="Photos">
  <table class="pcf-photo-table">
    <thead>
      <tr>
        <th>Pointer</th>
        <th>Player</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each $currentDbc.players as player (player.pointer)}
        <tr>
          <td>{player.pointer}</td>
          <td>{player.longName}</td>
          <td><button on:click={() => importPhoto(player.pointer)}>Import photo…</button></td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if photoOutcome}
    <p class="pcf-outcome">{photoOutcome}</p>
  {/if}
</BeveledPanel>

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-xs);
    margin-bottom: var(--pcf-spacing-sm);
  }
  .pcf-outcome {
    color: var(--pcf-color-success);
    font-size: var(--pcf-font-size-sm);
  }
  .pcf-photo-table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: left;
    padding: var(--pcf-spacing-xs) var(--pcf-spacing-sm);
  }
</style>
