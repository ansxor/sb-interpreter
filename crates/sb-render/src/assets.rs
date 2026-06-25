//! Firmware default graphic pages, baked into the binary.
//!
//! SmileBASIC boots with three system graphic pages already populated:
//! * **GRP4** — the default *sprite* sheet (what `SPSET` samples by default; see
//!   [`crate::sprite::SPRITE_PAGE_DEFAULT`]),
//! * **GRP5** — the default *BG* sheet (what `BGPUT` samples; [`crate::bg::BG_PAGE_DEFAULT`]),
//! * **GRPF** — the hidden *font* page (page index −1; the console glyph source). It is not on
//!   any `GPAGE` but is still `GSAVE`/`GLOAD`-addressable as `"GRPF"`.
//!
//! These are firmware ROM assets, harvested from SmileBASIC 3.6.0's own extdata
//! (`SPRITE`/`BACKGROUND`/`FONT` resources — 512×512 RGBA5551 graphic pages). They are stored
//! here raw-DEFLATE-compressed (~170 KB total vs 1.5 MB raw) and inflated on first use via the
//! dependency-free [`crate::inflate`] decoder. The companion [`atlas`] sub-module bakes the
//! font's codepoint→(x,y) layout table so the console can locate each glyph on the font page.

use crate::grp::{GrpPage, GRP_DIM};
use crate::inflate;
use crate::sprite::{SpdefEntry, SPDEF_TEMPLATE_COUNT};

/// Raw-DEFLATE blob of GRP4's 512×512 RGBA5551 pixels (firmware sprite sheet).
const SPRITE_DEFLATE: &[u8] = include_bytes!("assets/sprite.deflate");
/// Raw-DEFLATE blob of GRP5's 512×512 RGBA5551 pixels (firmware BG sheet).
const BACKGROUND_DEFLATE: &[u8] = include_bytes!("assets/background.deflate");
/// Raw-DEFLATE blob of the GRPF 512×512 RGBA5551 pixels (firmware font page).
const FONT_DEFLATE: &[u8] = include_bytes!("assets/font.deflate");

/// One page's worth of device halfwords (512×512).
const PAGE_HALFWORDS: usize = GRP_DIM * GRP_DIM;
/// One page's worth of compressed-payload bytes once inflated (2 bytes per halfword).
const PAGE_BYTES: usize = PAGE_HALFWORDS * 2;

/// Inflate a baked RGBA5551 page blob into a [`GrpPage`]. The committed assets are exactly one
/// 512×512 page each, so a decode failure (impossible for the committed bytes) falls back to a
/// blank page rather than panicking.
fn decode_page(deflate: &[u8]) -> GrpPage {
    let Some(bytes) = inflate::inflate(deflate, PAGE_BYTES) else {
        return GrpPage::new();
    };
    let pixels = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect::<Vec<u16>>();
    GrpPage { pixels }
}

/// The firmware default sprite sheet (GRP4).
pub fn default_sprite_page() -> GrpPage {
    decode_page(SPRITE_DEFLATE)
}

/// The firmware default BG sheet (GRP5).
pub fn default_bg_page() -> GrpPage {
    decode_page(BACKGROUND_DEFLATE)
}

/// The firmware font page (GRPF, page −1).
pub fn default_font_page() -> GrpPage {
    decode_page(FONT_DEFLATE)
}

/// The firmware **SPDEF definition table** — the 4096 built-in sprite templates `SPSET <id>`
/// copies from, and what `SPDEF` (no-arg) / `ACLS` reset to. Baked from SmileBASIC's default
/// `spdef.csv`; each template is `(u, v, w, h, origin_x, origin_y, attr)` stored as 7 packed
/// little-endian `i16`s in declaration order (template 0, 1, …).
const SPDEF_BIN: &[u8] = include_bytes!("assets/spdef.bin");
/// `i16`s per packed SPDEF template entry (u, v, w, h, origin_x, origin_y, attr).
const SPDEF_FIELDS: usize = 7;

pub fn default_spdef() -> Vec<SpdefEntry> {
    let i16_at = |i: usize| i16::from_le_bytes([SPDEF_BIN[i * 2], SPDEF_BIN[i * 2 + 1]]) as i32;
    (0..SPDEF_TEMPLATE_COUNT)
        .map(|t| {
            let b = t * SPDEF_FIELDS;
            SpdefEntry {
                u: i16_at(b),
                v: i16_at(b + 1),
                w: i16_at(b + 2),
                h: i16_at(b + 3),
                origin_x: i16_at(b + 4),
                origin_y: i16_at(b + 5),
                attr: i16_at(b + 6),
            }
        })
        .collect()
}

/// The font's codepoint→atlas-position table.
pub mod atlas {
    /// Sorted packed `(codepoint: u16, x: u16, y: u16)` triples (little-endian), one per glyph
    /// present on the font page, ascending by codepoint so [`pos`] can binary-search in place.
    /// Harvested from SmileBASIC's `fonttable.txt`; each glyph is an 8×8 cell with its top-left
    /// at `(x, y)` on the 512×512 font page.
    const ATLAS: &[u8] = include_bytes!("assets/font_atlas.bin");
    /// Bytes per atlas entry: three little-endian `u16`s.
    const ENTRY: usize = 6;

    /// Read the codepoint of entry `i`.
    #[inline]
    fn codepoint_at(i: usize) -> u16 {
        u16::from_le_bytes([ATLAS[i * ENTRY], ATLAS[i * ENTRY + 1]])
    }

    /// The top-left `(x, y)` of `ch`'s 8×8 glyph on the font page, or `None` if the font has no
    /// glyph for that codepoint. Binary-searches the sorted atlas.
    pub fn pos(ch: u16) -> Option<(u16, u16)> {
        let n = ATLAS.len() / ENTRY;
        let (mut lo, mut hi) = (0usize, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let cp = codepoint_at(mid);
            match cp.cmp(&ch) {
                core::cmp::Ordering::Less => lo = mid + 1,
                core::cmp::Ordering::Greater => hi = mid,
                core::cmp::Ordering::Equal => {
                    let b = mid * ENTRY;
                    let x = u16::from_le_bytes([ATLAS[b + 2], ATLAS[b + 3]]);
                    let y = u16::from_le_bytes([ATLAS[b + 4], ATLAS[b + 5]]);
                    return Some((x, y));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_inflate_to_full_size() {
        for page in [
            default_sprite_page(),
            default_bg_page(),
            default_font_page(),
        ] {
            assert_eq!(page.pixels.len(), PAGE_HALFWORDS);
        }
    }

    #[test]
    fn font_page_has_content() {
        // The default pages are not all-transparent — at least one opaque (alpha-bit) pixel.
        let f = default_font_page();
        assert!(f.pixels.iter().any(|&h| h & 1 != 0));
    }

    #[test]
    fn spdef_table_decodes_known_templates() {
        let t = default_spdef();
        assert_eq!(t.len(), SPDEF_TEMPLATE_COUNT);
        // From the firmware spdef.csv: template 0 = (0,0,16,16,0,0,1), template 1 = (16,0,...),
        // template 4095 = (192,480,96,32,48,16,1).
        assert_eq!((t[0].u, t[0].v, t[0].w, t[0].h, t[0].attr), (0, 0, 16, 16, 1));
        assert_eq!((t[1].u, t[1].v), (16, 0));
        let last = t[4095];
        assert_eq!(
            (last.u, last.v, last.w, last.h, last.origin_x, last.origin_y, last.attr),
            (192, 480, 96, 32, 48, 16, 1)
        );
    }

    #[test]
    fn atlas_locates_known_glyphs() {
        // Harvested fonttable.txt: 'A'=(8,8), 'a'=(264,8), space=(256,0), あ(U+3042)=(392,32).
        assert_eq!(atlas::pos(b'A' as u16), Some((8, 8)));
        assert_eq!(atlas::pos(b'a' as u16), Some((264, 8)));
        assert_eq!(atlas::pos(b' ' as u16), Some((256, 0)));
        assert_eq!(atlas::pos(0x3042), Some((392, 32)));
        // A codepoint with no glyph.
        assert_eq!(atlas::pos(0xFFFF), None);
    }

    #[test]
    fn font_glyph_a_matches_atlas_pixels() {
        // The glyph for 'A' at its atlas position has opaque pixels (the letter strokes).
        let f = default_font_page();
        let (gx, gy) = atlas::pos(b'A' as u16).unwrap();
        let opaque = (0..8)
            .flat_map(|dy| (0..8).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| {
                let p = (gy as usize + dy) * GRP_DIM + (gx as usize + dx);
                f.pixels[p] & 1 != 0
            })
            .count();
        assert!(opaque > 0, "'A' glyph should have opaque strokes");
    }
}
