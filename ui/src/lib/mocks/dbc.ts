// Fake data for offline UI development (Agent E/F work against this until
// Agent D's real IPC lands). Shapes must satisfy ui/src/lib/model.ts exactly.

import type { Dbc, TeamIndex } from "../model";

// Country code 3 = Argentina, pointer 9013 = Boca / 9001 = River — both
// confirmed against real reference data, see fixtures/pointers/*.csv.
export const mockTeamIndex: TeamIndex = [
  { pointer: 9013, shortName: "BOCA", country: 3 },
  { pointer: 9001, shortName: "RIVER", country: 3 },
];

export const mockDbc: Dbc = {
  header: { fileVersion: 0xfe01, language: 0, isForeign: false },
  team: {
    shortName: "BOCA",
    stadiumName: "LA BOMBONERA",
    longName: "CLUB ATLETICO BOCA JUNIORS",
    country: 3,
    capacity: 49000,
    standingCapacity: 0,
    founded: 1905,
    members: 70000,
    president: "JUAN ROMAN",
    budget: 4600,
    affiliate1: 0xffff,
    affiliate2: 0xffff,
    leagueHistory: Array.from({ length: 10 }, () => ({
      position: 1,
      division: "first" as const,
    })),
    stats: {
      played: 0,
      won: 0,
      drawn: 0,
      gf: 0,
      ga: 0,
      points: 0,
      champion: 0,
      runnerUp: 0,
    },
    jornada: new Array(92).fill(0),
    palmares: new Array(34).fill(0),
  },
  tactics: {
    touchPct: 70,
    counterPct: 57,
    attack: "offensive",
    tackling: "medium",
    marking: "zonal",
    clearance: "played",
    pressing: "own_half",
    formationBlob: [],
  },
  coach: {
    // Coach pointers are a separate namespace from the 1..=50 player block
    // (PLAN.md §4.2) — keep this clear of player pointers so it doesn't
    // trip the pointer-collision validator on this shared mock fixture.
    pointer: 1001,
    shortName: "BIANCHI",
    longName: "CARLOS BIANCHI",
    profile: "x",
    systems: "x",
    palmares: "x",
    anecdotes: "x",
    lastSeason: "x",
    careerCoach: "ND,ND,ND,ND,ND==",
    wasPlayer: true,
    careerPlayer: "ND,ND,ND,ND,ND==",
    declarations: "x",
  },
  players: [
    {
      pointer: 1,
      number: 9,
      shortName: "PALERMO",
      longName: "MARTIN PALERMO",
      slot: 0,
      origin: 0,
      roles: ["cf", "empty", "empty", "empty", "empty", "empty"],
      nationality: 3,
      skin: "white",
      hair: "dark",
      demarcation: "fwd",
      birth: { day: 5, month: 11, year: 1973 },
      heightCm: 178,
      weightKg: 78,
      birthCountry: 3,
      birthplace: "AVELLANEDA",
      debutClub: "x",
      international: "x",
      profile: "x",
      characteristics: "x",
      palmares: "x",
      internationality: "x",
      anecdotes: "x",
      lastSeason: "x",
      career: "ND,ND,ND,ND,ND==",
      attrs: {
        velocidad: 75,
        resistencia: 70,
        agresividad: 65,
        calidad: 80,
        remate: 90,
        regate: 70,
        pase: 65,
        tiro: 88,
        entradas: 40,
        portero: 10,
      },
    },
  ],
};
