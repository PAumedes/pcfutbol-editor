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
//! cited in `fixtures/PKF_FORMAT.md` §6.1/§6.2/§6.5/§6.6. The full
//! player-roster layout **is** confirmed (§6.6-§6.7: all 27 of River's real
//! players decode end-to-end, byte-for-byte matching `dbc.rs::read_player`'s
//! field order from `slot` onward) — [`parse_player_record`] and
//! [`parse_player_roster`] implement it, feeding [`ContainerTeamRecord::players`].
//! Only a small remainder after the best-effort roster walk (normally just
//! the 1-byte end-of-roster terminator, per §6.6) is kept as an opaque
//! `trailing_raw` blob.
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

/// Single-byte player-record marker (PKF_FORMAT.md §6.6) — matches the
/// override format's `PLAYER_MARKER` byte value, though the fields that
/// follow it differ (see [`parse_player_record`]'s doc comment).
const PLAYER_MARKER: u8 = 0x01;

/// Upper bound on how far [`find_short_name_start`] will search past
/// `number` for a plausible `short_name` length-prefix, before giving up.
/// Matches `examples/investigate_player_layout.rs`'s own cap — real data
/// never needed more than 3 bytes of gap (§6.6).
const GAP_SEARCH_MAX: usize = 8;

/// Plausible length range for a player's `short_name`/`long_name` string,
/// used both by the gap-search heuristic below and as a sanity bound.
/// PKF_FORMAT.md §6.6 doesn't cite an exact upper bound; 64 is a generous
/// margin above the longest real names seen (River's longest is well under
/// 20 bytes) while still ruling out a false-positive length prefix landing
/// on unrelated binary data.
const PLAYER_STRING_LEN_MAX: usize = 64;

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
    /// The full player roster (PKF_FORMAT.md §6.6-§6.7), parsed via
    /// [`parse_player_roster`] starting right after the coach chain's
    /// confirmed fields (or right after `president`, if no coach chain was
    /// found). **Not** the still-unconfirmed team-stats/history/tactics
    /// region (§6.3) that sits *before* the player roster in the real byte
    /// layout — that region is not parsed by this module at all and its
    /// bytes are silently skipped over by the coach-marker heuristic scan
    /// (see `find_coach_stub`'s doc comment) the same way they always were.
    /// Degrades gracefully rather than failing the whole team: if a player
    /// record fails to parse partway through the roster (not seen on the
    /// real 27-player River roster, but not proven impossible for some
    /// other team's data), whatever players parsed successfully before the
    /// failure are kept here rather than discarding the whole roster.
    pub players: Vec<ContainerPlayerRecord>,
    /// Bytes left over after the best-effort player-roster walk above
    /// stopped (see `players`' doc comment). Normally just the 1-byte
    /// end-of-roster terminator noted in §6.7's worked example, but if
    /// roster-walking stopped early because a player record failed to
    /// parse, this holds that player's bytes onward instead — kept
    /// verbatim rather than silently dropped, so no data disappears even
    /// on an edge case not seen in the River sample. (Previously this field
    /// held the *entire* unparsed remainder, back when player-roster
    /// parsing wasn't implemented yet — see PKF_FORMAT.md §6.6-§6.7 and
    /// this module's own history for that earlier placeholder meaning.)
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

/// One player's confirmed fields, decoded from a single container
/// player-record (PKF_FORMAT.md §6.6, very-high-confidence: all 27 of
/// River's real players decode end-to-end matching the real 1998-99 squad).
///
/// **Not** `pcf_model::Player`: this container's player-record framing has
/// two confirmed, real structural differences from the override `.dbc`
/// format (the `pointer`/`gap` fields before `short_name`, and the fact
/// `short_name`/`long_name` still come before `slot`/`origin`/etc, matching
/// `dbc.rs::read_player`'s ACTUAL field order rather than the naive "struct
/// literal order" a first reading of `pcf_model::Player` might suggest) —
/// see this module's own top-of-file doc comment for why `ContainerTeamRecord`
/// and `ContainerCoachStub` are similarly local, non-reused types. `roles`,
/// `skin`, `hair`, and `demarcation` are kept as raw bytes rather than
/// `pcf_model`'s enums for the same reason `ContainerTeamRecord` keeps
/// `country`/`pitch_size` raw: this container format's exact semantics for
/// an out-of-range byte aren't guaranteed identical to the override
/// format's strict enum validation, so applying that validation here could
/// turn a merely-unusual-but-real byte into a spurious hard parse failure.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerPlayerRecord {
    /// Large, team-external, load-order-style value (§6.6: confirmed real
    /// examples range from ~6400 to ~25600) — not the same kind of small
    /// "pointer" as `ContainerCoachStub::pointer`.
    pub pointer: u16,
    /// Shirt number (dorsal).
    pub number: u8,
    /// The variable-length, still-unexplained gap between `number` and
    /// `short_name`'s own length prefix (§6.6): **3 bytes** for the very
    /// first player in a roster, **0 bytes** for every other player
    /// confirmed so far. Kept verbatim rather than discarded, since its
    /// meaning isn't resolved.
    pub gap: Vec<u8>,
    pub short_name: String,
    pub long_name: String,
    pub slot: u8,
    /// `0` = continues (per override format semantics), per §6.6.
    pub origin: u8,
    /// Each byte confirmed `0x00..=0x12` on real data, matching the
    /// override format's `Role` enum range — kept raw, see this struct's
    /// own doc comment for why.
    pub roles: [u8; 6],
    pub nationality: u8,
    /// Confirmed `1..=3` on real data (matches override `Skin` enum range).
    pub skin: u8,
    /// Confirmed `1..=6` on real data (matches override `Hair` enum range).
    pub hair: u8,
    /// Confirmed `0..=3` on real data (matches override `Demarcation` enum
    /// range) — §6.6's `attrs.portero` cross-check confirms `0` = keeper.
    pub demarcation: u8,
    pub birth_day: u8,
    pub birth_month: u8,
    pub birth_year: u16,
    pub height_cm: u8,
    pub weight_kg: u8,
    pub birth_country: u8,
    /// `birthplace` and the 9 fields below it are decoded **losslessly but
    /// tolerantly** (see [`decode_lossy`]), unlike `short_name`/`long_name`
    /// above: §6.6 only confirmed those two identity fields decode with
    /// zero unmapped bytes across all 27 real River players. These 10
    /// free-text/biographical fields haven't been individually verified the
    /// same way, and this project's own charmap-provenance guardrails
    /// forbid inferring new charmap pairs from exactly this kind of
    /// biographical prose — so any byte the current charmap doesn't cover
    /// renders as `'\u{FFFD}'` (Unicode replacement character) rather than
    /// hard-failing the whole player record over a field nothing else here
    /// depends on structurally.
    pub birthplace: String,
    pub debut_club: String,
    pub international: String,
    pub profile: String,
    pub characteristics: String,
    pub palmares: String,
    pub internationality: String,
    pub anecdotes: String,
    pub last_season: String,
    pub career: String,
    /// `velocidad, resistencia, agresividad, calidad, remate, regate, pase,
    /// tiro, entradas, portero`, in that exact order (§6.6) — each byte
    /// confirmed independently in `0..=99` on real data.
    pub attrs: [u8; 10],
}

/// Everything a player record carries from `short_name` onward — i.e. the
/// whole confirmed fixed-field sequence, minus the `pointer`/`number`/`gap`
/// fields that come before it (see [`parse_player_record`]'s doc comment
/// for why those three are handled separately).
struct PlayerBody {
    short_name: String,
    long_name: String,
    slot: u8,
    origin: u8,
    roles: [u8; 6],
    nationality: u8,
    skin: u8,
    hair: u8,
    demarcation: u8,
    birth_day: u8,
    birth_month: u8,
    birth_year: u16,
    height_cm: u8,
    weight_kg: u8,
    birth_country: u8,
    birthplace: String,
    debut_club: String,
    international: String,
    profile: String,
    characteristics: String,
    palmares: String,
    internationality: String,
    anecdotes: String,
    last_season: String,
    career: String,
    attrs: [u8; 10],
}

/// `true` if `body`'s enum-shaped raw bytes all land in their confirmed
/// real-data ranges (§6.6) — used both by [`find_short_name_start`] (to
/// reject a `short_name` candidate that happens to decode but doesn't lead
/// into a real player body) and by [`find_first_player_record`] (to
/// distinguish a real player-record hit from a coincidental `0x01` byte
/// inside the still-unparsed team-stats/tactics/coach-continuation region
/// that precedes the roster, PKF_FORMAT.md §6.3/§6.5 — the same kind of
/// false-positive risk `find_coach_stub`'s own doc comment describes for
/// `0x02 0x02`).
fn plausible_body_ranges(body: &PlayerBody) -> bool {
    body.roles.iter().all(|&b| b <= 0x12)
        && (1..=3).contains(&body.skin)
        && (1..=6).contains(&body.hair)
        && (0..=3).contains(&body.demarcation)
        && body.attrs.iter().all(|&b| b <= 99)
}

/// Decodes one byte through `charmap`, falling back to `'\u{FFFD}'`
/// (Unicode replacement character) instead of propagating
/// `charmap_unknown_byte` — the per-byte building block for
/// [`read_lossy_string`].
fn decode_lossy_byte(charmap: &CharMap, byte: u8) -> char {
    charmap
        .decode(std::slice::from_ref(&byte))
        .ok()
        .and_then(|s| s.chars().next())
        .unwrap_or('\u{FFFD}')
}

/// Decodes a whole byte slice the same tolerant way as
/// [`decode_lossy_byte`], one byte at a time.
fn decode_lossy(charmap: &CharMap, bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| decode_lossy_byte(charmap, b))
        .collect()
}

/// Reads a length-prefixed string the same wire shape as
/// `crate::cursor::Reader::string`, but tolerating charmap gaps (see
/// [`decode_lossy`]) instead of hard-failing on the first unrecognized
/// byte. Still propagates a typed error on EOF (a truncated length prefix
/// or body is a real structural problem, unlike an unmapped glyph).
fn read_lossy_string(r: &mut Reader, charmap: &CharMap) -> Result<String, PcfError> {
    let len = r.u16_le()? as usize;
    let bytes = r.take(len)?;
    Ok(decode_lossy(charmap, bytes))
}

/// Parses everything from `short_name` onward (PKF_FORMAT.md §6.6), i.e.
/// the part of a player record that's byte-for-byte identical to
/// `dbc.rs::read_player`'s field order — reusing `crate::cursor::Reader`'s
/// helpers directly, since the wire shape is confirmed identical there.
/// Returns the parsed fields plus how many bytes of `bytes` were consumed.
///
/// **`short_name`/`long_name` decode strictly** (via `Reader::string`,
/// hard-erroring on any unmapped byte) — §6.6 confirmed zero unmapped
/// bytes across all 27 real River players for these two identity fields,
/// so a failure here is a genuine signal something's misaligned (used by
/// [`find_short_name_start`]'s gap search to reject a wrong gap length).
/// `birthplace` and the 9 free-text fields after it decode **tolerantly**
/// (via [`read_lossy_string`]) instead — see [`ContainerPlayerRecord`]'s
/// doc comment on those fields for why. This was found and fixed by
/// running against the real file: a strict decode of every string field
/// (the first version of this function) failed on real data because those
/// free-text/biographical fields contain bytes outside the current
/// 82-pair charmap that this project's guardrails don't allow inferring
/// from biographical prose — exactly the kind of bug the rest of this
/// module's history (PKF_FORMAT.md §8.1/§8.2) says only real-file testing
/// catches.
fn parse_player_body(bytes: &[u8], charmap: &CharMap) -> Result<(PlayerBody, usize), PcfError> {
    let mut r = Reader::new(bytes);

    let short_name = r.string(charmap)?;
    // `long_name` is decoded tolerantly, NOT strictly like `short_name` --
    // found and fixed by running against the real file: real players' full
    // legal names (e.g. "Jorge Daniel MART[byte]NEZ") turned out to contain
    // at least one accented-uppercase byte not yet in the 82-pair charmap
    // (uppercase Í, plausibly -- corroborated by exactly one clean,
    // byte-boundary-isolated citation; not confirmed with the same rigor
    // `fixtures/charmap/confirmed_real_map_v2.txt` requires, so not merged
    // there). Requiring `long_name` to decode strictly would block roster
    // parsing for every player whose full name happens to need an
    // uncommon accented capital the smaller name-only corpus never
    // exercised, which defeats the purpose of parsing 27 players end to
    // end. `short_name` staying strict is what `find_short_name_start`'s
    // gap-search rejection and the "zero unmapped bytes" confidence from
    // §6.6 actually depend on; `long_name`'s exact byte-perfect glyphs
    // don't gate anything structural.
    let long_name = read_lossy_string(&mut r, charmap)?;

    let slot = r.u8()?;
    let origin = r.u8()?;

    let mut roles = [0u8; 6];
    for role in roles.iter_mut() {
        *role = r.u8()?;
    }

    let nationality = r.u8()?;
    let skin = r.u8()?;
    let hair = r.u8()?;
    let demarcation = r.u8()?;

    let birth_bytes = r.take(4)?;
    let birth_day = birth_bytes[0];
    let birth_month = birth_bytes[1];
    let birth_year = u16::from_le_bytes([birth_bytes[2], birth_bytes[3]]);

    let height_cm = r.u8()?;
    let weight_kg = r.u8()?;
    let birth_country = r.u8()?;
    let birthplace = read_lossy_string(&mut r, charmap)?;

    let debut_club = read_lossy_string(&mut r, charmap)?;
    let international = read_lossy_string(&mut r, charmap)?;
    let profile = read_lossy_string(&mut r, charmap)?;
    let characteristics = read_lossy_string(&mut r, charmap)?;
    let palmares = read_lossy_string(&mut r, charmap)?;
    let internationality = read_lossy_string(&mut r, charmap)?;
    let anecdotes = read_lossy_string(&mut r, charmap)?;
    let last_season = read_lossy_string(&mut r, charmap)?;
    let career = read_lossy_string(&mut r, charmap)?;

    let attr_bytes = r.take(10)?;
    let mut attrs = [0u8; 10];
    attrs.copy_from_slice(attr_bytes);

    let body = PlayerBody {
        short_name,
        long_name,
        slot,
        origin,
        roles,
        nationality,
        skin,
        hair,
        demarcation,
        birth_day,
        birth_month,
        birth_year,
        height_cm,
        weight_kg,
        birth_country,
        birthplace,
        debut_club,
        international,
        profile,
        characteristics,
        palmares,
        internationality,
        anecdotes,
        last_season,
        career,
        attrs,
    };
    Ok((body, r.offset()))
}

/// Searches forward from `gap_start` (the position right after a player
/// record's `number` byte) for the first offset where the *entire* rest of
/// a player record — `short_name` onward, via [`parse_player_body`] — both
/// parses successfully and lands every enum-shaped byte in its confirmed
/// real-data range ([`plausible_body_ranges`]).
///
/// Re-derives `examples/investigate_player_layout.rs`'s gap-search
/// algorithm (PKF_FORMAT.md §6.6: the gap is genuinely variable — 3 bytes
/// for the very first player in a roster, 0 bytes for every other one
/// seen — so rather than assume a fixed width, this tries every candidate
/// offset in turn and accepts the first plausible one), but validates the
/// *whole* downstream structure rather than just checking that
/// `short_name`'s own bytes happen to decode. That stronger check matters
/// now that the charmap covers most of the byte range (82 confirmed
/// pairs): a short run of essentially arbitrary bytes at the wrong
/// (too-small) candidate gap length can easily happen to charmap-decode
/// without error even though it isn't really a string field at all — this
/// was found and fixed by running against the real file (River's true
/// gap=3 first-player record was being misidentified as gap=0 or gap=1
/// because those wrong offsets *also* decoded to *something*, only failing
/// later once the misaligned `long_name`/enum fields were reached).
/// Requiring the full body to parse AND satisfy [`plausible_body_ranges`]
/// rules that out.
fn find_short_name_start(bytes: &[u8], gap_start: usize, charmap: &CharMap) -> Option<usize> {
    for gap_len in 0..=GAP_SEARCH_MAX {
        let probe = gap_start + gap_len;
        if probe + 2 > bytes.len() {
            break;
        }
        // Cheap sanity check before attempting the (relatively expensive)
        // full-body trial parse below: a plausible `short_name` length
        // prefix should be small. This also guards against a `u16` LE
        // value that's technically in-bounds but absurdly large coinciding
        // with a wrong gap length.
        let len = u16::from_le_bytes([bytes[probe], bytes[probe + 1]]) as usize;
        if !(1..=PLAYER_STRING_LEN_MAX).contains(&len) {
            continue;
        }
        if let Ok((body, _consumed)) = parse_player_body(&bytes[probe..], charmap) {
            if plausible_body_ranges(&body) {
                return Some(probe);
            }
        }
    }
    None
}

/// Parses one player record out of `bytes` (expected to start exactly at
/// that player's own `0x01` marker byte, and to extend at least to the end
/// of that one record — trailing bytes belonging to later records are
/// fine, and are simply not consumed).
///
/// Implements PKF_FORMAT.md §6.6's confirmed layout: `marker` + `pointer` +
/// `number`, then the variable-length `gap` (see [`find_short_name_start`]),
/// then everything from `short_name` onward ([`parse_player_body`]).
///
/// Returns the parsed record plus how many bytes of `bytes` it consumed, so
/// [`parse_player_roster`] can find the next record.
pub fn parse_player_record(
    bytes: &[u8],
    charmap: &CharMap,
) -> Result<(ContainerPlayerRecord, usize), PcfError> {
    let mut r = Reader::new(bytes);

    r.expect_fixed(&[PLAYER_MARKER])?;
    let pointer = r.u16_le()?;
    let number = r.u8()?;

    let gap_start = r.offset();
    let short_name_start = find_short_name_start(bytes, gap_start, charmap).ok_or_else(|| {
        PcfError::new(
            "container_player_gap_not_found",
            format!(
                "no plausible short_name length-prefix found within {GAP_SEARCH_MAX} bytes \
                 after offset {gap_start}"
            ),
        )
        .with_context(format!("offset={gap_start}"))
    })?;
    let gap = bytes[gap_start..short_name_start].to_vec();

    // Already proven to parse successfully by `find_short_name_start`'s own
    // trial parse above; re-running it here (rather than threading the
    // already-parsed `PlayerBody` through) keeps this function's control
    // flow simple at the cost of one redundant parse per record, which is
    // negligible at this data size (a few hundred bytes).
    let (body, body_len) = parse_player_body(&bytes[short_name_start..], charmap)?;

    let record = ContainerPlayerRecord {
        pointer,
        number,
        gap,
        short_name: body.short_name,
        long_name: body.long_name,
        slot: body.slot,
        origin: body.origin,
        roles: body.roles,
        nationality: body.nationality,
        skin: body.skin,
        hair: body.hair,
        demarcation: body.demarcation,
        birth_day: body.birth_day,
        birth_month: body.birth_month,
        birth_year: body.birth_year,
        height_cm: body.height_cm,
        weight_kg: body.weight_kg,
        birth_country: body.birth_country,
        birthplace: body.birthplace,
        debut_club: body.debut_club,
        international: body.international,
        profile: body.profile,
        characteristics: body.characteristics,
        palmares: body.palmares,
        internationality: body.internationality,
        anecdotes: body.anecdotes,
        last_season: body.last_season,
        career: body.career,
        attrs: body.attrs,
    };
    Ok((record, short_name_start + body_len))
}

/// Scans `bytes` for the first `0x01` byte that starts a *fully* parseable
/// player record — i.e. the real start of the player roster, which per
/// §6.3/§6.5 does NOT sit immediately after `ContainerCoachStub`'s two
/// parsed strings (there's a large still-unconfirmed
/// team-stats/tactics/coach-continuation region in between). Returns the
/// byte offset of that marker, or `None` if nothing plausible is found
/// anywhere in `bytes`.
///
/// `parse_player_record` succeeding is itself already a strong plausibility
/// signal: its own gap-search ([`find_short_name_start`]) requires the
/// *entire* downstream field structure to parse AND satisfy
/// [`plausible_body_ranges`], not just that some bytes happen to
/// charmap-decode — so a coincidental `0x01` inside the unconfirmed region
/// preceding the roster would have to accidentally produce a
/// fully-structured, range-valid player record to be mistaken for a real
/// one here, the same low-probability false-positive risk
/// `find_coach_stub`'s own doc comment accepts for `0x02 0x02`.
fn find_first_player_record(bytes: &[u8], charmap: &CharMap) -> Option<usize> {
    for i in 0..bytes.len() {
        if bytes[i] != PLAYER_MARKER {
            continue;
        }
        if parse_player_record(&bytes[i..], charmap).is_ok() {
            return Some(i);
        }
    }
    None
}

/// Walks consecutive `0x01`-marker-delimited player records starting
/// exactly at `bytes[0]` (expected to already be a confirmed-plausible
/// first player marker — see [`find_first_player_record`]) through to the
/// end of `bytes`, re-deriving `examples/investigate_player_layout.rs`'s
/// own record-boundary walk (PKF_FORMAT.md §6.6-§6.7), which validated this
/// exact approach end-to-end on all 27 real River players with zero
/// drift/slack across the whole roster.
///
/// Stops (without erroring) as soon as the next byte isn't `0x01` — per
/// §6.7, the real roster's last player is immediately followed by a single
/// `0x00` end-of-roster terminator, not another marker.
pub fn parse_player_roster(
    bytes: &[u8],
    charmap: &CharMap,
) -> Result<Vec<ContainerPlayerRecord>, PcfError> {
    let mut players = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos] == PLAYER_MARKER {
        let (player, consumed) = parse_player_record(&bytes[pos..], charmap)?;
        pos += consumed;
        players.push(player);
    }
    Ok(players)
}

/// Best-effort roster walk for [`parse_team_record`]'s use, given `bytes`
/// already starting exactly at the roster's confirmed-real first marker
/// (from [`find_first_player_record`], called by the caller BEFORE the
/// coach-marker search — see `parse_team_record`'s own comment on why that
/// order matters for Vélez-shaped records).
///
/// Degrades gracefully instead of failing the whole team record: if the
/// roster walk fails partway through (a single record errors out — not seen
/// on the real 27-player River roster, but not proven impossible for some
/// other team's data), whatever parsed successfully before the failure is
/// kept, and everything from that failing record onward is returned as the
/// second tuple element instead of being discarded.
fn walk_player_roster_from(
    bytes: &[u8],
    charmap: &CharMap,
) -> (Vec<ContainerPlayerRecord>, Vec<u8>) {
    let mut players = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos] == PLAYER_MARKER {
        match parse_player_record(&bytes[pos..], charmap) {
            Ok((player, consumed)) => {
                players.push(player);
                pos += consumed;
            }
            Err(_) => break,
        }
    }
    (players, bytes[pos..].to_vec())
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

    // Locate the player roster's real start FIRST, then only search for the
    // coach marker in the region strictly before it (see `find_coach_stub`'s
    // doc comment and PKF_FORMAT.md's Vélez UPDATE for why the search order
    // matters). `find_first_player_record`'s own trial-parse validation
    // (full downstream structure + every enum byte in range) makes it a far
    // more reliable anchor than `find_coach_stub`'s "first `02 02` match"
    // heuristic, which was found to false-positive on ordinary Spanish prose
    // ("...dor lo convocó...", "...ra integrar el pl...") sitting INSIDE a
    // player's own free-text biography fields for at least one real team
    // (Vélez) -- unlike River, where the real coach marker happens to be the
    // very first `02 02` byte pair anywhere in `rest`, Vélez's record has no
    // plausible coach chain at all before its player roster begins, and the
    // old code searched the *entire* remainder (including all player data)
    // for a coach marker, matching that prose fragment deep inside the
    // roster and silently discarding every real player before it. Bounding
    // the coach search to `rest[..roster_start]` makes that false match
    // impossible: a `02 02` byte pair occurring inside player data can never
    // be mistaken for the coach chain again, and the roster walk always
    // starts at the roster's own confirmed-real first marker instead of
    // wherever a stray coach match happened to end.
    let roster_start = find_first_player_record(rest, charmap);
    let coach_search_region = match roster_start {
        Some(start) => &rest[..start],
        None => rest,
    };
    let coach_match = find_coach_stub(coach_search_region, charmap);
    let coach = coach_match.as_ref().map(|(_, coach)| coach.clone());

    let (players, trailing_raw) = match roster_start {
        Some(start) => walk_player_roster_from(&rest[start..], charmap),
        // No plausible player roster found at all: fall back to the
        // pre-roster-search behavior (mirrors the old `after_coach`
        // handling) so a coach-only record (no roster ever implemented/
        // present) still reports `trailing_raw` as everything after the
        // coach chain, not the coach chain's own bytes too.
        None => match coach_match {
            Some((end_in_region, _)) => (Vec::new(), rest[end_in_region..].to_vec()),
            None => (Vec::new(), rest.to_vec()),
        },
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
        players,
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
    // Player-record test helpers (PKF_FORMAT.md §6.6).
    // -----------------------------------------------------------------

    /// Builds one syntactically-valid synthetic player record: marker,
    /// pointer, number, an arbitrary `gap` (to exercise the gap-search
    /// logic with both zero-length and multi-byte gaps), then the
    /// confirmed fixed-field sequence. Only characters the synthetic
    /// charmap covers are used for the free-text fields (kept short and
    /// generic — not real player data).
    fn build_synthetic_player_record(charmap: &CharMap, gap: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(PLAYER_MARKER);
        buf.extend_from_slice(&6400u16.to_le_bytes()); // pointer
        buf.push(9); // number (dorsal)
        buf.extend_from_slice(gap);

        write_string(&mut buf, charmap, "Real"); // short_name
        write_string(&mut buf, charmap, "Real Madrid"); // long_name

        buf.push(1); // slot
        buf.push(0); // origin
        buf.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x12]); // roles[6], all <=0x12
        buf.push(0x20); // nationality (arbitrary)
        buf.push(2); // skin, 1..=3
        buf.push(3); // hair, 1..=6
        buf.push(1); // demarcation, 0..=3
        buf.extend_from_slice(&[16, 4, 0x69, 0x07]); // birth: day, month, year=1897 (LE)
        buf.push(180); // height_cm
        buf.push(75); // weight_kg
        buf.push(0x03); // birth_country

        write_string(&mut buf, charmap, "Real"); // birthplace
        write_string(&mut buf, charmap, "Real"); // debut_club
        write_string(&mut buf, charmap, "Real"); // international
        write_string(&mut buf, charmap, "Real"); // profile
        write_string(&mut buf, charmap, "Real"); // characteristics
        write_string(&mut buf, charmap, "Real"); // palmares
        write_string(&mut buf, charmap, "Real"); // internationality
        write_string(&mut buf, charmap, "Real"); // anecdotes
        write_string(&mut buf, charmap, "Real"); // last_season
        write_string(&mut buf, charmap, "Real"); // career

        buf.extend_from_slice(&[50, 60, 70, 80, 65, 55, 45, 40, 35, 90]); // attrs[10], all <=99

        buf
    }

    // -----------------------------------------------------------------
    // Low-level player-record unit tests.
    // -----------------------------------------------------------------

    #[test]
    fn parses_player_record_with_zero_length_gap() {
        let charmap = synthetic_charmap();
        let bytes = build_synthetic_player_record(&charmap, &[]);

        let (player, consumed) = parse_player_record(&bytes, &charmap).expect("should parse");

        assert_eq!(consumed, bytes.len());
        assert_eq!(player.pointer, 6400);
        assert_eq!(player.number, 9);
        assert!(player.gap.is_empty());
        assert_eq!(player.short_name, "Real");
        assert_eq!(player.long_name, "Real Madrid");
        assert_eq!(player.slot, 1);
        assert_eq!(player.origin, 0);
        assert_eq!(player.roles, [0x00, 0x01, 0x02, 0x03, 0x04, 0x12]);
        assert_eq!(player.nationality, 0x20);
        assert_eq!(player.skin, 2);
        assert_eq!(player.hair, 3);
        assert_eq!(player.demarcation, 1);
        assert_eq!(player.birth_day, 16);
        assert_eq!(player.birth_month, 4);
        assert_eq!(player.birth_year, 1897);
        assert_eq!(player.height_cm, 180);
        assert_eq!(player.weight_kg, 75);
        assert_eq!(player.birth_country, 0x03);
        assert_eq!(player.birthplace, "Real");
        assert_eq!(player.career, "Real");
        assert_eq!(player.attrs, [50, 60, 70, 80, 65, 55, 45, 40, 35, 90]);
    }

    #[test]
    fn parses_player_record_with_multi_byte_gap() {
        let charmap = synthetic_charmap();
        // A 3-byte gap, matching §6.6's confirmed shape for the very first
        // player in a real roster (Saccone). The gap bytes themselves are
        // arbitrary/unexplained (kept verbatim, not decoded).
        let gap = [0xAA, 0xBB, 0xCC];
        let bytes = build_synthetic_player_record(&charmap, &gap);

        let (player, consumed) = parse_player_record(&bytes, &charmap).expect("should parse");

        assert_eq!(consumed, bytes.len());
        assert_eq!(player.gap, gap);
        assert_eq!(player.short_name, "Real");
        assert_eq!(player.long_name, "Real Madrid");
    }

    #[test]
    fn rejects_wrong_player_marker_with_typed_error_not_panic() {
        let charmap = synthetic_charmap();
        let mut bytes = build_synthetic_player_record(&charmap, &[]);
        bytes[0] = 0x02; // not PLAYER_MARKER
        let err = parse_player_record(&bytes, &charmap).unwrap_err();
        assert_eq!(err.code, "dbc_fixed_bytes_mismatch");
    }

    #[test]
    fn player_record_reports_typed_error_when_no_plausible_gap_found() {
        let charmap = synthetic_charmap();
        // marker + pointer + number, then garbage that never yields a
        // plausible (length-prefix, decodable-string) pair within the
        // search window.
        let mut bytes = vec![PLAYER_MARKER];
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(1);
        bytes.extend_from_slice(&[0xFF; 32]);
        let err = parse_player_record(&bytes, &charmap).unwrap_err();
        assert_eq!(err.code, "container_player_gap_not_found");
    }

    #[test]
    fn parse_player_roster_walks_consecutive_records_and_stops_at_non_marker_byte() {
        let charmap = synthetic_charmap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&build_synthetic_player_record(&charmap, &[]));
        bytes.extend_from_slice(&build_synthetic_player_record(
            &charmap,
            &[0xAA, 0xBB, 0xCC],
        ));
        bytes.push(0x00); // end-of-roster terminator, per §6.7

        let players = parse_player_roster(&bytes, &charmap).expect("should parse both records");

        assert_eq!(players.len(), 2);
        assert!(players[0].gap.is_empty());
        assert_eq!(players[1].gap, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn find_first_player_record_skips_a_structurally_valid_but_implausible_leading_record() {
        let charmap = synthetic_charmap();
        // A fully-decodable, well-formed player record (same shape the
        // parser expects) but with an out-of-range `skin` byte -- the same
        // kind of false-positive risk `find_coach_stub`'s own doc comment
        // flags for its `0x02 0x02` marker: a byte sequence that happens to
        // parse structurally but isn't real player data (per §6.6's
        // confirmed `skin` range, `1..=3`).
        let mut bytes = build_synthetic_player_record(&charmap, &[]);
        let skin_offset = {
            // marker(1) + pointer(2) + number(1) + short_name(2+4) +
            // long_name(2+11) + slot(1) + origin(1) + roles(6) = offset of
            // nationality; skin is the next byte after that.
            1 + 2 + 1 + (2 + 4) + (2 + 11) + 1 + 1 + 6 + 1
        };
        assert_eq!(
            bytes[skin_offset], 2,
            "sanity-check the computed skin offset"
        );
        bytes[skin_offset] = 0x99; // out of the confirmed 1..=3 range

        let real_start = bytes.len();
        bytes.extend_from_slice(&build_synthetic_player_record(&charmap, &[]));

        let found = find_first_player_record(&bytes, &charmap);
        assert_eq!(found, Some(real_start));
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
    // Team-level wiring: `parse_team_record` should locate and parse the
    // player roster sitting after the coach chain, not just leave it
    // opaque.
    // -----------------------------------------------------------------

    #[test]
    fn parse_team_record_wires_up_the_player_roster_after_the_coach_chain() {
        let charmap = synthetic_charmap();
        // Simulate the real layout (§6.3/§6.5/§6.6): an unexplained region
        // between the coach chain's two parsed strings and the real player
        // roster start, which `find_first_player_record` must skip over.
        // Deliberately contains no `0x01` byte at all, so this test only
        // exercises the "skip past unrelated bytes" path, not the separate
        // "reject a structurally-valid-but-implausible 0x01" path already
        // covered by `find_first_player_record_skips_a_structurally_valid_
        // but_implausible_leading_record`.
        let unexplained_middle = vec![0x02, 0x03, 0x04];
        let mut bytes = build_synthetic_team_record(&charmap, &[]);
        bytes.extend_from_slice(&unexplained_middle);
        bytes.extend_from_slice(&build_synthetic_player_record(&charmap, &[]));
        bytes.extend_from_slice(&build_synthetic_player_record(
            &charmap,
            &[0xAA, 0xBB, 0xCC],
        ));
        bytes.push(0x00); // end-of-roster terminator

        let record = parse_team_record(&bytes, &charmap).expect("should parse");

        assert_eq!(record.players.len(), 2);
        assert_eq!(record.players[0].short_name, "Real");
        assert_eq!(record.players[1].gap, vec![0xAA, 0xBB, 0xCC]);
        assert_eq!(record.trailing_raw, vec![0x00]);
    }

    #[test]
    fn parse_team_record_leaves_players_empty_when_no_plausible_roster_found() {
        let charmap = synthetic_charmap();
        let trailing = [0x11, 0x22];
        let bytes = build_synthetic_team_record(&charmap, &trailing);

        let record = parse_team_record(&bytes, &charmap).expect("should parse");

        assert!(record.players.is_empty());
        assert_eq!(record.trailing_raw, trailing);
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

        // PKF_FORMAT.md §6.6-§6.7: the full player roster, confirmed
        // end-to-end against all 27 real 1998-99 River Plate first-team
        // players. Only club/player identifying names are asserted here
        // (short_name/long_name are factual identifiers, already publicly
        // cited in PKF_FORMAT.md itself) -- never any free-text
        // profile/career/anecdotes field.
        assert_eq!(
            river.players.len(),
            27,
            "expected the confirmed real River roster size (§6.7)"
        );

        // §6.6's worked table: the first 4 players are goalkeepers
        // (demarcation == 0) with `attrs.portero` clustered 75-90, and
        // every other player checked there is <=20.
        assert_eq!(river.players[0].short_name, "Saccone");
        assert_eq!(river.players[1].short_name, "Costanzo");
        assert_eq!(river.players[2].short_name, "Burgos");
        assert_eq!(river.players[2].long_name, "Germán Adrián Ramón BURGOS");
        assert_eq!(river.players[3].short_name, "Bonano");
        assert_eq!(river.players[3].long_name, "Roberto Oscar BONANO");
        for gk in &river.players[0..4] {
            assert_eq!(
                gk.demarcation, 0,
                "{} should be a goalkeeper",
                gk.short_name
            );
        }
        assert_eq!(
            river.players[2].attrs[9], 85,
            "Burgos' portero rating (§6.6)"
        );
        assert_eq!(
            river.players[3].attrs[9], 90,
            "Bonano's portero rating (§6.6)"
        );

        // §6.6: a handful more real, historically documented squad members,
        // at their confirmed roster positions (0-indexed: Saccone,
        // Costanzo, Burgos, Bonano, Biscay, Villalba, Acosta, Martínez,
        // Sarabia, Placente, Paz, Hernán Díaz, Berizzo, Sorín, Gómez,
        // Gallardo, Astrada, Escudero, Gancedo, Berti, Solari, Saviola,
        // Angel, Castillo, Pizzi, Rambert, Aimar).
        let sorin = &river.players[13];
        assert_eq!(sorin.short_name, "Sorín");
        let gallardo = &river.players[15];
        assert_eq!(gallardo.short_name, "Gallardo");
        let saviola = &river.players[21];
        assert_eq!(saviola.short_name, "Saviola");
        let aimar = &river.players[26];
        assert_eq!(aimar.short_name, "Aimar");
        assert_eq!(aimar.demarcation, 2, "Aimar should be a midfielder (§6.6)");
        assert_eq!(
            aimar.attrs[9], 12,
            "Aimar's portero rating, per §6.6's table"
        );

        // §6.7: the roster consumes the whole record with no drift -- only
        // the 1-byte end-of-roster terminator should be left over.
        assert_eq!(
            river.trailing_raw.len(),
            1,
            "expected only the 1-byte end-of-roster terminator left over (§6.7)"
        );
    }

    // -----------------------------------------------------------------
    // §10.4: the `0x56` charmap byte (confirmed via an unrelated external
    // corpus -- see fixtures/charmap/confirmed_real_map_v2.txt and
    // PKF_FORMAT.md §10) was the sole remaining blocker keeping San Martín
    // (SJ) from parsing (§8.3: 54/55 real domestic records). Same
    // real-fixture-optional pattern as the River test above: never fail
    // just because the real, gitignored `.PKF` isn't present.
    // -----------------------------------------------------------------

    #[test]
    fn parses_real_san_martin_sj_record_from_the_users_own_pkf_if_present() {
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

        let san_martin_sj = ranges
            .into_iter()
            .find_map(|(start, end)| {
                let record = parse_team_record(&bytes[start..end], &charmap).ok()?;
                (record.short_name == "San Martín (SJ)").then_some(record)
            })
            .expect(
                "expected to find a domestic team record decoding short_name == \
                 \"San Martín (SJ)\" -- this requires the 0x56 charmap byte (§10.4); if this \
                 fails, the charmap fix regressed",
            );

        // §10.4: the stadium name is the exact field that was blocked by the
        // unmapped 0x56 byte -- asserting it decodes cleanly (not erroring,
        // and not containing a `\u{FFFD}` lossy-decode placeholder) is the
        // real regression check here. Unlike River's independently
        // fact-checked stadium name, this specific date was not separately
        // verified against a known real-world fact for this specific club
        // (see PKF_FORMAT.md §10.4's caveat) -- 0x56='7' itself was
        // confirmed from unrelated real names (MyPa, AZ).
        assert_eq!(san_martin_sj.stadium_name, "27 de Septiembre");
        assert!(
            !san_martin_sj.players.is_empty(),
            "expected a non-empty roster (§8.4)"
        );
    }

    // -----------------------------------------------------------------
    // Real Vélez Sarsfield regression test (PKF_FORMAT.md UPDATE): the
    // user's own real record was reporting only 5 players (real 1998-99
    // Vélez squads run ~20-30) and a garbled coach name ("dor lo convocó" /
    // "ra integrar el pl" -- ordinary Spanish prose fragments, not a real
    // person's name). Root cause: `find_coach_stub`'s old "first `02 02`
    // match anywhere in the remainder" search wasn't bounded to the region
    // before the player roster, so for Vélez (whose record has no locatable
    // coach chain at all before the roster) it matched a coincidental
    // `02 02` byte pair deep inside a player's own free-text biography
    // field, consumed it as a fake "coach", and started the roster walk
    // from there -- silently discarding every real player before that
    // point (including Vélez's real, legendary goalkeeper José Luis
    // CHILAVERT). Fixed by locating the roster's real start FIRST (via
    // `find_first_player_record`'s much stronger full-structure validation)
    // and only searching for a coach marker in the bytes strictly before
    // it. Same real-fixture-optional pattern as the other real-file tests
    // above.
    // -----------------------------------------------------------------

    #[test]
    fn parses_real_velez_record_from_the_users_own_pkf_if_present() {
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

        let velez = ranges
            .into_iter()
            .find_map(|(start, end)| {
                let record = parse_team_record(&bytes[start..end], &charmap).ok()?;
                (record.short_name == "Vélez").then_some(record)
            })
            .expect("expected to find a domestic team record decoding short_name == \"Vélez\"");

        // Team-info fields were never actually broken for Vélez (only the
        // coach/roster region downstream of them) -- a couple of real,
        // independently-checkable facts as a sanity check.
        assert_eq!(velez.stadium_name, "José Amalfitani");
        assert_eq!(velez.long_name, "Club Atlético Vélez Sarsfield");
        assert_eq!(velez.founded, 1910);

        // The regression itself: a real top-flight Argentine squad has far
        // more than 5 registered players. Bug report's exact number (5) is
        // asserted as an explicit non-regression floor.
        assert!(
            velez.players.len() > 10,
            "expected a plausible full-squad player count, got {} (was 5 before the \
             coach/roster ordering fix)",
            velez.players.len()
        );

        // No plausible coach chain exists before Vélez's real roster start
        // (confirmed by hand -- the only `02 02` byte pair in that region
        // fails to parse as a real coach) -- `None` here is the honest
        // result, not a garbled/false-positive name.
        assert!(
            velez.coach.is_none(),
            "expected no coach match for Vélez (no real coach chain found before the roster); \
             got {:?}",
            velez.coach
        );

        // Real, historically documented Vélez Sarsfield 1998-99 players
        // that a correct roster walk must include: José Luis CHILAVERT
        // (Paraguayan international, one of the most famous goalkeepers in
        // the world at the time) and Ariel DE LA FUENTE, both of whom the
        // old buggy walk discarded entirely because they came before the
        // false-positive "coach" match.
        assert!(
            velez.players.iter().any(|p| p.short_name == "Chilavert"),
            "expected to find José Luis Chilavert in Vélez's real roster"
        );
        assert!(
            velez.players.iter().any(|p| p.short_name == "De la Fuente"),
            "expected to find Ariel De la Fuente in Vélez's real roster"
        );

        // Jersey number 0 (impossible for a real player) was reported as a
        // separate-looking bug but shares the same root cause: it's a real,
        // valid squad member's raw `number` byte (not a mis-decoded field
        // once the roster is correctly aligned) -- the file-wide sanity
        // pass (see `examples/investigate_velez.rs`) found `number == 0`
        // recurring across the great majority of teams' rosters, plausibly
        // representing reserve/not-yet-assigned squad members rather than a
        // parse error. Not asserted against here (it's real, not a bug),
        // but documented so a future reader doesn't reopen this as if it
        // were still open.
    }
}
