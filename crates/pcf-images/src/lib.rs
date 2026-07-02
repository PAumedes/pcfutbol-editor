//! 256-color BMP + palette handling for crests and photos.
//!
//! Owned by Agent B (PLAN.md §6 Agent B brief, Appendix B asset folders,
//! §9 risk #5 palette fidelity). Scope: read/write 8-bit indexed BMPs for
//! `MINIESC`/`NANOESC` crests and `MINIFOTOS` photos, palette
//! extraction/repair for imported truecolor art, and pointer-derived
//! output filenames.
//!
//! No real crest/photo BMP or real game `.pal` file has been supplied by
//! the user yet. `palette::Palette::active()` is a clearly-marked
//! synthetic placeholder -- see its doc comment for the one-line swap
//! needed once a real palette is available.

pub mod bmp;
pub mod import;
pub mod naming;
pub mod palette;

pub use bmp::{read_bmp8, write_bmp8, Bmp8};
pub use import::{expected_dimensions, import_image, quantize_to_palette};
pub use naming::{filename_for, AssetKind};
pub use palette::Palette;
