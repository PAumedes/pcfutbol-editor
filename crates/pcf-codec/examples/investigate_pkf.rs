//! Ad-hoc investigator for the `.PKF` "teams container" format (PLAN.md
//! Appendix B) — NOT part of the codec contract, NOT wired into any test.
//! Written during a reverse-engineering pass on `EQ003003.PKF` to answer
//! one question: is the per-team/per-player `Dbc` banner embedded verbatim
//! (uncompressed, unencrypted) inside the bigger container file, and if so,
//! how often and where?
//!
//! Usage (inside the dev container, from repo root):
//!   cargo run -p pcf-codec --example investigate_pkf -- /path/to/EQ003003.PKF
//!
//! Findings from the real file are written up in the investigation report,
//! not here — this only prints raw evidence (offsets, deltas, hex).

use std::env;
use std::fs;

/// The banner as it actually appears in real `.PKF` files: NO space between
/// "(c)" and "1996". This investigation is what caught `pcf_codec::dbc`'s
/// `BANNER` constant having an extra space (a transcription typo inherited
/// from PLAN.md's prose, though its own cited hex was correct) — since
/// fixed to match this. Kept as `BANNER_REAL` / `BANNER_CODEC` below purely
/// so this tool still shows both spellings side by side for any other
/// `.PKF` file you point it at.
const BANNER_REAL: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";

/// The banner as currently hardcoded in `pcf_codec::dbc::BANNER` (fixed to
/// match `BANNER_REAL` — kept as a separate constant here only so this
/// tool still reports both counts if you re-run it against a new file).
const BANNER_CODEC: &[u8] = b"Copyright (c)1996 Dinamic Multimedia";

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return out;
    }
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

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: investigate_pkf <path-to-pkf>");
        std::process::exit(1);
    });
    let bytes = fs::read(&path).expect("failed to read input file");
    println!("file: {} ({} bytes)", path, bytes.len());

    // 1. Raw banner search (with and without the space, per the codec's
    //    BANNER constant vs. what we've actually observed in real files).
    let hits_codec = find_all(&bytes, BANNER_CODEC);
    let hits_real = find_all(&bytes, BANNER_REAL);
    println!(
        "banner WITH space (codec's BANNER const): {} occurrence(s)",
        hits_codec.len()
    );
    println!(
        "banner WITHOUT space (real, observed):     {} occurrence(s)",
        hits_real.len()
    );

    if hits_real.is_empty() {
        println!("no banner found under either spelling — stopping here.");
        return;
    }

    println!(
        "first occurrence @ offset {} (0x{:X})",
        hits_real[0], hits_real[0]
    );
    println!(
        "last  occurrence @ offset {} (0x{:X})",
        hits_real[hits_real.len() - 1],
        hits_real[hits_real.len() - 1]
    );

    // 2. Delta stats between consecutive occurrences — tells us whether
    //    these look like fixed-size or variable-size records, and whether
    //    there's a bimodal distribution (e.g. player-sized vs team-sized
    //    gaps).
    let deltas: Vec<usize> = hits_real.windows(2).map(|w| w[1] - w[0]).collect();
    if !deltas.is_empty() {
        let min = *deltas.iter().min().unwrap();
        let max = *deltas.iter().max().unwrap();
        let sum: usize = deltas.iter().sum();
        let avg = sum as f64 / deltas.len() as f64;
        let large = deltas.iter().filter(|&&d| d > 10_000).count();
        let medium = deltas.iter().filter(|&&d| d > 3_000 && d <= 10_000).count();
        let small = deltas.iter().filter(|&&d| d <= 3_000).count();
        println!(
            "deltas: n={} min={} max={} avg={:.1}  (small<=3000: {}, medium 3000-10000: {}, large>10000: {})",
            deltas.len(),
            min,
            max,
            avg,
            small,
            medium,
            large
        );
    }

    // 3. Region before the first banner occurrence: dump it so it can be
    //    eyeballed for directory/header structure (this is where we found
    //    a repeating ~32-byte-period block on EQ003003.PKF).
    let head_len = hits_real[0].min(bytes.len());
    println!(
        "\n--- bytes [0, {}) preceding first banner (hex) ---",
        head_len
    );
    for (i, chunk) in bytes[..head_len].chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        println!("{:08x}: {}", i * 16, hex.join(" "));
    }

    // 4. Bytes immediately following the first banner — where the codec
    //    would expect MAGIC_FE06 (0xFE 0x06). Print them raw so a human can
    //    check by eye.
    let after = hits_real[0] + BANNER_REAL.len();
    let after_end = (after + 32).min(bytes.len());
    println!(
        "\n--- 32 bytes immediately after first banner (offset {}) ---",
        after
    );
    println!("{:02x?}", &bytes[after..after_end]);
    println!(
        "(codec expects [0xFE, 0x06] here; real byte is 0x{:02x})",
        bytes[after]
    );
}
