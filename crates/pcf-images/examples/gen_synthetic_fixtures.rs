//! Dev tool: (re)generate the synthetic test fixtures under
//! `fixtures/synthetic-images-agent-b/`. Not part of the public API and
//! not wired into any build/test path -- run manually with
//! `cargo run -p pcf-images --example gen_synthetic_fixtures` if the
//! fixtures ever need to be regenerated.
//!
//! Everything here is PLACEHOLDER synthetic data, not real game assets.

use pcf_images::{write_bmp8, Bmp8, Palette};

fn main() -> std::io::Result<()> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic-images-agent-b");
    std::fs::create_dir_all(&out_dir)?;

    let palette = Palette::synthetic_placeholder();

    // Raw 768-byte RGB palette dump (see Palette::from_raw_rgb_bytes).
    let mut raw_palette = Vec::with_capacity(256 * 3);
    for &(r, g, b) in palette.colors() {
        raw_palette.push(r);
        raw_palette.push(g);
        raw_palette.push(b);
    }
    std::fs::write(out_dir.join("synthetic_palette.rgb"), &raw_palette)?;

    // Tiny 4x4 checkerboard using two known palette indices.
    let mut pixels = Vec::with_capacity(16);
    for row in 0..4u8 {
        for col in 0..4u8 {
            pixels.push(if (row + col) % 2 == 0 { 10 } else { 250 });
        }
    }
    let checker = Bmp8::new(4, 4, palette.clone(), pixels).expect("valid pixel count");
    std::fs::write(
        out_dir.join("synthetic_checker_4x4.bmp"),
        write_bmp8(&checker),
    )?;

    // Tiny 2x3 gradient-ish strip using a range of indices.
    let strip_pixels: Vec<u8> = (0..6u16).map(|i| (i * 40) as u8).collect();
    let strip = Bmp8::new(2, 3, palette, strip_pixels).expect("valid pixel count");
    std::fs::write(out_dir.join("synthetic_strip_2x3.bmp"), write_bmp8(&strip))?;

    println!("wrote synthetic fixtures to {}", out_dir.display());
    Ok(())
}
