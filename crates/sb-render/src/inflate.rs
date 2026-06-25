//! Minimal, dependency-free **raw-DEFLATE** decoder (RFC 1951).
//!
//! The firmware default graphic pages ([`crate::assets`]) are committed raw-DEFLATE-
//! compressed (~170 KB vs 1.5 MB uncompressed) so they bake into the wasm/native binary
//! without a 1.5 MB cost. The whole workspace is deliberately third-party-dependency-free
//! (see the hand-rolled SHA-1/HMAC in `sb_core::storage`), so this is an in-house inflater
//! rather than a `flate2`/`miniz` dependency.
//!
//! It supports the three block types (stored, fixed-Huffman, dynamic-Huffman) — everything a
//! standard deflate stream uses — but NOT the zlib/gzip framing wrappers; the assets are
//! stored as *raw* deflate (no header/checksum), so the loader feeds the bytes straight in.
//! Decoding is checked: a malformed stream returns `None` rather than panicking.

/// A canonical Huffman decoder built from a list of code lengths (one per symbol).
struct Huffman {
    /// `counts[n]` = number of codes of length `n` (1..=15).
    counts: [u16; 16],
    /// Symbols ordered by (length, symbol), the canonical layout `decode` walks.
    symbols: Vec<u16>,
}

impl Huffman {
    /// Build from per-symbol code lengths (0 = symbol unused). Returns `None` if the lengths
    /// describe an over-subscribed (invalid) code.
    fn new(lengths: &[u8]) -> Option<Huffman> {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0; // length-0 symbols are absent, not a real code
                       // Offsets of each length's first symbol in the sorted `symbols` table.
        let mut offsets = [0u16; 16];
        for n in 1..16 {
            offsets[n] = offsets[n - 1] + counts[n - 1];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offsets[l as usize] as usize] = sym as u16;
                offsets[l as usize] += 1;
            }
        }
        Some(Huffman { counts, symbols })
    }

    /// Decode one symbol from the bit reader using the canonical-Huffman walk.
    fn decode(&self, br: &mut BitReader) -> Option<u16> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..16 {
            code |= br.bit()? as i32;
            let count = self.counts[len] as i32;
            if code - first < count {
                return self.symbols.get((index + (code - first)) as usize).copied();
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

/// LSB-first bit reader over a byte slice (deflate packs bits low-to-high within each byte).
struct BitReader<'a> {
    data: &'a [u8],
    byte: usize,
    bit: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            byte: 0,
            bit: 0,
        }
    }

    /// Read one bit (LSB-first). `None` past end of input.
    fn bit(&mut self) -> Option<u8> {
        let b = *self.data.get(self.byte)?;
        let v = (b >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Some(v)
    }

    /// Read `n` bits (LSB-first) as an integer (`n` ≤ 16).
    fn bits(&mut self, n: u32) -> Option<u32> {
        let mut v = 0u32;
        for i in 0..n {
            v |= (self.bit()? as u32) << i;
        }
        Some(v)
    }

    /// Discard bits up to the next byte boundary (used before a stored block's length field).
    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.byte += 1;
        }
    }
}

/// Length base values for the 29 length symbols 257..=285 (RFC 1951 §3.2.5).
const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
/// Extra length bits for each length symbol.
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// Distance base values for the 30 distance symbols 0..=29.
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
/// Extra distance bits for each distance symbol.
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// Order in which the dynamic-block code-length-code lengths are stored (RFC 1951 §3.2.7).
const CLEN_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Inflate a raw-DEFLATE stream into `out`, expecting exactly `expected_len` output bytes.
/// Returns the decoded bytes, or `None` on any malformed input (over-subscribed code,
/// truncated stream, back-reference before the start, or a size mismatch).
pub fn inflate(data: &[u8], expected_len: usize) -> Option<Vec<u8>> {
    let mut br = BitReader::new(data);
    let mut out: Vec<u8> = Vec::with_capacity(expected_len);

    loop {
        let final_block = br.bit()? == 1;
        let btype = br.bits(2)?;
        match btype {
            0 => inflate_stored(&mut br, &mut out)?,
            1 => inflate_block(&mut br, &mut out, &fixed_litlen()?, &fixed_dist()?)?,
            2 => {
                let (lit, dist) = read_dynamic_tables(&mut br)?;
                inflate_block(&mut br, &mut out, &lit, &dist)?;
            }
            _ => return None, // btype 3 is reserved/invalid
        }
        if final_block {
            break;
        }
    }

    (out.len() == expected_len).then_some(out)
}

/// A stored (uncompressed) block: byte-align, read LEN/~LEN, copy LEN literal bytes.
fn inflate_stored(br: &mut BitReader, out: &mut Vec<u8>) -> Option<()> {
    br.align();
    let len = br.bits(16)? as usize;
    let nlen = br.bits(16)?;
    if nlen != (!len as u32 & 0xFFFF) {
        return None;
    }
    for _ in 0..len {
        out.push(br.bits(8)? as u8);
    }
    Some(())
}

/// Decode one Huffman-coded block (fixed or dynamic) until the end-of-block symbol (256).
fn inflate_block(
    br: &mut BitReader,
    out: &mut Vec<u8>,
    lit: &Huffman,
    dist: &Huffman,
) -> Option<()> {
    loop {
        let sym = lit.decode(br)?;
        match sym {
            0..=255 => out.push(sym as u8),
            256 => return Some(()), // end of block
            257..=285 => {
                let li = (sym - 257) as usize;
                let length = LEN_BASE[li] as usize + br.bits(LEN_EXTRA[li] as u32)? as usize;
                let dsym = dist.decode(br)? as usize;
                if dsym >= DIST_BASE.len() {
                    return None;
                }
                let distance =
                    DIST_BASE[dsym] as usize + br.bits(DIST_EXTRA[dsym] as u32)? as usize;
                if distance == 0 || distance > out.len() {
                    return None;
                }
                let start = out.len() - distance;
                for i in 0..length {
                    out.push(out[start + i]); // overlapping copy is intentional (LZ77)
                }
            }
            _ => return None,
        }
    }
}

/// Read a dynamic block's literal/length and distance Huffman tables (RFC 1951 §3.2.7).
fn read_dynamic_tables(br: &mut BitReader) -> Option<(Huffman, Huffman)> {
    let hlit = br.bits(5)? as usize + 257;
    let hdist = br.bits(5)? as usize + 1;
    let hclen = br.bits(4)? as usize + 4;

    let mut cl_lengths = [0u8; 19];
    for i in 0..hclen {
        cl_lengths[CLEN_ORDER[i]] = br.bits(3)? as u8;
    }
    let cl_huff = Huffman::new(&cl_lengths)?;

    // Decode the run-length-encoded code lengths for the literal+distance alphabets.
    let total = hlit + hdist;
    let mut lengths = vec![0u8; total];
    let mut i = 0;
    while i < total {
        let sym = cl_huff.decode(br)?;
        match sym {
            0..=15 => {
                lengths[i] = sym as u8;
                i += 1;
            }
            16 => {
                // Repeat the previous length 3..=6 times.
                if i == 0 {
                    return None;
                }
                let rep = 3 + br.bits(2)? as usize;
                let prev = lengths[i - 1];
                for _ in 0..rep {
                    if i >= total {
                        return None;
                    }
                    lengths[i] = prev;
                    i += 1;
                }
            }
            17 => {
                let rep = 3 + br.bits(3)? as usize;
                i = fill_zeros(&mut lengths, i, rep)?;
            }
            18 => {
                let rep = 11 + br.bits(7)? as usize;
                i = fill_zeros(&mut lengths, i, rep)?;
            }
            _ => return None,
        }
    }

    let lit = Huffman::new(&lengths[..hlit])?;
    let dist = Huffman::new(&lengths[hlit..])?;
    Some((lit, dist))
}

/// Write `rep` zero code-lengths starting at `i`; returns the new index or `None` on overflow.
fn fill_zeros(lengths: &mut [u8], mut i: usize, rep: usize) -> Option<usize> {
    for _ in 0..rep {
        if i >= lengths.len() {
            return None;
        }
        lengths[i] = 0;
        i += 1;
    }
    Some(i)
}

/// The fixed literal/length Huffman code (RFC 1951 §3.2.6): lengths 8/9/7/8 across the ranges.
fn fixed_litlen() -> Option<Huffman> {
    let mut l = [0u8; 288];
    l[0..=143].fill(8);
    l[144..=255].fill(9);
    l[256..=279].fill(7);
    l[280..=287].fill(8);
    Huffman::new(&l)
}

/// The fixed distance Huffman code: all 30 distance symbols have length 5.
fn fixed_dist() -> Option<Huffman> {
    Huffman::new(&[5u8; 30])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "hello hello hello world" raw-deflated (python `zlib.compressobj(9, DEFLATED, -15)`).
    /// Exercises a Huffman block with a back-reference (the repeated "hello ").
    const HELLO: [u8; 15] = [
        0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x22, 0xcb, 0xf3, 0x8b, 0x72, 0x52, 0x00,
    ];

    #[test]
    fn inflates_huffman_block_with_backref() {
        let want = b"hello hello hello world";
        assert_eq!(inflate(&HELLO, want.len()).as_deref(), Some(&want[..]));
    }

    #[test]
    fn rejects_wrong_expected_length() {
        assert_eq!(inflate(&HELLO, 5), None);
    }

    #[test]
    fn rejects_truncated_stream() {
        assert_eq!(inflate(&[0x00], 100), None);
    }

    #[test]
    fn inflates_stored_block() {
        // A single final stored block: BFINAL=1,BTYPE=00 -> first byte 0x01, then LEN=3,~LEN,
        // then "abc".
        let comp = [0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c'];
        assert_eq!(inflate(&comp, 3).as_deref(), Some(&b"abc"[..]));
    }
}
