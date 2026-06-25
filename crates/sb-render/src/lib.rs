//! `sb-render` — SmileBASIC's screen compositor + an in-memory framebuffer.
//!
//! The VM composites SB's layers (backdrop → GRP → BG → sprite → console) into an
//! in-memory RGBA8888 [`Framebuffer`]. That single buffer is both (a) what the platform
//! crates blit to a window/canvas and (b) what the conformance harness pixel-diffs
//! against the emulator's framebuffer. One renderer, two consumers.
//!
//! M1 establishes the framebuffer + SB's exact color expansion (this module) and the
//! **console** text model + renderer ([`console`]). GRP pages, BG, and sprites arrive in
//! M2–M3. The crate stays I/O- and GUI-free so it builds for `wasm32-unknown-unknown`;
//! file I/O (golden PNGs) lives in the `png` encoder + tests only.

pub mod anim;
pub mod assets;
pub mod bg;
pub mod bitmap;
pub mod compositor;
pub mod console;
pub mod font;
pub mod grp;
pub mod inflate;
pub mod png;
pub mod raster;
pub mod sprite;

/// The 3DS upper screen is 400×240; the lower (touch) screen is 320×240.
pub const TOP_WIDTH: usize = 400;
pub const TOP_HEIGHT: usize = 240;
pub const BOTTOM_WIDTH: usize = 320;
pub const BOTTOM_HEIGHT: usize = 240;

/// A simple RGBA8888 framebuffer (`[r, g, b, a]` per pixel, row-major, top-left origin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>, // width * height * 4
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height * 4],
        }
    }

    pub fn top() -> Self {
        Self::new(TOP_WIDTH, TOP_HEIGHT)
    }

    pub fn bottom() -> Self {
        Self::new(BOTTOM_WIDTH, BOTTOM_HEIGHT)
    }

    /// Fill the whole buffer with a packed ARGB8888 color.
    pub fn clear(&mut self, argb: u32) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_argb(x, y, argb);
            }
        }
    }

    /// Set a pixel to a packed ARGB8888 color (`0xAARRGGBB`). Out-of-bounds is ignored.
    #[inline]
    pub fn set_argb(&mut self, x: usize, y: usize, argb: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = (y * self.width + x) * 4;
        self.pixels[i] = (argb >> 16) as u8; // R
        self.pixels[i + 1] = (argb >> 8) as u8; // G
        self.pixels[i + 2] = argb as u8; // B
        self.pixels[i + 3] = (argb >> 24) as u8; // A
    }

    /// Read a pixel back as packed ARGB8888. Out-of-bounds returns 0.
    #[inline]
    pub fn get_argb(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let i = (y * self.width + x) * 4;
        ((self.pixels[i + 3] as u32) << 24)
            | ((self.pixels[i] as u32) << 16)
            | ((self.pixels[i + 1] as u32) << 8)
            | (self.pixels[i + 2] as u32)
    }
}

/// Expand a SmileBASIC 5-bit color channel (0..=31) to 8 bits.
///
/// FIDELITY: SB3 expands by left-shift only (low 3 bits are zero), NOT the common
/// `(v<<3)|(v>>2)` rounding. Evidence (hw_verified, see
/// `spec/concepts/screen-and-color-model.md`): the constant `#WHITE = &HFFF8F8F8` has
/// channel value `0xF8 = 248 = 31<<3`, not `0xFF`; the disassembled pixel-read helper
/// `FUN_00191dfc` masks each output byte with `0xF8`/`0xF800`/`0xF80000`.
#[inline]
pub const fn expand5(c5: u8) -> u8 {
    (c5 & 0x1F) << 3
}

/// Quantize an 8-bit channel to SB's 5-bit device precision (top 5 bits, low 3 forced 0).
#[inline]
pub const fn quantize8(c8: u8) -> u8 {
    c8 & 0xF8
}

/// Build a packed ARGB8888 color from SB 5-bit RGB components + a 1-bit alpha.
#[inline]
pub const fn rgb555_to_argb8888(r5: u8, g5: u8, b5: u8, a1: bool) -> u32 {
    let a = if a1 { 0xFFu32 } else { 0x00 };
    (a << 24) | ((expand5(r5) as u32) << 16) | ((expand5(g5) as u32) << 8) | (expand5(b5) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_matches_documented_constant() {
        // #WHITE = &HFFF8F8F8 — proves the <<3 (low-bits-zero) expansion.
        assert_eq!(rgb555_to_argb8888(31, 31, 31, true), 0xFFF8F8F8);
    }

    #[test]
    fn expand5_drops_low_bits() {
        assert_eq!(expand5(31), 0xF8);
        assert_eq!(expand5(0), 0x00);
        assert_eq!(expand5(1), 0x08);
    }

    #[test]
    fn quantize8_keeps_top_five_bits() {
        assert_eq!(quantize8(0xFF), 0xF8);
        assert_eq!(quantize8(0x7F), 0x78);
        assert_eq!(quantize8(0x80), 0x80);
    }

    #[test]
    fn framebuffer_set_get_pixel() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_argb(1, 0, 0xFF112233);
        assert_eq!(&fb.pixels[4..8], &[0x11, 0x22, 0x33, 0xFF]);
        assert_eq!(fb.get_argb(1, 0), 0xFF112233);
        // Out-of-bounds is a no-op / 0.
        fb.set_argb(99, 99, 0xFFFFFFFF);
        assert_eq!(fb.get_argb(99, 99), 0);
    }
}
