<script lang="ts">
  // A chunky beveled surface, the base building block of the retro shell.
  // variant "raised" = outset (default panel look), "sunken" = recessed
  // (input-like areas).
  export let variant: "raised" | "sunken" = "raised";
  export let title: string | undefined = undefined;
  export let padding: "none" | "sm" | "md" = "md";
</script>

<div class="pcf-panel pcf-panel--{variant} pcf-panel--pad-{padding}">
  {#if title}
    <div class="pcf-panel__title">{title}</div>
  {/if}
  <div class="pcf-panel__body">
    <slot />
  </div>
</div>

<style>
  .pcf-panel {
    background: var(--pcf-color-panel);
    border-radius: var(--pcf-radius);
    color: var(--pcf-color-text);
    font-family: var(--pcf-font-body);
  }

  .pcf-panel--raised {
    border: var(--pcf-bevel-width) solid var(--pcf-color-panel-light);
    border-right-color: var(--pcf-color-panel-darker);
    border-bottom-color: var(--pcf-color-panel-darker);
    box-shadow:
      inset 1px 1px 0 var(--pcf-color-panel-light),
      2px 2px 0 var(--pcf-color-panel-dark);
  }

  .pcf-panel--sunken {
    background: var(--pcf-color-sunken);
    border: var(--pcf-bevel-width) solid var(--pcf-color-panel-darker);
    border-right-color: var(--pcf-color-panel-light);
    border-bottom-color: var(--pcf-color-panel-light);
    box-shadow: inset 1px 1px 0 var(--pcf-color-panel-darker);
  }

  .pcf-panel__title {
    font-family: var(--pcf-font-heading);
    font-weight: var(--pcf-font-weight-bold);
    letter-spacing: var(--pcf-letter-spacing-heading);
    font-size: var(--pcf-font-size-base);
    background: var(--pcf-color-panel-darker);
    color: var(--pcf-color-text-inverse);
    padding: var(--pcf-spacing-xs) var(--pcf-spacing-sm);
    /* Sentence case per design checklist; text-transform left to callers'
       content, we don't force uppercase here. */
  }

  .pcf-panel--pad-none .pcf-panel__body {
    padding: 0;
  }
  .pcf-panel--pad-sm .pcf-panel__body {
    padding: var(--pcf-spacing-sm);
  }
  .pcf-panel--pad-md .pcf-panel__body {
    padding: var(--pcf-spacing-md);
  }
</style>
