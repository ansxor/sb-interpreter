//! A tiny, dependency-free PNG **encoder** for committing render goldens.
//!
//! It writes a valid baseline PNG (8-bit RGBA, color type 6) using **uncompressed** zlib
//! "stored" deflate blocks — no compression, so the output is byte-for-byte deterministic
//! for a given framebuffer. That determinism is the whole point: the golden test re-encodes
//! the freshly rendered framebuffer and compares the bytes to the committed `.png`, so no
//! PNG *decoder* (inflate) is needed. The files are still ordinary, viewable PNGs.

use crate::Framebuffer;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Encode an [`Framebuffer`] (RGBA8888, row-major) as PNG bytes.
pub fn encode(fb: &Framebuffer) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&SIGNATURE);

    // IHDR: width, height, bit depth 8, color type 6 (RGBA), no compression/filter/interlace.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(fb.width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(fb.height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);

    // IDAT: filtered scanlines (filter byte 0 = None per row) wrapped in a stored zlib stream.
    let mut raw = Vec::with_capacity(fb.height * (1 + fb.width * 4));
    for y in 0..fb.height {
        raw.push(0); // filter type: None
        let start = y * fb.width * 4;
        raw.extend_from_slice(&fb.pixels[start..start + fb.width * 4]);
    }
    let zlib = zlib_store(&raw);
    write_chunk(&mut out, b"IDAT", &zlib);

    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let crc_start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[crc_start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Wrap `data` in a zlib stream using uncompressed deflate "stored" blocks.
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x78); // CMF: 32K window, deflate
    out.push(0x01); // FLG: chosen so (0x78<<8 | 0x01) % 31 == 0, no preset dict
    let mut chunks = data.chunks(0xFFFF).peekable();
    if data.is_empty() {
        // One empty final stored block.
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    }
    while let Some(chunk) = chunks.next() {
        let last = chunks.peek().is_none();
        out.push(if last { 0x01 } else { 0x00 }); // BFINAL + BTYPE=00 (stored)
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_signature_and_chunks() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_argb(0, 0, 0xFFFF0000);
        let png = encode(&fb);
        assert_eq!(&png[0..8], &SIGNATURE);
        // IHDR chunk type appears right after the 8-byte sig + 4-byte length.
        assert_eq!(&png[12..16], b"IHDR");
        // Ends with IEND + its CRC.
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn encoding_is_deterministic() {
        let mut fb = Framebuffer::new(3, 3);
        fb.clear(0xFF010203);
        assert_eq!(encode(&fb), encode(&fb));
    }

    #[test]
    fn crc32_known_vector() {
        // CRC-32 of "123456789" is 0xCBF43926.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn adler32_known_vector() {
        // Adler-32 of "Wikipedia" is 0x11E60398.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }
}
