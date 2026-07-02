//! Low-level byte cursor helpers shared by the record readers/writers.
//!
//! Numeric fields are little-endian; DBC strings are a 2-byte LE length
//! prefix followed by that many *encoded* bytes (via [`CharMap`], not
//! ASCII) — see PLAN.md §8 and Appendix A. A handful of numeric fields
//! (seated capacity, members, budget) are 3-byte ("u24") little-endian,
//! which has no native Rust type, so we read/write them as `u32`.

use pcf_model::PcfError;

use crate::charmap::CharMap;

/// Reads a DBC byte stream left to right, tracking the offset so errors can
/// name exactly where they happened.
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub fn offset(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn eof_error(&self, want: usize) -> PcfError {
        PcfError::new(
            "dbc_unexpected_eof",
            format!(
                "expected {want} more byte(s) at offset {} but only {} remain",
                self.pos,
                self.remaining()
            ),
        )
        .with_context(format!("offset={}", self.pos))
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], PcfError> {
        if self.remaining() < n {
            return Err(self.eof_error(n));
        }
        let slice = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, PcfError> {
        Ok(self.take(1)?[0])
    }

    /// Looks at the next byte without consuming it. Used for the coach
    /// chain's "was also a player" marker, which is only present
    /// conditionally (Appendix A: "if the byte after coach career != 03,
    /// skip straight to declarations").
    pub fn peek_u8(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub fn u16_le(&mut self) -> Result<u16, PcfError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// 3-byte little-endian value (no native Rust type), widened to `u32`.
    pub fn u24_le(&mut self) -> Result<u32, PcfError> {
        let b = self.take(3)?;
        Ok(u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16))
    }

    /// Consumes `n` bytes and errors if they don't equal `expected` — used
    /// for the format's fixed/magic byte sequences (separators, pitch
    /// size marker, etc.) so corruption is caught rather than silently
    /// accepted.
    pub fn expect_fixed(&mut self, expected: &[u8]) -> Result<(), PcfError> {
        let start = self.pos;
        let actual = self.take(expected.len())?;
        if actual != expected {
            return Err(PcfError::new(
                "dbc_fixed_bytes_mismatch",
                format!(
                    "expected fixed bytes {expected:02X?} at offset {start} but found {actual:02X?}"
                ),
            )
            .with_context(format!("offset={start}")));
        }
        Ok(())
    }

    /// Reads a length-prefixed, charmap-encoded string.
    pub fn string(&mut self, charmap: &CharMap) -> Result<String, PcfError> {
        let len = self.u16_le()? as usize;
        let bytes = self.take(len)?;
        charmap.decode(bytes)
    }

    /// Reads a length-prefixed *opaque* blob (no charmap translation) —
    /// used for the formation blob, which Appendix A treats as opaque
    /// for v1.
    pub fn opaque_blob(&mut self) -> Result<Vec<u8>, PcfError> {
        let len = self.u16_le()? as usize;
        Ok(self.take(len)?.to_vec())
    }
}

/// Accumulates bytes for the mirror-image write path.
#[derive(Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn u16_le(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// 3-byte little-endian value; `v` must fit in 24 bits.
    pub fn u24_le(&mut self, v: u32) {
        self.buf.push((v & 0xFF) as u8);
        self.buf.push(((v >> 8) & 0xFF) as u8);
        self.buf.push(((v >> 16) & 0xFF) as u8);
    }

    pub fn fixed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn string(&mut self, charmap: &CharMap, s: &str) -> Result<(), PcfError> {
        let encoded = charmap.encode(s)?;
        self.u16_le(encoded.len() as u16);
        self.buf.extend_from_slice(&encoded);
        Ok(())
    }

    pub fn opaque_blob(&mut self, blob: &[u8]) {
        self.u16_le(blob.len() as u16);
        self.buf.extend_from_slice(blob);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_charmap() -> CharMap {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("charmap")
            .join("synthetic_map.txt");
        CharMap::load(path).unwrap()
    }

    #[test]
    fn u24_le_round_trips_seated_capacity_example() {
        // Appendix A: `D85301` on disk -> reversed -> 0x0153D8 = 87000.
        let mut r = Reader::new(&[0xD8, 0x53, 0x01]);
        assert_eq!(r.u24_le().unwrap(), 87_000);

        let mut w = Writer::new();
        w.u24_le(87_000);
        assert_eq!(w.into_bytes(), vec![0xD8, 0x53, 0x01]);
    }

    #[test]
    fn string_round_trips_through_charmap() {
        let map = synthetic_charmap();
        let mut w = Writer::new();
        w.string(&map, "Real Madrid C.F.").unwrap();
        let bytes = w.into_bytes();

        // 16 chars -> length prefix 0x10 0x00, then the 16 encoded bytes.
        assert_eq!(&bytes[0..2], &[0x10, 0x00]);

        let mut r = Reader::new(&bytes);
        assert_eq!(r.string(&map).unwrap(), "Real Madrid C.F.");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn expect_fixed_errors_with_offset_on_mismatch() {
        let mut r = Reader::new(&[0x00, 0x99]);
        r.u8().unwrap();
        let err = r.expect_fixed(&[0x00]).unwrap_err();
        assert_eq!(err.code, "dbc_fixed_bytes_mismatch");
        assert!(err.message.contains("offset 1"));
    }

    #[test]
    fn take_past_end_errors_instead_of_panicking() {
        let mut r = Reader::new(&[0x01]);
        let err = r.take(5).unwrap_err();
        assert_eq!(err.code, "dbc_unexpected_eof");
    }
}
