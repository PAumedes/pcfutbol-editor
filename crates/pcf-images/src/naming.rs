//! Output filenames for crest/photo assets, derived from a pointer + kind.
//!
//! PLAN.md Appendix B says the asset folders `MINIESC`, `NANOESC` (crests)
//! and `MINIFOTOS` (photos) sit as siblings under `DBDAT`, holding 8-bit
//! BMPs with a valid game palette -- but it does not spell out the exact
//! filename convention inside those folders (unlike the DBC override name,
//! which Appendix B nails down precisely: `EQ97` + 4-digit decimal team
//! pointer). We mirror that DBC convention here as the best-guess default:
//! a fixed per-kind prefix + the 4-digit decimal pointer + `.BMP`. The
//! prefix carries the kind so a filename is self-describing even if it
//! ever needs to live outside its expected folder.
//!
//! TODO(B): confirm against real MINIESC/NANOESC/MINIFOTOS samples once
//! user supplies them -- the prefix and zero-padding width below are
//! placeholders, not verified against a real asset folder listing.

/// Which of the three asset kinds a pointer identifies, and therefore
/// which folder/size/naming rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// `MINIESC` -- team crest, keyed by team pointer.
    Crest,
    /// `NANOESC` -- small team crest, keyed by team pointer.
    SmallCrest,
    /// `MINIFOTOS` -- player photo, keyed by player pointer.
    Photo,
}

impl AssetKind {
    /// The sibling folder name under `DBDAT` (Appendix B).
    pub fn folder_name(self) -> &'static str {
        match self {
            AssetKind::Crest => "MINIESC",
            AssetKind::SmallCrest => "NANOESC",
            AssetKind::Photo => "MINIFOTOS",
        }
    }

    fn filename_prefix(self) -> &'static str {
        match self {
            AssetKind::Crest => "ESC",
            AssetKind::SmallCrest => "NSC",
            AssetKind::Photo => "FOT",
        }
    }
}

/// Derive the output BMP filename for `pointer` (team pointer for crest
/// kinds, player pointer for `Photo`) and `kind`.
///
/// e.g. `filename_for(9013, AssetKind::Crest)` -> `"ESC9013.BMP"`.
pub fn filename_for(pointer: u16, kind: AssetKind) -> String {
    format!("{}{:04}.BMP", kind.filename_prefix(), pointer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_for_crest_matches_dbc_pointer_convention() {
        assert_eq!(filename_for(9013, AssetKind::Crest), "ESC9013.BMP");
    }

    #[test]
    fn filename_for_small_crest_uses_distinct_prefix() {
        assert_eq!(filename_for(9013, AssetKind::SmallCrest), "NSC9013.BMP");
    }

    #[test]
    fn filename_for_photo_uses_player_pointer() {
        assert_eq!(filename_for(51, AssetKind::Photo), "FOT0051.BMP");
    }

    #[test]
    fn filename_for_zero_pads_small_pointers() {
        assert_eq!(filename_for(1, AssetKind::Crest), "ESC0001.BMP");
    }

    #[test]
    fn folder_name_matches_appendix_b() {
        assert_eq!(AssetKind::Crest.folder_name(), "MINIESC");
        assert_eq!(AssetKind::SmallCrest.folder_name(), "NANOESC");
        assert_eq!(AssetKind::Photo.folder_name(), "MINIFOTOS");
    }
}
