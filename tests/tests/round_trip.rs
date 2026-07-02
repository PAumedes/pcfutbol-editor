//! Byte-fidelity round-trip gate over `fixtures/golden/*.dbc`
//! (PLAN.md §1 "Byte-exact", §6 Agent A acceptance, §7 milestone M1).
//! Owned by Agent G.
//!
//! Design goals, per the Agent G brief:
//!
//! - **Never fail the build just because real fixtures haven't been
//!   supplied yet.** If `fixtures/golden/` has no real `*.dbc` files (only
//!   synthetic placeholders, or nothing at all), this test PASSES and
//!   reports the fixture set is empty — see `fixtures/README.md` for the
//!   checklist that turns this from a no-op into the real gate.
//! - **Never fail the *whole workspace's* `cargo test` just because
//!   another agent's crate is mid-flight.** While `pcf-codec` (Agent A) was
//!   still a stub, a direct, always-compiled call to `Dbc::read`/`write`
//!   failed to *compile*, and `cargo test --workspace` aborted the entire
//!   run (no test results for ANY crate) on that one compile error —
//!   verified while building this harness. That's a much bigger problem
//!   than "the gate is a no-op," so the codec-calling assertion lives
//!   behind the `codec-ready` Cargo feature (see `tests/Cargo.toml`) rather
//!   than being unconditional.
//!
//! `pcf-codec` now implements `DbcCodec` for `pcf_model::Dbc` (real
//! `read`/`write`), so `codec-ready` is on by default. If a future
//! contract change in `pcf-codec` breaks this file's compilation again
//! mid-flight, drop `codec-ready` from `tests/Cargo.toml`'s `default`
//! (one line) rather than letting it block every other crate's tests.

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "codec-ready")]
use pcf_codec::{CharMap, DbcCodec};
#[cfg(feature = "codec-ready")]
use pcf_model::Dbc;

/// `fixtures/golden` relative to the workspace root (this crate lives at
/// `<workspace>/tests`, so `..` gets us back to the root).
const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/golden");

/// Filenames (case-insensitive) containing this marker are synthetic
/// placeholders dropped by other agents for their own TDD — see
/// `fixtures/golden/README.md`. They're excluded from the round-trip gate
/// and from the "real fixtures validated" count.
const SYNTHETIC_MARKER: &str = "synthetic";

fn is_synthetic(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase().contains(SYNTHETIC_MARKER))
        .unwrap_or(false)
}

/// All `*.dbc` files directly under `dir`, real and synthetic alike.
/// Returns an empty vec (never errors) if `dir` doesn't exist yet — that's
/// the expected state before the user supplies fixtures.
fn collect_dbc_files(dir: &str) -> Vec<PathBuf> {
    let dir = Path::new(dir);
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("dbc"))
                    .unwrap_or(false)
        })
        .collect()
}

fn classify(dir: &str) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let files = collect_dbc_files(dir);
    files.into_iter().partition(|p| is_synthetic(p))
}

/// Real `fixtures/charmap/map.txt` if the user has supplied one, else
/// Agent A's synthetic placeholder. The round-trip identity
/// `write(read(bytes)) == bytes` holds under either charmap as long as
/// decoding doesn't hit an unrecognized byte — so this is a correct
/// choice for the byte-fidelity gate itself, but a real fixture that uses
/// glyphs outside the synthetic map's invented alphabet will (correctly)
/// fail to decode until a real `map.txt` is supplied — see
/// fixtures/README.md item 2.
#[cfg(feature = "codec-ready")]
fn load_charmap() -> CharMap {
    let real = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/charmap/map.txt");
    if Path::new(real).is_file() {
        CharMap::load(real).expect("fixtures/charmap/map.txt should parse")
    } else {
        pcf_codec::synthetic::load_synthetic_charmap()
            .expect("synthetic charmap fixture should parse")
    }
}

/// The real gate: for every non-synthetic `*.dbc` under `fixtures/golden/`,
/// assert `Dbc::write(Dbc::read(bytes)) == bytes` (PLAN.md's own phrasing).
/// Requires `pcf-codec` to actually implement `Dbc::read`/`Dbc::write` —
/// enable with `--features codec-ready` (see the module doc comment).
#[cfg(feature = "codec-ready")]
#[test]
fn round_trip_golden_fixtures() {
    let (synthetic, real) = classify(GOLDEN_DIR);

    if real.is_empty() {
        println!(
            "[fixtures] round-trip gate: 0 real fixtures found under \
             fixtures/golden/ ({} synthetic placeholder(s) present). This is \
             a graceful no-op, not a failure — see fixtures/README.md for \
             what to supply to make the gate meaningful.",
            synthetic.len()
        );
        return;
    }

    let charmap = load_charmap();
    let mut validated = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &real {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{}: could not read file ({e})", path.display()));
                continue;
            }
        };

        // The exact contract per PLAN.md §6 (Agent A) / §1 (byte-exact):
        //   Dbc::write(Dbc::read(bytes)) == bytes
        let decoded = match Dbc::read(&bytes, &charmap) {
            Ok(dbc) => dbc,
            Err(e) => {
                failures.push(format!(
                    "{}: failed to decode ({}: {})",
                    path.display(),
                    e.code,
                    e.message
                ));
                continue;
            }
        };
        let encoded = match decoded.write(&charmap) {
            Ok(bytes) => bytes,
            Err(e) => {
                failures.push(format!(
                    "{}: decoded but failed to re-encode ({}: {})",
                    path.display(),
                    e.code,
                    e.message
                ));
                continue;
            }
        };

        if encoded == bytes {
            validated += 1;
        } else {
            failures.push(format!(
                "{}: round-trip mismatch ({} bytes in, {} bytes out)",
                path.display(),
                bytes.len(),
                encoded.len()
            ));
        }
    }

    println!(
        "[fixtures] round-trip gate: {validated}/{} real fixture(s) byte-identical \
         ({} synthetic placeholder(s) skipped).",
        real.len(),
        synthetic.len()
    );

    assert!(
        failures.is_empty(),
        "round-trip gate failed for {}/{} real fixture(s):\n{}",
        failures.len(),
        real.len(),
        failures.join("\n")
    );
}

/// Placeholder that runs while `pcf-codec` is still a stub (default build,
/// `codec-ready` feature off). Never fails — it only reports what it found,
/// so `cargo test --workspace` stays green for everyone regardless of
/// pcf-codec's progress. See the module doc comment for how to graduate to
/// the real gate above.
#[cfg(not(feature = "codec-ready"))]
#[test]
fn round_trip_golden_fixtures() {
    let (synthetic, real) = classify(GOLDEN_DIR);
    println!(
        "[fixtures] round-trip gate: pcf-codec doesn't expose Dbc::read/Dbc::write yet \
         (Agent A's crate is mid-flight — PLAN.md §5 `A --> G`). Found {} real fixture(s) \
         and {} synthetic placeholder(s) under fixtures/golden/, but skipping byte-fidelity \
         validation until this test crate is built with `--features codec-ready` (flip the \
         default in tests/Cargo.toml once pcf-codec lands read/write). PASS (not a gate yet).",
        real.len(),
        synthetic.len()
    );
}
