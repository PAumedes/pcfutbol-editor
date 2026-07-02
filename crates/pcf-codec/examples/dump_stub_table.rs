//! Dumps ALL 15 "foreign reference clubs" stub-table directory blocks
//! (see `fixtures/PKF_FORMAT.md` §3) end to end, decoding each record's
//! length-prefixed `short_name` / `stadium_name` / `long_name` strings with
//! the current best-known charmap, in **lossy** mode: any byte not yet in
//! the charmap renders as `[XX]` (its own hex) instead of `?` or being
//! silently dropped, so charmap gaps are visible and directly actionable
//! against `fixtures/pointers/team_pointers.csv`.
//!
//! This is investigation-only code (like `investigate_pkf_dir.rs`, which it
//! borrows its block-detection logic from) — not part of the codec
//! contract, not wired into any test, and does not modify
//! `crates/pcf-codec/src/{dbc,charmap}.rs`.
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example dump_stub_table -- <path-to-pkf> [charmap-path]
//!
//! Output: one line per decoded string field, in physical file order, e.g.
//!   block=0 entry=0 field=short_name raw_off=1503 len=13 hex=[..] text=F.C. Barcelona
//!
//! Pipe stdout to a file for offline cross-referencing against the CSV.

use std::collections::HashMap;
use std::env;
use std::fs;

const SIG: &[u8] = &[
    0x31, 0x54, 0x41, 0xBB, 0xEF, 0xE8, 0xE3, 0xE0, 0x0B, 0xC9, 0xA3, 0xE8, 0x00,
];
const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";
const ENTRY_LEN: usize = 38;
const SIG_OFFSET_IN_ENTRY: usize = 8;
/// Stub-record marker per PKF_FORMAT.md §3: banner, then this 4-byte marker,
/// then the first 2-byte-length-prefixed string (short_name).
const MARKER: &[u8] = &[0x0D, 0x02, 0x00, 0x01];

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
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

struct Entry {
    offset: u32,
}

fn parse_entry(bytes: &[u8], entry_start: usize) -> Entry {
    let e = &bytes[entry_start..entry_start + ENTRY_LEN];
    let offset = u32::from_le_bytes(e[25..29].try_into().unwrap());
    Entry { offset }
}

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
    map
}

/// Lossy decode: mapped bytes render as their glyph; unmapped bytes render
/// as `[XX]` (hex) so gaps are visible and directly copy-pasteable as
/// "this hex byte needs a glyph" evidence.
fn lossy_decode(map: &HashMap<u8, char>, bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match map.get(&b) {
            Some(&c) => out.push(c),
            None => out.push_str(&format!("[{b:02X}]")),
        }
    }
    out
}

/// Reads one 2-byte-LE-length-prefixed string starting at `pos`. Returns
/// (raw_bytes, next_pos) or None if it doesn't fit / length looks bogus.
fn read_lp_string(bytes: &[u8], pos: usize) -> Option<(&[u8], usize)> {
    if pos + 2 > bytes.len() {
        return None;
    }
    let len = u16::from_le_bytes(bytes[pos..pos + 2].try_into().ok()?) as usize;
    // Sanity cap -- these are short team-info strings, never anywhere near
    // this large. Guards against misaligned reads spinning off huge slices.
    if len > 500 || pos + 2 + len > bytes.len() {
        return None;
    }
    Some((&bytes[pos + 2..pos + 2 + len], pos + 2 + len))
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: dump_stub_table <path-to-pkf> [charmap-path]");
        std::process::exit(1);
    });
    let charmap_path = env::args().nth(2).unwrap_or_else(|| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("confirmed_real_map.txt")
            .to_str()
            .unwrap()
            .to_string()
    });
    let map = load_lossy_map(&charmap_path);

    let bytes = fs::read(&path).expect("failed to read input file");
    eprintln!(
        "file: {} ({} bytes), charmap: {}",
        path,
        bytes.len(),
        charmap_path
    );

    let sig_positions = find_all(&bytes, SIG);

    // Group into contiguous 38-byte-spaced blocks (same logic as
    // investigate_pkf_dir.rs).
    let mut blocks: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for &pos in &sig_positions {
        if let Some(&last) = current.last() {
            if pos - last == ENTRY_LEN {
                current.push(pos);
                continue;
            } else {
                blocks.push(std::mem::take(&mut current));
            }
        }
        current.push(pos);
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    eprintln!("{} directory block(s) found", blocks.len());

    let mut global_record_idx = 0usize;
    for (bi, block) in blocks.iter().enumerate() {
        for (ei, &sig_pos) in block.iter().enumerate() {
            let e_start = sig_pos - SIG_OFFSET_IN_ENTRY;
            if e_start + ENTRY_LEN > bytes.len() {
                continue;
            }
            let entry = parse_entry(&bytes, e_start);
            let rec_start = entry.offset as usize;
            if rec_start + BANNER.len() > bytes.len()
                || &bytes[rec_start..rec_start + BANNER.len()] != BANNER
            {
                println!("block={bi} entry={ei} global={global_record_idx} raw_off={rec_start} ERROR: banner mismatch");
                global_record_idx += 1;
                continue;
            }
            let mut pos = rec_start + BANNER.len();
            // Find the marker within a bounded window after the banner
            // (should be immediate, but allow slack).
            let search_end = (pos + 16).min(bytes.len());
            let marker_pos = (pos..search_end.saturating_sub(MARKER.len()))
                .find(|&p| &bytes[p..p + MARKER.len()] == MARKER);
            let Some(mp) = marker_pos else {
                println!("block={bi} entry={ei} global={global_record_idx} raw_off={rec_start} ERROR: marker not found near banner");
                global_record_idx += 1;
                continue;
            };
            pos = mp + MARKER.len();

            let field_names = ["short_name", "stadium_name", "long_name"];
            for fname in field_names {
                match read_lp_string(&bytes, pos) {
                    Some((raw, next_pos)) => {
                        let hex: Vec<String> = raw.iter().map(|b| format!("{b:02x}")).collect();
                        let text = lossy_decode(&map, raw);
                        println!(
                            "block={bi} entry={ei} global={global_record_idx} field={fname} str_off={pos} len={} hex=[{}] text={}",
                            raw.len(),
                            hex.join(" "),
                            text
                        );
                        pos = next_pos;
                    }
                    None => {
                        println!("block={bi} entry={ei} global={global_record_idx} field={fname} ERROR: could not read length-prefixed string at {pos}");
                        break;
                    }
                }
            }
            global_record_idx += 1;
        }
    }
    eprintln!("total records processed: {global_record_idx}");
}
