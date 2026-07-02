//! End-to-end demo of `pcf_codec::container`: parses a whole `.PKF` file's
//! domestic team records (PKF_FORMAT.md §6) and prints a summary table.
//!
//! Unlike the earlier investigation tools (`investigate_pkf.rs`,
//! `investigate_pkf_dir.rs`, `investigate_domestic_team.rs`,
//! `dump_stub_table.rs`), this one exercises the *real* production parser
//! in `crates/pcf-codec/src/container.rs` (not a self-contained
//! reimplementation) — it's meant to double as a smoke test that the
//! module actually works end to end against a real file, and as a quick
//! way to see how many of the file's domestic team records currently parse
//! cleanly vs. fail (and why).
//!
//! Usage (inside the dev container):
//!   cargo run -p pcf-codec --example dump_container -- <path-to-pkf> [charmap-path]
//! `charmap-path` defaults to `fixtures/charmap/confirmed_real_map_v2.txt`
//! (the more complete of the two real charmaps — see
//! `fixtures/charmap/README.md`).

use std::env;
use std::path::PathBuf;

use pcf_codec::{parse_pkf_container_verbose, CharMap};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: dump_container <path-to-pkf> [charmap-path]");
        std::process::exit(1);
    });
    let charmap_path = env::args().nth(2).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("confirmed_real_map_v2.txt")
            .to_str()
            .unwrap()
            .to_string()
    });

    let charmap = match CharMap::load(&charmap_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "failed to load charmap {charmap_path}: {}: {}",
                e.code, e.message
            );
            std::process::exit(1);
        }
    };

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "file: {path} ({} bytes), charmap: {charmap_path}",
        bytes.len()
    );

    let outcomes = parse_pkf_container_verbose(&bytes, &charmap);
    println!("found {} domestic team record(s)\n", outcomes.len());

    println!(
        "{:<24} {:>10} {:>8}  {:<28} coach",
        "team", "capacity", "founded", "president"
    );
    println!("{}", "-".repeat(100));

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;
    for outcome in &outcomes {
        match &outcome.result {
            Ok(record) => {
                ok_count += 1;
                let coach_name = record
                    .coach
                    .as_ref()
                    .map(|c| c.short_name.as_str())
                    .unwrap_or("(none found)");
                println!(
                    "{:<24} {:>10} {:>8}  {:<28} {}",
                    record.short_name,
                    record.capacity,
                    record.founded,
                    record.president,
                    coach_name
                );
            }
            Err(e) => {
                fail_count += 1;
                println!(
                    "(offset {:#x}) FAILED TO PARSE: {}: {}",
                    outcome.start_offset, e.code, e.message
                );
            }
        }
    }

    println!(
        "\n{ok_count} parsed successfully, {fail_count} failed, out of {} total domestic \
         record(s) found.",
        outcomes.len()
    );
}
