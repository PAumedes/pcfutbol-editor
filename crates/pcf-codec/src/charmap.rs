//! The DBC text codec: strings on disk are **not ASCII** — every glyph is a
//! custom single-byte substitution (PLAN.md §8, Appendix A). `CharMap` loads
//! that substitution table from a `map.txt`-style file at runtime so the
//! mapping is user/community-supplied data, never hardcoded or bundled.
//!
//! Only the mapping loaded from `fixtures/charmap/synthetic_map.txt` is
//! currently available, and it is a SYNTHETIC PLACEHOLDER (see the README in
//! that directory) built from the one verified data point we have: PLAN.md
//! Appendix A's "Real Madrid C.F." proof. Swapping in the real community
//! `map.txt` later requires no code changes to callers — only a new file at
//! the path passed to [`CharMap::load`], and possibly a tweak to
//! [`CharMap::parse`] if the real file's on-disk shape differs from the
//! `HH\tC` format assumed here.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use pcf_model::PcfError;

/// A loaded byte↔char substitution table for DBC strings.
#[derive(Debug, Clone)]
pub struct CharMap {
    byte_to_char: HashMap<u8, char>,
    char_to_byte: HashMap<char, u8>,
}

impl CharMap {
    /// Parse a `map.txt`-style table from its text contents.
    ///
    /// Format: one mapping per line, `HH\tC` (two hex digits, a tab, then
    /// exactly one character). The literal space character is written as
    /// the two-char escape `\s` since trailing whitespace tends to get
    /// stripped by editors/tools. Blank lines and lines starting with `#`
    /// are ignored.
    pub fn parse(contents: &str) -> Result<Self, PcfError> {
        let mut byte_to_char = HashMap::new();
        let mut char_to_byte = HashMap::new();

        for (line_no, raw_line) in contents.lines().enumerate() {
            let line = raw_line.trim_end_matches('\r');
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let (hex, rest) = line.split_once('\t').ok_or_else(|| {
                PcfError::new(
                    "charmap_parse_error",
                    format!("line {}: expected \"HH\\tC\", got {line:?}", line_no + 1),
                )
            })?;
            let byte = u8::from_str_radix(hex, 16).map_err(|e| {
                PcfError::new(
                    "charmap_parse_error",
                    format!("line {}: invalid hex byte {hex:?}: {e}", line_no + 1),
                )
            })?;
            let ch = if rest == "\\s" {
                ' '
            } else {
                rest.chars().next().ok_or_else(|| {
                    PcfError::new(
                        "charmap_parse_error",
                        format!("line {}: missing character after byte {hex}", line_no + 1),
                    )
                })?
            };

            byte_to_char.insert(byte, ch);
            char_to_byte.insert(ch, byte);
        }

        Ok(Self {
            byte_to_char,
            char_to_byte,
        })
    }

    /// Load a `map.txt`-style table from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PcfError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|e| {
            PcfError::new(
                "charmap_load_error",
                format!("failed to read charmap {}: {e}", path.display()),
            )
        })?;
        Self::parse(&contents)
    }

    /// Decode a byte string into text, per-byte substitution.
    ///
    /// Returns a typed error naming the byte offset of the first
    /// unrecognized byte, rather than panicking or silently substituting.
    pub fn decode(&self, bytes: &[u8]) -> Result<String, PcfError> {
        let mut out = String::with_capacity(bytes.len());
        for (offset, &byte) in bytes.iter().enumerate() {
            match self.byte_to_char.get(&byte) {
                Some(&ch) => out.push(ch),
                None => {
                    return Err(PcfError::new(
                        "charmap_unknown_byte",
                        format!("unrecognized glyph byte 0x{byte:02X} at offset {offset}"),
                    )
                    .with_context(format!("offset={offset}")))
                }
            }
        }
        Ok(out)
    }

    /// Encode text into bytes, per-char substitution.
    ///
    /// Returns a typed error naming the byte offset (of the *output*
    /// stream) of the first character with no mapping.
    pub fn encode(&self, text: &str) -> Result<Vec<u8>, PcfError> {
        let mut out = Vec::with_capacity(text.len());
        for ch in text.chars() {
            match self.char_to_byte.get(&ch) {
                Some(&byte) => out.push(byte),
                None => {
                    let offset = out.len();
                    return Err(PcfError::new(
                        "charmap_unknown_char",
                        format!("no glyph mapping for {ch:?} (would be at output offset {offset})"),
                    )
                    .with_context(format!("offset={offset}")));
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The one verified data point in PLAN.md Appendix A.
    const REAL_MADRID_BYTES: &[u8] = &[
        0x33, 0x04, 0x00, 0x0D, 0x41, 0x2C, 0x00, 0x05, 0x13, 0x08, 0x05, 0x41, 0x22, 0x4F, 0x27,
        0x4F,
    ];
    const REAL_MADRID_TEXT: &str = "Real Madrid C.F.";

    fn synthetic_charmap_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("synthetic_map.txt")
    }

    fn load_synthetic() -> CharMap {
        CharMap::load(synthetic_charmap_path()).expect("synthetic charmap fixture should parse")
    }

    #[test]
    fn decodes_real_madrid_example_from_appendix_a() {
        let map = load_synthetic();
        assert_eq!(map.decode(REAL_MADRID_BYTES).unwrap(), REAL_MADRID_TEXT);
    }

    #[test]
    fn encodes_real_madrid_example_from_appendix_a() {
        let map = load_synthetic();
        assert_eq!(map.encode(REAL_MADRID_TEXT).unwrap(), REAL_MADRID_BYTES);
    }

    #[test]
    fn round_trips_real_madrid_example() {
        let map = load_synthetic();
        let decoded = map.decode(REAL_MADRID_BYTES).unwrap();
        let re_encoded = map.encode(&decoded).unwrap();
        assert_eq!(re_encoded, REAL_MADRID_BYTES);
    }

    #[test]
    fn unknown_byte_reports_offset_not_panic() {
        let map = load_synthetic();
        // 0xFF is not present in the synthetic table.
        let bytes = [0x33, 0x04, 0xFF, 0x0D];
        let err = map.decode(&bytes).unwrap_err();
        assert_eq!(err.code, "charmap_unknown_byte");
        assert!(err.message.contains("0xFF"));
        assert!(err.message.contains("offset 2"));
    }

    #[test]
    fn unknown_char_reports_offset_not_panic() {
        let map = load_synthetic();
        // '#' has no mapping in the synthetic table.
        let err = map.encode("Re#").unwrap_err();
        assert_eq!(err.code, "charmap_unknown_char");
        assert!(err.message.contains("offset 2"));
    }

    #[test]
    fn parse_rejects_malformed_line_without_panicking() {
        let err = CharMap::parse("not-a-valid-line").unwrap_err();
        assert_eq!(err.code, "charmap_parse_error");
    }

    #[test]
    fn parse_ignores_comments_and_blank_lines() {
        let map = CharMap::parse("# comment\n\n33\tR\n").unwrap();
        assert_eq!(map.decode(&[0x33]).unwrap(), "R");
    }
}
