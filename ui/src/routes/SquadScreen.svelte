<script lang="ts">
  // Squad list: pick a player to edit on PlayerScreen (the hero screen).
  // Deliberately thin — all the interesting logic lives in PlayerScreen and
  // ./lib/validation.ts. Surfaces pointer collisions right in the list so
  // they're visible before the user drills into a specific player.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import Advisor from "../lib/components/Advisor.svelte";
  import { createEventDispatcher } from "svelte";
  import { currentDbc, validationErrors } from "./lib/appStore";

  const dispatch = createEventDispatcher<{ select: number }>();

  $: pointerErrors = $validationErrors.filter((e) => e.code === "pointer-collision");
</script>

<BeveledPanel title="Squad">
  {#if pointerErrors.length > 0}
    <Advisor heading="Pointer collision" dismissible={false}>
      {#each pointerErrors as err}
        <p>{err.message}</p>
      {/each}
    </Advisor>
  {/if}

  <table class="pcf-squad-table">
    <thead>
      <tr>
        <th>#</th>
        <th>Pointer</th>
        <th>Name</th>
        <th>Position</th>
      </tr>
    </thead>
    <tbody>
      {#each $currentDbc.players as player, i (player.pointer)}
        <tr on:click={() => dispatch("select", i)} class="pcf-squad-row">
          <td>{player.number}</td>
          <td>{player.pointer}</td>
          <td>{player.longName}</td>
          <td>{player.demarcation.toUpperCase()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</BeveledPanel>

<style>
  .pcf-squad-table {
    width: 100%;
    border-collapse: collapse;
  }
  .pcf-squad-row {
    cursor: pointer;
  }
  .pcf-squad-row:hover {
    background: var(--pcf-color-panel-light);
  }
  th,
  td {
    text-align: left;
    padding: var(--pcf-spacing-xs) var(--pcf-spacing-sm);
  }
</style>
