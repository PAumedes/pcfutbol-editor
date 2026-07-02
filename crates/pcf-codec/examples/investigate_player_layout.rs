//! Fifth-pass investigator (PKF_FORMAT.md §5 Q1): walks the real domestic
//! team blob's player records byte-by-byte against the *override* format's
//! `read_player` field order (marker, pointer, number, short_name, long_name,
//! slot, origin, roles[6], nationality, skin, hair, demarcation, birth(4),
//! height, weight, birth_country, birthplace, 8 free-text fields, career,
//! attrs(10)) -- see `crates/pcf-codec/src/dbc.rs::read_player`.
//!
//! Crucially, `read_player` puts `short_name`/`long_name` BEFORE
//! slot/origin/roles/etc, not after -- `investigate_domestic_team.rs`'s
//! §6.6 writeup was written before this was checked against the override
//! reader's actual field order, which is why "a few unaccounted bytes"
//! were reported between `number` and `short_name`: there should be NONE,
//! since short_name comes right after number.
//!
//! This tool is investigation-only, decoupled from `pcf_codec` internals
//! (deliberately re-implements a minimal cursor), and does NOT feed into
//! `container.rs` (owned by a parallel effort).
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example investigate_player_layout -- [path-to-blob] [start-offset]
//! Defaults to `fixtures/golden/real_river_9001_container_blob.raw` and the
//! confirmed first-player marker offset 1238.

use std::collections::HashMap;
use std::env;
use std::fs;

fn load_map(path: &str) -> HashMap<u8, char> {
    let contents = fs::read_to_string(path).expect("failed to read charmap");
    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim_end_matches('\r');
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((hex, rest)) = line.split_once('\t') {
            if let Ok(byte) = u8::from_str_radix(hex.trim(), 16) {
                let ch = if rest == "\\s" {
                    ' '
                } else {
                    rest.chars().next().unwrap_or('?')
                };
                map.insert(byte, ch);
            }
        }
    }
    // 0x6b='=' seen in investigate_domestic_team.rs's EXTRA_INFERRED table
    // (career-field default suffix "ND,ND,ND,ND,ND=="), distinct from the
    // already-confirmed 0x6C='=' in the base 37-pair map -- kept as a local
    // overlay pending reconciliation (not yet in confirmed_real_map_v2.txt).
    map.entry(0x6b).or_insert('=');
    map
}

fn lossy_decode(map: &HashMap<u8, char>, bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| map.get(b).copied().unwrap_or('.'))
        .collect()
}

struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
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
    fn string(&mut self, map: &HashMap<u8, char>) -> (String, usize, usize) {
        let start = self.pos;
        let len = self.u16_le() as usize;
        let bytes_start = self.pos;
        let bytes = self.take(len);
        (lossy_decode(map, bytes), start, bytes_start + len)
    }
}

/// Attempts to parse one player record per the override `read_player` field
/// order, starting at a `0x01` marker byte. Returns the cursor position
/// right after the record (start of the next record, if this parse is
/// correct) plus a human-readable summary. Does not bail out on
/// out-of-range enum bytes -- reports them so we can judge plausibility
/// ourselves instead of hiding the evidence behind a hard error.
fn parse_player(data: &[u8], start: usize, map: &HashMap<u8, char>) -> (usize, String) {
    let mut out = String::new();
    let mut c = Cur { data, pos: start };
    let marker = c.u8();
    out.push_str(&format!("  marker @ {:#07x} = {:#04x}\n", start, marker));
    let pointer = c.u16_le();
    let number = c.u8();
    out.push_str(&format!("  pointer(u16 LE) = {pointer}\n"));
    out.push_str(&format!("  number (dorsal) = {number}\n"));

    // The gap between `number` and `short_name`'s own length prefix is NOT
    // fixed (3 bytes for the first player in the blob, 0 bytes for the
    // second -- confirmed by manual tracing). Rather than assume a fixed
    // width, search forward for the first plausible (length-prefix, mostly
    // decodable string) pair, treating any skipped bytes as an unexplained
    // variable-length gap and reporting them raw.
    let gap_start = c.pos;
    let mut gap_len = 0usize;
    loop {
        if gap_len > 8 {
            out.push_str("  WARNING: no plausible short_name length-prefix found within 8 bytes\n");
            break;
        }
        let probe_pos = gap_start + gap_len;
        if probe_pos + 2 > data.len() {
            break;
        }
        let len = u16::from_le_bytes([data[probe_pos], data[probe_pos + 1]]) as usize;
        if (1..=40).contains(&len) && probe_pos + 2 + len <= data.len() {
            let text = lossy_decode(map, &data[probe_pos + 2..probe_pos + 2 + len]);
            let unmapped = text.chars().filter(|&ch| ch == '.').count();
            if unmapped * 4 <= len {
                break; // plausible: accept this as the length prefix
            }
        }
        gap_len += 1;
    }
    let gap = c.take(gap_len).to_vec();
    out.push_str(&format!(
        "  gap[{gap_len}] @ {gap_start:#x} = {gap:02x?}  (variable-length, unexplained -- see PKF_FORMAT.md)\n"
    ));

    let (short_name, sn_start, sn_end) = c.string(map);
    out.push_str(&format!(
        "  short_name @ [{sn_start:#x}..{sn_end:#x}) = {short_name:?}\n"
    ));
    let (long_name, ln_start, ln_end) = c.string(map);
    out.push_str(&format!(
        "  long_name  @ [{ln_start:#x}..{ln_end:#x}) = {long_name:?}\n"
    ));

    let slot = c.u8();
    let origin = c.u8();
    out.push_str(&format!("  slot = {slot}, origin = {origin}\n"));

    let roles_start = c.pos;
    let roles: Vec<u8> = (0..6).map(|_| c.u8()).collect();
    let roles_ok = roles.iter().all(|&b| b <= 0x12);
    out.push_str(&format!(
        "  roles[6] @ {roles_start:#x} = {roles:?}  (all <=0x12: {roles_ok})\n"
    ));

    let nationality = c.u8();
    out.push_str(&format!("  nationality = {nationality:#04x}\n"));

    let skin = c.u8();
    let hair = c.u8();
    let demarcation = c.u8();
    out.push_str(&format!(
        "  skin = {skin} (1..=3 ok: {}), hair = {hair} (1..=6 ok: {}), demarcation = {demarcation} (0..=3 ok: {})\n",
        (1..=3).contains(&skin),
        (1..=6).contains(&hair),
        (0..=3).contains(&demarcation),
    ));

    let birth = c.take(4);
    let (day, month, year) = (birth[0], birth[1], u16::from_le_bytes([birth[2], birth[3]]));
    let birth_ok =
        (1..=31).contains(&day) && (1..=12).contains(&month) && (1900..=1985).contains(&year);
    out.push_str(&format!(
        "  birth = {day:02}/{month:02}/{year} raw={birth:02x?} (plausible: {birth_ok})\n"
    ));

    let height = c.u8();
    let weight = c.u8();
    let birth_country = c.u8();
    out.push_str(&format!(
        "  height_cm = {height}, weight_kg = {weight} (plausible: {}), birth_country = {birth_country:#04x}\n",
        (140..=210).contains(&height) && (50..=110).contains(&weight),
    ));

    let (birthplace, bp_start, bp_end) = c.string(map);
    out.push_str(&format!(
        "  birthplace @ [{bp_start:#x}..{bp_end:#x}) = {birthplace:?}\n"
    ));

    let field_names = [
        "debut_club",
        "international",
        "profile",
        "characteristics",
        "palmares",
        "internationality",
        "anecdotes",
        "last_season",
        "career",
    ];
    for name in field_names {
        let (text, fs, fe) = c.string(map);
        let preview: String = text.chars().take(40).collect();
        out.push_str(&format!(
            "  {name:<16} @ [{fs:#x}..{fe:#x}) len={} preview={preview:?}\n",
            fe - fs
        ));
    }

    let attrs_start = c.pos;
    let attrs = c.take(10).to_vec();
    let attrs_ok = attrs.iter().all(|&b| b <= 99);
    out.push_str(&format!(
        "  attrs[10] @ {attrs_start:#x} = {attrs:?} (all <=99: {attrs_ok})\n"
    ));
    let names10 = [
        "velocidad",
        "resistencia",
        "agresividad",
        "calidad",
        "remate",
        "regate",
        "pase",
        "tiro",
        "entradas",
        "portero",
    ];
    for (n, v) in names10.iter().zip(attrs.iter()) {
        out.push_str(&format!("    {n:<12} = {v}\n"));
    }

    out.push_str(&format!(
        "  --> record end / next byte @ {:#07x} = {:#04x}\n",
        c.pos,
        data.get(c.pos).copied().unwrap_or(0)
    ));

    (c.pos, out)
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "fixtures/golden/real_river_9001_container_blob.raw".to_string());
    let start_offset: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(1238);

    let charmap_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("charmap")
        .join("confirmed_real_map_v2.txt");
    let map = load_map(charmap_path.to_str().unwrap());

    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path} ({e})");
            std::process::exit(1);
        }
    };
    println!("file: {path} ({} bytes)\n", bytes.len());

    let max_records: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(6);
    let mut offset = start_offset;
    for i in 0..max_records {
        if offset >= bytes.len() || bytes[offset] != 0x01 {
            println!(
                "=== record {i}: byte at {offset:#07x} is {:#04x}, not 0x01 -- stopping ===",
                bytes.get(offset).copied().unwrap_or(0)
            );
            // Dump surrounding bytes to help hand-locate the real boundary.
            let lo = offset.saturating_sub(16);
            let hi = (offset + 32).min(bytes.len());
            println!("context [{lo:#x}..{hi:#x}): {:02x?}", &bytes[lo..hi]);
            break;
        }
        println!("=== record {i}: starting at {offset:#07x} ({offset}) ===");
        let (next, summary) = parse_player(&bytes, offset, &map);
        println!("{summary}");
        offset = next;
    }
}
