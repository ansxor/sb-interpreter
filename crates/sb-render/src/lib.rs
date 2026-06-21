//! `sb-render` — SmileBASIC's screen compositor.
//!
//! The VM composites SB's layers (backdrop -> GRP -> BG -> sprite -> console) into an
//! in-memory RGBA8888 [`Framebuffer`]. That single buffer is both (a) what the
//! platform crates blit to a window/canvas and (b) what the conformance harness
//! pixel-diffs against the emulator's framebuffer. One renderer, two consumers.
//!
//! Layers, GRP pages, BG, and sprites arrive in milestones M2–M3; M0 establishes the
//! framebuffer + SB's exact color expansion.

/// The 3DS upper screen is 400x240; the lower (touch) screen is 320x240.
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

    /// Set a pixel to a packed ARGB8888 color (`0xAARRGGBB`).
    pub fn set_argb(&mut self, x: usize, y: usize, argb: u32) {
        let i = (y * self.width + x) * 4;
        self.pixels[i] = (argb >> 16) as u8; // R
        self.pixels[i + 1] = (argb >> 8) as u8; // G
        self.pixels[i + 2] = argb as u8; // B
        self.pixels[i + 3] = (argb >> 24) as u8; // A
    }
}

/// Expand a SmileBASIC 5-bit color channel (0..=31) to 8 bits.
///
/// FIDELITY: SB3 expands by left-shift only (low 3 bits are zero), NOT the common
/// `(v<<3)|(v>>2)` rounding. Evidence: the builtin constant `#WHITE = &HFFF8F8F8`
/// (`sb-docs/.../reference/constants.md`) has channel value `0xF8 = 248 = 31<<3`, not
/// `0xFF`. Confirm the full ramp against the disassembly during harvest.
#[inline]
pub const fn expand5(c5: u8) -> u8 {
    (c5 & 0x1F) << 3
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
    fn framebuffer_set_pixel() {
        let mut fb = Framebuffer::new(2, 2);
        fb.set_argb(1, 0, 0xFF112233);
        assert_eq!(&fb.pixels[4..8], &[0x11, 0x22, 0x33, 0xFF]);
    }
}
