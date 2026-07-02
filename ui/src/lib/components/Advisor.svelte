<script lang="ts">
  import { fade } from "svelte/transition";
  import { createEventDispatcher } from "svelte";
  import { motionDuration } from "../design/tokens";

  /** Sentence-case heading, e.g. "Tip", "Warning" — kept short. */
  export let heading = "Tip";
  export let dismissible = true;

  let visible = true;
  const dispatch = createEventDispatcher<{ dismiss: void }>();

  function dismiss() {
    visible = false;
    dispatch("dismiss");
  }

  // Computed once per mount rather than reactively: prefers-reduced-motion
  // doesn't change mid-session in any case we need to handle here.
  const duration = motionDuration(200);
</script>

{#if visible}
  <div class="pcf-advisor" role="note" transition:fade={{ duration }}>
    <div class="pcf-advisor__icon" aria-hidden="true">i</div>
    <div class="pcf-advisor__body">
      <div class="pcf-advisor__heading">{heading}</div>
      <div class="pcf-advisor__content"><slot /></div>
    </div>
    {#if dismissible}
      <button class="pcf-advisor__dismiss" aria-label="Dismiss tip" on:click={dismiss}>×</button>
    {/if}
  </div>
{/if}

<style>
  .pcf-advisor {
    display: flex;
    align-items: flex-start;
    gap: var(--pcf-spacing-sm);
    background: var(--pcf-color-bg-alt);
    color: var(--pcf-color-text-inverse);
    border: var(--pcf-bevel-width) solid var(--pcf-color-accent-strong);
    border-radius: var(--pcf-radius);
    padding: var(--pcf-spacing-sm) var(--pcf-spacing-md);
    font-family: var(--pcf-font-body);
    font-size: var(--pcf-font-size-base);
  }

  .pcf-advisor__icon {
    flex: none;
    width: 1.4rem;
    height: 1.4rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--pcf-color-accent);
    color: var(--pcf-color-text);
    font-family: var(--pcf-font-heading);
    font-weight: var(--pcf-font-weight-bold);
  }

  .pcf-advisor__heading {
    font-family: var(--pcf-font-heading);
    font-weight: var(--pcf-font-weight-bold);
    margin-bottom: 2px;
  }

  .pcf-advisor__dismiss {
    appearance: none;
    background: transparent;
    border: none;
    color: var(--pcf-color-text-inverse);
    font-size: var(--pcf-font-size-lg);
    line-height: 1;
    cursor: pointer;
    padding: 0 var(--pcf-spacing-xs);
  }

  .pcf-advisor__dismiss:focus-visible {
    outline: 2px solid var(--pcf-color-accent);
  }
</style>
