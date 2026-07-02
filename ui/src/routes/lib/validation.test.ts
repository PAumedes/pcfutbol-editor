import { describe, expect, it } from "vitest";
import { mockDbc } from "../../lib/mocks/dbc";
import {
  MAX_ENCODABLE_STRING_LENGTH,
  STRING_LENGTH_LIMITS,
  validateCoachFields,
  validateDbc,
  validatePalmares,
  validatePlayerFields,
  validatePointerCollisions,
  validateStringLength,
  validateTeamFields,
} from "./validation";
import type { Coach, Player } from "../../lib/model";

const basePlayer: Player = mockDbc.players[0];
const baseCoach: Coach = mockDbc.coach as Coach;

describe("validateStringLength", () => {
  it("passes a value at exactly the limit", () => {
    expect(validateStringLength("abc", "f", 3)).toBeNull();
  });

  it("flags a value one over the limit", () => {
    const err = validateStringLength("abcd", "f", 3);
    expect(err).not.toBeNull();
    expect(err?.code).toBe("string-too-long");
    expect(err?.message).toMatch(/f/);
  });

  it("flags a value over the format ceiling even if maxLength is loosened", () => {
    const huge = "a".repeat(MAX_ENCODABLE_STRING_LENGTH + 1);
    const err = validateStringLength(huge, "f", MAX_ENCODABLE_STRING_LENGTH + 10);
    expect(err?.code).toBe("string-exceeds-format-limit");
  });
});

describe("validatePointerCollisions", () => {
  it("finds no errors when all pointers are unique", () => {
    const errors = validatePointerCollisions(
      [{ pointer: 1 }, { pointer: 2 }, { pointer: 3 }],
      { pointer: 100 },
    );
    expect(errors).toEqual([]);
  });

  it("flags two players sharing a pointer", () => {
    const errors = validatePointerCollisions([{ pointer: 5 }, { pointer: 5 }], null);
    expect(errors).toHaveLength(1);
    expect(errors[0].code).toBe("pointer-collision");
    expect(errors[0].field).toBe("players[0], players[1]");
  });

  it("flags a coach colliding with a player", () => {
    const errors = validatePointerCollisions([{ pointer: 1 }], { pointer: 1 });
    expect(errors).toHaveLength(1);
    expect(errors[0].message).toMatch(/coach/);
  });

  it("reports one error per colliding pointer value, not per pair", () => {
    const errors = validatePointerCollisions(
      [{ pointer: 1 }, { pointer: 1 }, { pointer: 2 }, { pointer: 2 }],
      null,
    );
    expect(errors).toHaveLength(2);
  });
});

describe("validatePalmares", () => {
  it("accepts a blob of the expected length", () => {
    expect(validatePalmares(new Array(34).fill(0), 34)).toEqual([]);
  });

  it("rejects a blob shorter than expected", () => {
    const errors = validatePalmares(new Array(30).fill(0), 34);
    expect(errors).toHaveLength(1);
    expect(errors[0].code).toBe("palmares-length");
  });

  it("rejects a blob longer than expected", () => {
    const errors = validatePalmares(new Array(68).fill(0), 34);
    expect(errors).toHaveLength(1);
  });
});

describe("validateTeamFields", () => {
  it("accepts the mock team as-is", () => {
    expect(validateTeamFields(mockDbc.team)).toEqual([]);
  });

  it("flags an overlong short name", () => {
    const team = { ...mockDbc.team, shortName: "X".repeat(STRING_LENGTH_LIMITS.teamShortName + 1) };
    const errors = validateTeamFields(team);
    expect(errors.some((e) => e.field === "team.shortName")).toBe(true);
  });

  it("flags a league history that isn't exactly 10 seasons", () => {
    const team = { ...mockDbc.team, leagueHistory: mockDbc.team.leagueHistory.slice(0, 3) };
    const errors = validateTeamFields(team);
    expect(errors.some((e) => e.code === "league-history-length")).toBe(true);
  });
});

describe("validatePlayerFields", () => {
  it("accepts the mock player as-is", () => {
    expect(validatePlayerFields(basePlayer, 0)).toEqual([]);
  });

  it("flags an attribute above 99", () => {
    const player = { ...basePlayer, attrs: { ...basePlayer.attrs, velocidad: 150 } };
    const errors = validatePlayerFields(player, 0);
    expect(errors.some((e) => e.code === "attribute-out-of-range")).toBe(true);
  });

  it("flags an attribute below 0", () => {
    const player = { ...basePlayer, attrs: { ...basePlayer.attrs, portero: -1 } };
    const errors = validatePlayerFields(player, 0);
    expect(errors.some((e) => e.code === "attribute-out-of-range")).toBe(true);
  });

  it("flags a roles array that isn't length 6", () => {
    const player = { ...basePlayer, roles: basePlayer.roles.slice(0, 2) };
    const errors = validatePlayerFields(player, 0);
    expect(errors.some((e) => e.code === "roles-length")).toBe(true);
  });
});

describe("validateCoachFields", () => {
  it("accepts the mock coach as-is", () => {
    expect(validateCoachFields(baseCoach)).toEqual([]);
  });

  it("flags an overlong long name", () => {
    const coach = { ...baseCoach, longName: "X".repeat(STRING_LENGTH_LIMITS.coachLongName + 1) };
    expect(validateCoachFields(coach).length).toBeGreaterThan(0);
  });
});

describe("validateDbc", () => {
  // Resolved: coach pointers are a separate namespace from the 1..=50
  // player block (PLAN.md §4.2) — the mock fixture's coach.pointer is
  // 1001, clear of any player pointer, so the otherwise-valid fixture
  // has zero collisions.
  it("has no validation errors on the otherwise-valid mock fixture", () => {
    const errors = validateDbc(mockDbc);
    expect(errors).toEqual([]);
  });

  it("aggregates pointer collisions between coach and a player", () => {
    const dbc = {
      ...mockDbc,
      coach: { ...baseCoach, pointer: basePlayer.pointer },
    };
    const errors = validateDbc(dbc);
    expect(errors.some((e) => e.code === "pointer-collision")).toBe(true);
  });
});
