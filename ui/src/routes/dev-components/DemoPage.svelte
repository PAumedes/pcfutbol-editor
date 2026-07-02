<script lang="ts">
  // Storybook-style demo route — Agent E's acceptance gate (PLAN.md §6):
  // renders every ui/src/lib/components/** primitive against the mock
  // fixtures in ui/src/lib/mocks/dbc.ts, with zero backend involved.
  import BeveledPanel from "../../lib/components/BeveledPanel.svelte";
  import TabBar from "../../lib/components/TabBar.svelte";
  import type { Tab } from "../../lib/components/tabBar";
  import AttributeBar from "../../lib/components/AttributeBar.svelte";
  import StatField from "../../lib/components/StatField.svelte";
  import Advisor from "../../lib/components/Advisor.svelte";
  import { mockDbc, mockTeamIndex } from "../../lib/mocks/dbc";

  const player = mockDbc.players[0];
  const team = mockDbc.team;

  // Sentence-case labels mapped from the on-disk attribute order (do not
  // reorder relative to model.ts Attributes — display order here is a
  // presentation choice, the underlying data order is frozen).
  const attributeLabels: [keyof typeof player.attrs, string][] = [
    ["velocidad", "Speed"],
    ["resistencia", "Stamina"],
    ["agresividad", "Aggression"],
    ["calidad", "Quality"],
    ["remate", "Finishing"],
    ["regate", "Dribbling"],
    ["pase", "Passing"],
    ["tiro", "Shooting"],
    ["entradas", "Tackling"],
    ["portero", "Goalkeeping"],
  ];

  const tabs: Tab[] = [
    { id: "team", label: "Team" },
    { id: "player", label: "Player" },
    { id: "misc", label: "Misc" },
  ];
  let activeTab = "team";

  let attrOverride = 250; // deliberately out-of-range to demo clamping live
</script>

<div class="demo">
  <h1>Component demo (mock data only)</h1>
  <p class="demo__note">
    Every component below renders against <code>ui/src/lib/mocks/dbc.ts</code>.
    No backend, no <code>window.__TAURI__</code> required.
  </p>

  <section>
    <h2>TabBar</h2>
    <TabBar {tabs} activeId={activeTab} on:change={(e) => (activeTab = e.detail)} />
    <p>Active tab: <strong>{activeTab}</strong></p>
  </section>

  <section>
    <h2>BeveledPanel</h2>
    <div class="demo__row">
      <BeveledPanel title="Team index (from mockTeamIndex)">
        <ul>
          {#each mockTeamIndex as entry (entry.pointer)}
            <li>{entry.pointer} — {entry.shortName} (country {entry.country})</li>
          {/each}
        </ul>
      </BeveledPanel>
      <BeveledPanel variant="sunken" title="Sunken variant">
        <p>Recessed surface, e.g. for input-like regions.</p>
      </BeveledPanel>
    </div>
  </section>

  <section>
    <h2>StatField</h2>
    <div class="demo__row">
      <StatField label="Short name" value={team.shortName} />
      <StatField label="Stadium" value={team.stadiumName} />
      <StatField label="Founded" value={team.founded} />
      <StatField label="Budget" value={team.budget} orientation="inline" />
      <StatField label="Members" value={team.members} orientation="inline" />
    </div>
  </section>

  <section>
    <h2>AttributeBar</h2>
    <p>Rendering all 10 of {player.longName}'s attributes, in on-disk order:</p>
    <div class="demo__attrs">
      {#each attributeLabels as [key, label] (key)}
        <AttributeBar {label} value={player.attrs[key]} />
      {/each}
    </div>

    <p class="demo__note">Clamping demo — try an out-of-range value:</p>
    <label>
      Raw value
      <input type="number" bind:value={attrOverride} />
    </label>
    <AttributeBar label="Clamp demo" value={attrOverride} />
  </section>

  <section>
    <h2>Advisor</h2>
    <Advisor heading="Tip">
      Pointer collisions are the most common cause of save corruption — keep
      every player pointer unique within a team.
    </Advisor>
    <Advisor heading="Warning" dismissible={false}>
      This is a non-dismissible advisor, e.g. for a blocking validation error.
    </Advisor>
  </section>
</div>

<style>
  .demo {
    padding: var(--pcf-spacing-lg);
    max-width: 48rem;
    font-family: var(--pcf-font-body);
    color: var(--pcf-color-text-inverse);
  }

  h1,
  h2 {
    font-family: var(--pcf-font-heading);
  }

  .demo__note {
    color: var(--pcf-color-text-inverse);
    opacity: 0.8;
  }

  .demo__row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--pcf-spacing-md);
    align-items: flex-start;
  }

  .demo__attrs {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-xs);
    background: var(--pcf-color-panel);
    padding: var(--pcf-spacing-md);
    border-radius: var(--pcf-radius);
    max-width: 24rem;
  }

  section {
    margin-bottom: var(--pcf-spacing-lg);
  }
</style>
