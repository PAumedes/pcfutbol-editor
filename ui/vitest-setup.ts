import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

// @testing-library/svelte doesn't auto-register cleanup when vitest globals
// are disabled, so each rendered component would otherwise leak into the
// next test's DOM and produce ambiguous queries.
afterEach(() => {
  cleanup();
});
