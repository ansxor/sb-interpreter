//! GRP graphics pages (M2-T1) — SmileBASIC's drawable bitmap layers.
//!
//! SmileBASIC keeps **6 graphic pages** (GRP0..GRP5), each a **512×512** bitmap stored in
//! the device's 16-bit **RGBA5551** color format (bit layout `R[15:11] G[10:6] B[5:1]
//! A[0]`, little-endian — see `spec/concepts/screen-and-color-model.md`, disassembled from
//! the pixel-read helper `FUN_00191dfc`). By default GRP4 backs sprites and GRP5 backs BG.
//!
//! This module owns the page buffers + the graphics command state the VM mutates:
//! the **display** page (shown on screen), the **manipulation** page (target of drawing),
//! the current draw **color** (`GCOLOR`, ARGB8888), the screen **Z priority** (`GPRIO`),
//! and the display/write **clip** rectangles (`GCLIP`). The compositor (M2-T4) turns the
//! display page into the RGBA8888 [`Framebuffer`](crate::Framebuffer); drawing primitives
//! (M2-T2) write the manipulation page.
//!
//! Colors cross the boundary as user-facing **ARGB8888** (e.g. `RGB()`/`#WHITE`) and are
//! truncated to the device's 5-bit-per-channel precision on write; reading a pixel back
//! (`GSPOIT`) expands the 5 bits to 8 by left-shift (low 3 bits zero), so a written color
//! generally does **not** round-trip exactly — exactly the documented "passed through the
//! internal color representation" caveat. See [`argb8888_to_rgba5551`] /
//! [`rgba5551_to_argb8888`].

use crate::rgb555_to_argb8888;

/// Number of graphic pages: GRP0..GRP5.
pub const GRP_PAGE_COUNT: usize = 6;
/// Side length of each (square) graphic page, in pixels.
pub const GRP_DIM: usize = 512;
/// Visible draw-area width on the top screen (X 0..=399).
pub const GRP_VISIBLE_WIDTH: i32 = 400;
/// Visible draw-area height on the top screen (Y 0..=239).
pub const GRP_VISIBLE_HEIGHT: i32 = 240;
/// Default display area width (matches the boot top screen).
pub const GRP_DISPLAY_WIDTH_DEFAULT: i32 = GRP_VISIBLE_WIDTH;
/// Default display area height (matches the boot top screen).
pub const GRP_DISPLAY_HEIGHT_DEFAULT: i32 = GRP_VISIBLE_HEIGHT;

/// Pack an ARGB8888 color into the device's 16-bit RGBA5551 halfword.
///
/// Each 8-bit channel keeps its top 5 bits (`>> 3`); the alpha bit is set only when the
/// source alpha is fully opaque (`A == 255`), matching `GCOLOR`/`GCLS` semantics (any
/// other alpha is transparent). Inverse of [`rgba5551_to_argb8888`].
#[inline]
pub const fn argb8888_to_rgba5551(argb: u32) -> u16 {
    let a = (argb >> 24) & 0xff;
    let r5 = ((argb >> 16) & 0xff) >> 3;
    let g5 = ((argb >> 8) & 0xff) >> 3;
    let b5 = (argb & 0xff) >> 3;
    let a1 = if a == 255 { 1 } else { 0 };
    ((r5 << 11) | (g5 << 6) | (b5 << 1) | a1) as u16
}

/// Expand a 16-bit RGBA5551 halfword back to a user-facing ARGB8888 color.
///
/// The 5-bit channels expand to 8 bits by left-shift (low 3 bits zero, see
/// [`expand5`](crate::expand5)); alpha becomes 255 when the alpha bit is set, else 0.
/// Inverse of [`argb8888_to_rgba5551`] (lossy — the 8→5 truncation is not recovered).
#[inline]
pub const fn rgba5551_to_argb8888(h: u16) -> u32 {
    let r5 = ((h >> 11) & 0x1f) as u8;
    let g5 = ((h >> 6) & 0x1f) as u8;
    let b5 = ((h >> 1) & 0x1f) as u8;
    let a1 = (h & 1) != 0;
    rgb555_to_argb8888(r5, g5, b5, a1)
}

/// One 512×512 RGBA5551 graphic page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpPage {
    /// `GRP_DIM * GRP_DIM` device pixels, row-major, top-left origin.
    pub pixels: Vec<u16>,
}

impl GrpPage {
    /// A fresh page, cleared to transparent black (halfword 0).
    pub fn new() -> Self {
        Self {
            pixels: vec![0; GRP_DIM * GRP_DIM],
        }
    }
}

impl Default for GrpPage {
    fn default() -> Self {
        Self::new()
    }
}

/// An inclusive clip rectangle `(x0, y0, x1, y1)` with `x0 <= x1` and `y0 <= y1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// The default Touch-screen display area (width, height). The lower screen is 320×240 in
/// every `XSCREEN` mode (the Upper screen is 400×240 in the 3D modes, 320×240 otherwise).
pub const TOUCH_DISPLAY_WIDTH_DEFAULT: i32 = 320;
/// The default Touch-screen display area height.
pub const TOUCH_DISPLAY_HEIGHT_DEFAULT: i32 = GRP_DISPLAY_HEIGHT_DEFAULT;

/// The number of physical DISPLAY screens: 0 = Upper, 1 = Touch.
pub const GRP_SCREEN_COUNT: usize = 2;

/// The per-screen GRP **draw context**: which page each screen shows + draws, its Z priority,
/// and its display/write clip rectangles. The 6 pixel pages and `GCOLOR` are *not* here — they
/// are a shared pool / single global on [`GrpState`] (matching the osb structural model:
/// `GraphicPage[6]` shared, `showPage[2]`/`usePage[2]`/`gprios[2]`/`writeArea[2]`/
/// `displayArea[2]` per-display, one global `uint gcolor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrpScreen {
    /// Page shown on this screen (`GPAGE` display page), 0..=5.
    pub display_page: u8,
    /// Page targeted by this screen's drawing instructions (`GPAGE` manipulation page), 0..=5.
    pub manip_page: u8,
    /// This screen's Z priority (`GPRIO`), -256..=1024 (lower = nearer the viewer).
    pub prio: i32,
    /// Default display area (width, height) used by this screen's `GCLIP 0` reset. The VM
    /// updates this when `XSCREEN` / `DISPLAY` change the screen size.
    pub display_area: (i32, i32),
    /// Display clip rectangle (whole screen by default).
    pub display_clip: ClipRect,
    /// Write (drawing) clip rectangle (whole page by default).
    pub write_clip: ClipRect,
}

impl GrpScreen {
    /// A fresh draw context displaying + manipulating `display_page`, screen-surface Z, a
    /// display clip covering `display_area`, and a full-page write clip.
    fn new(display_page: u8, display_area: (i32, i32)) -> Self {
        Self {
            display_page,
            manip_page: 0,
            prio: 0,
            display_area,
            display_clip: ClipRect {
                x0: 0,
                y0: 0,
                x1: display_area.0 - 1,
                y1: display_area.1 - 1,
            },
            write_clip: ClipRect {
                x0: 0,
                y0: 0,
                x1: GRP_DIM as i32 - 1,
                y1: GRP_DIM as i32 - 1,
            },
        }
    }
}

/// The full GRP graphics state: the **shared** page buffers + global draw color, plus the
/// **per-screen** draw context (selected pages, Z priority, clip rectangles) for each of the
/// two DISPLAY screens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpState {
    /// The 6 graphic pages, GRP0..GRP5. A single shared pool — both screens draw into / display
    /// from the same pages.
    pub pages: Vec<GrpPage>,
    /// The hidden **GRPF** font page (GSAVE/GLOAD page index −1). Not on any `GPAGE`; it backs
    /// the console glyph source and the `"GRPF"` bitmap ops. Blank in [`GrpState::new`], seeded
    /// with the firmware font in [`GrpState::with_defaults`].
    pub grpf: GrpPage,
    /// Current draw color (`GCOLOR`), ARGB8888. A single global, shared by both screens.
    /// Default opaque white.
    pub color: u32,
    /// The per-screen draw context: index 0 = Upper, 1 = Touch.
    pub screens: [GrpScreen; GRP_SCREEN_COUNT],
    /// The DISPLAY screen subsequent GRP commands target (0 = Upper, 1 = Touch). Set by the
    /// `DISPLAY n` statement; defaults to the Upper screen.
    pub active: usize,
}

impl GrpState {
    /// A fresh GRP state: 6 blank shared pages, opaque-white draw color, and the two screens'
    /// default draw contexts — the Upper screen shows GRP0 (400×240), the Touch screen shows
    /// GRP1 (320×240) (osb `showPage = [0, 1]`), both drawing into GRP0 with full clips.
    pub fn new() -> Self {
        Self {
            pages: (0..GRP_PAGE_COUNT).map(|_| GrpPage::new()).collect(),
            grpf: GrpPage::new(),
            color: 0xFFFF_FFFF, // opaque white
            screens: [
                GrpScreen::new(0, (GRP_DISPLAY_WIDTH_DEFAULT, GRP_DISPLAY_HEIGHT_DEFAULT)),
                GrpScreen::new(
                    1,
                    (TOUCH_DISPLAY_WIDTH_DEFAULT, TOUCH_DISPLAY_HEIGHT_DEFAULT),
                ),
            ],
            active: 0,
        }
    }

    /// The active screen's draw context (the screen subsequent GRP commands target).
    pub fn cur(&self) -> &GrpScreen {
        &self.screens[self.active]
    }

    /// The active screen's draw context, mutably.
    pub fn cur_mut(&mut self) -> &mut GrpScreen {
        &mut self.screens[self.active]
    }

    /// A GRP state with the **firmware default pages** loaded, as SmileBASIC boots: GRP4 holds
    /// the default sprite sheet, GRP5 the default BG sheet, and the hidden GRPF page the system
    /// font (see [`crate::assets`]). GRP0..GRP3 stay blank. This is what the VM constructs; the
    /// bare [`GrpState::new`] keeps all pages blank for the unit/golden tests that draw their
    /// own pixels.
    pub fn with_defaults() -> Self {
        let mut state = Self::new();
        state.reload_defaults();
        state
    }

    /// Reload the firmware default pages: GRP4 ← the sprite sheet, GRP5 ← the BG sheet, GRPF ←
    /// the font. This is the "LOAD DEFSP/DEFBG" + font-reset part of boot and `ACLS` (see
    /// `spec/instructions/acls.yaml`). GRP0..GRP3 and the draw-state (selected pages, color,
    /// clips) are left untouched — those are separate ACLS reset steps.
    pub fn reload_defaults(&mut self) {
        self.pages[4] = crate::assets::default_sprite_page();
        self.pages[5] = crate::assets::default_bg_page();
        self.grpf = crate::assets::default_font_page();
    }

    /// Clear the active screen's manipulation page to `color` (`GCLS`).
    pub fn gcls(&mut self, color: u32) {
        let h = argb8888_to_rgba5551(color);
        let page = self.cur().manip_page as usize;
        for px in &mut self.pages[page].pixels {
            *px = h;
        }
    }

    /// Read one pixel's color from the active screen's manipulation page as ARGB8888
    /// (`GSPOIT`). Coordinates outside the 512×512 page return 0 (transparent black) — matching
    /// real SB 3.6.0 (the PTC/DSi `-1` does *not* apply, hw_verified).
    pub fn gspoit(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 || x >= GRP_DIM as i32 || y >= GRP_DIM as i32 {
            return 0;
        }
        let h = self.pages[self.cur().manip_page as usize].pixels[y as usize * GRP_DIM + x as usize];
        rgba5551_to_argb8888(h)
    }

    /// Reset a clip rectangle to its whole area (`GCLIP mode` with no rectangle) on the active
    /// screen: the screen's display size for display mode, the whole page for write mode.
    pub fn gclip_reset(&mut self, write: bool) {
        let screen = self.cur_mut();
        if write {
            screen.write_clip = ClipRect {
                x0: 0,
                y0: 0,
                x1: GRP_DIM as i32 - 1,
                y1: GRP_DIM as i32 - 1,
            };
        } else {
            let (w, h) = screen.display_area;
            screen.display_clip = ClipRect {
                x0: 0,
                y0: 0,
                x1: w - 1,
                y1: h - 1,
            };
        }
    }

    /// Set the active screen's display area and reset its display clip to match. Called by the
    /// VM when `XSCREEN` or `DISPLAY` changes the active screen size.
    pub fn set_display_area(&mut self, width: i32, height: i32) {
        let screen = self.cur_mut();
        screen.display_area = (width, height);
        screen.display_clip = ClipRect {
            x0: 0,
            y0: 0,
            x1: width - 1,
            y1: height - 1,
        };
    }

    /// Set a clip rectangle on the active screen (`GCLIP mode, x0, y0, x1, y1`). The corners
    /// are normalized so the smaller coordinate is the start, so they may be given in any
    /// order.
    pub fn gclip_rect(&mut self, write: bool, x0: i32, y0: i32, x1: i32, y1: i32) {
        let rect = ClipRect {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        };
        let screen = self.cur_mut();
        if write {
            screen.write_clip = rect;
        } else {
            screen.display_clip = rect;
        }
    }
}

impl Default for GrpState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba5551_round_trip_truncates_like_hardware() {
        // RGB(255,0,0) = 0xFFFF0000 truncates to 0xFFF80000 on read-back (R 255 -> 248).
        let argb = 0xFFFF_0000;
        let dev = argb8888_to_rgba5551(argb);
        assert_eq!(rgba5551_to_argb8888(dev), 0xFFF8_0000);
        // #WHITE round-trips to itself (already 5-bit aligned, 0xF8 per channel).
        assert_eq!(
            rgba5551_to_argb8888(argb8888_to_rgba5551(0xFFFF_FFFF)),
            0xFFF8_F8F8
        );
        // RGB(0,100,0): green top 5 bits = 12 -> 96 (0x60).
        assert_eq!(
            rgba5551_to_argb8888(argb8888_to_rgba5551(0xFF00_6400)),
            0xFF00_6000
        );
    }

    #[test]
    fn non_opaque_alpha_clears_the_alpha_bit() {
        // Any alpha != 255 is transparent: the device alpha bit is 0, so it reads back 0.
        let dev = argb8888_to_rgba5551(0x80FF_0000);
        assert_eq!(dev & 1, 0);
        assert_eq!(rgba5551_to_argb8888(dev), 0x00F8_0000);
    }

    #[test]
    fn fresh_state_defaults() {
        let g = GrpState::new();
        assert_eq!(g.pages.len(), GRP_PAGE_COUNT);
        assert_eq!(g.pages[0].pixels.len(), GRP_DIM * GRP_DIM);
        // osb showPage = [0, 1]: the Upper screen shows GRP0, the Touch screen GRP1; both
        // manipulate GRP0. The active (command-target) screen defaults to the Upper screen.
        assert_eq!((g.screens[0].display_page, g.screens[0].manip_page), (0, 0));
        assert_eq!((g.screens[1].display_page, g.screens[1].manip_page), (1, 0));
        assert_eq!(g.active, 0);
        assert_eq!((g.cur().display_page, g.cur().manip_page), (0, 0));
        assert_eq!(g.color, 0xFFFF_FFFF);
    }

    #[test]
    fn gcls_then_gspoit_reads_truncated_color() {
        let mut g = GrpState::new();
        g.gcls(0xFFFF_0000); // opaque red
        assert_eq!(g.gspoit(0, 0), 0xFFF8_0000);
        assert_eq!(g.gspoit(511, 511), 0xFFF8_0000);
    }

    #[test]
    fn gspoit_off_page_returns_zero() {
        let mut g = GrpState::new();
        g.gcls(0xFFFF_FFFF);
        assert_eq!(g.gspoit(-1, -1), 0);
        assert_eq!(g.gspoit(512, 0), 0);
        assert_eq!(g.gspoit(0, 512), 0);
        // A blank in-page pixel reads 0 too: GSPOIT(400,240) on a blank page is 0.
        let blank = GrpState::new();
        assert_eq!(blank.gspoit(400, 240), 0);
    }

    #[test]
    fn gcls_targets_only_the_manip_page() {
        let mut g = GrpState::new();
        g.cur_mut().manip_page = 2;
        g.gcls(0xFFFF_FFFF);
        assert_eq!(g.pages[2].pixels[0], argb8888_to_rgba5551(0xFFFF_FFFF));
        assert_eq!(g.pages[0].pixels[0], 0); // GRP0 untouched
    }

    #[test]
    fn gclip_rect_normalizes_corners() {
        let mut g = GrpState::new();
        g.gclip_rect(true, 200, 200, 100, 100);
        assert_eq!(
            g.cur().write_clip,
            ClipRect {
                x0: 100,
                y0: 100,
                x1: 200,
                y1: 200
            }
        );
    }

    #[test]
    fn per_screen_draw_context_is_isolated() {
        let mut g = GrpState::new();
        // Touch screen defaults to displaying GRP1 (osb showPage = [0, 1]).
        assert_eq!(g.screens[1].display_page, 1);
        // The Touch screen's default display area is 320×240, not the Upper screen's 400×240.
        assert_eq!(g.screens[0].display_area, (400, 240));
        assert_eq!(g.screens[1].display_area, (320, 240));

        // Selecting the Touch screen routes draw-state mutations to screen 1 only.
        g.active = 1;
        g.cur_mut().manip_page = 4;
        g.cur_mut().display_page = 3;
        g.gclip_rect(false, 10, 10, 50, 50);
        assert_eq!(g.screens[1].manip_page, 4);
        assert_eq!(g.screens[1].display_page, 3);
        // Screen 0 (Upper) is untouched.
        assert_eq!(g.screens[0].manip_page, 0);
        assert_eq!(g.screens[0].display_page, 0);
        assert_eq!(
            g.screens[0].display_clip,
            ClipRect {
                x0: 0,
                y0: 0,
                x1: 399,
                y1: 239
            }
        );
    }
}
