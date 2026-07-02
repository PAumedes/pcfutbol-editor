//! A hand-built, entirely SYNTHETIC `Dbc` value used to generate
//! `fixtures/golden/synthetic_minimal.dbc` and to drive round-trip tests.
//!
//! This is **not** real game data — see `fixtures/golden/README.md`. Every
//! string here is chosen to be encodable by the placeholder charmap in
//! `fixtures/charmap/synthetic_map.txt` (real teams/players are used as
//! flavor text only, not sourced from a real save).

use pcf_model::{
    AttackType, Attributes, Clearance, Coach, Date, Dbc, DbcHeader, Demarcation, Division, Hair,
    LeagueResult, Marking, PcfError, Player, Pressing, Role, Skin, Tackling, Tactics, Team,
    TeamStats,
};

use crate::charmap::CharMap;

/// Path (relative to this crate's manifest dir) to the synthetic charmap.
pub fn synthetic_charmap_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("charmap")
        .join("synthetic_map.txt")
}

pub fn load_synthetic_charmap() -> Result<CharMap, PcfError> {
    CharMap::load(synthetic_charmap_path())
}

/// Builds a minimal-but-complete synthetic `Dbc`: one national/playable
/// team, tactics, a coach who was also a player, and two outfield players.
/// Every string uses only glyphs present in the synthetic charmap.
pub fn synthetic_minimal_dbc() -> Dbc {
    Dbc {
        header: DbcHeader {
            file_version: 0x01FE,
            language: 0,
            is_foreign: false,
        },
        team: Team {
            short_name: "BOCA".to_string(),
            stadium_name: "La Bombonera".to_string(),
            long_name: "Boca Juniors".to_string(),
            country: 3,
            capacity: 87_000,
            standing_capacity: 12_000,
            founded: 1902,
            members: 70_000,
            president: "Mauricio Macri".to_string(),
            budget: 18_000,
            affiliate1: 0xFFFF,
            affiliate2: 0xFFFF,
            league_history: [LeagueResult {
                position: 0,
                division: Division::First,
            }; 10],
            stats: TeamStats {
                played: 38,
                won: 22,
                drawn: 10,
                gf: 60,
                ga: 25,
                points: 76,
                champion: 1,
                runner_up: 0,
            },
            jornada: vec![0u8; 92],
            palmares: vec![0u8; 34],
        },
        tactics: Tactics {
            touch_pct: 0x46,
            counter_pct: 0x39,
            attack: AttackType::Offensive,
            tackling: Tackling::Medium,
            marking: Marking::Zonal,
            clearance: Clearance::Played,
            pressing: Pressing::Medium,
            formation_blob: b"4-4-2 diamond".to_vec(),
        },
        coach: Some(Coach {
            pointer: 1,
            short_name: "Bianchi".to_string(),
            long_name: "Carlos Bianchi".to_string(),
            profile: "x".to_string(),
            systems: "x".to_string(),
            palmares: "x".to_string(),
            anecdotes: "x".to_string(),
            last_season: "x".to_string(),
            career_coach: "ND,ND,ND,ND,ND==".to_string(),
            was_player: true,
            career_player: "x".to_string(),
            declarations: "x".to_string(),
        }),
        players: vec![
            Player {
                pointer: 1,
                number: 10,
                short_name: "Riquelme".to_string(),
                long_name: "Juan Roman Riquelme".to_string(),
                slot: 1,
                origin: 0,
                roles: [
                    Role::CentralAm,
                    Role::Empty,
                    Role::Empty,
                    Role::Empty,
                    Role::Empty,
                    Role::Empty,
                ],
                nationality: 3,
                skin: Skin::Mixed,
                hair: Hair::Dark,
                demarcation: Demarcation::Mid,
                birth: Date {
                    day: 20,
                    month: 6,
                    year: 1978,
                },
                height_cm: 180,
                weight_kg: 74,
                birth_country: 3,
                birthplace: "Buenos Aires".to_string(),
                debut_club: "x".to_string(),
                international: "x".to_string(),
                profile: "x".to_string(),
                characteristics: "x".to_string(),
                palmares: "x".to_string(),
                internationality: "x".to_string(),
                anecdotes: "x".to_string(),
                last_season: "x".to_string(),
                career: "ND,ND,ND,ND,ND==".to_string(),
                attrs: Attributes {
                    velocidad: 65,
                    resistencia: 70,
                    agresividad: 40,
                    calidad: 95,
                    remate: 80,
                    regate: 90,
                    pase: 92,
                    tiro: 85,
                    entradas: 50,
                    portero: 10,
                },
            },
            Player {
                pointer: 2,
                number: 9,
                short_name: "Palermo".to_string(),
                long_name: "Martin Palermo".to_string(),
                slot: 2,
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
                hair: Hair::Brown,
                demarcation: Demarcation::Fwd,
                birth: Date {
                    day: 11,
                    month: 11,
                    year: 1973,
                },
                height_cm: 176,
                weight_kg: 78,
                birth_country: 3,
                birthplace: "Ingeniero Maschwitz".to_string(),
                debut_club: "x".to_string(),
                international: "x".to_string(),
                profile: "x".to_string(),
                characteristics: "x".to_string(),
                palmares: "x".to_string(),
                internationality: "x".to_string(),
                anecdotes: "x".to_string(),
                last_season: "x".to_string(),
                career: "ND,ND,ND,ND,ND==".to_string(),
                attrs: Attributes {
                    velocidad: 75,
                    resistencia: 80,
                    agresividad: 60,
                    calidad: 85,
                    remate: 96,
                    regate: 70,
                    pase: 60,
                    tiro: 94,
                    entradas: 30,
                    portero: 8,
                },
            },
        ],
    }
}
