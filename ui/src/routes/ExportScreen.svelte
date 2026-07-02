<script lang="ts">
  // Export screen: the final GUI step, wired to ipc.ts's export_dbdat
  // (falls back to a mock report until Agent D's backend lands). Export is
  // blocked while there are outstanding validation errors — better a
  // friendly refusal here than a corrupted DBDAT tree on disk.
  import BeveledPanel from "../lib/components/BeveledPanel.svelte";
  import Advisor from "../lib/components/Advisor.svelte";
  import { currentDbc, gameDir, validationErrors } from "./lib/appStore";
  import type { ExportReport } from "../lib/model";
  import * as ipc from "../lib/ipc";

  let targetDir = "";
  let report: ExportReport | null = null;
  let exporting = false;

  $: targetDir ||= $gameDir ?? "";
  $: blocked = $validationErrors.length > 0;

  async function runExport() {
    if (blocked || targetDir.trim().length === 0) return;
    exporting = true;
    try {
      report = await ipc.exportDbdat({ dbcs: [$currentDbc], gameDir: targetDir }, targetDir);
    } finally {
      exporting = false;
    }
  }
</script>

<BeveledPanel title="Export">
  {#if blocked}
    <Advisor heading="Fix these before exporting" dismissible={false}>
      <ul>
        {#each $validationErrors as err}
          <li>{err.message}</li>
        {/each}
      </ul>
    </Advisor>
  {/if}

  <label>
    Game folder
    <input type="text" bind:value={targetDir} placeholder="C:\Games\PC Apertura 98-99" />
  </label>

  <button disabled={blocked || exporting} on:click={runExport}>
    {exporting ? "Exporting…" : "Export"}
  </button>

  {#if report}
    <div class="pcf-report">
      <p>Wrote {report.writtenFiles.length} file(s):</p>
      <ul>
        {#each report.writtenFiles as file}
          <li>{file}</li>
        {/each}
      </ul>
      {#if report.warnings.length > 0}
        <Advisor heading="Warnings" dismissible={false}>
          <ul>
            {#each report.warnings as w}
              <li>{w}</li>
            {/each}
          </ul>
        </Advisor>
      {/if}
    </div>
  {/if}
</BeveledPanel>

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: var(--pcf-spacing-xs);
    margin-bottom: var(--pcf-spacing-sm);
  }
  .pcf-report {
    margin-top: var(--pcf-spacing-md);
  }
</style>
