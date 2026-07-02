//! Shared data model for the PC Apertura 98/99 editor.
//!
//! This is the single source of truth for the on-disk record shape and the
//! Tauri IPC payloads (PLAN.md §4). Every other crate and the UI mirror
//! these types; changing a field here is a contract change (bump the crate
//! version and note it in the PR title).

pub mod error;
pub mod pointers;

pub use error::PcfError;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dbc {
    pub header: DbcHeader,
    pub team: Team,
    pub tactics: Tactics,
    /// `None` when the team is "foreign" (flag = 01).
    pub coach: Option<Coach>,
    /// Empty when foreign.
    pub players: Vec<Player>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbcHeader {
    /// The FE01-style marker; governs which optional fields exist.
    pub file_version: u16,
    /// 0 = Spanish (editor default).
    pub language: u8,
    /// false = national/playable, true = foreign league.
    pub is_foreign: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub short_name: String,
    pub stadium_name: String,
    pub long_name: String,
    pub country: u8,
    /// Seated capacity.
    pub capacity: u32,
    /// May be absent in some file versions.
    pub standing_capacity: u32,
    pub founded: u16,
    pub members: u32,
    pub president: String,
    /// In pesetas.
    pub budget: u32,
    /// 0xFFFF = none.
    pub affiliate1: u16,
    /// 0xFFFF = none.
    pub affiliate2: u16,
    /// Last 10 seasons.
    pub league_history: [LeagueResult; 10],
    pub stats: TeamStats,
    /// The "jornada positions" blob (Appendix A). Fixed-length (92 bytes in
    /// the confirmed layout); the editor always writes zeros here, but the
    /// bytes are preserved on read so round-tripping an arbitrary real DBC
    /// doesn't silently drop whatever it holds.
    pub jornada: Vec<u8>,
    /// Fixed-length blob; length is version-dependent.
    pub palmares: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeagueResult {
    pub position: u8,
    pub division: Division,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Division {
    First = 0x00,
    Second = 0x01,
    SecondB = 0x02,
    Third = 0x03,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStats {
    pub played: u16,
    pub won: u16,
    pub drawn: u16,
    pub gf: u16,
    pub ga: u16,
    pub points: u16,
    pub champion: u8,
    pub runner_up: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tactics {
    pub touch_pct: u8,
    pub counter_pct: u8,
    pub attack: AttackType,
    pub tackling: Tackling,
    pub marking: Marking,
    pub clearance: Clearance,
    pub pressing: Pressing,
    /// The long positional string, treated as opaque for v1.
    pub formation_blob: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackType {
    Offensive = 0x00,
    Speculative = 0x01,
    Mixed = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tackling {
    Soft = 0x00,
    Medium = 0x01,
    Aggressive = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Marking {
    Zonal = 0x00,
    Man = 0x01,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Clearance {
    Played = 0x00,
    Long = 0x01,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pressing {
    OwnHalf = 0x00,
    Medium = 0x01,
    RivalHalf = 0x02,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coach {
    pub pointer: u16,
    pub short_name: String,
    pub long_name: String,
    pub profile: String,
    pub systems: String,
    pub palmares: String,
    pub anecdotes: String,
    pub last_season: String,
    pub career_coach: String,
    /// The 0x03 separator.
    pub was_player: bool,
    pub career_player: String,
    pub declarations: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    /// Unique within the team.
    pub pointer: u16,
    /// Dorsal.
    pub number: u8,
    pub short_name: String,
    pub long_name: String,
    pub slot: u8,
    /// 0 = continues.
    pub origin: u8,
    pub roles: [Role; 6],
    pub nationality: u8,
    pub skin: Skin,
    pub hair: Hair,
    pub demarcation: Demarcation,
    pub birth: Date,
    pub height_cm: u8,
    pub weight_kg: u8,
    pub birth_country: u8,
    pub birthplace: String,
    // The 8 free-text fields Appendix A documents between birthplace and
    // attributes, plus career — editor default is "x" for each of these
    // 8, and "ND,ND,ND,ND,ND==" for career (mirrors Coach's defaults).
    pub debut_club: String,
    pub international: String,
    pub profile: String,
    pub characteristics: String,
    pub palmares: String,
    pub internationality: String,
    pub anecdotes: String,
    pub last_season: String,
    pub career: String,
    pub attrs: Attributes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Empty = 0x00,
    Gk = 0x01,
    Rb = 0x02,
    Lb = 0x03,
    Sweeper = 0x04,
    Lcb = 0x05,
    Rcb = 0x06,
    Rm = 0x07,
    Rim = 0x08,
    Cf = 0x09,
    DeepPlaymaker = 0x0a,
    Lm = 0x0b,
    Rw = 0x0c,
    CentralAm = 0x0d,
    Lw = 0x0e,
    Dm = 0x0f,
    RightAm = 0x10,
    LeftAm = 0x11,
    Lim = 0x12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skin {
    White = 0x01,
    Black = 0x02,
    Mixed = 0x03,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hair {
    Blond = 0x01,
    Bald = 0x02,
    Dark = 0x03,
    WhiteGrey = 0x04,
    Red = 0x05,
    Brown = 0x06,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Demarcation {
    Gk = 0x00,
    Def = 0x01,
    Mid = 0x02,
    Fwd = 0x03,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Date {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

/// Order on disk is EXACTLY this. Do not reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attributes {
    pub velocidad: u8,
    pub resistencia: u8,
    pub agresividad: u8,
    pub calidad: u8,
    pub remate: u8,
    pub regate: u8,
    pub pase: u8,
    pub tiro: u8,
    pub entradas: u8,
    pub portero: u8,
}

// ---------------------------------------------------------------------
// IPC surface (PLAN.md §4.3)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamIndexEntry {
    pub pointer: u16,
    pub short_name: String,
    pub country: u8,
}

pub type TeamIndex = Vec<TeamIndexEntry>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerMode {
    Auto,
    PreserveFromFile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResult {
    pub filename: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub dbcs: Vec<Dbc>,
    pub game_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub written_files: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerPatch {
    pub y2k: bool,
    pub start_year: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchReport {
    pub already_patched: bool,
    pub backup_path: String,
    pub applied: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharmapInfo {
    pub loaded: bool,
    pub missing_glyphs: Vec<u16>,
}
