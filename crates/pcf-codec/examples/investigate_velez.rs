//! Throwaway investigator for the Vélez Sarsfield bug report (5 players
//! instead of a full ~20-30 squad, jersey number 0, garbled coach name).
//!
//! Isolates Vélez's real domestic record bytes (via
//! `container::find_domestic_team_records`) and walks the coach-marker
//! (`02 02`) and player-marker (`0x01`) candidates by hand, printing every
//! candidate hit and whether `container::parse_player_record`/the coach
//! parse succeeds there, so we can see exactly where the real production
//! walk goes wrong (a false-positive match earlier in the file than the
//! true coach/roster start) vs. where it should have landed.
//!
//! Usage: cargo run -p pcf-codec --example investigate_velez -- <path.pkf> [charmap-path]

use pcf_codec::charmap::CharMap;
use pcf_codec::container;
use std::env;
use std::fs;

fn main() {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .expect("usage: investigate_velez <path-to-EQ003003.PKF> [charmap-path]");
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
    let charmap = CharMap::load(&charmap_path).expect("failed to load charmap");

    let bytes = fs::read(&path).expect("failed to read PKF file");
    println!("file: {path} ({} bytes)\n", bytes.len());

    let records = container::find_domestic_team_records(&bytes);
    println!("found {} domestic team records\n", records.len());

    // Find Vélez by decoding each record's short_name field directly
    // (banner + 6-byte header + length-prefixed short_name).
    const BANNER_LEN: usize = 36; // "Copyright (c)1996 Dinamic Multimedia"
    let mut velez_range: Option<(usize, usize)> = None;
    for &(start, end) in &records {
        let sn_len_pos = start + BANNER_LEN + 6;
        if sn_len_pos + 2 > bytes.len() {
            continue;
        }
        let len = u16::from_le_bytes([bytes[sn_len_pos], bytes[sn_len_pos + 1]]) as usize;
        let sn_start = sn_len_pos + 2;
        if sn_start + len > bytes.len() {
            continue;
        }
        match charmap.decode(&bytes[sn_start..sn_start + len]) {
            Ok(name) => {
                println!("record start={start:#x} short_name={name:?}");
                if name.contains("lez") || name.contains("elez") {
                    velez_range = Some((start, end));
                }
            }
            Err(e) => {
                println!("record start={start:#x} short_name decode FAILED: {e:?}");
            }
        }
    }

    println!("\n=== budget-offset check: bytes right after `president` for every team ===");
    for &(rs, re) in &records {
        let rec = &bytes[rs..re];
        let after_pres = find_after_president_offset(rec, &charmap);
        if after_pres + 3 > rec.len() {
            continue;
        }
        let b = &rec[after_pres..after_pres + 3];
        let u24 = b[0] as u32 | ((b[1] as u32) << 8) | ((b[2] as u32) << 16);
        // also grab short_name for labeling
        let sn_len_pos = rs + 36 + 6;
        let sn_len = u16::from_le_bytes([bytes[sn_len_pos], bytes[sn_len_pos + 1]]) as usize;
        let sn = charmap
            .decode(&bytes[sn_len_pos + 2..sn_len_pos + 2 + sn_len])
            .unwrap_or_default();
        println!(
            "  {sn:?}: bytes_after_president={:02X} {:02X} {:02X} {:02X} {:02X} {:02X}  as_u24_le={u24}",
            b[0],
            b[1],
            b[2],
            rec.get(after_pres + 3).copied().unwrap_or(0),
            rec.get(after_pres + 4).copied().unwrap_or(0),
            rec.get(after_pres + 5).copied().unwrap_or(0),
        );
    }

    println!("\n=== extended hex dump: 40 bytes right after `president`, several teams ===");
    let interesting = [
        "River",
        "Boca",
        "San Lorenzo",
        "Vélez",
        "El Porvenir",
        "Racing",
        "Independiente",
    ];
    for &(rs, re) in &records {
        let sn_len_pos = rs + 36 + 6;
        let sn_len = u16::from_le_bytes([bytes[sn_len_pos], bytes[sn_len_pos + 1]]) as usize;
        let sn = charmap
            .decode(&bytes[sn_len_pos + 2..sn_len_pos + 2 + sn_len])
            .unwrap_or_default();
        if !interesting.contains(&sn.as_str()) {
            continue;
        }
        let rec = &bytes[rs..re];
        let after_pres = find_after_president_offset(rec, &charmap);
        let end = (after_pres + 40).min(rec.len());
        let hex: Vec<String> = rec[after_pres..end]
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect();
        println!("  {sn:>15?}: {}", hex.join(" "));

        // Try decoding the length-prefixed string starting at offset+8
        // (right after the u24 budget-candidate + 3 zero bytes), the same
        // shape as the still-unexplained 22-byte block's opening string.
        if after_pres + 10 <= rec.len() {
            let len = u16::from_le_bytes([rec[after_pres + 8], rec[after_pres + 9]]) as usize;
            let str_start = after_pres + 10;
            if len > 0 && len < 30 && str_start + len <= rec.len() {
                match charmap.decode(&rec[str_start..str_start + len]) {
                    Ok(s) => println!("      -> string@+8 (len={len}): {s:?}"),
                    Err(e) => println!("      -> string@+8 (len={len}): decode failed: {e:?}"),
                }
            }
        }
    }

    println!("\n=== file-wide sanity pass: player counts + any number==0 ===");
    for &(rs, re) in &records {
        if let Ok(team) = container::parse_team_record(&bytes[rs..re], &charmap) {
            let zeros: Vec<&str> = team
                .players
                .iter()
                .filter(|p| p.number == 0)
                .map(|p| p.short_name.as_str())
                .collect();
            println!(
                "  {:?}: players={} coach={:?} number==0 players={:?}",
                team.short_name,
                team.players.len(),
                team.coach.as_ref().map(|c| &c.short_name),
                zeros
            );
        }
    }

    let (start, end) = velez_range.expect("Vélez record not found");
    let record = &bytes[start..end];
    println!(
        "\n=== Vélez record isolated: {} bytes (file offsets {start}..{end}) ===\n",
        record.len()
    );

    // Run the real production parser and show what it currently produces.
    match container::parse_team_record(record, &charmap) {
        Ok(team) => {
            println!("parse_team_record OK:");
            println!("  short_name = {:?}", team.short_name);
            println!("  stadium_name = {:?}", team.stadium_name);
            println!("  long_name = {:?}", team.long_name);
            println!("  capacity = {}", team.capacity);
            println!("  founded = {}", team.founded);
            println!("  president = {:?}", team.president);
            println!("  coach = {:?}", team.coach);
            println!("  players.len() = {}", team.players.len());
            for (i, p) in team.players.iter().enumerate() {
                println!(
                    "    [{i}] number={} short_name={:?} long_name={:?} pointer={}",
                    p.number, p.short_name, p.long_name, p.pointer
                );
            }
            println!("  trailing_raw.len() = {}", team.trailing_raw.len());
        }
        Err(e) => println!("parse_team_record FAILED: {e:?}"),
    }

    // Find cursor position right after `president` by re-parsing the
    // team-info-only fields by hand (mirrors parse_team_record up to that
    // point) so we can scan the "rest" the same way find_coach_stub does.
    let after_president_offset = find_after_president_offset(record, &charmap);
    println!(
        "\ncursor offset right after `president` = {after_president_offset} (record-relative)"
    );

    let rest = &record[after_president_offset..];

    // Scan every `02 02` occurrence in `rest` and show if a coach parse
    // would succeed there (mirroring find_coach_stub/try_parse_coach_at,
    // reimplemented minimally here since those are private to container.rs).
    println!("\n--- scanning for `02 02` coach-marker candidates in `rest` ---");
    for i in 0..rest.len().saturating_sub(2) {
        if rest[i] != 0x02 || rest[i + 1] != 0x02 {
            continue;
        }
        match try_parse_coach(&rest[i..], &charmap) {
            Some((short, long, consumed)) => {
                println!(
                    "  [rest+{i}] (abs {}) COACH CANDIDATE OK -> short={short:?} long={long:?} consumed={consumed}",
                    after_president_offset + i
                );
            }
            None => {
                // only print near-misses within first 2000 bytes to avoid spam
                if i < 2000 {
                    println!(
                        "  [rest+{i}] (abs {}) 02 02 found, parse failed",
                        after_president_offset + i
                    );
                }
            }
        }
    }

    // Scan every `0x01` occurrence in `rest` and show if parse_player_record
    // succeeds there (using the REAL, production parse_player_record).
    println!("\n--- scanning for `0x01` player-marker candidates in `rest` (first 4000 bytes) ---");
    let scan_end = rest.len().min(4000);
    for i in 0..scan_end {
        if rest[i] != 0x01 {
            continue;
        }
        if let Ok((player, consumed)) = container::parse_player_record(&rest[i..], &charmap) {
            println!(
                "  [rest+{i}] (abs {}) PLAYER CANDIDATE OK -> number={} short_name={:?} long_name={:?} consumed={consumed}",
                after_president_offset + i,
                player.number,
                player.short_name,
                player.long_name
            );
        }
    }
}

/// Re-derives the cursor offset right after `president` by replaying
/// `parse_team_record`'s team-info field sequence (PKF_FORMAT.md §6.1-§6.2)
/// using only public `Reader`-shaped primitives reimplemented inline, since
/// `container::parse_team_record` doesn't expose intermediate cursor state.
fn find_after_president_offset(record: &[u8], charmap: &CharMap) -> usize {
    let mut pos = 0usize;
    // banner
    pos += 36;
    // header (2 prefix + 4 tail)
    pos += 6;
    // short_name
    pos += skip_string(record, pos);
    // stadium_name
    pos += skip_string(record, pos);
    // country
    pos += 1;
    // unexplained_byte_after_country
    pos += 1;
    // long_name
    pos += skip_string(record, pos);
    // capacity (u24) + zero
    pos += 4;
    // standing_capacity (u24) + zero
    pos += 4;
    // pitch_size (4 bytes)
    pos += 4;
    // founded (2 bytes)
    pos += 2;
    // unexplained_bytes_after_founded
    pos += 2;
    // members (u24) + zero
    pos += 4;
    // president
    pos += skip_string(record, pos);

    let _ = charmap; // charmap not needed for length-only skipping
    pos
}

fn skip_string(record: &[u8], pos: usize) -> usize {
    let len = u16::from_le_bytes([record[pos], record[pos + 1]]) as usize;
    2 + len
}

fn try_parse_coach(bytes: &[u8], charmap: &CharMap) -> Option<(String, String, usize)> {
    // marker (2) + pointer (2) + short_name + long_name
    if bytes.len() < 4 {
        return None;
    }
    let mut pos = 4usize;
    let short_len = u16::from_le_bytes([*bytes.get(pos)?, *bytes.get(pos + 1)?]) as usize;
    let short_start = pos + 2;
    if short_start + short_len > bytes.len() {
        return None;
    }
    let short = charmap
        .decode(&bytes[short_start..short_start + short_len])
        .ok()?;
    if short.is_empty() || short.len() > 64 {
        return None;
    }
    pos = short_start + short_len;
    let long_len = u16::from_le_bytes([*bytes.get(pos)?, *bytes.get(pos + 1)?]) as usize;
    let long_start = pos + 2;
    if long_start + long_len > bytes.len() {
        return None;
    }
    let long = charmap
        .decode(&bytes[long_start..long_start + long_len])
        .ok()?;
    if long.is_empty() || long.len() > 64 {
        return None;
    }
    Some((short, long, long_start + long_len))
}
