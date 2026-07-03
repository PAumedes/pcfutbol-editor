<script lang="ts">
  // Team screen: edit the club record. Validation runs on every keystroke
  // via ./lib/validation.ts (through the appStore's derived store) and
  // renders as inline messages, never a stack trace.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import StatField from "../lib/components/StatField.svelte";
  import { currentDbc, setDbc, validationErrors } from "./lib/appStore";
  import { STRING_LENGTH_LIMITS } from "./lib/validation";
  import { countryName } from "../lib/countryNames";

  function errorsFor(field: string) {
    return $validationErrors.filter((e) => e.field === field);
  }

  function update<K extends keyof (typeof $currentDbc)["team"]>(key: K, value: (typeof $currentDbc)["team"][K]) {
    setDbc({ ...$currentDbc, team: { ...$currentDbc.team, [key]: value } });
  }
</script>

<BeveledPanel title="Team">
  <div class="pcf-form-grid">
    <label>
      Short name
      <input
        type="text"
        maxlength={STRING_LENGTH_LIMITS.teamShortName}
        value={$currentDbc.team.shortName}
        on:input={(e) => update("shortName", e.currentTarget.value)}
      />
      {#each errorsFor("team.shortName") as err}
        <span class="pcf-error">{err.message}</span>
      {/each}
    </label>

    <label>
      Long name
      <input
        type="text"
        maxlength={STRING_LENGTH_LIMITS.teamLongName}
        value={$currentDbc.team.longName}
        on:input={(e) => update("longName", e.currentTarget.value)}
      />
      {#each errorsFor("team.longName") as err}
        <span class="pcf-error">{err.message}</span>
      {/each}
    </label>

    <label>
      Stadium name
      <input
        type="text"
        maxlength={STRING_LENGTH_LIMITS.teamStadiumName}
        value={$currentDbc.team.stadiumName}
        on:input={(e) => update("stadiumName", e.currentTarget.value)}
      />
      {#each errorsFor("team.stadiumName") as err}
        <span class="pcf-error">{err.message}</span>
      {/each}
    </label>

    <label>
      President
      <input
        type="text"
        maxlength={STRING_LENGTH_LIMITS.teamPresident}
        value={$currentDbc.team.president}
        on:input={(e) => update("president", e.currentTarget.value)}
      />
      {#each errorsFor("team.president") as err}
        <span class="pcf-error">{err.message}</span>
      {/each}
    </label>

    <label>
      Founded
      <input
        type="number"
        min="1800"
        max="2100"
        value={$currentDbc.team.founded}
        on:input={(e) => update("founded", Number(e.currentTarget.value))}
      />
    </label>

    <label>
      Budget (Pesos Argentinos)
      <input
        type="number"
        min="0"
        value={$currentDbc.team.budget}
        on:input={(e) => update("budget", Number(e.currentTarget.value))}
      />
    </label>
  </div>

  <div class="pcf-stats-row">
    <label>
      Capacity
      <input
        type="number"
        min="0"
        value={$currentDbc.team.capacity}
        on:input={(e) => update("capacity", Number(e.currentTarget.value))}
      />
    </label>

    <label>
      Members
      <input
        type="number"
        min="0"
        value={$currentDbc.team.members}
        on:input={(e) => update("members", Number(e.currentTarget.value))}
      />
    </label>

    <StatField label="Country" value={countryName($currentDbc.team.country)} />
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
  .pcf-error {
    color: var(--pcf-color-danger);
    font-size: var(--pcf-font-size-sm);
  }
  .pcf-stats-row {
    display: flex;
    align-items: flex-end;
    gap: var(--pcf-spacing-lg);
    margin-top: var(--pcf-spacing-md);
  }
  .pcf-stats-row input {
    width: 8rem;
  }
</style>
