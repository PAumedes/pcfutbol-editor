<script lang="ts">
  import { attributeColor } from "../design/tokens";
  import { attributePercent, clampAttribute } from "./attributeBar";

  /** Sentence-case attribute name, e.g. "Speed", not "SPEED" or "velocidad". */
  export let label: string;
  /** Raw attribute value; clamped to 0-99 before rendering. */
  export let value: number;

  $: shown = clampAttribute(value);
  $: percent = attributePercent(value);
  $: color = attributeColor(shown);
</script>

<div class="pcf-attrbar">
  <span class="pcf-attrbar__label">{label}</span>
  <div
    class="pcf-attrbar__track"
    role="progressbar"
    aria-label={label}
    aria-valuemin={0}
    aria-valuemax={99}
    aria-valuenow={shown}
  >
    <div class="pcf-attrbar__fill" style:width="{percent}%" style:background={color}></div>
  </div>
  <span class="pcf-attrbar__value">{shown}</span>
</div>

<style>
  .pcf-attrbar {
    display: grid;
    grid-template-columns: 8rem 1fr 2ch;
    align-items: center;
    gap: var(--pcf-spacing-sm);
    font-family: var(--pcf-font-body);
    font-size: var(--pcf-font-size-base);
    color: var(--pcf-color-text);
  }

  .pcf-attrbar__label {
    font-weight: var(--pcf-font-weight-regular);
  }

  .pcf-attrbar__track {
    height: 0.9rem;
    background: var(--pcf-color-sunken);
    border: 1px solid var(--pcf-color-panel-darker);
    border-radius: var(--pcf-radius);
    overflow: hidden;
  }

  .pcf-attrbar__fill {
    height: 100%;
    transition: width var(--pcf-motion-base) ease-out;
  }

  .pcf-attrbar__value {
    font-weight: var(--pcf-font-weight-bold);
    text-align: right;
  }
</style>
