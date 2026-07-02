//! Minimal 8-bit indexed (256-color) BMP reader/writer.
//!
//! We hand-roll this instead of leaning on the `image` crate's BMP codec
//! because the game needs a *specific* palette-conformant 8bpp file (see
//! `palette.rs`), and we want full control over the color table bytes to
//! guarantee byte-identical round-trips.

use pcf_model::PcfError;

use crate::palette::Palette;

const FILE_HEADER_LEN: u32 = 14;
const DIB_HEADER_LEN: u32 = 40;
const COLOR_TABLE_LEN: u32 = 256 * 4;
const PIXEL_DATA_OFFSET: u32 = FILE_HEADER_LEN + DIB_HEADER_LEN + COLOR_TABLE_LEN;

/// An 8-bit indexed bitmap: dimensions, its 256-color palette, and one
/// index byte per pixel, stored top-to-bottom, left-to-right (row-major).
#[derive(Debug, Clone, PartialEq)]
pub struct Bmp8 {
    pub width: u32,
    pub height: u32,
    pub palette: Palette,
    /// len == width * height; row-major, top row first.
    pub pixels: Vec<u8>,
}

impl Bmp8 {
    pub fn new(
        width: u32,
        height: u32,
        palette: Palette,
        pixels: Vec<u8>,
    ) -> Result<Self, PcfError> {
        let expected = width as usize * height as usize;
        if pixels.len() != expected {
            return Err(PcfError::new(
                "pcf_images.bmp.pixel_count_mismatch",
                format!(
                    "expected {expected} pixels for a {width}x{height} image, got {}",
                    pixels.len()
                ),
            ));
        }
        Ok(Self {
            width,
            height,
            palette,
            pixels,
        })
    }

    fn row_stride(&self) -> u32 {
        self.width.div_ceil(4) * 4
    }
}

fn u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Encode `bmp` as a standard 8bpp BMP file (`BITMAPFILEHEADER` +
/// `BITMAPINFOHEADER` + 256-entry BGR0 color table + bottom-up,
/// row-padded-to-4-bytes pixel data).
pub fn write_bmp8(bmp: &Bmp8) -> Vec<u8> {
    let stride = bmp.row_stride();
    let pixel_data_len = stride * bmp.height;
    let file_size = PIXEL_DATA_OFFSET + pixel_data_len;

    let mut out = Vec::with_capacity(file_size as usize);

    // --- BITMAPFILEHEADER ---
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved
    out.extend_from_slice(&PIXEL_DATA_OFFSET.to_le_bytes());

    // --- BITMAPINFOHEADER ---
    out.extend_from_slice(&DIB_HEADER_LEN.to_le_bytes());
    out.extend_from_slice(&(bmp.width as i32).to_le_bytes());
    out.extend_from_slice(&(bmp.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&8u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // compression = BI_RGB
    out.extend_from_slice(&pixel_data_len.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes()); // x pixels/meter
    out.extend_from_slice(&0i32.to_le_bytes()); // y pixels/meter
    out.extend_from_slice(&256u32.to_le_bytes()); // colors used
    out.extend_from_slice(&0u32.to_le_bytes()); // colors important

    // --- Color table (BGR0, 256 entries) ---
    for &(r, g, b) in bmp.palette.colors() {
        out.push(b);
        out.push(g);
        out.push(r);
        out.push(0);
    }

    // --- Pixel data: bottom-up, each row padded to a 4-byte boundary ---
    let pad_len = (stride - bmp.width) as usize;
    for row in (0..bmp.height).rev() {
        let start = (row * bmp.width) as usize;
        let end = start + bmp.width as usize;
        out.extend_from_slice(&bmp.pixels[start..end]);
        out.extend(std::iter::repeat_n(0u8, pad_len));
    }

    out
}

/// Decode a standard 8bpp BMP file into a [`Bmp8`]. Rejects anything that
/// isn't an uncompressed 8-bit indexed bitmap with a 256-entry color
/// table.
pub fn read_bmp8(bytes: &[u8]) -> Result<Bmp8, PcfError> {
    if bytes.len() < (FILE_HEADER_LEN + DIB_HEADER_LEN) as usize {
        return Err(PcfError::new(
            "pcf_images.bmp.truncated",
            "file is too short to contain a BMP header",
        ));
    }
    if &bytes[0..2] != b"BM" {
        return Err(PcfError::new(
            "pcf_images.bmp.bad_signature",
            "missing 'BM' BMP signature",
        ));
    }

    let pixel_data_offset = u32_le(bytes, 10);
    let dib_header_len = u32_le(bytes, 14);
    if dib_header_len != DIB_HEADER_LEN {
        return Err(PcfError::new(
            "pcf_images.bmp.unsupported_header",
            format!("only the 40-byte BITMAPINFOHEADER is supported, got {dib_header_len}"),
        ));
    }

    let width = i32_le(bytes, 18);
    let height = i32_le(bytes, 22);
    if width <= 0 || height <= 0 {
        return Err(PcfError::new(
            "pcf_images.bmp.bad_dimensions",
            format!("width/height must be positive, got {width}x{height}"),
        ));
    }
    let width = width as u32;
    let height = height as u32;

    let bpp = u16_le(bytes, 28);
    if bpp != 8 {
        return Err(PcfError::new(
            "pcf_images.bmp.unsupported_bpp",
            format!("expected an 8-bit indexed BMP, got {bpp} bits per pixel"),
        ));
    }

    let compression = u32_le(bytes, 30);
    if compression != 0 {
        return Err(PcfError::new(
            "pcf_images.bmp.unsupported_compression",
            format!("expected uncompressed (BI_RGB) data, got compression method {compression}"),
        ));
    }

    let color_table_offset = (FILE_HEADER_LEN + DIB_HEADER_LEN) as usize;
    let color_table_end = color_table_offset + 256 * 4;
    if bytes.len() < color_table_end {
        return Err(PcfError::new(
            "pcf_images.bmp.truncated",
            "file is too short to contain a 256-entry color table",
        ));
    }
    let mut colors = [(0u8, 0u8, 0u8); 256];
    for (i, entry) in colors.iter_mut().enumerate() {
        let off = color_table_offset + i * 4;
        let b = bytes[off];
        let g = bytes[off + 1];
        let r = bytes[off + 2];
        *entry = (r, g, b);
    }
    let palette = Palette::from_colors(colors);

    let stride = width.div_ceil(4) * 4;
    let pixel_data_len = stride as usize * height as usize;
    let pixel_data_start = pixel_data_offset as usize;
    let pixel_data_end = pixel_data_start + pixel_data_len;
    if bytes.len() < pixel_data_end {
        return Err(PcfError::new(
            "pcf_images.bmp.truncated",
            "file is too short to contain the declared pixel data",
        ));
    }

    // Pixel data is stored bottom-up; flip back to top-to-bottom row-major.
    let mut pixels = vec![0u8; (width * height) as usize];
    for row in 0..height {
        let src_row = height - 1 - row;
        let src_start = pixel_data_start + (src_row * stride) as usize;
        let dst_start = (row * width) as usize;
        pixels[dst_start..dst_start + width as usize]
            .copy_from_slice(&bytes[src_start..src_start + width as usize]);
    }

    Ok(Bmp8 {
        width,
        height,
        palette,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_bmp() -> Bmp8 {
        // 2x2 image, indices chosen so rows/columns are distinguishable
        // after the bottom-up flip.
        Bmp8::new(2, 2, Palette::synthetic_placeholder(), vec![1, 2, 3, 4])
            .expect("valid pixel count")
    }

    #[test]
    fn new_rejects_pixel_count_mismatch() {
        let err = Bmp8::new(2, 2, Palette::synthetic_placeholder(), vec![1, 2, 3]).unwrap_err();
        assert_eq!(err.code, "pcf_images.bmp.pixel_count_mismatch");
    }

    #[test]
    fn write_then_read_round_trips_pixels_and_palette() {
        let original = tiny_bmp();
        let bytes = write_bmp8(&original);
        let parsed = read_bmp8(&bytes).expect("valid bmp");
        assert_eq!(parsed, original);
    }

    #[test]
    fn write_then_write_again_is_byte_identical() {
        let original = tiny_bmp();
        let bytes_a = write_bmp8(&original);
        let parsed = read_bmp8(&bytes_a).expect("valid bmp");
        let bytes_b = write_bmp8(&parsed);
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn read_rejects_bad_signature() {
        let mut bytes = write_bmp8(&tiny_bmp());
        bytes[0] = b'X';
        let err = read_bmp8(&bytes).unwrap_err();
        assert_eq!(err.code, "pcf_images.bmp.bad_signature");
    }

    #[test]
    fn read_rejects_non_8bpp() {
        let mut bytes = write_bmp8(&tiny_bmp());
        // bpp field lives at offset 28, LE u16.
        bytes[28] = 24;
        bytes[29] = 0;
        let err = read_bmp8(&bytes).unwrap_err();
        assert_eq!(err.code, "pcf_images.bmp.unsupported_bpp");
    }

    #[test]
    fn read_rejects_truncated_file() {
        let err = read_bmp8(&[0u8; 4]).unwrap_err();
        assert_eq!(err.code, "pcf_images.bmp.truncated");
    }
}
