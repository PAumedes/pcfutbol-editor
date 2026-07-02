//! Mock data used by commands whose real implementation lives in a
//! sibling crate (`pcf-codec`, `pcf-images`, `pcf-manager`) that hasn't
//! landed yet. Every call site that uses this module is tagged with a
//! `// TODO(D): swap for pcf_<crate>::... once Agent <letter> lands it`
//! comment in `commands.rs` — this module only holds the schema-correct
//! placeholder values, no IPC logic.
//!
//! Shapes mirror `ui/src/lib/mocks/dbc.ts` so the frontend sees the same
//! plausible data whether it's talking to the mock TS fixtures or this
//! mock backend.

use pcf_model::{
    AttackType, Attributes, Clearance, Coach, Date, Dbc, DbcHeader, Demarcation, Division, Hair,
    LeagueResult, Marking, Player, Pressing, Role, Skin, Tackling, Tactics, Team, TeamStats,
};

// `team_index()` (the old 2-entry BOCA/RIVER fixture) was removed: `load_pkf`
// now parses the real `.PKF` container (`pcf_codec::container`) instead of
// returning this mock, and nothing else in this crate referenced it.

pub fn dbc() -> Dbc {
    Dbc {
        header: DbcHeader {
            file_version: 0xfe01,
            language: 0,
            is_foreign: false,
        },
        team: Team {
            short_name: "BOCA".into(),
            stadium_name: "LA BOMBONERA".into(),
            long_name: "CLUB ATLETICO BOCA JUNIORS".into(),
            country: 3,
            capacity: 49_000,
            standing_capacity: 0,
            founded: 1905,
            members: 70_000,
            president: "JUAN ROMAN".into(),
            budget: 4_600,
            affiliate1: 0xffff,
            affiliate2: 0xffff,
            league_history: [LeagueResult {
                position: 1,
                division: Division::First,
            }; 10],
            stats: TeamStats {
                played: 0,
                won: 0,
                drawn: 0,
                gf: 0,
                ga: 0,
                points: 0,
                champion: 0,
                runner_up: 0,
            },
            jornada: vec![0; 92],
            palmares: vec![0; 34],
        },
        tactics: Tactics {
            touch_pct: 70,
            counter_pct: 57,
            attack: AttackType::Offensive,
            tackling: Tackling::Medium,
            marking: Marking::Zonal,
            clearance: Clearance::Played,
            pressing: Pressing::OwnHalf,
            formation_blob: vec![],
        },
        coach: Some(Coach {
            // Coach pointers are a separate namespace from the 1..=50
            // player block (PLAN.md §4.2) — kept clear of player pointers.
            pointer: 1001,
            short_name: "BIANCHI".into(),
            long_name: "CARLOS BIANCHI".into(),
            profile: "x".into(),
            systems: "x".into(),
            palmares: "x".into(),
            anecdotes: "x".into(),
            last_season: "x".into(),
            career_coach: "ND,ND,ND,ND,ND==".into(),
            was_player: true,
            career_player: "ND,ND,ND,ND,ND==".into(),
            declarations: "x".into(),
        }),
        players: vec![Player {
            pointer: 1,
            number: 9,
            short_name: "PALERMO".into(),
            long_name: "MARTIN PALERMO".into(),
            slot: 0,
            origin: 0,
            roles: [
                Role::Cf,
                Role::Empty,
                Role::Empty,
                Role::Empty,
                Role::Empty,
                Role::Empty,
            ],
            nationality: 3,
            skin: Skin::White,
            hair: Hair::Dark,
            demarcation: Demarcation::Fwd,
            birth: Date {
                day: 5,
                month: 11,
                year: 1973,
            },
            height_cm: 178,
            weight_kg: 78,
            birth_country: 3,
            birthplace: "AVELLANEDA".into(),
            debut_club: "x".into(),
            international: "x".into(),
            profile: "x".into(),
            characteristics: "x".into(),
            palmares: "x".into(),
            internationality: "x".into(),
            anecdotes: "x".into(),
            last_season: "x".into(),
            career: "ND,ND,ND,ND,ND==".into(),
            attrs: Attributes {
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
        }],
    }
}

/// A blank template for `new_dbc`: same shape as `dbc()`'s defaults, but
/// with empty/zeroed identity fields so the UI doesn't show a fake team.
pub fn blank_dbc() -> Dbc {
    Dbc {
        header: DbcHeader {
            file_version: 0xfe01,
            language: 0,
            is_foreign: false,
        },
        team: Team {
            short_name: String::new(),
            stadium_name: String::new(),
            long_name: String::new(),
            country: 0,
            capacity: 0,
            standing_capacity: 0,
            founded: 0,
            members: 0,
            president: String::new(),
            budget: 0,
            affiliate1: 0xffff,
            affiliate2: 0xffff,
            league_history: [LeagueResult {
                position: 0,
                division: Division::First,
            }; 10],
            stats: TeamStats {
                played: 0,
                won: 0,
                drawn: 0,
                gf: 0,
                ga: 0,
                points: 0,
                champion: 0,
                runner_up: 0,
            },
            jornada: vec![0; 92],
            palmares: vec![0; 34],
        },
        tactics: Tactics {
            touch_pct: 70,
            counter_pct: 57,
            attack: AttackType::Offensive,
            tackling: Tackling::Medium,
            marking: Marking::Zonal,
            clearance: Clearance::Played,
            pressing: Pressing::OwnHalf,
            formation_blob: vec![],
        },
        // A domestic (`is_foreign: false`) team must carry a `Coach` for
        // `pcf_codec::DbcCodec::write` to succeed (see `dbc.rs`'s
        // `dbc_missing_coach` check) — this used to be `None`, which made
        // `new_dbc(None)`'s output silently unwritable by `save_dbc` once
        // `save_dbc` started calling the real codec instead of a JSON
        // placeholder. An empty-but-present `Coach` (same "identity fields
        // empty, free-text fields \"x\"" convention this file already uses
        // for `Team`) keeps the blank template both a believable "nothing
        // entered yet" state and a real, writable `Dbc`.
        coach: Some(Coach {
            pointer: 0,
            short_name: String::new(),
            long_name: String::new(),
            profile: "x".into(),
            systems: "x".into(),
            palmares: "x".into(),
            anecdotes: "x".into(),
            last_season: "x".into(),
            career_coach: "ND,ND,ND,ND,ND==".into(),
            was_player: false,
            career_player: "x".into(),
            declarations: "x".into(),
        }),
        players: vec![],
    }
}
