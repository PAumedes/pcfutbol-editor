//! `binrw` structs for the purely fixed-size, regularly-shaped chunks of
//! the DBC record layout (PLAN.md Appendix A). Variable-length pieces
//! (charmap-encoded strings, the version-dependent standing-capacity /
//! palmarés fields) are *not* modeled here — those need conditional /
//! context-aware logic and are handled by hand in `dbc.rs` using the
//! `cursor` helpers instead of fighting `binrw`'s derive for a case it
//! doesn't fit well.
//!
//! These are private-to-the-crate wire structs, converted to/from the
//! frozen `pcf_model` types at the edges — `pcf_model` types can't derive
//! `BinRead`/`BinWrite` themselves (foreign crate, frozen contract).

use binrw::binrw;
use pcf_model::{Attributes, Date, TeamStats};

/// Attributes on disk, in the exact order Appendix A specifies:
/// VE, RE, AG, CA, RM, RG, PA, TI, EN, PO. Do not reorder.
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributesRaw {
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

impl From<AttributesRaw> for Attributes {
    fn from(r: AttributesRaw) -> Self {
        Attributes {
            velocidad: r.velocidad,
            resistencia: r.resistencia,
            agresividad: r.agresividad,
            calidad: r.calidad,
            remate: r.remate,
            regate: r.regate,
            pase: r.pase,
            tiro: r.tiro,
            entradas: r.entradas,
            portero: r.portero,
        }
    }
}

impl From<Attributes> for AttributesRaw {
    fn from(a: Attributes) -> Self {
        AttributesRaw {
            velocidad: a.velocidad,
            resistencia: a.resistencia,
            agresividad: a.agresividad,
            calidad: a.calidad,
            remate: a.remate,
            regate: a.regate,
            pase: a.pase,
            tiro: a.tiro,
            entradas: a.entradas,
            portero: a.portero,
        }
    }
}

/// Played/won/drawn/gf/ga/points (u16 each) + champion/runner-up (u8 each).
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamStatsRaw {
    pub played: u16,
    pub won: u16,
    pub drawn: u16,
    pub gf: u16,
    pub ga: u16,
    pub points: u16,
    pub champion: u8,
    pub runner_up: u8,
}

impl From<TeamStatsRaw> for TeamStats {
    fn from(r: TeamStatsRaw) -> Self {
        TeamStats {
            played: r.played,
            won: r.won,
            drawn: r.drawn,
            gf: r.gf,
            ga: r.ga,
            points: r.points,
            champion: r.champion,
            runner_up: r.runner_up,
        }
    }
}

impl From<TeamStats> for TeamStatsRaw {
    fn from(s: TeamStats) -> Self {
        TeamStatsRaw {
            played: s.played,
            won: s.won,
            drawn: s.drawn,
            gf: s.gf,
            ga: s.ga,
            points: s.points,
            champion: s.champion,
            runner_up: s.runner_up,
        }
    }
}

/// day, month, year(LE u16) — 4 bytes.
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRaw {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

impl From<DateRaw> for Date {
    fn from(r: DateRaw) -> Self {
        Date {
            day: r.day,
            month: r.month,
            year: r.year,
        }
    }
}

impl From<Date> for DateRaw {
    fn from(d: Date) -> Self {
        DateRaw {
            day: d.day,
            month: d.month,
            year: d.year,
        }
    }
}

/// The 7-byte fixed tactics chunk: touch%, counter%, attack, tackling,
/// marking, clearance, pressing. Example from Appendix A: `46390001000001`
/// (touch=70, counter=57, attack=off, tackling=medium, marking=zonal,
/// clearance=played, pressing=medium).
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticsFixedRaw {
    pub touch_pct: u8,
    pub counter_pct: u8,
    pub attack: u8,
    pub tackling: u8,
    pub marking: u8,
    pub clearance: u8,
    pub pressing: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::io::Cursor;
    use binrw::{BinRead, BinWrite};

    #[test]
    fn attributes_raw_round_trips_and_preserves_field_order() {
        let raw = AttributesRaw {
            velocidad: 1,
            resistencia: 2,
            agresividad: 3,
            calidad: 4,
            remate: 5,
            regate: 6,
            pase: 7,
            tiro: 8,
            entradas: 9,
            portero: 10,
        };
        let mut buf = Vec::new();
        raw.write(&mut Cursor::new(&mut buf)).unwrap();
        assert_eq!(buf, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let read_back = AttributesRaw::read(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(read_back, raw);
    }

    #[test]
    fn tactics_fixed_raw_matches_appendix_a_example() {
        // `46390001000001`
        let bytes = [0x46, 0x39, 0x00, 0x01, 0x00, 0x00, 0x01];
        let tactics = TacticsFixedRaw::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(tactics.touch_pct, 0x46);
        assert_eq!(tactics.counter_pct, 0x39);
        assert_eq!(tactics.attack, 0x00);
        assert_eq!(tactics.tackling, 0x01);
        assert_eq!(tactics.marking, 0x00);
        assert_eq!(tactics.clearance, 0x00);
        assert_eq!(tactics.pressing, 0x01);

        let mut buf = Vec::new();
        tactics.write(&mut Cursor::new(&mut buf)).unwrap();
        assert_eq!(buf, bytes);
    }

    #[test]
    fn date_raw_round_trips() {
        // Appendix A founded example: `6E07` -> 0x076E = 1902 (plain LE u16,
        // not the pair-reversal the prose implies — verified against the
        // worked example, not just the description).
        let bytes = [0x0E, 0x00, 0x6E, 0x07];
        let date = DateRaw::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(date.day, 0x0E);
        assert_eq!(date.month, 0x00);
        assert_eq!(date.year, 1902);
    }
}
