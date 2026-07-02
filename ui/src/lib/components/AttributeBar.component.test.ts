import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import AttributeBar from "./AttributeBar.svelte";

describe("AttributeBar (component)", () => {
  it("renders the clamped value when given an in-range attribute", () => {
    render(AttributeBar, { props: { label: "Speed", value: 75 } });
    expect(screen.getByText("75")).toBeInTheDocument();
    expect(screen.getByText("Speed")).toBeInTheDocument();
  });

  it("clamps a value above 99 to 99 in the rendered output", () => {
    render(AttributeBar, { props: { label: "Speed", value: 250 } });
    expect(screen.getByText("99")).toBeInTheDocument();
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "99");
  });

  it("clamps a negative value to 0 in the rendered output", () => {
    render(AttributeBar, { props: { label: "Speed", value: -20 } });
    expect(screen.getByText("0")).toBeInTheDocument();
  });
});
