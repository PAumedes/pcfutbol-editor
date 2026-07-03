//! Bridges a container-format [`ContainerTeamRecord`] (`crate::container`,
//! the read-only `EQ003003.PKF` "teams container" parser) onto the frozen
//! override-`.DBC` shape (`pcf_model::Dbc`), so a team loaded straight out of
//! the game's own bundled data can flow through the same UI screens /
//! `save_dbc` write path as a team opened from an existing override file.
//!
//! ## Why this lives in `pcf-codec`, not `src-tauri`
//!
//! It's pure data mapping with no filesystem/IPC concerns, so it belongs
//! next to the two formats it bridges (`container.rs` and `dbc.rs`), not in
//! the Tauri command layer (PLAN.md §4: "no business logic lives in
//! `src-tauri`").
//!
//! ## Why no `&CharMap` parameter
//!
//! The brief sketch in this feature's task description suggested a
//! signature like `container_team_to_dbc(record, charmap: &CharMap)`, but
//! [`ContainerTeamRecord`]'s fields are already fully-decoded `String`s (the
//! container parser did that decoding itself) and every "default" this
//! module fills in is either a plain number or a short ASCII placeholder
//! (`"x"`, `"ND,ND,ND,ND,ND=="`) that's charmap-representable by
//! construction (same literals `dbc.rs`'s own mocks/tests use) — there is
//! no text in this module that needs *encoding*. Threading an unused
//! `&CharMap` through would be misleading (the module doesn't touch the
//! byte-level charmap at all), so the signature is
//! `container_team_to_dbc(record: &ContainerTeamRecord) -> Dbc`.
//!
//! ## Default-filling policy
//!
//! `pcf_model::Team`/`Coach`/`Player` are frozen contracts modeled on the
//! override `.DBC` format (PLAN.md Appendix A), which has several fields
//! this container format's confirmed layout (`fixtures/PKF_FORMAT.md` §6)
//! either doesn't carry at all, carries in a shape `container.rs` doesn't
//! parse yet (kept in `trailing_raw`), or carries with only
//! medium-confidence field identity. Every default below cites exactly
//! which of those three cases applies and where. The general convention
//! (matching `dbc.rs`'s own documented defaults and `src-tauri/src/mock.rs`'s
//! `blank_dbc`) is: identity/free-text fields default to `"x"`, "career"
//! fields default to `"ND,ND,ND,ND,ND=="`, and numeric "no value" fields use
//! the format's own sentinel (`0xFFFF` for affiliate pointers) rather than 0
//! where the override format documents such a sentinel.

use pcf_model::{
    AttackType, Attributes, Clearance, Coach, Date, Dbc, DbcHeader, Demarcation, Division, Hair,
    LeagueResult, Marking, Player, Pressing, Role, Skin, Tackling, Tactics, Team, TeamStats,
};

use crate::container::{ContainerPlayerRecord, ContainerTeamRecord};

/// Replacement character `container.rs`'s lossy free-text decode
/// (`read_lossy_string`) substitutes for any byte the charmap doesn't
/// cover yet. Real biographical prose (a player's `long_name`,
/// `birthplace`, and the 9 free-text fields) routinely contains bytes the
/// charmap — built from short team/player identifying names, not full
/// prose — doesn't confirm, so those fields may contain this character.
/// It has no charmap *encode* mapping, so a bridged `Dbc` containing it
/// verbatim fails `DbcCodec::write` (`charmap_unknown_char`) the instant a
/// user tries to save, even without editing anything — found by actually
/// running `load_pkf_team_bridges_rivers_real_container_record_into_a_real_dbc`
/// against the real file, not assumed.
const LOSSY_PLACEHOLDER: char = '\u{FFFD}';

/// Replaces [`LOSSY_PLACEHOLDER`] with a plain space — one of the very
/// first bytes confirmed charmap-encodable (PLAN.md Appendix A's own
/// worked example, byte `0x41`, re-verified in every investigation pass
/// since). This doesn't recover the real, still-unconfirmed glyph (that
/// requires expanding the charmap further — an open item, not something to
/// guess at here); it only guarantees a bridged `Dbc` stays immediately
/// writable via `save_dbc`, the same "documented placeholder, not a
/// fabricated real value" spirit as this module's other defaults.
fn sanitize_lossy_text(s: &str) -> String {
    if s.contains(LOSSY_PLACEHOLDER) {
        s.replace(LOSSY_PLACEHOLDER, " ")
    } else {
        s.to_string()
    }
}

/// Length in bytes of the "jornada positions" blob (`Team::jornada`).
/// Mirrors `dbc.rs::JORNADA_LEN` (kept as a separate local constant since
/// that one is private to `dbc.rs`) — the override format's
/// `DbcCodec::write` rejects any other length. PKF_FORMAT.md §6.3 confirms
/// this container format *does* have a same-length (92-byte) jornada-shaped
/// block for real teams, but `container.rs` doesn't parse it out as a
/// confirmed field yet (it's part of `trailing_raw`), so this bridge can't
/// forward real bytes — it writes zeros, matching the override format's own
/// documented default ("the editor always writes zeros here").
const JORNADA_LEN: usize = 92;

/// Length in bytes of the palmarés blob (`Team::palmares`). Mirrors
/// `dbc.rs::PALMARES_LEN`. PKF_FORMAT.md §6.3 found **no room for a
/// separate palmarés blob at all** in this container's domestic-record
/// layout (the jornada-shaped block is immediately followed by the
/// tactics-formation region with zero bytes between them) — so there is no
/// real container data to forward here under any interpretation, not just
/// an unparsed one. Zeros, sized to what `DbcCodec::write` requires.
const PALMARES_LEN: usize = 34;

/// The override format's own file-version marker (PLAN.md Appendix A:
/// "`FE01` — file-version marker"). The container format doesn't carry an
/// equivalent per-record version field, so this is simply the fixed value
/// every override file uses.
const DEFAULT_FILE_VERSION: u16 = 0xfe01;

/// `DbcHeader::language`: `0` = Spanish, the editor's own documented
/// default (PLAN.md Appendix A). Not a container field.
const DEFAULT_LANGUAGE: u8 = 0;

/// Editor-canonical tactics default, taken verbatim from PLAN.md Appendix
/// A's own worked example (`46390001000001` = touch 70%, counter 57%,
/// offensive/medium/zonal/played/own-half). PKF_FORMAT.md §6.3 found a
/// *candidate* tactics tail in the container's own bytes for one real team
/// (touch=80, counter=40 for River) but flagged it low-medium confidence
/// (one byte short of the override's 7-byte tail, `pressing` unaccounted
/// for) and — critically — `ContainerTeamRecord` doesn't expose it as a
/// parsed field at all (it's inside `trailing_raw`), so there is nothing
/// real to forward here yet; using the documented editor default is more
/// honest than fabricating a `Tactics` value that looks real but isn't.
const DEFAULT_TOUCH_PCT: u8 = 70;
const DEFAULT_COUNTER_PCT: u8 = 57;

/// `Coach` free-text default for fields the container format doesn't carry
/// at all for coaches (`profile`, `systems`, `palmares`, `anecdotes`,
/// `last_season`, `declarations`) — PKF_FORMAT.md §6.5 only confirms
/// `pointer`/`short_name`/`long_name` for the container's coach chain.
/// Matches `dbc.rs`'s own documented free-text default.
const FREE_TEXT_DEFAULT: &str = "x";

/// `Coach`/`Player` career-field default when there's no real data to put
/// there. Matches `dbc.rs`'s own documented default verbatim (and the real
/// "ND,ND,ND,ND,ND==" byte pattern PKF_FORMAT.md §6.5 found in the actual
/// game file at a career-field position, confirming this literal is
/// authentic to the format, not an editor invention).
const CAREER_DEFAULT: &str = "ND,ND,ND,ND,ND==";

/// `Team::affiliate1`/`affiliate2` "none" sentinel (PLAN.md §4.1's own doc
/// comment: "0xFFFF = none"). PKF_FORMAT.md §6.3 found a plausible
/// `FF FF FF FF` pair in the container's own bytes at a position consistent
/// with these two fields, but flagged it medium confidence and, again, not
/// a field `ContainerTeamRecord` exposes — so this bridge can't forward a
/// real value, only the documented "none" convention.
const NO_AFFILIATE: u16 = 0xffff;

/// Maps a container-format domestic team record onto the frozen override
/// `pcf_model::Dbc` shape.
///
/// Every field either comes straight from a confirmed [`ContainerTeamRecord`]
/// field (see that struct's own doc comments for the byte-offset evidence),
/// or is a documented default per this module's header comment — never a
/// guess at a real, unconfirmed value.
pub fn container_team_to_dbc(record: &ContainerTeamRecord) -> Dbc {
    Dbc {
        header: DbcHeader {
            file_version: DEFAULT_FILE_VERSION,
            language: DEFAULT_LANGUAGE,
            // Always false: `record` came from `find_domestic_team_records`,
            // which only matches the domestic header shape (tail byte
            // `0x00`) — foreign-club stubs (tail byte `0x01`) are a
            // structurally different, much shorter record this module
            // never receives (PKF_FORMAT.md §3/§4).
            is_foreign: false,
        },
        team: container_team_info(record),
        tactics: default_tactics(),
        // Non-foreign `Dbc`s must carry a `Coach` for `DbcCodec::write` to
        // succeed (see `dbc.rs::write`'s `dbc_missing_coach` check) — a
        // domestic container record is always non-foreign, so even when no
        // coach chain was located (`record.coach == None`, e.g. thinner
        // records for smaller clubs, PKF_FORMAT.md §8 UPDATE 2) this bridge
        // must still produce *some* `Coach`, not `None`, or the resulting
        // `Dbc` would be unwritable by `save_dbc`. `container_coach`
        // handles both cases explicitly.
        coach: Some(container_coach(record)),
        // `ContainerTeamRecord::players` (PKF_FORMAT.md §6.6-§6.7) is now a
        // confirmed, parsed field — map each entry through to
        // `pcf_model::Player` the same way `container_coach` does for the
        // coach stub.
        players: record.players.iter().map(container_player).collect(),
    }
}

fn container_team_info(record: &ContainerTeamRecord) -> Team {
    Team {
        short_name: record.short_name.clone(),
        stadium_name: record.stadium_name.clone(),
        long_name: record.long_name.clone(),
        country: record.country,
        capacity: record.capacity,
        standing_capacity: record.standing_capacity,
        founded: record.founded,
        members: record.members,
        president: record.president.clone(),
        // Not parsed by `ContainerTeamRecord`, and **deliberately still
        // not implemented** after a real-file investigation (Vélez bug
        // report, PKF_FORMAT.md UPDATE, "budget field investigation"):
        // PKF_FORMAT.md §6.3 originally hypothesized `budget` as a u24 LE
        // right after `president`, by analogy with the override format's
        // field order. Checking that exact position against 55 real
        // domestic records found real, structured data there, but it does
        // NOT look like a per-team currency budget: (1) the 2-3 byte value
        // is immediately followed (after a few zero-padding bytes) by a
        // length-prefixed string that decodes to real, historically
        // accurate sponsor names ("QUILMES", "CABLEVISION", "MULTICANAL",
        // "NO TIENE" = "none") — this whole region is a sponsor block, not
        // an economy block; (2) the numeric value ties in an implausible
        // way for a real budget: River and Boca (Argentina's two biggest,
        // most storied clubs) share the exact same value (2025), San
        // Lorenzo and Independiente share a different exact same value
        // (1860), roughly half of all 55 teams read exactly 0 (including
        // several real, well-known top-flight clubs of the era), and the
        // value does NOT correlate with whether the team even has a
        // sponsor (Independiente reads "NO TIENE" yet still has a nonzero
        // value identical to sponsored San Lorenzo). This pattern — small
        // integer, shared across peer-tier clubs, decorrelated from the
        // adjacent sponsor field — looks far more like a "reputation" or
        // "tier" rating used internally by the game's economy/AI than a
        // literal peso figure a user would manage, but there's no
        // independently-checkable real-world fact (unlike stadium capacity
        // or a club president's name) to confirm either reading. Per this
        // project's own charmap-provenance rigor standard, an unconfirmed
        // guess isn't wired in just to make the UI show a nonzero number:
        // `budget` stays honestly `0` until a real fact (a documented real
        // club budget figure, or evidence pinning down what this value
        // actually drives in-game) can confirm or refute either reading.
        // See `fixtures/PKF_FORMAT.md`'s UPDATE note for the full
        // evidence trail (all 55 teams' raw bytes at this offset).
        budget: 0,
        affiliate1: NO_AFFILIATE,
        affiliate2: NO_AFFILIATE,
        // Not parsed by `ContainerTeamRecord`. PKF_FORMAT.md §6.3 identifies
        // a league-history-shaped 20-byte block in the container's raw
        // bytes, but in **(division, position) order**, the reverse of the
        // override format's (position, division) — reusing those raw bytes
        // here without `container.rs` itself confirming and re-ordering
        // them would silently bake in a field-order bug. Default to "no
        // history" (division First, position 0) for all 10 seasons.
        league_history: [LeagueResult {
            position: 0,
            division: Division::First,
        }; 10],
        // Not parsed by `ContainerTeamRecord`. PKF_FORMAT.md §6.3's
        // candidate stats block reads an implausible `played=17,408`,
        // flagged unresolved — not something to forward as real data.
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
        // See `JORNADA_LEN`'s doc comment: real-shaped data exists in the
        // container but isn't a confirmed, extracted field yet.
        jornada: vec![0u8; JORNADA_LEN],
        // See `PALMARES_LEN`'s doc comment: PKF_FORMAT.md §6.3 found no
        // room for this blob in the container's layout at all.
        palmares: vec![0u8; PALMARES_LEN],
    }
}

/// Editor-canonical tactics default — see `DEFAULT_TOUCH_PCT`'s doc comment
/// for why this isn't sourced from the container's own (unparsed,
/// low-confidence) tactics-tail bytes.
fn default_tactics() -> Tactics {
    Tactics {
        touch_pct: DEFAULT_TOUCH_PCT,
        counter_pct: DEFAULT_COUNTER_PCT,
        attack: AttackType::Offensive,
        tackling: Tackling::Medium,
        marking: Marking::Zonal,
        clearance: Clearance::Played,
        pressing: Pressing::OwnHalf,
        // No length-prefix / fixed-size candidate for this in the container
        // was confirmed enough to extract (PKF_FORMAT.md §6.3: "low-medium
        // confidence" on the formation_blob identification). Empty is a
        // valid, round-trippable value for the override format's
        // opaque-blob encoding (`Writer::opaque_blob` just writes a
        // zero-length prefix).
        formation_blob: Vec::new(),
    }
}

/// Builds the `Coach` half of the bridged `Dbc`, handling both cases: a
/// coach-chain stub was found (`record.coach.is_some()`) or not.
fn container_coach(record: &ContainerTeamRecord) -> Coach {
    match &record.coach {
        Some(stub) => Coach {
            pointer: stub.pointer,
            short_name: stub.short_name.clone(),
            long_name: stub.long_name.clone(),
            // PKF_FORMAT.md §6.5 confirms only `pointer`/`short_name`/
            // `long_name` for the container's coach chain; everything past
            // `long_name` (profile, systems, palmarés, anecdotes, last
            // season, career, declarations) is real prose in the file
            // (§6.5's "Metro River Plate"/"Nacional River Plate" find) but
            // deliberately NOT reproduced by `container.rs` — parsing it
            // field-by-field isn't confirmed/implemented, and this doc's
            // own guardrail against committing free-text biographical prose
            // means this bridge shouldn't try to smuggle it through even if
            // it had the bytes. Free-text fields default to "x".
            profile: FREE_TEXT_DEFAULT.to_string(),
            systems: FREE_TEXT_DEFAULT.to_string(),
            palmares: FREE_TEXT_DEFAULT.to_string(),
            anecdotes: FREE_TEXT_DEFAULT.to_string(),
            last_season: FREE_TEXT_DEFAULT.to_string(),
            career_coach: CAREER_DEFAULT.to_string(),
            // Not confirmed from `ContainerCoachStub` (it has no
            // `was_player` field) — PKF_FORMAT.md §6.5 discusses real
            // evidence Ramón Díaz specifically *was* a player, but that's
            // from prose analysis outside what `container.rs` parses.
            // Default to `false` (the more conservative "no extra section"
            // choice — `dbc.rs::write_coach` only emits the
            // `career_player` section at all when `was_player` is true).
            was_player: false,
            career_player: FREE_TEXT_DEFAULT.to_string(),
            declarations: FREE_TEXT_DEFAULT.to_string(),
        },
        None => Coach {
            // No coach-chain marker was found at all for this record
            // (PKF_FORMAT.md §8 UPDATE 2: expected for smaller/lower-tier
            // clubs' thinner records). `0` isn't a documented "no coach"
            // sentinel anywhere in the frozen contract (unlike
            // `affiliate1`/`affiliate2`'s `0xFFFF`), so this is a plain
            // placeholder, not a meaningful value — callers that care
            // whether a coach was actually found should check
            // `ContainerTeamRecord::coach` themselves before calling this
            // function, rather than infer it from the placeholder `Coach`.
            pointer: 0,
            // Identity fields default to empty string, matching
            // `mock::blank_dbc`'s own convention for "nothing known" (as
            // opposed to free-text fields, which use "x").
            short_name: String::new(),
            long_name: String::new(),
            profile: FREE_TEXT_DEFAULT.to_string(),
            systems: FREE_TEXT_DEFAULT.to_string(),
            palmares: FREE_TEXT_DEFAULT.to_string(),
            anecdotes: FREE_TEXT_DEFAULT.to_string(),
            last_season: FREE_TEXT_DEFAULT.to_string(),
            career_coach: CAREER_DEFAULT.to_string(),
            was_player: false,
            career_player: FREE_TEXT_DEFAULT.to_string(),
            declarations: FREE_TEXT_DEFAULT.to_string(),
        },
    }
}

/// Maps one confirmed [`ContainerPlayerRecord`] (PKF_FORMAT.md §6.6) onto
/// the frozen override `pcf_model::Player` shape. Every field here comes
/// straight from a confirmed container field — this container's per-player
/// layout is byte-for-byte identical to `dbc.rs::read_player`'s field order
/// from `slot` onward (see `ContainerPlayerRecord`'s own doc comment), so
/// unlike `container_team_info`/`container_coach` above, there are no
/// "not parsed yet" defaults needed here.
///
/// The one adaptation needed: `ContainerPlayerRecord` keeps `roles`/`skin`/
/// `hair`/`demarcation` as raw bytes rather than `pcf_model`'s enums
/// (deliberately — see that struct's doc comment on why this container
/// format's byte semantics aren't assumed identical to the override
/// format's strict validation). `role_from_raw`/`skin_from_raw`/
/// `hair_from_raw`/`demarcation_from_raw` below convert them, falling back
/// to a safe default variant for any byte outside the confirmed real-data
/// range rather than panicking or failing the whole bridge — real data
/// (all 27 of River's players, §6.6) never exercises that fallback path.
fn container_player(p: &ContainerPlayerRecord) -> Player {
    Player {
        pointer: p.pointer,
        number: p.number,
        short_name: p.short_name.clone(),
        long_name: sanitize_lossy_text(&p.long_name),
        slot: p.slot,
        origin: p.origin,
        roles: [
            role_from_raw(p.roles[0]),
            role_from_raw(p.roles[1]),
            role_from_raw(p.roles[2]),
            role_from_raw(p.roles[3]),
            role_from_raw(p.roles[4]),
            role_from_raw(p.roles[5]),
        ],
        nationality: p.nationality,
        skin: skin_from_raw(p.skin),
        hair: hair_from_raw(p.hair),
        demarcation: demarcation_from_raw(p.demarcation),
        birth: Date {
            day: p.birth_day,
            month: p.birth_month,
            year: p.birth_year,
        },
        height_cm: p.height_cm,
        weight_kg: p.weight_kg,
        birth_country: p.birth_country,
        birthplace: sanitize_lossy_text(&p.birthplace),
        debut_club: sanitize_lossy_text(&p.debut_club),
        international: sanitize_lossy_text(&p.international),
        profile: sanitize_lossy_text(&p.profile),
        characteristics: sanitize_lossy_text(&p.characteristics),
        palmares: sanitize_lossy_text(&p.palmares),
        internationality: sanitize_lossy_text(&p.internationality),
        anecdotes: sanitize_lossy_text(&p.anecdotes),
        last_season: sanitize_lossy_text(&p.last_season),
        career: sanitize_lossy_text(&p.career),
        attrs: Attributes {
            velocidad: p.attrs[0],
            resistencia: p.attrs[1],
            agresividad: p.attrs[2],
            calidad: p.attrs[3],
            remate: p.attrs[4],
            regate: p.attrs[5],
            pase: p.attrs[6],
            tiro: p.attrs[7],
            entradas: p.attrs[8],
            portero: p.attrs[9],
        },
    }
}

/// `Role`'s discriminants are the contiguous range `0x00..=0x12` — see
/// `pcf_model::Role`. Falls back to `Empty` for any other raw byte (not
/// expected on real data, per `ContainerPlayerRecord::roles`' doc comment).
fn role_from_raw(byte: u8) -> Role {
    match byte {
        0x00 => Role::Empty,
        0x01 => Role::Gk,
        0x02 => Role::Rb,
        0x03 => Role::Lb,
        0x04 => Role::Sweeper,
        0x05 => Role::Lcb,
        0x06 => Role::Rcb,
        0x07 => Role::Rm,
        0x08 => Role::Rim,
        0x09 => Role::Cf,
        0x0a => Role::DeepPlaymaker,
        0x0b => Role::Lm,
        0x0c => Role::Rw,
        0x0d => Role::CentralAm,
        0x0e => Role::Lw,
        0x0f => Role::Dm,
        0x10 => Role::RightAm,
        0x11 => Role::LeftAm,
        0x12 => Role::Lim,
        _ => Role::Empty,
    }
}

/// Falls back to `White` for any raw byte outside the confirmed `1..=3`
/// range (see `ContainerPlayerRecord::skin`'s doc comment).
fn skin_from_raw(byte: u8) -> Skin {
    match byte {
        0x01 => Skin::White,
        0x02 => Skin::Black,
        0x03 => Skin::Mixed,
        _ => Skin::White,
    }
}

/// Falls back to `Blond` for any raw byte outside the confirmed `1..=6`
/// range (see `ContainerPlayerRecord::hair`'s doc comment).
fn hair_from_raw(byte: u8) -> Hair {
    match byte {
        0x01 => Hair::Blond,
        0x02 => Hair::Bald,
        0x03 => Hair::Dark,
        0x04 => Hair::WhiteGrey,
        0x05 => Hair::Red,
        0x06 => Hair::Brown,
        _ => Hair::Blond,
    }
}

/// Falls back to `Gk` for any raw byte outside the confirmed `0..=3` range
/// (see `ContainerPlayerRecord::demarcation`'s doc comment).
fn demarcation_from_raw(byte: u8) -> Demarcation {
    match byte {
        0x00 => Demarcation::Gk,
        0x01 => Demarcation::Def,
        0x02 => Demarcation::Mid,
        0x03 => Demarcation::Fwd,
        _ => Demarcation::Gk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerCoachStub;

    /// Builds a syntactically-valid `ContainerTeamRecord` with made-up
    /// (not-real, per this project's data guardrails) values, exercising
    /// every field this module reads from.
    fn synthetic_record(coach: Option<ContainerCoachStub>) -> ContainerTeamRecord {
        ContainerTeamRecord {
            header_prefix: [0xAB, 0xCD],
            short_name: "Testclub".to_string(),
            stadium_name: "Test Stadium".to_string(),
            country: 3,
            long_name: "Testclub Athletic Club".to_string(),
            capacity: 12_345,
            standing_capacity: 678,
            pitch_size: (70, 105),
            founded: 1950,
            members: 9_000,
            president: "Test President".to_string(),
            unexplained_byte_after_country: 0,
            unexplained_bytes_after_founded: [0, 0],
            coach,
            players: vec![],
            trailing_raw: vec![0xDE, 0xAD],
        }
    }

    #[test]
    fn maps_confirmed_team_info_fields_straight_through() {
        let record = synthetic_record(None);
        let dbc = container_team_to_dbc(&record);

        assert_eq!(dbc.team.short_name, "Testclub");
        assert_eq!(dbc.team.stadium_name, "Test Stadium");
        assert_eq!(dbc.team.long_name, "Testclub Athletic Club");
        assert_eq!(dbc.team.country, 3);
        assert_eq!(dbc.team.capacity, 12_345);
        assert_eq!(dbc.team.standing_capacity, 678);
        assert_eq!(dbc.team.founded, 1950);
        assert_eq!(dbc.team.members, 9_000);
        assert_eq!(dbc.team.president, "Test President");
    }

    #[test]
    fn domestic_record_is_never_foreign() {
        let dbc = container_team_to_dbc(&synthetic_record(None));
        assert!(!dbc.header.is_foreign);
    }

    #[test]
    fn defaulted_blobs_match_the_lengths_dbccodec_write_requires() {
        let dbc = container_team_to_dbc(&synthetic_record(None));
        assert_eq!(dbc.team.jornada.len(), JORNADA_LEN);
        assert_eq!(dbc.team.palmares.len(), PALMARES_LEN);
    }

    #[test]
    fn coach_stub_maps_confirmed_fields_and_defaults_the_rest() {
        let coach = ContainerCoachStub {
            pointer: 4242,
            short_name: "Test Coach".to_string(),
            long_name: "Test Head Coach".to_string(),
        };
        let dbc = container_team_to_dbc(&synthetic_record(Some(coach)));
        let coach = dbc.coach.expect("non-foreign team must carry a coach");

        assert_eq!(coach.pointer, 4242);
        assert_eq!(coach.short_name, "Test Coach");
        assert_eq!(coach.long_name, "Test Head Coach");
        assert_eq!(coach.profile, "x");
        assert_eq!(coach.career_coach, "ND,ND,ND,ND,ND==");
        assert!(!coach.was_player);
    }

    #[test]
    fn missing_coach_stub_still_produces_a_writable_placeholder_coach() {
        let dbc = container_team_to_dbc(&synthetic_record(None));
        let coach = dbc
            .coach
            .expect("non-foreign team must carry a coach even with no located stub");

        assert_eq!(coach.short_name, "");
        assert_eq!(coach.long_name, "");
    }

    #[test]
    fn players_are_empty_when_the_container_record_has_none() {
        let dbc = container_team_to_dbc(&synthetic_record(None));
        assert!(dbc.players.is_empty());
    }

    #[test]
    fn players_map_through_from_the_container_roster() {
        use crate::container::ContainerPlayerRecord;

        let mut record = synthetic_record(None);
        record.players.push(ContainerPlayerRecord {
            pointer: 6400,
            number: 9,
            gap: vec![],
            short_name: "Test Player".to_string(),
            long_name: "Testy McPlayer".to_string(),
            slot: 1,
            origin: 0,
            roles: [0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
            nationality: 0x20,
            skin: 2,
            hair: 3,
            demarcation: 0,
            birth_day: 16,
            birth_month: 4,
            birth_year: 1970,
            height_cm: 188,
            weight_kg: 82,
            birth_country: 0x03,
            birthplace: "Testville".to_string(),
            debut_club: "x".to_string(),
            international: "x".to_string(),
            profile: "x".to_string(),
            characteristics: "x".to_string(),
            palmares: "x".to_string(),
            internationality: "x".to_string(),
            anecdotes: "x".to_string(),
            last_season: "x".to_string(),
            career: "ND,ND,ND,ND,ND==".to_string(),
            attrs: [50, 60, 70, 80, 65, 55, 45, 40, 35, 90],
        });

        let dbc = container_team_to_dbc(&record);

        assert_eq!(dbc.players.len(), 1);
        let player = &dbc.players[0];
        assert_eq!(player.pointer, 6400);
        assert_eq!(player.number, 9);
        assert_eq!(player.short_name, "Test Player");
        assert_eq!(player.long_name, "Testy McPlayer");
        assert_eq!(player.roles[0], pcf_model::Role::Gk);
        assert_eq!(player.skin, pcf_model::Skin::Black);
        assert_eq!(player.hair, pcf_model::Hair::Dark);
        assert_eq!(player.demarcation, pcf_model::Demarcation::Gk);
        assert_eq!(player.birth.day, 16);
        assert_eq!(player.birth.month, 4);
        assert_eq!(player.birth.year, 1970);
        assert_eq!(player.birthplace, "Testville");
        assert_eq!(player.attrs.portero, 90);
    }

    #[test]
    fn out_of_range_raw_player_enum_bytes_fall_back_instead_of_panicking() {
        use crate::container::ContainerPlayerRecord;

        let mut record = synthetic_record(None);
        record.players.push(ContainerPlayerRecord {
            pointer: 1,
            number: 1,
            gap: vec![],
            short_name: "x".to_string(),
            long_name: "x".to_string(),
            slot: 0,
            origin: 0,
            roles: [0xFF; 6],
            nationality: 0,
            skin: 0xFF,
            hair: 0xFF,
            demarcation: 0xFF,
            birth_day: 1,
            birth_month: 1,
            birth_year: 1970,
            height_cm: 0,
            weight_kg: 0,
            birth_country: 0,
            birthplace: "x".to_string(),
            debut_club: "x".to_string(),
            international: "x".to_string(),
            profile: "x".to_string(),
            characteristics: "x".to_string(),
            palmares: "x".to_string(),
            internationality: "x".to_string(),
            anecdotes: "x".to_string(),
            last_season: "x".to_string(),
            career: "ND,ND,ND,ND,ND==".to_string(),
            attrs: [0; 10],
        });

        let dbc = container_team_to_dbc(&record);
        let player = &dbc.players[0];
        assert_eq!(player.roles[0], pcf_model::Role::Empty);
        assert_eq!(player.skin, pcf_model::Skin::White);
        assert_eq!(player.hair, pcf_model::Hair::Blond);
        assert_eq!(player.demarcation, pcf_model::Demarcation::Gk);
    }

    #[test]
    fn no_affiliate_sentinel_matches_the_documented_convention() {
        let dbc = container_team_to_dbc(&synthetic_record(None));
        assert_eq!(dbc.team.affiliate1, 0xffff);
        assert_eq!(dbc.team.affiliate2, 0xffff);
    }
}
