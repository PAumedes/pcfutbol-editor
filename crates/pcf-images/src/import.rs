//! Import truecolor art (crests/photos supplied by the user) into a
//! palette-conformant 8-bit indexed [`Bmp8`] (PLAN.md §9 risk #5).
//!
//! The critical step is *palette repair*: every pixel of the source image
//! is remapped onto [`Palette::active`] via nearest-color search, so the
//! resulting BMP never contains a color outside the game's palette --
//! naively writing the source's own colors as indices would render as
//! garbage in-game.

use image::RgbImage;
use pcf_model::PcfError;

use crate::bmp::Bmp8;
use crate::naming::AssetKind;
use crate::palette::Palette;

/// Expected pixel dimensions for each asset kind.
///
/// PLACEHOLDER sizes -- no real MINIESC/NANOESC/MINIFOTOS sample has been
/// supplied yet, so these are reasonable guesses (crests roughly square
/// and small, photos a small portrait) rather than confirmed values.
///
/// TODO(B): confirm against real MINIESC/NANOESC/MINIFOTOS samples once
/// user supplies them.
pub fn expected_dimensions(kind: AssetKind) -> (u32, u32) {
    match kind {
        AssetKind::Crest => (24, 24),
        AssetKind::SmallCrest => (16, 16),
        AssetKind::Photo => (40, 48),
    }
}

/// Quantize every pixel of `img` onto `palette` via nearest-color search.
/// This is the palette-repair step; it never emits an index whose color
/// isn't exactly a `palette` entry.
pub fn quantize_to_palette(img: &RgbImage, palette: &Palette) -> Vec<u8> {
    img.pixels()
        .map(|p| palette.nearest_index((p[0], p[1], p[2])))
        .collect()
}

/// Import a truecolor source image as `kind`, validating its dimensions
/// against [`expected_dimensions`] and quantizing it onto `palette`
/// (typically [`Palette::active`]).
pub fn import_image(img: &RgbImage, kind: AssetKind, palette: &Palette) -> Result<Bmp8, PcfError> {
    let (width, height) = (img.width(), img.height());
    let (expected_w, expected_h) = expected_dimensions(kind);
    if width != expected_w || height != expected_h {
        return Err(PcfError::new(
            "pcf_images.import.bad_dimensions",
            format!(
                "{:?} must be {expected_w}x{expected_h}, got {width}x{height}",
                kind
            ),
        )
        .with_context(format!("kind={kind:?}")));
    }

    let pixels = quantize_to_palette(img, palette);
    Bmp8::new(width, height, palette.clone(), pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn solid_image(width: u32, height: u32, color: (u8, u8, u8)) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb([color.0, color.1, color.2]))
    }

    #[test]
    fn import_rejects_wrong_dimensions_with_typed_error() {
        let palette = Palette::synthetic_placeholder();
        let img = solid_image(10, 10, (200, 30, 30));
        let err = import_image(&img, AssetKind::Crest, &palette).unwrap_err();
        assert_eq!(err.code, "pcf_images.import.bad_dimensions");
    }

    #[test]
    fn import_produces_bmp_conforming_to_palette() {
        let palette = Palette::synthetic_placeholder();
        let (w, h) = expected_dimensions(AssetKind::Crest);
        // An off-palette truecolor color that must be remapped, not
        // passed through.
        let img = solid_image(w, h, (137, 61, 200));
        let bmp = import_image(&img, AssetKind::Crest, &palette).expect("valid dimensions");

        assert_eq!(bmp.width, w);
        assert_eq!(bmp.height, h);
        assert!(palette.matches(bmp.palette.colors()));
        // Every pixel index must resolve to a color that is actually in
        // the palette (trivially true since indices are u8 into a
        // 256-entry table, but assert the round-trip anyway).
        for &idx in &bmp.pixels {
            let color = bmp.palette.get(idx);
            assert!(palette.colors().contains(&color));
        }
    }

    #[test]
    fn import_of_palette_exact_color_maps_to_that_exact_index() {
        let palette = Palette::synthetic_placeholder();
        let target_index = 200u8;
        let target_color = palette.get(target_index);
        let (w, h) = expected_dimensions(AssetKind::SmallCrest);
        let img = solid_image(w, h, target_color);

        let bmp = import_image(&img, AssetKind::SmallCrest, &palette).expect("valid dimensions");
        assert!(bmp.pixels.iter().all(|&idx| idx == target_index));
    }
}
