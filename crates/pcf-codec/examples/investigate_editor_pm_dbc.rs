//! THROWAWAY investigation tool (not part of the codec contract, not wired
//! into any test) for cross-referencing this project's charmap against a
//! large external corpus of real `EQ97####.DBC` override files from the
//! community "EDITOR-PM9798" tool (PM97/PM98/PCPREMIER60 — earlier/related
//! entries in the same Dinamic Multimedia "PC Fútbol"/"PC Premier Manager"
//! engine family, confirmed same banner/override-file shape as this
//! project's own `EQ97####.DBC` format).
//!
//! Purpose: batch-decode every DBC file's `short_name`/`stadium_name`/
//! `long_name` team-info strings, LOSSILY (unmapped bytes render as `[XX]`),
//! so charmap gaps -- especially accented Spanish/European letters -- can be
//! spotted and cross-referenced against real known team/stadium names.
//! Mirrors `dump_stub_table.rs`'s lossy-decode methodology.
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example investigate_editor_pm_dbc -- <dir-with-EQ97-files> [charmap-path]

use std::collections::HashMap;
use std::env;
use std::fs;

const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";
/// Confirmed-constant pitch-size fixed bytes (dbc.rs::PITCH_SIZE) -- used
/// here only as a sanity gate to detect whether the fixed-field region past
/// long_name looks like a plausible match to this project's known layout
/// (not assumed to hold across editions, just checked as a data point).
const PITCH_SIZE: &[u8] = &[0x46, 0x00, 0x6A, 0x00];

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

fn read_lp_string(bytes: &[u8], pos: usize) -> Option<(&[u8], usize)> {
    if pos + 2 > bytes.len() {
        return None;
    }
    let len = u16::from_le_bytes(bytes[pos..pos + 2].try_into().ok()?) as usize;
    if len > 500 || pos + 2 + len > bytes.len() {
        return None;
    }
    Some((&bytes[pos + 2..pos + 2 + len], pos + 2 + len))
}

fn main() {
    let dir = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: investigate_editor_pm_dbc <dir-with-EQ97-files> [charmap-path]");
        std::process::exit(1);
    });
    let charmap_path = env::args().nth(2).unwrap_or_else(|| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("confirmed_real_map_v2.txt")
            .to_str()
            .unwrap()
            .to_string()
    });
    let map = load_lossy_map(&charmap_path);

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read dir {dir}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_uppercase().starts_with("EQ97") && n.to_uppercase().ends_with(".DBC"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort();

    eprintln!("found {} EQ97*.DBC files in {}", entries.len(), dir);

    let mut n_ok = 0usize;
    let mut n_bad_banner = 0usize;
    let mut n_read_fail = 0usize;
    let mut n_pitch_match = 0usize;

    for path in &entries {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                println!("file={} ERROR reading: {e}", path.display());
                continue;
            }
        };
        if bytes.len() < BANNER.len() || &bytes[..BANNER.len()] != BANNER {
            n_bad_banner += 1;
            println!("file={} ERROR: banner mismatch", path.display());
            continue;
        }

        // banner + FE 06 (2) + file_version (2) + language (1) + league_flag (1)
        let mut pos = BANNER.len() + 2 + 2 + 1 + 1;

        let Some((short_raw, p1)) = read_lp_string(&bytes, pos) else {
            n_read_fail += 1;
            println!(
                "file={} ERROR: short_name read failed at {pos}",
                path.display()
            );
            continue;
        };
        pos = p1;
        let Some((stadium_raw, p2)) = read_lp_string(&bytes, pos) else {
            n_read_fail += 1;
            println!(
                "file={} short_name={} ERROR: stadium_name read failed at {pos}",
                path.display(),
                lossy_decode(&map, short_raw)
            );
            continue;
        };
        pos = p2;

        // country: 1 byte
        if pos + 1 > bytes.len() {
            n_read_fail += 1;
            continue;
        }
        pos += 1;

        let Some((long_raw, p3)) = read_lp_string(&bytes, pos) else {
            n_read_fail += 1;
            println!(
                "file={} short_name={} stadium_name={} ERROR: long_name read failed at {pos}",
                path.display(),
                lossy_decode(&map, short_raw),
                lossy_decode(&map, stadium_raw)
            );
            continue;
        };
        pos = p3;

        // Sanity gate: capacity(u24)+00 [+ standing_capacity(u24)+00] + pitch_size(4).
        // Try both with and without standing_capacity present to see which
        // (if either) lands on the known PITCH_SIZE constant -- purely
        // informational, doesn't block printing the names either way.
        let mut pitch_hit = false;
        for standing_present in [true, false] {
            let mut probe = pos + 3 + 1;
            if standing_present {
                probe += 3 + 1;
            }
            if probe + 4 <= bytes.len() && &bytes[probe..probe + 4] == PITCH_SIZE {
                pitch_hit = true;
            }
        }
        if pitch_hit {
            n_pitch_match += 1;
        }

        n_ok += 1;
        let short_hex: Vec<String> = short_raw.iter().map(|b| format!("{b:02x}")).collect();
        let stadium_hex: Vec<String> = stadium_raw.iter().map(|b| format!("{b:02x}")).collect();
        let long_hex: Vec<String> = long_raw.iter().map(|b| format!("{b:02x}")).collect();
        println!(
            "file={} pitch_match={} short_name=[{}]{} stadium_name=[{}]{} long_name=[{}]{}",
            path.file_name().unwrap().to_string_lossy(),
            pitch_hit,
            short_hex.join(" "),
            lossy_decode(&map, short_raw),
            stadium_hex.join(" "),
            lossy_decode(&map, stadium_raw),
            long_hex.join(" "),
            lossy_decode(&map, long_raw),
        );

        // Also lossy-decode the REST of the file raw (byte-for-byte, no
        // length-prefix framing assumed) so any readable prose further in
        // (coach/president/etc. names, even if this file's later structure
        // doesn't match dbc.rs's assumed layout) can be eyeballed for
        // additional charmap gaps -- especially uppercase-surname-style runs
        // like "MART[XX]NEZ".
        if pos < bytes.len() {
            let tail = &bytes[pos..];
            println!(
                "  TAIL[{}]: {}",
                path.file_name().unwrap().to_string_lossy(),
                lossy_decode(&map, tail)
            );
        }
    }

    eprintln!(
        "done: {n_ok} ok, {n_bad_banner} bad banner, {n_read_fail} read failures, {n_pitch_match} pitch-size-constant matches"
    );
}
