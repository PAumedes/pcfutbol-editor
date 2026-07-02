// Pure logic for AttributeBar, split out from the .svelte file so it's
// trivially unit-testable with Vitest (no DOM/component harness needed).

/** Clamps a raw attribute value to the game's 0-99 scale, rounding to int. */
export function clampAttribute(value: number): number {
  if (Number.isNaN(value)) return 0;
  const rounded = Math.round(value);
  return Math.min(99, Math.max(0, rounded));
}

/** Percent width (0-100) for the filled portion of the bar. */
export function attributePercent(value: number): number {
  return (clampAttribute(value) / 99) * 100;
}
