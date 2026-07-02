//! Agent A acceptance tests (PLAN.md §6): round-trip gate, decoded human
//! values, single-field diff test, and the typed unknown-glyph error path.
//!
//! Runs against `fixtures/golden/synthetic_minimal.dbc`, a SYNTHETIC
//! placeholder (see `fixtures/golden/README.md`) — not a real game file.
//! Once the user supplies real golden DBCs, this same harness (or the one
//! in `tests/` at the workspace root) is what proves byte-fidelity against
//! them; today it only proves the codec is internally consistent.

use pcf_codec::synthetic::load_synthetic_charmap;
use pcf_codec::DbcCodec;
use pcf_model::Dbc;

fn golden_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("golden")
        .join("synthetic_minimal.dbc")
}

fn read_golden() -> Vec<u8> {
    std::fs::read(golden_path()).expect(
        "fixtures/golden/synthetic_minimal.dbc missing — run \
         `cargo run -p pcf-codec --example build_synthetic_golden`",
    )
}

#[test]
fn round_trip_is_byte_identical() {
    let charmap = load_synthetic_charmap().unwrap();
    let original = read_golden();

    let dbc = Dbc::read(&original, &charmap).expect("synthetic golden must parse");
    let rewritten = dbc.write(&charmap).expect("re-encoding must succeed");

    assert_eq!(
        rewritten, original,
        "write(read(bytes)) must equal bytes for the synthetic golden fixture"
    );
}

#[test]
fn decodes_known_team_to_expected_human_values() {
    let charmap = load_synthetic_charmap().unwrap();
    let bytes = read_golden();
    let dbc = Dbc::read(&bytes, &charmap).unwrap();

    assert_eq!(dbc.team.short_name, "BOCA");
    assert_eq!(dbc.team.long_name, "Boca Juniors");
    assert_eq!(dbc.team.capacity, 87_000);
    assert_eq!(dbc.team.founded, 1902);
    assert_eq!(dbc.team.budget, 18_000);
    assert_eq!(dbc.players.len(), 2);
    assert_eq!(dbc.players[0].short_name, "Riquelme");
}

#[test]
fn editing_one_field_changes_only_expected_bytes() {
    let charmap = load_synthetic_charmap().unwrap();
    let bytes = read_golden();
    let mut dbc = Dbc::read(&bytes, &charmap).unwrap();

    // Chosen so *both* LE bytes of the u16 change (1902 = 0x076E vs.
    // 2200 = 0x0898) — a value that only flips the low byte would make
    // this assertion accidentally pass with 1 diff instead of the 2 we're
    // actually claiming.
    dbc.team.founded = 2200;

    let edited_bytes = dbc.write(&charmap).unwrap();

    // founded is a fixed-width u16 field, so editing it must not change the
    // overall file length...
    assert_eq!(
        edited_bytes.len(),
        bytes.len(),
        "editing a fixed-width numeric field must not change file length"
    );

    let diff_positions: Vec<usize> = bytes
        .iter()
        .zip(edited_bytes.iter())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();

    // ...and must touch exactly the 2 bytes of the `founded` field, nothing
    // else in the record.
    assert_eq!(
        diff_positions.len(),
        2,
        "expected exactly 2 changed bytes (the `founded` u16), got diffs at {diff_positions:?}"
    );
}

#[test]
fn unknown_glyph_in_charmap_is_a_typed_error_not_a_panic() {
    let charmap = load_synthetic_charmap().unwrap();
    let mut bytes = read_golden();

    // Corrupt a byte inside the short_name string's encoded bytes (right
    // after the 2-byte length prefix that opens the team record, which is
    // the very first field in the file after the fixed header) with a
    // value absent from the synthetic charmap.
    let header_len = "Copyright (c)1996 Dinamic Multimedia".len() + 2 + 2 + 1 + 1;
    let short_name_bytes_start = header_len + 2; // skip the 2-byte length prefix
    bytes[short_name_bytes_start] = 0xFF; // not in the synthetic charmap

    let err = Dbc::read(&bytes, &charmap).expect_err("corrupted glyph must not decode");
    assert_eq!(err.code, "charmap_unknown_byte");
    assert!(err.message.contains("0xFF"));
    assert!(
        err.context.is_some(),
        "error should carry a byte offset in context"
    );
}
