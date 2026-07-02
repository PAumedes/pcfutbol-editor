//! Third-pass investigator: walks a *real domestic team* container record
//! (as opposed to the short "foreign reference club" stubs `investigate_pkf_dir.rs`
//! already decoded) byte-by-byte, using the confirmed 37-pair charmap plus a
//! handful of *newly inferred* byte->glyph pairs this investigation pass
//! turned up (see the `EXTRA_INFERRED` table below — these are NOT yet
//! merged into `fixtures/charmap/confirmed_real_map.txt`; treat them as
//! working hypotheses, corroborated by real-world facts about River Plate
//! that would be an implausible coincidence otherwise, e.g. the founding
//! year, stadium name, and the club president's and head coach's real
//! names all decoding correctly).
//!
//! This is a prototype/investigation tool only — it does NOT feed into
//! `pcf_codec::dbc::Dbc::read`/`write` (that integration decision is
//! explicitly out of scope; see `fixtures/PKF_FORMAT.md` §6).
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example investigate_domestic_team -- [path-to-blob]
//! Defaults to `fixtures/golden/real_river_9001_container_blob.raw` if no
//! path is given (that file is gitignored/local-only; regenerate it per
//! that file's sibling `.README.md` if missing).

use std::collections::HashMap;
use std::env;
use std::fs;

const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";

/// New byte->glyph pairs inferred during this investigation pass, layered
/// on top of the 37 already-confirmed pairs. Each is corroborated by a
/// real-world fact (see `fixtures/PKF_FORMAT.md` §6 for the full writeup
/// of *why* each one is justified) rather than guessed in isolation:
///   0x17 'v' - completes "Ri_er" -> "River" (short_name), corroborated by
///              completing "Vespucio"/"Ramón Angel D_AZ" only if paired
///              with the other inferences below (self-consistent).
///   0x37 'V' - completes "Antonio _espucio Liberti", River's real stadium.
///   0x11 'p' - completes "Vesp_ucio".
///   0x31 'P' - completes "Club Atlético River _late", River's real long name.
///   0x07 'f' - completes "Al_redo Angel Dávicce", a real River Plate
///              president (1997-2001).
///   0x80 'á' - completes "D_ávicce" (accented a).
///   0x6b '=' - completes "ND,ND,ND,ND,ND=_", the manual's documented
///              career-field default suffix.
///   0x50 '0' - completes a "0,0,0,0,0====" numeric-default field seen
///              near the coach chain.
///   0x0c 'm' - completes "Ra_món" -> "Ramón" (coach short_name).
///   0x92 'ó' - completes "Ram_ón".
///   0x8c 'í' - completes "D_íaz" -> "Díaz".
///   0x3b 'Z' - completes "Ramón Angel DIA_Z" (coach long_name, surname
///              capitalized).
///   0x0b 'j' - completes "Ale_jandro" (first player's long_name).
/// Real head coach of River Plate in 1998 was Ramón Ángel Díaz -- these
/// pairs make his short_name/long_name decode exactly, independently of
/// the president finding, which is strong cross-corroboration.
const EXTRA_INFERRED: &[(u8, char)] = &[
    (0x17, 'v'),
    (0x37, 'V'),
    (0x11, 'p'),
    (0x31, 'P'),
    (0x07, 'f'),
    (0x80, 'á'),
    (0x6b, '='),
    (0x50, '0'),
    (0x0c, 'm'),
    (0x92, 'ó'),
    (0x8c, 'í'),
    (0x3b, 'Z'),
    (0x0b, 'j'),
];

fn load_lossy_map(path: &str) -> HashMap<u8, char> {
    let contents = fs::read_to_string(path).expect("failed to read charmap");
    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((hex, rest)) = line.split_once('\t') {
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                let ch = if rest == "\\s" {
                    ' '
                } else {
                    rest.chars().next().unwrap_or('?')
                };
                map.insert(byte, ch);
            }
        }
    }
    for &(b, c) in EXTRA_INFERRED {
        map.insert(b, c);
    }
    map
}

fn lossy_decode(map: &HashMap<u8, char>, bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| map.get(b).copied().unwrap_or('.'))
        .collect()
}

/// Minimal read cursor: NOT the real `pcf_codec::cursor::Reader` (this
/// example intentionally stays decoupled from crate-internal, non-`pub`
/// helpers) -- just enough to walk the fields this investigation confirmed.
struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn u8(&mut self) -> u8 {
        let v = self.data[self.pos];
        self.pos += 1;
        v
    }
    fn take(&mut self, n: usize) -> &'a [u8] {
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        s
    }
    fn u16_le(&mut self) -> u16 {
        let b = self.take(2);
        u16::from_le_bytes([b[0], b[1]])
    }
    fn u24_le(&mut self) -> u32 {
        let b = self.take(3);
        u32::from_le_bytes([b[0], b[1], b[2], 0])
    }
    /// Length-prefixed (u16 LE) charmap string -- same wire shape as
    /// `pcf_codec::cursor::Reader::string`, confirmed to hold for
    /// short_name/stadium_name/long_name/president in this container too.
    fn string(&mut self, map: &HashMap<u8, char>) -> String {
        let len = self.u16_le() as usize;
        let bytes = self.take(len);
        lossy_decode(map, bytes)
    }
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "fixtures/golden/real_river_9001_container_blob.raw".to_string());
    let charmap_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("charmap")
        .join("confirmed_real_map.txt");
    let map = load_lossy_map(charmap_path.to_str().unwrap());

    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "failed to read {path} ({e}) -- this is a local, gitignored fixture; see \
                 fixtures/golden/real_river_9001_container_blob.README.md to regenerate it"
            );
            std::process::exit(1);
        }
    };
    println!("file: {path} ({} bytes)", bytes.len());

    if !bytes.starts_with(BANNER) {
        eprintln!("warning: input does not start with the expected banner");
    }

    let mut c = Cur::new(&bytes);
    c.take(BANNER.len());

    // Post-banner header. Confirmed shape: `E9 07 0D 02 00 00` then the
    // short_name string's own length prefix. Byte 5 (0-indexed within this
    // 6-byte header) distinguishes domestic (`00`) from foreign-stub (`01`)
    // records -- see PKF_FORMAT.md §3-4.
    let header = c.take(6);
    println!("post-banner header: {header:02x?}");
    let is_domestic = header == [0xe9, 0x07, 0x0d, 0x02, 0x00, 0x00];
    if !is_domestic {
        println!("note: header doesn't match the expected domestic-record shape exactly");
    }

    // --- Team info (field order matches pcf_model::Team / dbc.rs::read_team
    // positionally, up through `president`; see PKF_FORMAT.md §6 for the
    // byte-offset evidence backing each field). ---
    let short_name = c.string(&map);
    let stadium_name = c.string(&map);
    let country = c.u8();
    let mystery1 = c.u8(); // unexplained; positionally where override's single `country` byte alone would end
    let long_name = c.string(&map);

    let capacity = c.u24_le();
    let cap_sep = c.u8();
    let standing_capacity = c.u24_le();
    let standing_sep = c.u8();

    let pitch = c.take(4);
    let pitch_a = u16::from_le_bytes([pitch[0], pitch[1]]);
    let pitch_b = u16::from_le_bytes([pitch[2], pitch[3]]);

    let founded = c.u16_le();
    let mystery2 = c.take(2);

    let members = c.u24_le();
    let members_sep = c.u8();

    let president = c.string(&map);

    println!("\n--- team info (through president) ---");
    println!("short_name       = {short_name:?}");
    println!("stadium_name     = {stadium_name:?}");
    println!("country byte     = {country:#04x}");
    println!("mystery1 byte    = {mystery1:#04x}  (unexplained extra byte, see §6)");
    println!("long_name        = {long_name:?}");
    println!("capacity         = {capacity} (sep={cap_sep:#04x})");
    println!("standing_capacity= {standing_capacity} (sep={standing_sep:#04x})");
    println!("pitch dims       = ({pitch_a}, {pitch_b})  raw={pitch:02x?}");
    println!("founded          = {founded}");
    println!("mystery2 bytes   = {mystery2:02x?}  (unexplained, see §6)");
    println!("members          = {members} (sep={members_sep:#04x})");
    println!("president        = {president:?}");
    println!(
        "\ncursor position after president: {} (0x{:x})",
        c.pos, c.pos
    );

    // --- Coach chain heuristic scan ---
    // Confirmed anchor (see §6): a 2-byte marker `02 02` (NOT the
    // override-format's single `0x02` COACH_MARKER), a 2-byte pointer, then
    // short_name/long_name strings. Search the whole blob for this shape and
    // print the first few plausible hits.
    println!("\n--- coach-marker heuristic scan (marker `02 02` + u16 pointer + string) ---");
    let mut coach_hits = 0;
    for i in 0..bytes.len().saturating_sub(8) {
        if bytes[i] == 0x02 && bytes[i + 1] == 0x02 {
            let ptr = u16::from_le_bytes([bytes[i + 2], bytes[i + 3]]);
            let len = u16::from_le_bytes([bytes[i + 4], bytes[i + 5]]) as usize;
            if len == 0 || len > 40 || i + 6 + len > bytes.len() {
                continue;
            }
            let text = lossy_decode(&map, &bytes[i + 6..i + 6 + len]);
            // Heuristic: plausible name text has few '.' (unmapped) chars.
            let unmapped = text.chars().filter(|&ch| ch == '.').count();
            if unmapped * 3 > len {
                continue;
            }
            println!("  offset={i:#06x} ptr={ptr} len={len} text={text:?}");
            coach_hits += 1;
            if coach_hits >= 5 {
                break;
            }
        }
    }
    if coach_hits == 0 {
        println!("  (no plausible hits found)");
    }

    // --- Player-marker heuristic scan ---
    // Heuristic: `0x01` + u16 LE "pointer"-shaped value + a plausible dorsal
    // number (1-40). This under-counts (see §6 -- it misses at least the
    // first player, confirmed by manual tracing to start much earlier), but
    // gives a rough spacing/count sanity check.
    println!("\n--- player-marker heuristic scan (`0x01` + u16<2000 + dorsal 1..=40) ---");
    let mut cands = Vec::new();
    for i in 0..bytes.len().saturating_sub(4) {
        if bytes[i] != 0x01 {
            continue;
        }
        let ptr = u16::from_le_bytes([bytes[i + 1], bytes[i + 2]]);
        let dorsal = bytes[i + 3];
        if (1..=2000).contains(&ptr) && (1..=40).contains(&dorsal) {
            cands.push((i, ptr, dorsal));
        }
    }
    println!("candidates: {}", cands.len());
    let mut prev: Option<usize> = None;
    for (off, ptr, dorsal) in &cands {
        let delta = prev.map(|p| *off as i64 - p as i64).unwrap_or(0);
        println!("  offset={off:#07x} ({off:>6}) ptr={ptr:>5} dorsal={dorsal:>3} delta={delta:>6}");
        prev = Some(*off);
    }
    println!(
        "\n(NOTE: manual tracing found the *actual* first player record starting well before \
         the first heuristic hit above -- this scan is a rough lower-bound signal, not a \
         reliable player-count method. See PKF_FORMAT.md §6.)"
    );
}
