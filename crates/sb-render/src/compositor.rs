//! Compositor (M2-T4) — stacks SmileBASIC's screen layers into the final framebuffer.
//!
//! SmileBASIC's displayed image is the front-to-back composite of several layers
//! (`spec/concepts/screen-and-color-model.md`):
//!
//! 1. **Backdrop** (a solid color — `BACKCOLOR`)
//! 2. **GRP** graphics pages (the `GPAGE` *display* page)
//! 3. **BG** tilemap layers  *(content arrives in M3 — slot reserved here)*
//! 4. **Sprites**            *(content arrives in M3 — slot reserved here)*
//! 5. **Console** (text)
//!
//! Each layer carries a **Z depth**; the model spans `1024` (rear) … `0` (screen plane) …
//! `-256` (front) and **smaller Z draws in front**. The compositor paints rear→front
//! (largest Z first, smallest Z last) so the front-most layer wins where layers overlap.
//! At an equal Z the *caller's slice order* breaks the tie (a stable sort), which encodes
//! the documented back→front layer order GRP < BG < sprite < console.
//!
//! ## Layer interface (M3 plugs in here)
//! A layer is anything implementing [`Layer`]: it reports its [`z`](Layer::z) and knows how
//! to [`composite`](Layer::composite) itself onto the framebuffer. M2 ships the [`GrpLayer`]
//! (a GRP display page) and the [`ConsoleLayer`]; M3's BG/sprite layers will be additional
//! `impl Layer` types dropped into the same [`compose`] call — the slots are reserved by the
//! Z model, not hard-coded here.
//!
//! Pixel transparency: the device GRP page is **1-bit alpha**, so a GRP pixel either fully
//! covers (alpha bit set) or is skipped (alpha bit clear → the layers behind show through).
//! The console paints over via its own [`Console::render`](crate::console::Console::render)
//! (palette index 0 = transparent). Partial (8-bit) sprite/console alpha blending is a
//! composite-capture question for O-T6 (queued) — M2 composites with the device 1-bit key.

use crate::console::Console;
use crate::grp::{rgba5551_to_argb8888, ClipRect, GrpPage, GrpState, GRP_DIM};
use crate::{Framebuffer, TOP_HEIGHT, TOP_WIDTH};

/// Default backdrop: opaque black. The console's default background is transparent, so a
/// visible backdrop is required for the composite to land on a surface.
///
/// FIDELITY: the exact `BACKCOLOR`→backdrop composite (and its default) is oracle-pending —
/// the *composite* framebuffer capture (O-T6) hasn't been harvested; queued in
/// `HARVEST_QUEUE.md`. Callers may pass any ARGB8888 backdrop to [`compose`].
pub const DEFAULT_BACKDROP: u32 = 0xFF00_0000;

/// Default Z for the console layer (screen plane). The per-layer *default* Z values and the
/// exact equal-Z tie-break are oracle-pending (O-T6 composite capture, queued); M2 places
/// the console at the screen plane and relies on the stable slice-order tie-break to keep it
/// in front of a default-priority GRP page (GRP default `GPRIO` is also `0`).
pub const CONSOLE_DEFAULT_Z: i32 = 0;

/// A composited screen layer: it knows its Z depth and how to paint itself onto `fb`.
///
/// Implement this for each layer kind. M2 ships [`GrpLayer`] + [`ConsoleLayer`]; M3 adds
/// BG/sprite layers. [`compose`] sorts a set of layers rear→front by [`z`](Layer::z) and
/// calls [`composite`](Layer::composite) on each in order.
pub trait Layer {
    /// Z depth; smaller draws in front (`1024` rear … `0` screen … `-256` front).
    fn z(&self) -> i32;
    /// Paint this layer onto `fb`, leaving transparent pixels untouched so layers already
    /// painted behind it show through.
    fn composite(&self, fb: &mut Framebuffer);
}

/// A GRP graphics page as a composited layer: the visible window is the **top-left crop** of
/// the 512×512 page, masked by the display `clip`. Device pixels with the alpha bit clear are
/// transparent (skipped); opaque pixels expand RGBA5551→ARGB8888 ([`rgba5551_to_argb8888`]).
pub struct GrpLayer<'a> {
    /// The page to display (typically the `GPAGE` display page).
    pub page: &'a GrpPage,
    /// The display clip rectangle (inclusive) — pixels outside it are not drawn.
    pub clip: ClipRect,
    /// This layer's Z depth (`GPRIO`).
    pub z: i32,
}

impl Layer for GrpLayer<'_> {
    fn z(&self) -> i32 {
        self.z
    }

    fn composite(&self, fb: &mut Framebuffer) {
        // Intersect the display clip with the framebuffer window and the page extent.
        let x0 = self.clip.x0.max(0);
        let y0 = self.clip.y0.max(0);
        let x1 = self
            .clip
            .x1
            .min(fb.width as i32 - 1)
            .min(GRP_DIM as i32 - 1);
        let y1 = self
            .clip
            .y1
            .min(fb.height as i32 - 1)
            .min(GRP_DIM as i32 - 1);
        for y in y0..=y1 {
            let row = y as usize * GRP_DIM;
            for x in x0..=x1 {
                let h = self.page.pixels[row + x as usize];
                // 1-bit alpha key: a clear alpha bit means "show the layers behind".
                if h & 1 == 0 {
                    continue;
                }
                fb.set_argb(x as usize, y as usize, rgba5551_to_argb8888(h));
            }
        }
    }
}

/// The text console as a composited layer (delegates to [`Console::render`], which already
/// honors the per-cell transparent background).
pub struct ConsoleLayer<'a> {
    /// The console grid to paint.
    pub console: &'a Console,
    /// This layer's Z depth.
    pub z: i32,
}

impl Layer for ConsoleLayer<'_> {
    fn z(&self) -> i32 {
        self.z
    }

    fn composite(&self, fb: &mut Framebuffer) {
        self.console.render(fb);
    }
}

/// Composite `layers` onto a fresh `width`×`height` framebuffer filled with `backdrop`
/// (ARGB8888). Layers paint rear→front: the largest Z first, the smallest Z (front-most)
/// last, so the front-most layer wins where opaque pixels overlap. Equal Z keeps the slice
/// order (stable sort) — pass layers in back→front order (GRP, BG, sprite, console) so the
/// tie resolves to the documented layer stack.
pub fn compose(width: usize, height: usize, backdrop: u32, layers: &[&dyn Layer]) -> Framebuffer {
    let mut fb = Framebuffer::new(width, height);
    fb.clear(backdrop);
    let mut order: Vec<&dyn Layer> = layers.to_vec();
    // Stable sort by descending Z: rear (large Z) first, front (small Z) last.
    order.sort_by_key(|l| core::cmp::Reverse(l.z()));
    for layer in order {
        layer.composite(&mut fb);
    }
    fb
}

/// Compose the standard top-screen scene from the VM's graphics state: backdrop → the GRP
/// display page (at its `GPRIO` Z, cropped to its display clip) → console (front). BG and
/// sprite layers (M3) slot in between by Z. Returns the 400×240 top-screen framebuffer.
pub fn compose_top_screen(grp: &GrpState, console: &Console, backdrop: u32) -> Framebuffer {
    let grp_layer = GrpLayer {
        page: &grp.pages[grp.display_page as usize],
        clip: grp.display_clip,
        z: grp.prio,
    };
    let console_layer = ConsoleLayer {
        console,
        z: CONSOLE_DEFAULT_Z,
    };
    compose(
        TOP_WIDTH,
        TOP_HEIGHT,
        backdrop,
        &[&grp_layer, &console_layer],
    )
}

/// Render the top-left `width`×`height` crop of a GRP page to an RGBA8888 framebuffer — the
/// **golden surface** pixel-diffed against an oracle GRP capture (M2-T5).
///
/// Unlike [`GrpLayer::composite`] (which alpha-keys transparent pixels onto a backdrop),
/// this reproduces the raw page exactly as `SAVE"GRPn:NAME"` does on real SB 3.6.0: every
/// device pixel is expanded RGBA5551→ARGB8888 ([`rgba5551_to_argb8888`]), so an alpha-bit-
/// clear pixel becomes fully-transparent black `0x00000000`. That matches the oracle decode
/// (`sb_grp.py`, shift expansion, O-T6) byte-for-byte, so a committed golden PNG diffs to
/// zero against a clean renderer. The page is 512×512; the visible top screen is its top-left
/// 400×240 crop (`GRP_VISIBLE_WIDTH`×`GRP_VISIBLE_HEIGHT`), the usual `width`/`height`.
pub fn grp_page_to_framebuffer(page: &GrpPage, width: usize, height: usize) -> Framebuffer {
    let mut fb = Framebuffer::new(width, height);
    for y in 0..height.min(GRP_DIM) {
        let row = y * GRP_DIM;
        for x in 0..width.min(GRP_DIM) {
            fb.set_argb(x, y, rgba5551_to_argb8888(page.pixels[row + x]));
        }
    }
    fb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grp::{argb8888_to_rgba5551, GrpState};

    fn full_clip() -> ClipRect {
        ClipRect {
            x0: 0,
            y0: 0,
            x1: TOP_WIDTH as i32 - 1,
            y1: TOP_HEIGHT as i32 - 1,
        }
    }

    #[test]
    fn grp_opaque_pixel_shows_over_backdrop() {
        let mut grp = GrpState::new();
        grp.gcls(0xFFFF_0000); // opaque red fill on the display page
        let fb = compose_top_screen(&grp, &Console::top(), DEFAULT_BACKDROP);
        // Red truncates through the device format to 0xFFF80000.
        assert_eq!(fb.get_argb(0, 0), 0xFFF8_0000);
        assert_eq!(fb.get_argb(399, 239), 0xFFF8_0000);
    }

    #[test]
    fn grp_transparent_pixel_lets_backdrop_through() {
        let grp = GrpState::new(); // blank page = all halfword 0 = alpha bit clear
        let fb = compose_top_screen(&grp, &Console::top(), DEFAULT_BACKDROP);
        // Nothing drawn: the opaque-black backdrop survives.
        assert_eq!(fb.get_argb(10, 10), DEFAULT_BACKDROP);
    }

    #[test]
    fn console_paints_in_front_of_grp_at_equal_z() {
        let mut grp = GrpState::new();
        grp.gcls(0xFFFF_0000); // red GRP behind
        let mut console = Console::top();
        console.print_str("A"); // white-on-transparent glyph at (0,0)
                                // GRP prio 0 ties with the console's screen-plane Z; console wins by slice order.
        let fb = compose_top_screen(&grp, &console, DEFAULT_BACKDROP);
        // Some glyph foreground pixel in the first cell is white (not the red GRP).
        let mut saw_white = false;
        for y in 0..8 {
            for x in 0..8 {
                if fb.get_argb(x, y) == 0xFFF8_F8F8 {
                    saw_white = true;
                }
            }
        }
        assert!(saw_white, "console glyph should paint over the GRP layer");
        // The console cell's transparent background still shows the red GRP beneath.
        assert_eq!(fb.get_argb(7, 7), 0xFFF8_0000);
    }

    #[test]
    fn smaller_z_draws_in_front() {
        // Two full-page layers; the one with the smaller Z must win.
        let mut rear = GrpPage::new();
        for p in &mut rear.pixels {
            *p = argb8888_to_rgba5551(0xFFFF_0000); // red, rear
        }
        let mut front = GrpPage::new();
        for p in &mut front.pixels {
            *p = argb8888_to_rgba5551(0xFF00_FF00); // green, front
        }
        let rear_layer = GrpLayer {
            page: &rear,
            clip: full_clip(),
            z: 100,
        };
        let front_layer = GrpLayer {
            page: &front,
            clip: full_clip(),
            z: -100,
        };
        // Pass rear first; pass front first — result is identical (sorted by Z, not order).
        let a = compose(
            TOP_WIDTH,
            TOP_HEIGHT,
            DEFAULT_BACKDROP,
            &[&rear_layer, &front_layer],
        );
        let b = compose(
            TOP_WIDTH,
            TOP_HEIGHT,
            DEFAULT_BACKDROP,
            &[&front_layer, &rear_layer],
        );
        assert_eq!(a.get_argb(0, 0), 0xFF00_F800); // green wins (front)
        assert_eq!(a.pixels, b.pixels);
    }

    #[test]
    fn display_clip_crops_the_grp_layer() {
        let mut grp = GrpState::new();
        grp.gcls(0xFFFF_FFFF); // opaque white everywhere
        grp.display_clip = ClipRect {
            x0: 5,
            y0: 5,
            x1: 9,
            y1: 9,
        };
        let fb = compose_top_screen(&grp, &Console::top(), DEFAULT_BACKDROP);
        assert_eq!(fb.get_argb(4, 4), DEFAULT_BACKDROP); // outside clip = backdrop
        assert_eq!(fb.get_argb(5, 5), 0xFFF8_F8F8); // inside clip = white
        assert_eq!(fb.get_argb(9, 9), 0xFFF8_F8F8);
        assert_eq!(fb.get_argb(10, 10), DEFAULT_BACKDROP); // past clip = backdrop
    }

    #[test]
    fn compose_top_screen_honors_display_page() {
        let mut grp = GrpState::new();
        grp.manip_page = 3;
        grp.gcls(0xFF00_00FF); // draw blue on page 3
        grp.display_page = 3; // and show page 3
        let fb = compose_top_screen(&grp, &Console::top(), DEFAULT_BACKDROP);
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8); // blue (5-bit truncated)
    }
}
