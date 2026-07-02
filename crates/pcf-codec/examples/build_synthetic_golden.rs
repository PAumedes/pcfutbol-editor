//! Regenerates `fixtures/golden/synthetic_minimal.dbc` from the in-code
//! synthetic `Dbc` builder. Run with:
//!   cargo run -p pcf-codec --example build_synthetic_golden
//!
//! This is SYNTHETIC placeholder data (see fixtures/golden/README.md), not
//! a real game file. Regenerate only if `synthetic::synthetic_minimal_dbc`
//! changes.

use pcf_codec::synthetic::{load_synthetic_charmap, synthetic_minimal_dbc};
use pcf_codec::DbcCodec;

fn main() {
    let charmap = load_synthetic_charmap().expect("synthetic charmap fixture should parse");
    let dbc = synthetic_minimal_dbc();
    let bytes = dbc.write(&charmap).expect("synthetic dbc must encode");

    let out_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("golden")
        .join("synthetic_minimal.dbc");

    std::fs::write(&out_path, &bytes).expect("failed to write fixture");
    println!("wrote {} bytes to {}", bytes.len(), out_path.display());
}
