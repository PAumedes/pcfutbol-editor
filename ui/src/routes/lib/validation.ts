// Pure, framework-free validation logic for the edit screens (Agent F).
//
// These functions are deliberately dumb: given plain data, return a list of
// ValidationError. No Svelte, no IPC, no DOM. Screens call these on every
// edit and render the results as friendly banners/inline messages — never a
// stack trace.
//
// Field-length bounds mirror PLAN.md Appendix A: every string on disk is a
// 2-byte length-prefixed blob (hard ceiling 65535 chars), but the game's own
// fixed-width UI never lets a value get anywhere near that. We use tighter,
// practical caps per field so the editor doesn't produce technically-legal
// but garbage-looking records. Bump these only against a golden fixture.
import type { Coach, Dbc, Player, Team } from "../../lib/model";

export interface ValidationError {
  /** Stable machine-readable id, e.g. "pointer-collision", "palmares-length". */
  code: string;
  /** Sentence-case, no jargon — shown directly in the UI. */
  message: string;
  /** Optional field path for inline display, e.g. "players[2].pointer". */
  field?: string;
}

// The on-disk length prefix is 2 bytes (0..65535), but that's a format
// ceiling, not a sane UI limit. These are the practical caps we enforce.
export const STRING_LENGTH_LIMITS = {
  teamShortName: 20,
  teamStadiumName: 40,
  teamLongName: 60,
  teamPresident: 40,
  playerShortName: 20,
  playerLongName: 40,
  playerBirthplace: 40,
  coachShortName: 20,
  coachLongName: 40,
  coachFreeText: 200, // profile / systems / palmares / anecdotes / lastSeason / declarations
  coachCareer: 200,
} as const;

/** Hard format ceiling: a 2-byte little-endian length prefix. */
export const MAX_ENCODABLE_STRING_LENGTH = 0xffff;

export function validateStringLength(
  value: string,
  field: string,
  maxLength: number,
): ValidationError | null {
  if (value.length > maxLength) {
    return {
      code: "string-too-long",
      field,
      message: `"${field}" is ${value.length} characters, but the game only allows up to ${maxLength}. Shorten it.`,
    };
  }
  if (value.length > MAX_ENCODABLE_STRING_LENGTH) {
    // Unreachable given the caps above, but keep the format-level guard in
    // case a limit is loosened later without checking the format ceiling.
    return {
      code: "string-exceeds-format-limit",
      field,
      message: `"${field}" is too long to store — the file format allows at most ${MAX_ENCODABLE_STRING_LENGTH} characters.`,
    };
  }
  return null;
}

/**
 * Pointer collisions: every player pointer within a team must be unique,
 * and a coach's pointer must not collide with a player's. This is the #1
 * cause of silent save corruption in the reference editor's community
 * reports, so it's checked on every edit, not just on save.
 */
export function validatePointerCollisions(
  players: Pick<Player, "pointer">[],
  coach?: Pick<Coach, "pointer"> | null,
): ValidationError[] {
  const errors: ValidationError[] = [];
  const seen = new Map<number, string[]>();

  players.forEach((p, i) => {
    const owners = seen.get(p.pointer) ?? [];
    owners.push(`players[${i}]`);
    seen.set(p.pointer, owners);
  });

  if (coach) {
    const owners = seen.get(coach.pointer) ?? [];
    owners.push("coach");
    seen.set(coach.pointer, owners);
  }

  for (const [pointer, owners] of seen) {
    if (owners.length > 1) {
      errors.push({
        code: "pointer-collision",
        field: owners.join(", "),
        message: `Pointer ${pointer} is used by more than one record (${owners.join(
          " and ",
        )}). Give each player and the coach a unique pointer before saving.`,
      });
    }
  }

  return errors;
}

/**
 * Palmarés is a fixed-length blob (PLAN.md risk #2: 34 vs 68 bytes depending
 * on file version — never hardcode one length across versions). Callers
 * resolve `expectedLength` from the Dbc's header/file version first.
 */
export function validatePalmares(
  bytes: ArrayLike<number>,
  expectedLength: number,
): ValidationError[] {
  const errors: ValidationError[] = [];
  if (bytes.length !== expectedLength) {
    errors.push({
      code: "palmares-length",
      field: "team.palmares",
      message: `The palmarés data is ${bytes.length} bytes, but this file version expects exactly ${expectedLength}. This usually means it was edited with a mismatched tool — re-import from a known-good file before saving.`,
    });
  }
  return errors;
}

export function validateTeamFields(team: Team): ValidationError[] {
  const errors: ValidationError[] = [];
  const checks: [string, string, number][] = [
    [team.shortName, "team.shortName", STRING_LENGTH_LIMITS.teamShortName],
    [team.stadiumName, "team.stadiumName", STRING_LENGTH_LIMITS.teamStadiumName],
    [team.longName, "team.longName", STRING_LENGTH_LIMITS.teamLongName],
    [team.president, "team.president", STRING_LENGTH_LIMITS.teamPresident],
  ];
  for (const [value, field, max] of checks) {
    const err = validateStringLength(value, field, max);
    if (err) errors.push(err);
  }
  if (team.leagueHistory.length !== 10) {
    errors.push({
      code: "league-history-length",
      field: "team.leagueHistory",
      message: `League history must cover exactly 10 seasons (found ${team.leagueHistory.length}).`,
    });
  }
  return errors;
}

export function validatePlayerFields(player: Player, index: number): ValidationError[] {
  const errors: ValidationError[] = [];
  const checks: [string, string, number][] = [
    [player.shortName, `players[${index}].shortName`, STRING_LENGTH_LIMITS.playerShortName],
    [player.longName, `players[${index}].longName`, STRING_LENGTH_LIMITS.playerLongName],
    [player.birthplace, `players[${index}].birthplace`, STRING_LENGTH_LIMITS.playerBirthplace],
  ];
  for (const [value, field, max] of checks) {
    const err = validateStringLength(value, field, max);
    if (err) errors.push(err);
  }
  if (player.roles.length !== 6) {
    errors.push({
      code: "roles-length",
      field: `players[${index}].roles`,
      message: `Each player needs exactly 6 role slots (found ${player.roles.length}).`,
    });
  }
  for (const [attr, value] of Object.entries(player.attrs)) {
    if (value < 0 || value > 99) {
      errors.push({
        code: "attribute-out-of-range",
        field: `players[${index}].attrs.${attr}`,
        message: `"${attr}" must be between 0 and 99 (got ${value}).`,
      });
    }
  }
  return errors;
}

export function validateCoachFields(coach: Coach): ValidationError[] {
  const errors: ValidationError[] = [];
  const checks: [string, string, number][] = [
    [coach.shortName, "coach.shortName", STRING_LENGTH_LIMITS.coachShortName],
    [coach.longName, "coach.longName", STRING_LENGTH_LIMITS.coachLongName],
    [coach.profile, "coach.profile", STRING_LENGTH_LIMITS.coachFreeText],
    [coach.systems, "coach.systems", STRING_LENGTH_LIMITS.coachFreeText],
    [coach.palmares, "coach.palmares", STRING_LENGTH_LIMITS.coachFreeText],
    [coach.anecdotes, "coach.anecdotes", STRING_LENGTH_LIMITS.coachFreeText],
    [coach.lastSeason, "coach.lastSeason", STRING_LENGTH_LIMITS.coachFreeText],
    [coach.careerCoach, "coach.careerCoach", STRING_LENGTH_LIMITS.coachCareer],
    [coach.careerPlayer, "coach.careerPlayer", STRING_LENGTH_LIMITS.coachCareer],
    [coach.declarations, "coach.declarations", STRING_LENGTH_LIMITS.coachFreeText],
  ];
  for (const [value, field, max] of checks) {
    const err = validateStringLength(value, field, max);
    if (err) errors.push(err);
  }
  return errors;
}

/**
 * The palmarés byte length is version-dependent (risk #2). This is the
 * editor-canonical table until Agent A confirms otherwise against a golden
 * fixture — keep it in one place so it's a one-line fix, not a grep.
 */
export function expectedPalmaresLength(fileVersion: number): number {
  // FE01 is the only marker documented so far; default to 34 (team-record
  // breakdown) rather than 68 (editor-generated) until proven otherwise.
  if (fileVersion === 0xfe01) return 34;
  return 34;
}

/** Runs every check that applies to a whole Dbc and flattens the results. */
export function validateDbc(dbc: Dbc): ValidationError[] {
  const errors: ValidationError[] = [];
  errors.push(...validateTeamFields(dbc.team));
  errors.push(
    ...validatePalmares(dbc.team.palmares, expectedPalmaresLength(dbc.header.fileVersion)),
  );
  if (dbc.coach) errors.push(...validateCoachFields(dbc.coach));
  dbc.players.forEach((p, i) => errors.push(...validatePlayerFields(p, i)));
  errors.push(...validatePointerCollisions(dbc.players, dbc.coach));
  return errors;
}
