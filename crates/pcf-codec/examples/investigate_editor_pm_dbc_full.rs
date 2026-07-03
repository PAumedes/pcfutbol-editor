//! THROWAWAY investigation tool, extension of `investigate_editor_pm_dbc.rs`:
//! walks a whole `EQ97####.DBC` file end to end (team info, tactics, coach
//! chain, full player roster) using the EXACT layout this project's own
//! `crates/pcf-codec/src/dbc.rs` already implements and has tested against
//! this project's own override-format understanding -- but decodes every
//! string LOSSILY (unmapped bytes render as `[XX]`) instead of hard-erroring,
//! so a much bigger corpus of real biographical names (coach/player
//! long_name, in particular) can be harvested for charmap cross-referencing
//! without one bad byte stopping the whole walk.
//!
//! Not part of the codec contract; does not modify dbc.rs/charmap.rs.
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example investigate_editor_pm_dbc_full -- <dir> [charmap-path]

use std::collections::HashMap;
use std::env;
use std::fs;

const BANNER: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";
const COACH_MARKER: u8 = 0x02;
const COACH_WAS_PLAYER_MARKER: u8 = 0x03;
const PLAYER_MARKER: u8 = 0x01;
const JORNADA_LEN: usize = 92;
const PALMARES_LEN: usize = 34;

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

/// Simple forward-only byte cursor, panicking (caught via catch_unwind at
/// the call site) rather than threading `Result` everywhere -- this is
/// throwaway investigation code, not production.
struct Cur<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }
    fn take(&mut self, n: usize) -> &'a [u8] {
        assert!(self.pos + n <= self.b.len(), "eof at {}", self.pos);
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        s
    }
    fn u8(&mut self) -> u8 {
        self.take(1)[0]
    }
    fn peek_u8(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }
    fn u16_le(&mut self) -> u16 {
        let b = self.take(2);
        u16::from_le_bytes([b[0], b[1]])
    }
    fn u24_le(&mut self) -> u32 {
        let b = self.take(3);
        u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16)
    }
    fn expect_fixed(&mut self, expected: &[u8]) {
        let a = self.take(expected.len());
        assert_eq!(
            a,
            expected,
            "fixed mismatch at pos {}",
            self.pos - expected.len()
        );
    }
    fn lossy_string(&mut self, map: &HashMap<u8, char>) -> String {
        let len = self.u16_le() as usize;
        let raw = self.take(len);
        lossy_decode(map, raw)
    }
    fn opaque_blob(&mut self) -> Vec<u8> {
        let len = self.u16_le() as usize;
        self.take(len).to_vec()
    }
}

fn walk_file(bytes: &[u8], map: &HashMap<u8, char>, tag: &str) {
    let mut c = Cur::new(bytes);
    c.expect_fixed(BANNER);
    // NOTE: this project's own dbc.rs assumes a literal `FE 06` magic right
    // after the banner (PLAN.md Appendix A), but that's never been checked
    // against a real override-format sample before now. Empirically, for
    // this PM97/PM98/PCPREMIER60 corpus, the two bytes right after the
    // banner are NOT always `FE 06` -- yet skipping exactly 6 bytes here
    // (matching Appendix A's documented total header width) reliably lands
    // on a valid short_name length prefix (verified in
    // investigate_editor_pm_dbc.rs: 478/478/480 files decoded clean,
    // zero read failures). So: skip 6 bytes positionally, without asserting
    // their content -- this tool is investigating text/charmap gaps, not
    // re-litigating the header's own byte meaning.
    let header6 = c.take(6);
    let league_flag = header6[5];
    let is_foreign = league_flag != 0x00;

    let short_name = c.lossy_string(map);
    let stadium_name = c.lossy_string(map);
    let _country = c.u8();
    let long_name = c.lossy_string(map);
    println!("== {tag} == (names only) short_name={short_name} stadium_name={stadium_name} long_name={long_name}");

    let _capacity = c.u24_le();
    c.expect_fixed(&[0x00]);
    let _standing_capacity = c.u24_le();
    c.expect_fixed(&[0x00]);

    // Per PKF_FORMAT.md §6.4, pitch_size is genuinely per-team variable, not
    // a fixed constant -- confirmed again here (many files' bytes here
    // don't match PITCH_SIZE at all). Skip without asserting.
    let _pitch_size = c.take(4);

    let _founded = c.u16_le();
    let _members = c.u24_le();
    c.expect_fixed(&[0x00]);

    let president = c.lossy_string(map);

    println!(
        "== {tag} == short_name={short_name} stadium_name={stadium_name} long_name={long_name} president={president}"
    );

    let _budget = c.u24_le();
    let _affiliate1 = c.u16_le();
    let _affiliate2 = c.u16_le();

    for _ in 0..10 {
        c.u8();
        c.u8();
    }
    c.take(14); // team stats

    c.take(JORNADA_LEN);
    c.take(PALMARES_LEN);

    // tactics
    let _formation_blob = c.opaque_blob();
    c.take(7);

    if is_foreign {
        println!("   (foreign flag set -- no coach/players per override format)");
        return;
    }

    // coach
    c.expect_fixed(&[COACH_MARKER]);
    let _pointer = c.u16_le();
    let coach_short = c.lossy_string(map);
    let coach_long = c.lossy_string(map);
    let _profile = c.lossy_string(map);
    let _systems = c.lossy_string(map);
    let _palmares = c.lossy_string(map);
    let _anecdotes = c.lossy_string(map);
    let _last_season = c.lossy_string(map);
    let _career_coach = c.lossy_string(map);
    if c.peek_u8() == Some(COACH_WAS_PLAYER_MARKER) {
        c.u8();
        let _career_player = c.lossy_string(map);
    }
    let _declarations = c.lossy_string(map);

    println!("   coach: short_name={coach_short} long_name={coach_long}");

    let mut player_idx = 0;
    while c.pos < bytes.len() {
        let start = c.pos;
        if c.peek_u8() != Some(PLAYER_MARKER) {
            println!("   (stopped player walk at pos {start}: no marker byte)");
            break;
        }
        c.u8();
        let _pointer = c.u16_le();
        let _number = c.u8();
        let p_short = c.lossy_string(map);
        let p_long = c.lossy_string(map);
        c.u8(); // slot
        c.u8(); // origin
        for _ in 0..6 {
            c.u8();
        } // roles
        c.u8(); // nationality
        c.u8(); // skin
        c.u8(); // hair
        c.u8(); // demarcation
        c.take(4); // birth date
        c.u8(); // height
        c.u8(); // weight
        c.u8(); // birth_country
        let _birthplace = c.lossy_string(map);
        let _debut_club = c.lossy_string(map);
        let _international = c.lossy_string(map);
        let _profile = c.lossy_string(map);
        let _characteristics = c.lossy_string(map);
        let _palmares = c.lossy_string(map);
        let _internationality = c.lossy_string(map);
        let _anecdotes = c.lossy_string(map);
        let _last_season = c.lossy_string(map);
        let _career = c.lossy_string(map);
        c.take(10); // attrs

        println!("   player[{player_idx}]: short_name={p_short} long_name={p_long}");
        player_idx += 1;
        if player_idx > 60 {
            println!("   (safety cap: stopping after 60 players)");
            break;
        }
    }
}

fn main() {
    let dir = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: investigate_editor_pm_dbc_full <dir-with-EQ97-files> [charmap-path]");
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
    // std::panic::set_hook(Box::new(|_| {})); // silence default panic backtraces; we log failures ourselves

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
    let mut n_panic = 0usize;
    for path in &entries {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                println!("file={} ERROR reading: {e}", path.display());
                continue;
            }
        };
        let tag = path.file_name().unwrap().to_string_lossy().to_string();
        let map_ref = &map;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            walk_file(&bytes, map_ref, &tag);
        }));
        match result {
            Ok(()) => n_ok += 1,
            Err(_) => {
                n_panic += 1;
                println!("file={tag} WALK FAILED (layout mismatch or eof) -- see above for partial output");
            }
        }
    }
    eprintln!("done: {n_ok} fully walked, {n_panic} failed partway");
}
