//! Follow-up investigator for the `.PKF` container's directory structure,
//! building on `investigate_pkf.rs`'s finding that the literal banner
//! ("Copyright (c)1996 Dinamic Multimedia", no space) recurs once per small
//! sub-record throughout the file, and that a repeating 13-byte signature
//! `31 54 41 BB EF E8 E3 E0 0B C9 A3 E8 00` marks 38-byte directory entries.
//!
//! This tool:
//!   1. Locates every occurrence of that 13-byte signature.
//!   2. Groups consecutive occurrences that are exactly 38 bytes apart into
//!      "directory blocks" (the hypothesis: one block per team, one entry
//!      per team/coach/player sub-record).
//!   3. Fully decodes each 38-byte entry's fields (id, sub, offset, length,
//!      flag, trailing byte) and cross-checks the offset/length fields
//!      against the real, independently-found banner positions.
//!   4. For a chosen block, dumps a lossy charmap decode of the first N
//!      bytes of each entry's record body (banner + tiny header + text) so
//!      record "shape" (team vs coach vs player) can be told apart by eye.
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example investigate_pkf_dir -- <path-to-pkf> [block_to_dump]
//!
//! Not part of the codec contract or test suite -- pure investigation code.

use std::env;
use std::fs;

const SIG: &[u8] = &[
    0x31, 0x54, 0x41, 0xBB, 0xEF, 0xE8, 0xE3, 0xE0, 0x0B, 0xC9, 0xA3, 0xE8, 0x00,
];
const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";
const ENTRY_LEN: usize = 38;
/// Signature starts 8 bytes into the entry (after the "id" field).
const SIG_OFFSET_IN_ENTRY: usize = 8;

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
    id: [u8; 8],
    sub: [u8; 4],
    offset: u32,
    length: u32,
    flag: u32,
    trailing: u8,
}

fn parse_entry(bytes: &[u8], entry_start: usize) -> Entry {
    let e = &bytes[entry_start..entry_start + ENTRY_LEN];
    let id: [u8; 8] = e[0..8].try_into().unwrap();
    // e[8..21] is the 13-byte constant signature.
    let sub: [u8; 4] = e[21..25].try_into().unwrap();
    let offset = u32::from_le_bytes(e[25..29].try_into().unwrap());
    let length = u32::from_le_bytes(e[29..33].try_into().unwrap());
    let flag = u32::from_le_bytes(e[33..37].try_into().unwrap());
    let trailing = e[37];
    Entry {
        id,
        sub,
        offset,
        length,
        flag,
        trailing,
    }
}

/// Lossy charmap decode using confirmed_real_map.txt-style `HH\tC` lines,
/// printing `.` for unmapped bytes instead of erroring — this is exploration
/// code, not the real codec, so partial decodes are fine and expected.
fn load_lossy_map(path: &str) -> std::collections::HashMap<u8, char> {
    let contents = fs::read_to_string(path).expect("failed to read charmap");
    let mut map = std::collections::HashMap::new();
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

fn lossy_decode(map: &std::collections::HashMap<u8, char>, bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| map.get(b).copied().unwrap_or('.'))
        .collect()
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: investigate_pkf_dir <path-to-pkf> [block_index_to_dump]");
        std::process::exit(1);
    });
    let dump_block: Option<usize> = env::args().nth(2).and_then(|s| s.parse().ok());

    let charmap_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("charmap")
        .join("confirmed_real_map.txt");
    let map = load_lossy_map(charmap_path.to_str().unwrap());

    let bytes = fs::read(&path).expect("failed to read input file");
    println!("file: {} ({} bytes)", path, bytes.len());

    let sig_positions = find_all(&bytes, SIG);
    let banner_positions = find_all(&bytes, BANNER);
    println!(
        "signature occurrences: {}  |  banner occurrences: {}",
        sig_positions.len(),
        banner_positions.len()
    );

    // Group into contiguous 38-byte-spaced blocks.
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

    println!("\n{} directory block(s) found:", blocks.len());
    let mut mismatches = 0usize;
    let mut total_entries = 0usize;
    for (bi, block) in blocks.iter().enumerate() {
        let first_sig_pos = block[0];
        let entry_start = first_sig_pos - SIG_OFFSET_IN_ENTRY;
        let last_sig_pos = *block.last().unwrap();
        let block_end = last_sig_pos - SIG_OFFSET_IN_ENTRY + ENTRY_LEN;
        println!(
            "  block {bi}: {} entries, file bytes [{}, {})",
            block.len(),
            entry_start,
            block_end
        );
        total_entries += block.len();

        // Verify each entry's offset/length fields against the real,
        // independently-found banner list — the single most important
        // sanity check in this whole investigation.
        for (i, &sig_pos) in block.iter().enumerate() {
            let e_start = sig_pos - SIG_OFFSET_IN_ENTRY;
            if e_start + ENTRY_LEN > bytes.len() {
                continue;
            }
            let entry = parse_entry(&bytes, e_start);
            let global_idx = total_entries - block.len() + i;
            let expected_offset = banner_positions.get(global_idx).copied();
            let ok_offset = expected_offset == Some(entry.offset as usize);
            let expected_len = banner_positions
                .get(global_idx + 1)
                .map(|&next| next - entry.offset as usize);
            let ok_len = expected_len == Some(entry.length as usize)
                || (global_idx + 1 == banner_positions.len()); // last entry: no "next" to compare
            if !ok_offset || !ok_len {
                mismatches += 1;
                println!(
                    "    entry {i}: id={:02x?} sub={:02x?} offset={} (expected {:?}) length={} (expected {:?}) flag={} trailing={:#04x}  <-- MISMATCH",
                    entry.id, entry.sub, entry.offset, expected_offset, entry.length, expected_len, entry.flag, entry.trailing
                );
            }
        }
    }
    println!(
        "\nverified {total_entries} entries against real banner list: {} mismatches",
        mismatches
    );

    // Dump a requested block's record bodies so team/coach/player "shape"
    // can be told apart by eye.
    if let Some(bi) = dump_block {
        if let Some(block) = blocks.get(bi) {
            println!("\n=== dumping block {bi} ({} entries) ===", block.len());
            for (i, &sig_pos) in block.iter().enumerate() {
                let e_start = sig_pos - SIG_OFFSET_IN_ENTRY;
                let entry = parse_entry(&bytes, e_start);
                let rec_start = entry.offset as usize;
                let rec_len = (entry.length as usize).min(200);
                if rec_start + rec_len > bytes.len() {
                    continue;
                }
                let rec = &bytes[rec_start..rec_start + rec_len];
                let after_banner = &rec[BANNER.len().min(rec.len())..];
                let head = &after_banner[..after_banner.len().min(120)];
                println!(
                    "\n-- entry {i}: banner_offset={} length={} id_last_byte={:#04x} sub={:02x?}",
                    entry.offset, entry.length, entry.id[7], entry.sub
                );
                println!("   hex : {:02x?}", head);
                println!("   text: {}", lossy_decode(&map, head));
            }
        } else {
            println!("block {bi} does not exist (only {} blocks)", blocks.len());
        }
    }
}
