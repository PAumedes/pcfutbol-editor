<script lang="ts">
  // Coach & Tactics screen. Coach is optional (null for "foreign" teams —
  // DbcHeader.isForeign — PLAN.md §4.1); tactics always exist.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import Advisor from "../lib/components/Advisor.svelte";
  import { currentDbc, setDbc, validationErrors } from "./lib/appStore";
  import { STRING_LENGTH_LIMITS } from "./lib/validation";
  import type { AttackType, Clearance, Marking, Pressing, Tackling } from "../lib/model";

  $: coachErrors = $validationErrors.filter((e) => e.field?.startsWith("coach"));

  function updateCoach<K extends string>(key: K, value: unknown) {
    if (!$currentDbc.coach) return;
    setDbc({ ...$currentDbc, coach: { ...$currentDbc.coach, [key]: value } });
  }

  function updateTactics<K extends string>(key: K, value: unknown) {
    setDbc({ ...$currentDbc, tactics: { ...$currentDbc.tactics, [key]: value } });
  }
</script>

<BeveledPanel title="Coach">
  {#if $currentDbc.header.isForeign}
    <Advisor heading="Foreign team" dismissible={false}>
      This is a foreign-league team (header.isForeign). Foreign teams have no coach or player
      data in the DBC format.
    </Advisor>
  {:else if $currentDbc.coach}
    <div class="pcf-form-grid">
      <label>
        Short name
        <input
          type="text"
          maxlength={STRING_LENGTH_LIMITS.coachShortName}
          value={$currentDbc.coach.shortName}
          on:input={(e) => updateCoach("shortName", e.currentTarget.value)}
        />
      </label>
      <label>
        Long name
        <input
          type="text"
          maxlength={STRING_LENGTH_LIMITS.coachLongName}
          value={$currentDbc.coach.longName}
          on:input={(e) => updateCoach("longName", e.currentTarget.value)}
        />
      </label>
      <label>
        Pointer
        <input
          type="number"
          value={$currentDbc.coach.pointer}
          on:input={(e) => updateCoach("pointer", Number(e.currentTarget.value))}
        />
      </label>
      <label class="pcf-checkbox">
        <input
          type="checkbox"
          checked={$currentDbc.coach.wasPlayer}
          on:change={(e) => updateCoach("wasPlayer", e.currentTarget.checked)}
        />
        Was also a player
      </label>
    </div>
    {#each coachErrors as err}
      <p class="pcf-error">{err.message}</p>
    {/each}
  {:else}
    <p>No coach record.</p>
  {/if}
</BeveledPanel>

<BeveledPanel title="Tactics">
  <div class="pcf-form-grid">
    <label>
      Touch %
      <input
        type="number"
        min="0"
        max="100"
        value={$currentDbc.tactics.touchPct}
        on:input={(e) => updateTactics("touchPct", Number(e.currentTarget.value))}
      />
    </label>
    <label>
      Counter %
      <input
        type="number"
        min="0"
        max="100"
        value={$currentDbc.tactics.counterPct}
        on:input={(e) => updateTactics("counterPct", Number(e.currentTarget.value))}
      />
    </label>
    <label>
      Attack
      <select
        value={$currentDbc.tactics.attack}
        on:change={(e) => updateTactics("attack", e.currentTarget.value as AttackType)}
      >
        <option value="offensive">Offensive</option>
        <option value="speculative">Speculative</option>
        <option value="mixed">Mixed</option>
      </select>
    </label>
    <label>
      Tackling
      <select
        value={$currentDbc.tactics.tackling}
        on:change={(e) => updateTactics("tackling", e.currentTarget.value as Tackling)}
      >
        <option value="soft">Soft</option>
        <option value="medium">Medium</option>
        <option value="aggressive">Aggressive</option>
      </select>
    </label>
    <label>
      Marking
      <select
        value={$currentDbc.tactics.marking}
        on:change={(e) => updateTactics("marking", e.currentTarget.value as Marking)}
      >
        <option value="zonal">Zonal</option>
        <option value="man">Man</option>
      </select>
    </label>
    <label>
      Clearance
      <select
        value={$currentDbc.tactics.clearance}
        on:change={(e) => updateTactics("clearance", e.currentTarget.value as Clearance)}
      >
        <option value="played">Played out</option>
        <option value="long">Long</option>
      </select>
    </label>
    <label>
      Pressing
      <select
        value={$currentDbc.tactics.pressing}
        on:change={(e) => updateTactics("pressing", e.currentTarget.value as Pressing)}
      >
        <option value="own_half">Own half</option>
        <option value="medium">Medium</option>
        <option value="rival_half">Rival half</option>
      </select>
    </label>
  </div>
</BeveledPanel>

<style>
  .pcf-form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--pcf-spacing-md);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-xs);
  }
  .pcf-checkbox {
    flex-direction: row;
    align-items: center;
  }
  .pcf-error {
    color: var(--pcf-color-danger);
    font-size: var(--pcf-font-size-sm);
  }
</style>
