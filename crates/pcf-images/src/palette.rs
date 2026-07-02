//! 256-color palette handling (PLAN.md §9 risk #5: palette fidelity).
//!
//! The game only renders 8-bit indexed BMPs correctly if every pixel index
//! resolves through the *game's* palette. Naively indexing a truecolor
//! image (nearest color against an arbitrary/invented palette) will look
//! wrong or corrupt in-game if that palette isn't the one the engine
//! actually uses for MINIESC/NANOESC/MINIFOTOS.
//!
//! We don't have the real game palette yet (no `.pal` sample has been
//! supplied). `Palette::active()` is the single seam that will need to
//! change once we get one -- see the TODO below.

use pcf_model::PcfError;

/// A 24-bit RGB triple.
pub type Rgb = (u8, u8, u8);

/// A 256-entry color table, in on-disk order (index 0..=255).
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    colors: [Rgb; 256],
}

impl Palette {
    /// Build a palette from exactly 256 RGB triples.
    pub fn from_colors(colors: [Rgb; 256]) -> Self {
        Self { colors }
    }

    pub fn colors(&self) -> &[Rgb; 256] {
        &self.colors
    }

    pub fn get(&self, index: u8) -> Rgb {
        self.colors[index as usize]
    }

    /// Nearest palette index to `rgb` by squared Euclidean distance in RGB
    /// space. Ties resolve to the lowest index. This is the "palette
    /// repair" step: any imported truecolor pixel is remapped onto this
    /// table rather than being indexed arbitrarily.
    pub fn nearest_index(&self, rgb: Rgb) -> u8 {
        let (r, g, b) = (rgb.0 as i32, rgb.1 as i32, rgb.2 as i32);
        let mut best_index = 0usize;
        let mut best_dist = i32::MAX;
        for (i, &(pr, pg, pb)) in self.colors.iter().enumerate() {
            let dr = r - pr as i32;
            let dg = g - pg as i32;
            let db = b - pb as i32;
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best_index = i;
            }
        }
        best_index as u8
    }

    /// True when every entry of `colors` matches this palette exactly, in
    /// order. Used to assert imported BMPs stay palette-conformant.
    pub fn matches(&self, colors: &[Rgb; 256]) -> bool {
        &self.colors == colors
    }

    /// Parse a palette from a flat buffer of 256 * 3 raw RGB bytes
    /// (`R,G,B` per entry, no header). This is a placeholder loader: real
    /// `.pal` files may use a different container (e.g. a RIFF PAL chunk,
    /// or BGR order matching the BMP color table). Once the user supplies
    /// a real game palette we should confirm the byte layout here.
    ///
    /// TODO(B): confirm against real MINIESC/NANOESC/MINIFOTOS samples
    /// once user supplies them.
    pub fn from_raw_rgb_bytes(bytes: &[u8]) -> Result<Self, PcfError> {
        if bytes.len() != 256 * 3 {
            return Err(PcfError::new(
                "pcf_images.palette.bad_length",
                format!(
                    "expected a 768-byte raw RGB palette, got {} bytes",
                    bytes.len()
                ),
            ));
        }
        let mut colors = [(0u8, 0u8, 0u8); 256];
        for (i, chunk) in bytes.chunks_exact(3).enumerate() {
            colors[i] = (chunk[0], chunk[1], chunk[2]);
        }
        Ok(Self { colors })
    }

    /// A hand-constructed, deterministic 256-color palette.
    ///
    /// PLACEHOLDER -- this is NOT the real Apertura 98/99 game palette. It
    /// exists only so we can write and round-trip tests before a real
    /// `.pal` / crest sample is available. It is a simple 8x8x4 color cube
    /// (8 levels of R, 8 of G, 4 of B) so every entry is distinct and
    /// nearest-color mapping is well-defined and deterministic.
    ///
    /// TODO(B): confirm against real MINIESC/NANOESC/MINIFOTOS samples
    /// once user supplies them, then swap the body of `active()` below.
    pub fn synthetic_placeholder() -> Self {
        let mut colors = [(0u8, 0u8, 0u8); 256];
        let mut i = 0usize;
        for r in 0..8u32 {
            for g in 0..8u32 {
                for b in 0..4u32 {
                    let rr = (r * 255 / 7) as u8;
                    let gg = (g * 255 / 7) as u8;
                    let bb = (b * 255 / 3) as u8;
                    colors[i] = (rr, gg, bb);
                    i += 1;
                }
            }
        }
        debug_assert_eq!(i, 256);
        Self { colors }
    }

    /// The palette every import/export in this crate conforms to.
    ///
    /// This is the ONE seam to change when the user supplies a real game
    /// palette: swap the body for
    /// `Palette::from_raw_rgb_bytes(&std::fs::read(real_pal_path)?)` (or
    /// whatever the confirmed `.pal` layout turns out to be).
    ///
    /// TODO(B): confirm against real MINIESC/NANOESC/MINIFOTOS samples
    /// once user supplies them.
    pub fn active() -> Self {
        Self::synthetic_placeholder()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_placeholder_has_256_distinct_entries_mostly() {
        let palette = Palette::synthetic_placeholder();
        assert_eq!(palette.colors().len(), 256);
    }

    #[test]
    fn nearest_index_finds_exact_match() {
        let palette = Palette::synthetic_placeholder();
        let target = palette.get(42);
        assert_eq!(palette.nearest_index(target), 42);
    }

    #[test]
    fn from_raw_rgb_bytes_rejects_wrong_length() {
        let err = Palette::from_raw_rgb_bytes(&[0u8; 10]).unwrap_err();
        assert_eq!(err.code, "pcf_images.palette.bad_length");
    }

    #[test]
    fn from_raw_rgb_bytes_round_trips_synthetic_palette() {
        let original = Palette::synthetic_placeholder();
        let mut raw = Vec::with_capacity(256 * 3);
        for &(r, g, b) in original.colors() {
            raw.push(r);
            raw.push(g);
            raw.push(b);
        }
        let parsed = Palette::from_raw_rgb_bytes(&raw).expect("valid length");
        assert_eq!(parsed, original);
    }
}
