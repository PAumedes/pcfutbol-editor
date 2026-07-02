//! Seventh-pass investigator (PKF_FORMAT.md §5 Q3): scans the whole
//! `EQ003003.PKF` file for banner occurrences past the point the "foreign
//! reference clubs" stub table ends (~628,300, see PKF_FORMAT.md §4), reads
//! each one's post-banner header, keeps the ones matching the confirmed
//! domestic-record signature (`0D 02 00 00` at banner+38 -- NOT the full
//! 6-byte `E9 07 0D 02 00 00` from §6.1, which turned out to only hold for
//! River's own record; the leading 2 bytes vary per team), and decodes just
//! the `short_name` field (first length-prefixed string) using the 77-pair
//! `confirmed_real_map_v2.txt` charmap.
//!
//! Investigation-only; does not feed `container.rs`.
//!
//! Usage: cargo run -p pcf-codec --example enumerate_domestic_teams -- <path-to-EQ003003.PKF> [charmap-path]

use std::collections::HashMap;
use std::env;
use std::fs;

const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";

fn load_map(path: &str) -> HashMap<u8, char> {
    let contents = fs::read_to_string(path).expect("failed to read charmap");
    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim_end_matches('\r').trim();
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
    map
}

fn lossy_decode(map: &HashMap<u8, char>, bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| map.get(b).copied().unwrap_or('.'))
        .collect()
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
        }
        i += 1;
    }
    out
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .expect("usage: enumerate_domestic_teams <path-to-EQ003003.PKF> [charmap-path]");
    let charmap_path = args.next().unwrap_or_else(|| {
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
    let map = load_map(&charmap_path);

    let bytes = fs::read(&path).expect("failed to read PKF file");
    println!("file: {path} ({} bytes)\n", bytes.len());

    let banners = find_all(&bytes, BANNER);
    println!("total banner occurrences in whole file: {}", banners.len());

    // Per §3/§4: past ~628,300 the foreign-stub marker `0D 02 00 01` stops
    // appearing and spacing jumps to full-team-record scale. Scan from
    // there onward (using a slightly conservative floor of 620,000 in case
    // the exact boundary differs by a few records) for domestic-header hits.
    let floor = 600_000usize;
    let mut domestic = Vec::new();
    for &b in &banners {
        if b < floor {
            continue;
        }
        let header_start = b + BANNER.len();
        if header_start + 8 > bytes.len() {
            continue;
        }
        // The first 2 bytes right after the banner are NOT a fixed constant
        // across domestic records -- only River's happens to be `E9 07`.
        // Cross-checking a handful of other post-628,300 banners shows the
        // first 2 bytes vary per record (e.g. `21 07`, `E2 05`, `47 06`,
        // `39 07`) while bytes [2..6) are constant `0D 02 00 00` for all of
        // them. The true domestic-record signature is this 4-byte constant
        // at header offset +2 (banner + 38), not the naive 6-byte match
        // used in earlier passes (which only worked by coincidence for the
        // already-known River record). See PKF_FORMAT.md.
        let sig = &bytes[header_start + 2..header_start + 6];
        if sig == [0x0d, 0x02, 0x00, 0x00] {
            let len_pos = header_start + 6;
            let len = u16::from_le_bytes([bytes[len_pos], bytes[len_pos + 1]]) as usize;
            let str_start = len_pos + 2;
            if len == 0 || len > 60 || str_start + len > bytes.len() {
                println!(
                    "  banner@{b:#08x} ({b}): domestic header OK but implausible short_name len={len} -- skipping decode"
                );
                domestic.push((b, format!("<len={len} implausible>")));
                continue;
            }
            let short_name = lossy_decode(&map, &bytes[str_start..str_start + len]);
            domestic.push((b, short_name));
        }
    }

    println!(
        "\ndomestic-header (`E9 07 0D 02 00 00`) records found past offset {floor}: {}\n",
        domestic.len()
    );
    for (i, (off, name)) in domestic.iter().enumerate() {
        let next_off = domestic.get(i + 1).map(|(o, _)| *o);
        let span = next_off.map(|n| n - off);
        let span_str = span
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "  [{i:>3}] banner@{off:#08x} ({off:>7})  span={span_str:>7}  short_name={name:?}"
        );
    }
}
