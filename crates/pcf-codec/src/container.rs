//! Parser for the `EQ003003.PKF` "teams container" format.
//!
//! This is a genuinely **different** on-disk record framing than the
//! override-file format `crate::dbc` implements — confirmed structural
//! differences (extra bytes at some field boundaries, a 2-byte coach
//! marker instead of 1-byte, a shorter player header) are documented in
//! `fixtures/PKF_FORMAT.md` §6.4/§6.5/§6.6. So this module is a new, local
//! parser with its own types ([`ContainerTeamRecord`], [`ContainerCoachStub`])
//! rather than a reuse of `pcf_model::Team`/`Coach` — those are frozen
//! contracts for the *override* `.DBC` format and shouldn't be stretched to
//! also describe this container's own, independently-confirmed shape.
//!
//! Every field on [`ContainerTeamRecord`] traces to a specific byte offset
//! cited in `fixtures/PKF_FORMAT.md` §6.1/§6.2/§6.5. The full player-roster
//! layout is **not** confirmed yet (§6.6-§6.7: the marker/name-string shape
//! roughly matches the override format, but the fixed fields between
//! `number` and `short_name` don't) — rather than guess at it, everything
//! from after the coach chain (or after `president`, if no coach chain is
//! found) onward is kept as an opaque `trailing_raw` blob.
//!
//! String fields use the exact same wire shape as the override format's
//! `crate::cursor::Reader::string` (u16 LE length prefix + charmap bytes,
//! no padding) — confirmed in PKF_FORMAT.md §6.2 — so this module reuses
//! `crate::cursor::Reader` directly rather than re-implementing it.

use pcf_model::PcfError;

use crate::charmap::CharMap;
use crate::cursor::Reader;

/// `Copyright (c)1996 Dinamic Multimedia` — same banner used by the
/// override format and the foreign-club stub table (PKF_FORMAT.md §1).
const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";

/// The confirmed-constant TAIL of a domestic team record's 6-byte header
/// (PKF_FORMAT.md §8's UPDATE / §9): only the last 4 bytes, `0D 02 00 00`,
/// are constant across teams. The leading 2 bytes (River's happened to be
/// `E9 07`) vary per team — an earlier version of this module wrongly
/// treated all 6 bytes as a fixed constant, which worked for River by
/// coincidence but rejected every other real team, hiding 54 of the 55
/// real domestic records in the file. The last byte here (`0x00`) is what
/// distinguishes a domestic record from a foreign-club stub, which carries
/// `0x01` in the same position (§3-§4).
const DOMESTIC_HEADER_TAIL: [u8; 4] = [0x0D, 0x02, 0x00, 0x00];
/// Byte length of the leading, per-team-varying part of the header.
const DOMESTIC_HEADER_PREFIX_LEN: usize = 2;

/// Single zero-byte separator that follows both `capacity` and
/// `standing_capacity` (and `members`) — PKF_FORMAT.md §6.2.
const ZERO_SEPARATOR: [u8; 1] = [0x00];

/// The 13-byte directory-entry signature (PKF_FORMAT.md §2). NOT used by
/// [`find_domestic_team_records`] any more (see its doc comment and
/// PKF_FORMAT.md §9 for why the directory-based approach was dropped) —
/// kept only so tests can build directory-shaped filler bytes to prove
/// they don't interfere with banner/header scanning.
#[cfg(test)]
const DIR_SIG: [u8; 13] = [
    0x31, 0x54, 0x41, 0xBB, 0xEF, 0xE8, 0xE3, 0xE0, 0x0B, 0xC9, 0xA3, 0xE8, 0x00,
];
/// Directory entries are 38 bytes each (PKF_FORMAT.md §2). Test-only, see
/// [`DIR_SIG`]'s doc comment.
#[cfg(test)]
const DIR_ENTRY_LEN: usize = 38;

/// 2-byte coach-chain marker (PKF_FORMAT.md §6.5, absolute blob offset
/// 482 in the worked River example) — **not** the override format's
/// single-byte `0x02` `COACH_MARKER`.
const COACH_MARKER: [u8; 2] = [0x02, 0x02];

/// A domestic team's confirmed fields, decoded from a single `.PKF`
/// container record. See PKF_FORMAT.md §6.2 for the byte-offset evidence
/// behind each field (offsets below are cited relative to the record's own
/// banner, i.e. `record_start + N`).
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerTeamRecord {
    /// The leading 2 bytes of the 6-byte post-banner header, which vary
    /// per team (River's are `E9 07`) — unlike the trailing 4 bytes
    /// (`0D 02 00 00`), which are constant and used to detect a domestic
    /// record at all. Meaning unconfirmed; kept verbatim. See
    /// `DOMESTIC_HEADER_TAIL`'s doc comment and PKF_FORMAT.md §8/§9.
    pub header_prefix: [u8; 2],
    /// §6.2 offset 44-48 in the worked example (length-prefixed string).
    pub short_name: String,
    /// §6.2 offset 49-72.
    pub stadium_name: String,
    /// §6.2 offset 75 — positionally where the override format's single
    /// `country` byte sits.
    pub country: u8,
    /// §6.2 offset 79-103.
    pub long_name: String,
    /// §6.2 offset 104-107 (u24 LE, zero-separated).
    pub capacity: u32,
    /// §6.2 offset 108-111 (u24 LE, zero-separated).
    pub standing_capacity: u32,
    /// §6.2 offset 112-115: two u16 LE values, `(width, length)`. One byte
    /// off the override format's hardcoded `PITCH_SIZE` constant — §6.4
    /// treats this as plausibly-genuine per-team pitch data, not a decode
    /// error, so it's kept as a real field rather than asserted fixed.
    pub pitch_size: (u16, u16),
    /// §6.2 offset 116-117.
    pub founded: u16,
    /// §6.2 offset 120-123 (u24 LE, zero-separated).
    pub members: u32,
    /// §6.2 offset 124-146.
    pub president: String,
    /// Unexplained extra byte the container inserts right after `country`
    /// and before `long_name`'s length prefix (§6.2 offset 76, discussed in
    /// §6.4). Kept verbatim since its meaning isn't confirmed yet, rather
    /// than silently discarded.
    pub unexplained_byte_after_country: u8,
    /// Unexplained extra 2 bytes right after `founded` and before `members`
    /// (§6.2 offset 118-119, discussed in §6.4).
    pub unexplained_bytes_after_founded: [u8; 2],
    /// The coach-chain fields confirmed in §6.5, if a plausible coach
    /// marker was found in the record. `None` if no confident match was
    /// found (e.g. an unrecognized special entry, or if the byte-exact
    /// boundary between `president` and the coach marker — still
    /// unconfirmed per §6.3 — happens to not contain one).
    pub coach: Option<ContainerCoachStub>,
    /// Everything from right after the coach chain's confirmed fields
    /// (or right after `president`, if no coach chain was found) through
    /// the end of the record. Covers the still-unconfirmed team-stats /
    /// history / tactics / palmarés region (§6.3) and the full player
    /// roster (§6.6-§6.7) — deliberately left opaque rather than guessed
    /// at, per the project's "don't force a shaky parser" guardrail.
    pub trailing_raw: Vec<u8>,
}

/// The confirmed-start of a domestic team record's coach chain
/// (PKF_FORMAT.md §6.5): the 2-byte marker, a u16 "pointer", then two
/// length-prefixed strings. Fields like `profile`/`systems`/`palmares`
/// that the override format's `Coach` has are NOT included here — their
/// presence/shape in this container format hasn't been confirmed at all
/// (§6.3's reclassification of the post-`long_name` prose into the coach
/// chain notes this region is real but not yet parsed field-by-field).
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerCoachStub {
    /// §6.5 offset 484-485 (`AA 04` = 1194 in the worked example).
    pub pointer: u16,
    /// §6.5 offset 486-497 (length-prefixed) — e.g. "Ramón Díaz".
    pub short_name: String,
    /// §6.5 offset 498+ (length-prefixed) — e.g. "Ramón Angel DIAZ".
    pub long_name: String,
}

/// Parses one domestic team record's confirmed fields out of `bytes`
/// (expected to start exactly at that record's own banner and cover the
/// whole record, e.g. one `(start, end)` pair from
/// [`find_domestic_team_records`]).
///
/// Implements PKF_FORMAT.md §6.1-§6.2 exactly (banner, the 6-byte
/// domestic-record header, then the confirmed team-info field sequence
/// through `president`), then makes a best-effort attempt to also locate
/// the confirmed coach-chain fields (§6.5). Everything after that is kept
/// as an opaque trailing blob (see `ContainerTeamRecord::trailing_raw`).
pub fn parse_team_record(bytes: &[u8], charmap: &CharMap) -> Result<ContainerTeamRecord, PcfError> {
    let mut r = Reader::new(bytes);

    r.expect_fixed(BANNER)?;
    let prefix_bytes = r.take(DOMESTIC_HEADER_PREFIX_LEN)?;
    let header_prefix = [prefix_bytes[0], prefix_bytes[1]];
    r.expect_fixed(&DOMESTIC_HEADER_TAIL)?;

    let short_name = r.string(charmap)?;
    let stadium_name = r.string(charmap)?;
    let country = r.u8()?;
    let unexplained_byte_after_country = r.u8()?;
    let long_name = r.string(charmap)?;

    let capacity = r.u24_le()?;
    r.expect_fixed(&ZERO_SEPARATOR)?;

    let standing_capacity = r.u24_le()?;
    r.expect_fixed(&ZERO_SEPARATOR)?;

    let pitch_bytes = r.take(4)?;
    let pitch_size = (
        u16::from_le_bytes([pitch_bytes[0], pitch_bytes[1]]),
        u16::from_le_bytes([pitch_bytes[2], pitch_bytes[3]]),
    );

    let founded = r.u16_le()?;
    let after_founded = r.take(2)?;
    let unexplained_bytes_after_founded = [after_founded[0], after_founded[1]];

    let members = r.u24_le()?;
    r.expect_fixed(&ZERO_SEPARATOR)?;

    let president = r.string(charmap)?;

    let rest = &bytes[r.offset()..];
    let (coach, trailing_raw) = match find_coach_stub(rest, charmap) {
        Some((end_in_rest, coach)) => (Some(coach), rest[end_in_rest..].to_vec()),
        None => (None, rest.to_vec()),
    };

    Ok(ContainerTeamRecord {
        header_prefix,
        short_name,
        stadium_name,
        country,
        long_name,
        capacity,
        standing_capacity,
        pitch_size,
        founded,
        members,
        president,
        unexplained_byte_after_country,
        unexplained_bytes_after_founded,
        coach,
        trailing_raw,
    })
}

/// Sanity cap on a coach short/long name's decoded length, to keep the
/// heuristic marker scan below from treating an implausibly long "string"
/// (i.e. a coincidental `02 02` byte pair inside unrelated binary data) as
/// a real hit. Real examples top out well under this (§6.5: 10 and 16
/// bytes respectively).
const MAX_PLAUSIBLE_NAME_LEN: usize = 64;

/// Scans `rest` for the confirmed coach-chain shape (PKF_FORMAT.md §6.5):
/// marker `02 02`, then a u16 "pointer", then two length-prefixed
/// charmap-decodable strings. Returns the first match plus the byte offset
/// (within `rest`) immediately after the second string, or `None` if
/// nothing plausible is found.
///
/// **Heuristic, by design** (§6.5 itself notes `02 02` recurs elsewhere in
/// ordinary prose and isn't a uniquely-identifying marker on its own): this
/// requires both strings to *fully* decode through the charmap (no
/// unrecognized byte) and to be a plausible name length, then takes the
/// **first** such match. That's a much stronger filter than the
/// investigation tooling's original "few unmapped chars" check now that
/// the charmap has grown to 77 confirmed pairs — a coincidental match
/// requires an accidental `02 02` byte pair whose next 2+2 bytes decode as
/// two independently plausible-length, fully-charmap-valid strings, which
/// is unlikely enough in practice that the *first* match is trusted to be
/// the real coach chain start (matching the real-name cross-check that
/// confirmed offset 482 in the worked River example — see §6.5's note that
/// 3 further `02 02` hits later in the same blob look like mid-sentence
/// prose fragments, not new coach records).
fn find_coach_stub(rest: &[u8], charmap: &CharMap) -> Option<(usize, ContainerCoachStub)> {
    if rest.len() < 2 {
        return None;
    }
    for i in 0..=rest.len() - 2 {
        if rest[i..i + 2] != COACH_MARKER {
            continue;
        }
        if let Some(hit) = try_parse_coach_at(rest, i, charmap) {
            return Some(hit);
        }
    }
    None
}

fn try_parse_coach_at(
    rest: &[u8],
    start: usize,
    charmap: &CharMap,
) -> Option<(usize, ContainerCoachStub)> {
    let mut r = Reader::new(&rest[start..]);
    r.expect_fixed(&COACH_MARKER).ok()?;
    let pointer = r.u16_le().ok()?;

    let short_name = r.string(charmap).ok()?;
    if short_name.is_empty() || short_name.len() > MAX_PLAUSIBLE_NAME_LEN {
        return None;
    }

    let long_name = r.string(charmap).ok()?;
    if long_name.is_empty() || long_name.len() > MAX_PLAUSIBLE_NAME_LEN {
        return None;
    }

    let end = start + r.offset();
    Some((
        end,
        ContainerCoachStub {
            pointer,
            short_name,
            long_name,
        },
    ))
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return out;
    }
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            out.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Locates every domestic team record's `(start, end)` byte range inside a
/// full `.PKF` container file.
///
/// Method (PKF_FORMAT.md §6.1/§8/§9, final form): every banner occurrence
/// in the WHOLE file whose header's trailing 4 bytes match the confirmed
/// domestic-record tail (`0D 02 00 00`, vs `...01` for foreign stubs) is
/// treated as a domestic team record start; each record's end is the next
/// domestic record's start (or EOF for the last one). The header's leading
/// 2 bytes vary per team and are NOT part of the match (see
/// `DOMESTIC_HEADER_TAIL`'s doc comment) — matching all 6 bytes, as an
/// earlier version of this function did, only found River (whose prefix
/// happened to be checked first) and silently hid the other 54 real
/// domestic records, confirmed by `examples/enumerate_domestic_teams.rs`.
///
/// **Does not** gate on a "stub table end" computed from the directory
/// (an even earlier version of this function did, using the last directory
/// block's last entry's `offset + length` — that assumption turned out to
/// be wrong: real data shows at least two physically separate directory
/// block clusters, one ending around file offset ~628,272 and another
/// starting around ~1,654,868, so "the last block's last entry" does not
/// mark the boundary between foreign and domestic data at all. See
/// PKF_FORMAT.md §9 for the full writeup and verification. The header-tail
/// check alone is sufficient and doesn't depend on that incorrect
/// assumption: foreign stubs and domestic records are unambiguously
/// distinguished by that one byte regardless of where in the file they
/// physically sit.
pub fn find_domestic_team_records(bytes: &[u8]) -> Vec<(usize, usize)> {
    let domestic_starts: Vec<usize> = find_all(bytes, BANNER)
        .into_iter()
        .filter(|&p| {
            let tail_start = p + BANNER.len() + DOMESTIC_HEADER_PREFIX_LEN;
            let tail_end = tail_start + DOMESTIC_HEADER_TAIL.len();
            tail_end <= bytes.len() && bytes[tail_start..tail_end] == DOMESTIC_HEADER_TAIL
        })
        .collect();

    domestic_starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = domestic_starts.get(i + 1).copied().unwrap_or(bytes.len());
            (start, end)
        })
        .collect()
}

/// Pairs a domestic team record's file-byte start offset with its parse
/// outcome — used by [`parse_pkf_container_verbose`] so callers (like
/// `examples/dump_container.rs`) can report exactly which teams parsed and
/// which didn't, and why, rather than only getting a flattened list of
/// successes.
#[derive(Debug)]
pub struct TeamParseOutcome {
    pub start_offset: usize,
    pub result: Result<ContainerTeamRecord, PcfError>,
}

/// Finds every domestic team record ([`find_domestic_team_records`]) and
/// attempts to parse each one, returning one outcome per record (success
/// or typed failure) rather than collapsing failures away.
pub fn parse_pkf_container_verbose(bytes: &[u8], charmap: &CharMap) -> Vec<TeamParseOutcome> {
    find_domestic_team_records(bytes)
        .into_iter()
        .map(|(start, end)| TeamParseOutcome {
            start_offset: start,
            result: parse_team_record(&bytes[start..end], charmap),
        })
        .collect()
}

/// Top-level convenience wrapper: finds every domestic team record and
/// parses each one, returning only the successes.
///
/// **Design choice:** a single record's parse failure (e.g. an
/// unrecognized special entry like "Jugadores Libres" that doesn't match
/// the ordinary team-info shape, or a charmap gap) does NOT abort the
/// whole container — one bad/edge-case record shouldn't make every other,
/// perfectly good team record unusable. This function's signature (a
/// single `Result` wrapping one `Vec`) has no channel to report *which*
/// records failed alongside the successes, so it never itself returns
/// `Err` for per-record issues; callers that need per-record failure
/// visibility should call [`parse_pkf_container_verbose`] instead (see
/// `examples/dump_container.rs` for exactly that).
pub fn parse_pkf_container(
    bytes: &[u8],
    charmap: &CharMap,
) -> Result<Vec<ContainerTeamRecord>, PcfError> {
    Ok(parse_pkf_container_verbose(bytes, charmap)
        .into_iter()
        .filter_map(|outcome| outcome.result.ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// A full 6-byte domestic-record header for test fixtures: an
    /// arbitrary-but-realistic 2-byte prefix (matches River's real one)
    /// plus the confirmed-constant 4-byte tail.
    fn domestic_header() -> [u8; 6] {
        let mut h = [0u8; 6];
        h[..2].copy_from_slice(&[0xE9, 0x07]);
        h[2..].copy_from_slice(&DOMESTIC_HEADER_TAIL);
        h
    }

    fn synthetic_charmap() -> CharMap {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("synthetic_map.txt");
        CharMap::load(path).expect("synthetic charmap fixture should parse")
    }

    fn confirmed_v2_charmap() -> CharMap {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("confirmed_real_map_v2.txt");
        CharMap::load(path).expect("confirmed_real_map_v2 fixture should parse")
    }

    /// Builds a syntactically-valid (though not real-content) domestic team
    /// record using only characters the synthetic charmap covers (see
    /// `fixtures/charmap/README.md`: it's built entirely from the
    /// PLAN.md-Appendix-A "Real Madrid C.F." proof plus invented filler),
    /// so unit tests don't depend on the gitignored real `.PKF` file.
    fn build_synthetic_team_record(charmap: &CharMap, trailing: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(BANNER);
        buf.extend_from_slice(&domestic_header());

        write_string(&mut buf, charmap, "Real"); // short_name
        write_string(&mut buf, charmap, "Real Madrid"); // stadium_name
        buf.push(0x03); // country
        buf.push(0xDE); // unexplained_byte_after_country
        write_string(&mut buf, charmap, "Real Madrid C.F."); // long_name

        write_u24_le(&mut buf, 76_687); // capacity
        buf.push(0x00);
        write_u24_le(&mut buf, 0); // standing_capacity
        buf.push(0x00);

        buf.extend_from_slice(&[0x46, 0x00, 0x69, 0x00]); // pitch_size (70, 105)

        buf.extend_from_slice(&1901u16.to_le_bytes()); // founded
        buf.extend_from_slice(&[0x00, 0x00]); // unexplained_bytes_after_founded

        write_u24_le(&mut buf, 63_000); // members
        buf.push(0x00);

        write_string(&mut buf, charmap, "Real Madrid"); // president

        // Coach chain (§6.5 shape): marker, pointer, short_name, long_name.
        buf.extend_from_slice(&COACH_MARKER);
        buf.extend_from_slice(&1194u16.to_le_bytes());
        write_string(&mut buf, charmap, "Real"); // coach short_name
        write_string(&mut buf, charmap, "Real Madrid"); // coach long_name

        buf.extend_from_slice(trailing);
        buf
    }

    fn write_string(buf: &mut Vec<u8>, charmap: &CharMap, s: &str) {
        let encoded = charmap.encode(s).expect("test string should be encodable");
        buf.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        buf.extend_from_slice(&encoded);
    }

    fn write_u24_le(buf: &mut Vec<u8>, v: u32) {
        buf.push((v & 0xFF) as u8);
        buf.push(((v >> 8) & 0xFF) as u8);
        buf.push(((v >> 16) & 0xFF) as u8);
    }

    // -----------------------------------------------------------------
    // Low-level unit tests (no real file needed) — header validation.
    // -----------------------------------------------------------------

    #[test]
    fn rejects_wrong_banner_with_typed_error_not_panic() {
        let charmap = synthetic_charmap();
        // Deliberately the same length as the real banner (36 bytes) but
        // wrong content, so this exercises the mismatch path rather than
        // an unrelated EOF.
        let bytes = vec![0x99u8; BANNER.len() + 8];
        let err = parse_team_record(&bytes, &charmap).unwrap_err();
        assert_eq!(err.code, "dbc_fixed_bytes_mismatch");
    }

    #[test]
    fn rejects_foreign_header_byte_with_typed_error_not_panic() {
        let charmap = synthetic_charmap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BANNER);
        // Foreign-stub marker (0x01 in the last header byte) instead of the
        // domestic 0x00 — PKF_FORMAT.md §6.1.
        bytes.extend_from_slice(&[0xE9, 0x07, 0x0D, 0x02, 0x00, 0x01]);
        let err = parse_team_record(&bytes, &charmap).unwrap_err();
        assert_eq!(err.code, "dbc_fixed_bytes_mismatch");
    }

    #[test]
    fn reports_eof_instead_of_panicking_on_truncated_record() {
        let charmap = synthetic_charmap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BANNER);
        bytes.extend_from_slice(&domestic_header());
        // Truncated right after the header — no short_name length prefix.
        let err = parse_team_record(&bytes, &charmap).unwrap_err();
        assert_eq!(err.code, "dbc_unexpected_eof");
    }

    // -----------------------------------------------------------------
    // Full confirmed-field-sequence test on a hand-built synthetic record.
    // -----------------------------------------------------------------

    #[test]
    fn parses_confirmed_team_info_fields_from_synthetic_record() {
        let charmap = synthetic_charmap();
        let trailing = [0xAA, 0xBB, 0xCC];
        let bytes = build_synthetic_team_record(&charmap, &trailing);

        let record = parse_team_record(&bytes, &charmap).expect("should parse");

        assert_eq!(record.short_name, "Real");
        assert_eq!(record.stadium_name, "Real Madrid");
        assert_eq!(record.country, 0x03);
        assert_eq!(record.unexplained_byte_after_country, 0xDE);
        assert_eq!(record.long_name, "Real Madrid C.F.");
        assert_eq!(record.capacity, 76_687);
        assert_eq!(record.standing_capacity, 0);
        assert_eq!(record.pitch_size, (70, 105));
        assert_eq!(record.founded, 1901);
        assert_eq!(record.unexplained_bytes_after_founded, [0x00, 0x00]);
        assert_eq!(record.members, 63_000);
        assert_eq!(record.president, "Real Madrid");
    }

    #[test]
    fn parses_coach_stub_when_present() {
        let charmap = synthetic_charmap();
        let trailing = [0x11, 0x22];
        let bytes = build_synthetic_team_record(&charmap, &trailing);

        let record = parse_team_record(&bytes, &charmap).expect("should parse");

        let coach = record.coach.expect("coach stub should be found");
        assert_eq!(coach.pointer, 1194);
        assert_eq!(coach.short_name, "Real");
        assert_eq!(coach.long_name, "Real Madrid");
        assert_eq!(record.trailing_raw, trailing);
    }

    #[test]
    fn coach_is_none_and_trailing_raw_is_everything_after_president_when_no_marker_found() {
        let charmap = synthetic_charmap();
        // Build a record with no coach marker at all: team-info fields
        // followed directly by arbitrary trailing bytes.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BANNER);
        bytes.extend_from_slice(&domestic_header());
        write_string(&mut bytes, &charmap, "Real");
        write_string(&mut bytes, &charmap, "Real Madrid");
        bytes.push(0x03);
        bytes.push(0xDE);
        write_string(&mut bytes, &charmap, "Real Madrid C.F.");
        write_u24_le(&mut bytes, 76_687);
        bytes.push(0x00);
        write_u24_le(&mut bytes, 0);
        bytes.push(0x00);
        bytes.extend_from_slice(&[0x46, 0x00, 0x69, 0x00]);
        bytes.extend_from_slice(&1901u16.to_le_bytes());
        bytes.extend_from_slice(&[0x00, 0x00]);
        write_u24_le(&mut bytes, 63_000);
        bytes.push(0x00);
        write_string(&mut bytes, &charmap, "Real Madrid");
        let trailing = [0x99, 0x88, 0x77, 0x66];
        bytes.extend_from_slice(&trailing);

        let record = parse_team_record(&bytes, &charmap).expect("should parse");
        assert!(record.coach.is_none());
        assert_eq!(record.trailing_raw, trailing);
    }

    // -----------------------------------------------------------------
    // Directory-boundary / domestic-record-scanning tests.
    // -----------------------------------------------------------------

    /// Builds a minimal synthetic 38-byte directory entry (PKF_FORMAT.md
    /// §2) whose `offset`/`length` fields describe where the (fake) stub
    /// table's last referenced record ends.
    fn build_dir_entry(offset: u32, length: u32) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend_from_slice(&[0u8; 8]); // id (unused by this parser)
        e.extend_from_slice(&DIR_SIG);
        e.extend_from_slice(&[0u8; 4]); // sub (unused by this parser)
        e.extend_from_slice(&offset.to_le_bytes());
        e.extend_from_slice(&length.to_le_bytes());
        e.extend_from_slice(&1u32.to_le_bytes()); // flag
        e.push(0x04); // trailing byte
        assert_eq!(e.len(), DIR_ENTRY_LEN);
        e
    }

    #[test]
    fn find_domestic_team_records_rejects_foreign_headers_and_ignores_stray_directory_bytes() {
        let charmap = synthetic_charmap();

        // Directory-shaped filler bytes elsewhere in the file must not
        // confuse banner/header scanning (regression test for a real bug:
        // an earlier version of this function tried to use a directory
        // entry's offset/length fields to compute a "stub table end" floor
        // and gate banner scanning on it, which turned out to rest on a
        // false assumption -- see PKF_FORMAT.md §9 -- and silently excluded
        // real domestic records. The current implementation never looks at
        // directory bytes at all, so this is now purely a check that they
        // don't accidentally get matched as a banner or header).
        let mut bytes = vec![0u8; 26]; // filler before the directory-shaped bytes
        bytes.extend_from_slice(&build_dir_entry(64, 36));
        bytes.resize(100, 0xFF); // more filler

        // A foreign-club stub record (header ends in 0x01) -- must NOT be
        // picked up as domestic.
        let foreign_start = bytes.len();
        bytes.extend_from_slice(BANNER);
        bytes.extend_from_slice(&[0xE9, 0x07, 0x0D, 0x02, 0x00, 0x01]);
        bytes.extend_from_slice(&[0u8; 8]);

        // A real domestic record right after it.
        let domestic_start = bytes.len();
        let record = build_synthetic_team_record(&charmap, &[0xAB, 0xCD]);
        bytes.extend_from_slice(&record);

        let ranges = find_domestic_team_records(&bytes);

        assert_eq!(
            ranges,
            vec![(domestic_start, bytes.len())],
            "must find exactly the one domestic record and reject the foreign-header one \
             (foreign_start={foreign_start}), regardless of the directory-shaped filler bytes \
             earlier in the file"
        );
    }

    #[test]
    fn find_domestic_team_records_enumerates_multiple_consecutive_teams() {
        let charmap = synthetic_charmap();
        let mut bytes = Vec::new();

        let first_start = bytes.len();
        bytes.extend_from_slice(&build_synthetic_team_record(&charmap, &[]));
        let second_start = bytes.len();
        bytes.extend_from_slice(&build_synthetic_team_record(&charmap, &[]));

        // No directory blocks at all in this file -- stub_table_end
        // defaults to 0, so scanning starts from byte 0.
        let ranges = find_domestic_team_records(&bytes);
        assert_eq!(
            ranges,
            vec![(first_start, second_start), (second_start, bytes.len())]
        );
    }

    // -----------------------------------------------------------------
    // Real-fixture-aware test, mirroring `crates/pcf-manager/src/lib.rs`'s
    // `verify_recognizes_a_real_manager_exe_if_the_user_has_supplied_one`
    // and `tests/tests/round_trip.rs`'s golden-fixture harness: never fail
    // just because the real, gitignored `.PKF` isn't present on this
    // machine (CI never has it). If it *is* present, actually parse the
    // real River record and assert real, independently-checkable facts
    // (PKF_FORMAT.md §6.2/§6.5).
    // -----------------------------------------------------------------

    #[test]
    fn parses_real_river_record_from_the_users_own_pkf_if_present() {
        let path = Path::new("/c/PCF6AR/DBDAT/EQ003003.PKF");
        let Ok(bytes) = std::fs::read(path) else {
            println!(
                "{} not found -- skipping, this test only runs meaningfully on a machine with \
                 the user's own legally-owned copy of the game (never committed, see \
                 fixtures/PKF_FORMAT.md)",
                path.display()
            );
            return;
        };

        let charmap = confirmed_v2_charmap();
        let ranges = find_domestic_team_records(&bytes);
        assert!(
            !ranges.is_empty(),
            "expected at least one domestic team record in the real file"
        );

        let river = ranges
            .into_iter()
            .find_map(|(start, end)| {
                let record = parse_team_record(&bytes[start..end], &charmap).ok()?;
                (record.short_name == "River").then_some(record)
            })
            .expect("expected to find a domestic team record decoding short_name == \"River\"");

        // PKF_FORMAT.md §6.2's cited, independently-verifiable real-world
        // facts for this exact record.
        assert_eq!(river.stadium_name, "Antonio Vespucio Liberti");
        assert_eq!(river.capacity, 76_687);
        assert_eq!(river.founded, 1901);
        assert_eq!(river.long_name, "Club Atlético River Plate");
        assert_eq!(river.president, "Alfredo Angel Dávicce");

        let coach = river
            .coach
            .expect("expected the confirmed coach-chain marker to be found (§6.5)");
        assert_eq!(coach.short_name, "Ramón Díaz");
        assert_eq!(coach.long_name, "Ramón Angel DIAZ");
    }
}
