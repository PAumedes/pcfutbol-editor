//! Integration tests against the on-disk synthetic fixtures in
//! `fixtures/synthetic-images-agent-b/` (hand-constructed placeholders,
//! not real game assets -- see the README in that folder).

use std::path::PathBuf;

use pcf_images::{read_bmp8, write_bmp8, Palette};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic-images-agent-b")
}

#[test]
fn synthetic_checker_bmp_round_trips_byte_identically() {
    let path = fixtures_dir().join("synthetic_checker_4x4.bmp");
    let original_bytes = std::fs::read(&path).expect("fixture exists");

    let parsed = read_bmp8(&original_bytes).expect("valid synthetic BMP");
    assert_eq!(parsed.width, 4);
    assert_eq!(parsed.height, 4);

    let rewritten_bytes = write_bmp8(&parsed);
    assert_eq!(
        original_bytes, rewritten_bytes,
        "round trip must be byte-identical"
    );
}

#[test]
fn synthetic_strip_bmp_round_trips_byte_identically() {
    let path = fixtures_dir().join("synthetic_strip_2x3.bmp");
    let original_bytes = std::fs::read(&path).expect("fixture exists");

    let parsed = read_bmp8(&original_bytes).expect("valid synthetic BMP");
    assert_eq!(parsed.width, 2);
    assert_eq!(parsed.height, 3);

    let rewritten_bytes = write_bmp8(&parsed);
    assert_eq!(
        original_bytes, rewritten_bytes,
        "round trip must be byte-identical"
    );
}

#[test]
fn synthetic_bmps_use_the_active_placeholder_palette() {
    let active = Palette::active();
    for name in ["synthetic_checker_4x4.bmp", "synthetic_strip_2x3.bmp"] {
        let bytes = std::fs::read(fixtures_dir().join(name)).expect("fixture exists");
        let parsed = read_bmp8(&bytes).expect("valid synthetic BMP");
        assert!(
            active.matches(parsed.palette.colors()),
            "{name} should conform to the active placeholder palette"
        );
    }
}

#[test]
fn palette_loads_from_the_on_disk_raw_rgb_fixture() {
    let raw = std::fs::read(fixtures_dir().join("synthetic_palette.rgb")).expect("fixture exists");
    let loaded = Palette::from_raw_rgb_bytes(&raw).expect("valid 768-byte palette");
    assert_eq!(loaded, Palette::active());
}
