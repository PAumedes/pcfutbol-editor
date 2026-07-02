<script lang="ts">
  // Squad/Player screen — the hero screen. Edits every field on a Player,
  // with the 10 Attributes rendered in pcf-model's EXACT on-disk order
  // (VE, RE, AG, CA, RM, RG, PA, TI, EN, PO — see ./lib/attributeLabels.ts).
  // Do not reorder that list; it mirrors PLAN.md Appendix A byte-for-byte.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import AttributeBar from "../lib/components/AttributeBar.svelte";
  import { currentDbc, setDbc, validationErrors } from "./lib/appStore";
  import { ATTRIBUTE_ORDER } from "./lib/attributeLabels";
  import { STRING_LENGTH_LIMITS } from "./lib/validation";

  export let playerIndex: number;

  $: player = $currentDbc.players[playerIndex];
  $: playerErrors = $validationErrors.filter((e) => e.field?.startsWith(`players[${playerIndex}]`));

  function updateAttr(key: (typeof ATTRIBUTE_ORDER)[number]["key"], value: number) {
    const next = { ...$currentDbc };
    const players = [...next.players];
    players[playerIndex] = {
      ...player,
      attrs: { ...player.attrs, [key]: value },
    };
    setDbc({ ...next, players });
  }

  function updateField<K extends keyof typeof player>(key: K, value: (typeof player)[K]) {
    const next = { ...$currentDbc };
    const players = [...next.players];
    players[playerIndex] = { ...player, [key]: value };
    setDbc({ ...next, players });
  }
</script>

{#if player}
  <BeveledPanel title={`Player — ${player.longName}`}>
    <div class="pcf-player-grid">
      <div class="pcf-player-identity">
        <label>
          Pointer
          <input
            type="number"
            value={player.pointer}
            on:input={(e) => updateField("pointer", Number(e.currentTarget.value))}
          />
        </label>
        <label>
          Number
          <input
            type="number"
            min="0"
            max="99"
            value={player.number}
            on:input={(e) => updateField("number", Number(e.currentTarget.value))}
          />
        </label>
        <label>
          Short name
          <input
            type="text"
            maxlength={STRING_LENGTH_LIMITS.playerShortName}
            value={player.shortName}
            on:input={(e) => updateField("shortName", e.currentTarget.value)}
          />
        </label>
        <label>
          Long name
          <input
            type="text"
            maxlength={STRING_LENGTH_LIMITS.playerLongName}
            value={player.longName}
            on:input={(e) => updateField("longName", e.currentTarget.value)}
          />
        </label>

        {#each playerErrors as err}
          <p class="pcf-error">{err.message}</p>
        {/each}
      </div>

      <div class="pcf-player-attrs">
        {#each ATTRIBUTE_ORDER as attr (attr.code)}
          <div class="pcf-attr-row">
            <AttributeBar label={`${attr.label} (${attr.code})`} value={player.attrs[attr.key]} />
            <input
              type="number"
              min="0"
              max="99"
              value={player.attrs[attr.key]}
              on:input={(e) => updateAttr(attr.key, Number(e.currentTarget.value))}
            />
          </div>
        {/each}
      </div>
    </div>
  </BeveledPanel>
{:else}
  <p>No player selected.</p>
{/if}

<style>
  .pcf-player-grid {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: var(--pcf-spacing-lg);
  }
  .pcf-player-identity label {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-xs);
    margin-bottom: var(--pcf-spacing-sm);
  }
  .pcf-attr-row {
    display: flex;
    align-items: center;
    gap: var(--pcf-spacing-sm);
    margin-bottom: var(--pcf-spacing-xs);
  }
  .pcf-attr-row input {
    width: 4ch;
  }
  .pcf-error {
    color: var(--pcf-color-danger);
    font-size: var(--pcf-font-size-sm);
  }
</style>
