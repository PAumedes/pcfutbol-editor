// TODO(E): re-derive palette/typography from user-supplied screenshots of
// the real game. This TS mirror exists so components can read tokens
// programmatically (e.g. AttributeBar's fill-color thresholds) without
// parsing CSS. Keep every value in lockstep with ./tokens.css — if you
// change one, change the other.

export const colors = {
  bg: "#0b1f3a",
  bgAlt: "#102a4c",
  panel: "#c6cfe0",
  panelLight: "#f3f6fb",
  panelDark: "#6b7593",
  panelDarker: "#3c4560",
  sunken: "#aab4cc",
  accent: "#e0a527",
  accentStrong: "#b97e0f",
  danger: "#c23b3b",
  dangerStrong: "#8f2626",
  success: "#3f8f52",
  text: "#142033",
  textDim: "#4a5670",
  textInverse: "#f3f6fb",
  attrLow: "#c23b3b",
  attrMid: "#e0a527",
  attrHigh: "#3f8f52",
} as const;

export const fonts = {
  heading: '"Lucida Console", "Courier New", ui-monospace, monospace',
  body: '"Tahoma", "Verdana", ui-sans-serif, sans-serif',
} as const;

export const fontWeights = {
  regular: 400,
  bold: 700,
} as const;

/** Cross-agent convention (PLAN.md §8): every animation <= 250ms. */
export const motion = {
  fast: 120,
  base: 200,
  max: 250,
} as const;

/**
 * Returns 0 when the user prefers reduced motion, otherwise the given
 * duration (ms). Use this to compute Svelte transition `duration` params
 * so every animation stays skippable per the design checklist.
 */
export function motionDuration(ms: number): number {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return ms;
  }
  const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  return prefersReduced ? 0 : ms;
}

/** Attribute value (0-99) -> a token color, for AttributeBar fills. */
export function attributeColor(value: number): string {
  if (value >= 70) return colors.attrHigh;
  if (value >= 40) return colors.attrMid;
  return colors.attrLow;
}
