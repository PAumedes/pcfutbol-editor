// Hand-mirrored TypeScript types for the shared data model (PLAN.md §4.1).
// This is the ONLY other place these shapes may be declared — keep it in
// lockstep with crates/pcf-model/src/lib.rs. A field added there must be
// added here in the same PR.

export interface Dbc {
  header: DbcHeader;
  team: Team;
  tactics: Tactics;
  coach: Coach | null;
  players: Player[];
}

export interface DbcHeader {
  fileVersion: number;
  language: number;
  isForeign: boolean;
}

export interface Team {
  shortName: string;
  stadiumName: string;
  longName: string;
  country: number;
  capacity: number;
  standingCapacity: number;
  founded: number;
  members: number;
  president: string;
  budget: number;
  affiliate1: number;
  affiliate2: number;
  leagueHistory: LeagueResult[]; // length 10
  stats: TeamStats;
  /** The "jornada positions" blob (Appendix A), preserved byte-for-byte. */
  jornada: number[];
  palmares: number[];
}

export interface LeagueResult {
  position: number;
  division: Division;
}

export type Division = "first" | "second" | "second_b" | "third";

export interface TeamStats {
  played: number;
  won: number;
  drawn: number;
  gf: number;
  ga: number;
  points: number;
  champion: number;
  runnerUp: number;
}

export interface Tactics {
  touchPct: number;
  counterPct: number;
  attack: AttackType;
  tackling: Tackling;
  marking: Marking;
  clearance: Clearance;
  pressing: Pressing;
  formationBlob: number[];
}

export type AttackType = "offensive" | "speculative" | "mixed";
export type Tackling = "soft" | "medium" | "aggressive";
export type Marking = "zonal" | "man";
export type Clearance = "played" | "long";
export type Pressing = "own_half" | "medium" | "rival_half";

export interface Coach {
  pointer: number;
  shortName: string;
  longName: string;
  profile: string;
  systems: string;
  palmares: string;
  anecdotes: string;
  lastSeason: string;
  careerCoach: string;
  wasPlayer: boolean;
  careerPlayer: string;
  declarations: string;
}

export interface Player {
  pointer: number;
  number: number;
  shortName: string;
  longName: string;
  slot: number;
  origin: number;
  roles: Role[]; // length 6
  nationality: number;
  skin: Skin;
  hair: Hair;
  demarcation: Demarcation;
  birth:Dob;
  heightCm: number;
  weightKg: number;
  birthCountry: number;
  birthplace: string;
  debutClub: string;
  international: string;
  profile: string;
  characteristics: string;
  palmares: string;
  internationality: string;
  anecdotes: string;
  lastSeason: string;
  career: string;
  attrs: Attributes;
}

export type Role =
  | "empty" | "gk" | "rb" | "lb" | "sweeper" | "lcb" | "rcb" | "rm" | "rim"
  | "cf" | "deep_playmaker" | "lm" | "rw" | "central_am" | "lw" | "dm"
  | "right_am" | "left_am" | "lim";

export type Skin = "white" | "black" | "mixed";
export type Hair = "blond" | "bald" | "dark" | "white_grey" | "red" | "brown";
export type Demarcation = "gk" | "def" | "mid" | "fwd";

export interface Dob {
  day: number;
  month: number;
  year: number;
}

/** Order matches the on-disk layout exactly. Do not reorder. */
export interface Attributes {
  velocidad: number;
  resistencia: number;
  agresividad: number;
  calidad: number;
  remate: number;
  regate: number;
  pase: number;
  tiro: number;
  entradas: number;
  portero: number;
}

// ---------------------------------------------------------------------
// IPC surface (PLAN.md §4.3)
// ---------------------------------------------------------------------

export interface TeamIndexEntry {
  pointer: number;
  shortName: string;
  country: number;
}

export type TeamIndex = TeamIndexEntry[];

export type PointerMode = "auto" | "preserve_from_file";

export interface AssetResult {
  filename: string;
  width: number;
  height: number;
}

export interface Project {
  dbcs: Dbc[];
  gameDir: string | null;
}

export interface ExportReport {
  writtenFiles: string[];
  warnings: string[];
}

export interface ManagerPatch {
  y2k: boolean;
  startYear: number | null;
}

export interface PatchReport {
  alreadyPatched: boolean;
  backupPath: string;
  applied: string[];
}

export interface CharmapInfo {
  loaded: boolean;
  missingGlyphs: number[];
}

export interface PcfError {
  code: string;
  message: string;
  context: string | null;
}
