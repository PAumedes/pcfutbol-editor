<script lang="ts">
  /** Sentence-case label, e.g. "Founded", not "FOUNDED". */
  export let label: string;
  export let value: string | number;
  export let orientation: "inline" | "stacked" = "stacked";
</script>

<div class="pcf-statfield pcf-statfield--{orientation}">
  <span class="pcf-statfield__label">{label}</span>
  <span class="pcf-statfield__value">{value}</span>
</div>

<style>
  .pcf-statfield {
    font-family: var(--pcf-font-body);
    color: var(--pcf-color-text);
  }

  .pcf-statfield--stacked {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pcf-statfield--inline {
    display: flex;
    align-items: baseline;
    gap: var(--pcf-spacing-sm);
  }

  .pcf-statfield__label {
    font-size: var(--pcf-font-size-sm);
    color: var(--pcf-color-text-dim);
    font-weight: var(--pcf-font-weight-regular);
  }

  .pcf-statfield__value {
    font-family: var(--pcf-font-heading);
    font-size: var(--pcf-font-size-base);
    font-weight: var(--pcf-font-weight-bold);
  }
</style>
