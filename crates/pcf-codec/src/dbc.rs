//! `Dbc::read` / `Dbc::write` — the top-level glue that walks a whole DBC
//! file in the order PLAN.md Appendix A specifies: banner, `FE06`, version
//! marker, language, league flag, team record, tactics, optional coach
//! chain, then player records.
//!
//! `pcf_model::Dbc` is frozen and owned by contract, so instead of adding
//! inherent methods to a foreign type we expose a [`DbcCodec`] extension
//! trait implemented for it — `use pcf_codec::DbcCodec;` gets you
//! `dbc.write(&charmap)` / `Dbc::read(bytes, &charmap)` call syntax.
//!
//! Deviation from the literal brief signature: `read`/`write` take a
//! `&CharMap` parameter. Text decode is impossible without one, and the
//! whole point of PLAN.md §9 risk #1 is that the charmap is
//! user/community-supplied at runtime, not baked into the codec — so it
//! must be threaded through explicitly rather than assumed global state.
//!
//! ## Known round-trip gaps (flagged, not silently papered over)
//!
//! Several fields documented in Appendix A have **no home in the frozen
//! `pcf_model` types** (they're not in the M0 contract): the team's
//! "jornada positions" blob (92 bytes, editor always zeros it) and, per
//! player, eight free-text fields (debut club, international, profile,
//! characteristics, palmarés, internationality, anecdotes, last season)
//! plus the free-text career field. This codec reads and discards those
//! bytes, then re-emits the documented *editor defaults*
//! (`"x"` / `"ND,ND,ND,ND,ND=="` / zeros) on write. That means
//! `write(read(bytes)) == bytes` only holds for files whose original bytes
//! already match those defaults — true for anything the editor itself
//! produced, but **not guaranteed for arbitrary real-game DBCs** whose
//! unmodeled fields hold other data. This is a contract gap, not a codec
//! bug: closing it requires a `pcf-model` change (out of Agent A's owned
//! scope) to add fields for that free text, or an explicit decision that
//! v1 never needs to preserve it byte-for-byte.
use binrw::io::Cursor;
use binrw::{BinRead, BinWrite};
use pcf_model::{
    AttackType, Attributes, Clearance, Coach, Date, Dbc, DbcHeader, Demarcation, Division, Hair,
    LeagueResult, Marking, PcfError, Player, Pressing, Role, Skin, Tackling, Tactics, Team,
    TeamStats,
};

use crate::charmap::CharMap;
use crate::cursor::{Reader, Writer};
use crate::layout::{AttributesRaw, DateRaw, TacticsFixedRaw, TeamStatsRaw};

const BANNER: &[u8] = b"Copyright (c) 1996 Dinamic Multimedia";
const MAGIC_FE06: [u8; 2] = [0xFE, 0x06];
const SEPARATOR_ZERO: [u8; 1] = [0x00];
const PITCH_SIZE: [u8; 4] = [0x46, 0x00, 0x6A, 0x00];
const COACH_MARKER: u8 = 0x02;
const COACH_WAS_PLAYER_MARKER: u8 = 0x03;
const PLAYER_MARKER: u8 = 0x01;

/// Number of `(position, division)` pairs in the last-10-seasons table.
const LEAGUE_HISTORY_SEASONS: usize = 10;

/// Length in bytes of the "jornada positions" blob the editor always
/// zeroes. Not modeled in `pcf_model::Team` (no field for it in the M0
/// contract) — read and discarded, always re-written as zeros.
/// TODO(A): confirm against a real Apertura fixture.
const JORNADA_LEN: usize = 92;

/// Length in bytes of the palmarés blob. PLAN.md §9 risk #2: Appendix A's
/// team-record table shows 34 chars but the competitions tab elsewhere
/// references 68 for editor-generated files — treated as version-dependent
/// and **not hardcoded as verified**. Kept as a single named constant so
/// it's a one-line change once a real fixture confirms the real value.
/// TODO(A): confirm against a real Apertura fixture.
const PALMARES_LEN: usize = 34;

/// Whether the standing-capacity field (+ its trailing separator) is
/// present. PLAN.md §9 risk #3: this is version-variant and unconfirmed;
/// wired as a single flag (rather than scattered assumptions) so it can be
/// switched or made conditional on `DbcHeader::file_version` once a real
/// fixture is available. TODO(A): confirm against a real Apertura fixture.
const STANDING_CAPACITY_PRESENT: bool = true;

/// The 8 per-player free-text fields Appendix A documents between
/// birthplace and the career field, none of which have a home in the
/// frozen `pcf_model::Player` (see module docs). Read and discarded;
/// always re-written as this default.
const PLAYER_TEXT_DEFAULT: &str = "x";
const PLAYER_TEXT_FIELD_COUNT: usize = 8;
/// Per-player and per-coach career field default.
const CAREER_DEFAULT: &str = "ND,ND,ND,ND,ND==";

/// Extension trait giving `Dbc::read` / `dbc.write` call syntax on the
/// frozen, foreign `pcf_model::Dbc` type. See module docs for why `read`
/// takes a `&CharMap` parameter.
pub trait DbcCodec: Sized {
    fn read(bytes: &[u8], charmap: &CharMap) -> Result<Self, PcfError>;
    fn write(&self, charmap: &CharMap) -> Result<Vec<u8>, PcfError>;
}

impl DbcCodec for Dbc {
    fn read(bytes: &[u8], charmap: &CharMap) -> Result<Self, PcfError> {
        let mut r = Reader::new(bytes);

        r.expect_fixed(BANNER)?;
        r.expect_fixed(&MAGIC_FE06)?;
        let file_version = r.u16_le()?;
        let language = r.u8()?;
        let league_flag = r.u8()?;
        let is_foreign = league_flag != 0x00;

        let team = read_team(&mut r, charmap)?;
        let tactics = read_tactics(&mut r, charmap)?;

        let coach = if is_foreign {
            None
        } else {
            Some(read_coach(&mut r, charmap)?)
        };

        let mut players = Vec::new();
        if !is_foreign {
            while r.remaining() > 0 {
                players.push(read_player(&mut r, charmap)?);
            }
        }

        Ok(Dbc {
            header: DbcHeader {
                file_version,
                language,
                is_foreign,
            },
            team,
            tactics,
            coach,
            players,
        })
    }

    fn write(&self, charmap: &CharMap) -> Result<Vec<u8>, PcfError> {
        let mut w = Writer::new();

        w.fixed(BANNER);
        w.fixed(&MAGIC_FE06);
        w.u16_le(self.header.file_version);
        w.u8(self.header.language);
        w.u8(if self.header.is_foreign { 0x01 } else { 0x00 });

        write_team(&mut w, charmap, &self.team)?;
        write_tactics(&mut w, charmap, &self.tactics)?;

        if !self.header.is_foreign {
            let coach = self.coach.as_ref().ok_or_else(|| {
                PcfError::new(
                    "dbc_missing_coach",
                    "non-foreign team must have a coach to write a valid DBC",
                )
            })?;
            write_coach(&mut w, charmap, coach)?;
            for player in &self.players {
                write_player(&mut w, charmap, player)?;
            }
        }

        Ok(w.into_bytes())
    }
}

/// Serializes a fixed-layout `binrw` value into a `Vec<u8>`, mapping the
/// (practically unreachable — an in-memory `Vec` write can't hit an IO
/// error) failure mode into a typed `PcfError` rather than `expect`/panic.
fn binwrite_to_vec<T>(value: &T) -> Result<Vec<u8>, PcfError>
where
    T: for<'a> BinWrite<Args<'a> = ()> + binrw::meta::WriteEndian,
{
    let mut buf = Vec::new();
    value
        .write(&mut Cursor::new(&mut buf))
        .map_err(|e| PcfError::new("dbc_layout_error", format!("writing fixed layout: {e}")))?;
    Ok(buf)
}

fn read_team(r: &mut Reader, charmap: &CharMap) -> Result<Team, PcfError> {
    let short_name = r.string(charmap)?;
    let stadium_name = r.string(charmap)?;
    let country = r.u8()?;
    let long_name = r.string(charmap)?;

    let capacity = r.u24_le()?;
    r.expect_fixed(&SEPARATOR_ZERO)?;

    let standing_capacity = if STANDING_CAPACITY_PRESENT {
        let v = r.u24_le()?;
        r.expect_fixed(&SEPARATOR_ZERO)?;
        v
    } else {
        0
    };

    r.expect_fixed(&PITCH_SIZE)?;

    let founded = r.u16_le()?;
    let members = r.u24_le()?;
    r.expect_fixed(&SEPARATOR_ZERO)?;

    let president = r.string(charmap)?;
    let budget = r.u24_le()?;
    let affiliate1 = r.u16_le()?;
    let affiliate2 = r.u16_le()?;

    let league_history = read_league_history(r)?;
    let stats = read_team_stats(r)?;

    // Jornada positions blob: not modeled (see module docs) — discard.
    r.take(JORNADA_LEN)?;

    let palmares = r.take(PALMARES_LEN)?.to_vec();

    Ok(Team {
        short_name,
        stadium_name,
        long_name,
        country,
        capacity,
        standing_capacity,
        founded,
        members,
        president,
        budget,
        affiliate1,
        affiliate2,
        league_history,
        stats,
        palmares,
    })
}

fn write_team(w: &mut Writer, charmap: &CharMap, team: &Team) -> Result<(), PcfError> {
    w.string(charmap, &team.short_name)?;
    w.string(charmap, &team.stadium_name)?;
    w.u8(team.country);
    w.string(charmap, &team.long_name)?;

    w.u24_le(team.capacity);
    w.fixed(&SEPARATOR_ZERO);

    if STANDING_CAPACITY_PRESENT {
        w.u24_le(team.standing_capacity);
        w.fixed(&SEPARATOR_ZERO);
    }

    w.fixed(&PITCH_SIZE);

    w.u16_le(team.founded);
    w.u24_le(team.members);
    w.fixed(&SEPARATOR_ZERO);

    w.string(charmap, &team.president)?;
    w.u24_le(team.budget);
    w.u16_le(team.affiliate1);
    w.u16_le(team.affiliate2);

    write_league_history(w, &team.league_history);
    write_team_stats(w, &team.stats)?;

    w.fixed(&[0u8; JORNADA_LEN]);

    if team.palmares.len() != PALMARES_LEN {
        return Err(PcfError::new(
            "dbc_palmares_length_mismatch",
            format!(
                "palmarés must be exactly {PALMARES_LEN} bytes (see PALMARES_LEN TODO), got {}",
                team.palmares.len()
            ),
        ));
    }
    w.fixed(&team.palmares);

    Ok(())
}

fn read_league_history(r: &mut Reader) -> Result<[LeagueResult; 10], PcfError> {
    let mut out = Vec::with_capacity(LEAGUE_HISTORY_SEASONS);
    for _ in 0..LEAGUE_HISTORY_SEASONS {
        let position = r.u8()?;
        let division_byte = r.u8()?;
        let offset = r.offset();
        let division = division_from_u8(division_byte, offset)?;
        out.push(LeagueResult { position, division });
    }
    out.try_into().map_err(|_| {
        PcfError::new(
            "dbc_internal_error",
            "league history length invariant broken",
        )
    })
}

fn write_league_history(w: &mut Writer, history: &[LeagueResult; 10]) {
    for result in history {
        w.u8(result.position);
        w.u8(result.division as u8);
    }
}

fn read_team_stats(r: &mut Reader) -> Result<TeamStats, PcfError> {
    // 6 * u16 (played/won/drawn/gf/ga/points) + champion(u8) + runner_up(u8).
    let bytes = r.take(14)?;
    let raw = TeamStatsRaw::read(&mut Cursor::new(bytes))
        .map_err(|e| PcfError::new("dbc_layout_error", format!("team stats: {e}")))?;
    Ok(raw.into())
}

fn write_team_stats(w: &mut Writer, stats: &TeamStats) -> Result<(), PcfError> {
    let raw: TeamStatsRaw = (*stats).into();
    w.fixed(&binwrite_to_vec(&raw)?);
    Ok(())
}

fn read_tactics(r: &mut Reader, _charmap: &CharMap) -> Result<Tactics, PcfError> {
    let formation_blob = r.opaque_blob()?;

    let bytes = r.take(7)?;
    let raw = TacticsFixedRaw::read(&mut Cursor::new(bytes))
        .map_err(|e| PcfError::new("dbc_layout_error", format!("tactics: {e}")))?;
    let offset = r.offset();

    Ok(Tactics {
        touch_pct: raw.touch_pct,
        counter_pct: raw.counter_pct,
        attack: attack_type_from_u8(raw.attack, offset)?,
        tackling: tackling_from_u8(raw.tackling, offset)?,
        marking: marking_from_u8(raw.marking, offset)?,
        clearance: clearance_from_u8(raw.clearance, offset)?,
        pressing: pressing_from_u8(raw.pressing, offset)?,
        formation_blob,
    })
}

fn write_tactics(w: &mut Writer, _charmap: &CharMap, tactics: &Tactics) -> Result<(), PcfError> {
    w.opaque_blob(&tactics.formation_blob);

    let raw = TacticsFixedRaw {
        touch_pct: tactics.touch_pct,
        counter_pct: tactics.counter_pct,
        attack: tactics.attack as u8,
        tackling: tactics.tackling as u8,
        marking: tactics.marking as u8,
        clearance: tactics.clearance as u8,
        pressing: tactics.pressing as u8,
    };
    w.fixed(&binwrite_to_vec(&raw)?);
    Ok(())
}

fn read_coach(r: &mut Reader, charmap: &CharMap) -> Result<Coach, PcfError> {
    r.expect_fixed(&[COACH_MARKER])?;
    let pointer = r.u16_le()?;
    let short_name = r.string(charmap)?;
    let long_name = r.string(charmap)?;
    let profile = r.string(charmap)?;
    let systems = r.string(charmap)?;
    let palmares = r.string(charmap)?;
    let anecdotes = r.string(charmap)?;
    let last_season = r.string(charmap)?;
    let career_coach = r.string(charmap)?;

    let (was_player, career_player) = if r.peek_u8() == Some(COACH_WAS_PLAYER_MARKER) {
        r.u8()?;
        (true, r.string(charmap)?)
    } else {
        (false, String::from(PLAYER_TEXT_DEFAULT))
    };

    let declarations = r.string(charmap)?;

    Ok(Coach {
        pointer,
        short_name,
        long_name,
        profile,
        systems,
        palmares,
        anecdotes,
        last_season,
        career_coach,
        was_player,
        career_player,
        declarations,
    })
}

fn write_coach(w: &mut Writer, charmap: &CharMap, coach: &Coach) -> Result<(), PcfError> {
    w.u8(COACH_MARKER);
    w.u16_le(coach.pointer);
    w.string(charmap, &coach.short_name)?;
    w.string(charmap, &coach.long_name)?;
    w.string(charmap, &coach.profile)?;
    w.string(charmap, &coach.systems)?;
    w.string(charmap, &coach.palmares)?;
    w.string(charmap, &coach.anecdotes)?;
    w.string(charmap, &coach.last_season)?;
    w.string(charmap, &coach.career_coach)?;

    if coach.was_player {
        w.u8(COACH_WAS_PLAYER_MARKER);
        w.string(charmap, &coach.career_player)?;
    }

    w.string(charmap, &coach.declarations)?;
    Ok(())
}

fn read_player(r: &mut Reader, charmap: &CharMap) -> Result<Player, PcfError> {
    r.expect_fixed(&[PLAYER_MARKER])?;
    let pointer = r.u16_le()?;
    let number = r.u8()?;
    let short_name = r.string(charmap)?;
    let long_name = r.string(charmap)?;
    let slot = r.u8()?;
    let origin = r.u8()?;

    let mut roles = [Role::Empty; 6];
    for role in roles.iter_mut() {
        let byte = r.u8()?;
        let offset = r.offset();
        *role = role_from_u8(byte, offset)?;
    }

    let nationality = r.u8()?;

    let skin_byte = r.u8()?;
    let skin = skin_from_u8(skin_byte, r.offset())?;

    let hair_byte = r.u8()?;
    let hair = hair_from_u8(hair_byte, r.offset())?;

    let demarcation_byte = r.u8()?;
    let demarcation = demarcation_from_u8(demarcation_byte, r.offset())?;

    let birth_bytes = r.take(4)?;
    let birth: Date = DateRaw::read(&mut Cursor::new(birth_bytes))
        .map_err(|e| PcfError::new("dbc_layout_error", format!("player birth date: {e}")))?
        .into();

    let height_cm = r.u8()?;
    let weight_kg = r.u8()?;
    let birth_country = r.u8()?;
    let birthplace = r.string(charmap)?;

    // 8 free-text fields with no home in the frozen model — discard.
    for _ in 0..PLAYER_TEXT_FIELD_COUNT {
        r.string(charmap)?;
    }
    // Career field — same story.
    r.string(charmap)?;

    let attr_bytes = r.take(10)?;
    let attrs: Attributes = AttributesRaw::read(&mut Cursor::new(attr_bytes))
        .map_err(|e| PcfError::new("dbc_layout_error", format!("player attributes: {e}")))?
        .into();

    Ok(Player {
        pointer,
        number,
        short_name,
        long_name,
        slot,
        origin,
        roles,
        nationality,
        skin,
        hair,
        demarcation,
        birth,
        height_cm,
        weight_kg,
        birth_country,
        birthplace,
        attrs,
    })
}

fn write_player(w: &mut Writer, charmap: &CharMap, player: &Player) -> Result<(), PcfError> {
    w.u8(PLAYER_MARKER);
    w.u16_le(player.pointer);
    w.u8(player.number);
    w.string(charmap, &player.short_name)?;
    w.string(charmap, &player.long_name)?;
    w.u8(player.slot);
    w.u8(player.origin);

    for role in &player.roles {
        w.u8(*role as u8);
    }

    w.u8(player.nationality);
    w.u8(player.skin as u8);
    w.u8(player.hair as u8);
    w.u8(player.demarcation as u8);

    let date_raw: DateRaw = player.birth.into();
    w.fixed(&binwrite_to_vec(&date_raw)?);

    w.u8(player.height_cm);
    w.u8(player.weight_kg);
    w.u8(player.birth_country);
    w.string(charmap, &player.birthplace)?;

    for _ in 0..PLAYER_TEXT_FIELD_COUNT {
        w.string(charmap, PLAYER_TEXT_DEFAULT)?;
    }
    w.string(charmap, CAREER_DEFAULT)?;

    let attr_raw: AttributesRaw = player.attrs.into();
    w.fixed(&binwrite_to_vec(&attr_raw)?);

    Ok(())
}

// ---------------------------------------------------------------------
// Small byte -> enum conversions.
//
// `pcf_model`'s enums are foreign types and `TryFrom` is a foreign trait,
// so we can't `impl TryFrom<u8> for AttackType` here (orphan rule) — these
// free functions are the local equivalent, each returning a typed,
// offset-carrying error instead of panicking on out-of-range bytes.
// ---------------------------------------------------------------------

macro_rules! enum_from_u8 {
    ($fn_name:ident, $ty:ty, { $($val:expr => $variant:ident),+ $(,)? }) => {
        fn $fn_name(v: u8, offset: usize) -> Result<$ty, PcfError> {
            match v {
                $($val => Ok(<$ty>::$variant),)+
                other => Err(PcfError::new(
                    "dbc_invalid_enum_byte",
                    format!(
                        concat!("invalid ", stringify!($ty), " byte 0x{:02X} at offset {}"),
                        other, offset
                    ),
                )
                .with_context(format!("offset={offset}"))),
            }
        }
    };
}

enum_from_u8!(division_from_u8, Division, {
    0x00 => First, 0x01 => Second, 0x02 => SecondB, 0x03 => Third,
});

enum_from_u8!(attack_type_from_u8, AttackType, {
    0x00 => Offensive, 0x01 => Speculative, 0x02 => Mixed,
});

enum_from_u8!(tackling_from_u8, Tackling, {
    0x00 => Soft, 0x01 => Medium, 0x02 => Aggressive,
});

enum_from_u8!(marking_from_u8, Marking, {
    0x00 => Zonal, 0x01 => Man,
});

enum_from_u8!(clearance_from_u8, Clearance, {
    0x00 => Played, 0x01 => Long,
});

enum_from_u8!(pressing_from_u8, Pressing, {
    0x00 => OwnHalf, 0x01 => Medium, 0x02 => RivalHalf,
});

enum_from_u8!(skin_from_u8, Skin, {
    0x01 => White, 0x02 => Black, 0x03 => Mixed,
});

enum_from_u8!(hair_from_u8, Hair, {
    0x01 => Blond, 0x02 => Bald, 0x03 => Dark, 0x04 => WhiteGrey, 0x05 => Red, 0x06 => Brown,
});

enum_from_u8!(demarcation_from_u8, Demarcation, {
    0x00 => Gk, 0x01 => Def, 0x02 => Mid, 0x03 => Fwd,
});

enum_from_u8!(role_from_u8, Role, {
    0x00 => Empty, 0x01 => Gk, 0x02 => Rb, 0x03 => Lb, 0x04 => Sweeper, 0x05 => Lcb,
    0x06 => Rcb, 0x07 => Rm, 0x08 => Rim, 0x09 => Cf, 0x0a => DeepPlaymaker, 0x0b => Lm,
    0x0c => Rw, 0x0d => CentralAm, 0x0e => Lw, 0x0f => Dm, 0x10 => RightAm, 0x11 => LeftAm,
    0x12 => Lim,
});
