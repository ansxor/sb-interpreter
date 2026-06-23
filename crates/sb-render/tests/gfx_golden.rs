//! M2-T5 acceptance (Rust gate): the GRP rasterizers + color math + page→framebuffer
//! conversion are **deterministic** and produce the exact device pixels the committed
//! graphics goldens encode.
//!
//! These lock the same behavior the golden PNGs in `harness/corpus/golden/gfx/` pin, but
//! WITHOUT a PNG decode: each test draws on a [`GrpState`] page (the very ops the golden
//! programs run — `GCLS`/`GPSET`/`GFILL`/`GLINE`), converts the page to an RGBA8888
//! [`Framebuffer`] with [`grp_page_to_framebuffer`] (the surface `sb-run --grp` writes and
//! `replay.py` pixel-diffs), and asserts known pixels. The file-level pixel-diff against the
//! committed goldens runs in `harness/diff/replay.py` (hermetic, no emulator).
//!
//! Color math here is the device truth (hw_verified, S-C2): 8→5-bit truncation on write,
//! 5→8 left-shift expansion on read, so e.g. `RGB(255,128,0)` stores then reads back as
//! `0xFFF88000`, and an alpha-bit-clear (unset) pixel reads back as transparent black.

use sb_render::compositor::grp_page_to_framebuffer;
use sb_render::grp::{rgba5551_to_argb8888, GrpState, GRP_VISIBLE_HEIGHT, GRP_VISIBLE_WIDTH};
use sb_render::Framebuffer;

const W: usize = GRP_VISIBLE_WIDTH as usize;
const H: usize = GRP_VISIBLE_HEIGHT as usize;

/// Opaque ARGB8888 from 8-bit channels (the `RGB(r,g,b)` the golden programs use).
fn rgb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Draw on the default page, then snapshot it as the visible 400×240 framebuffer.
fn render(draw: impl FnOnce(&mut GrpState)) -> Framebuffer {
    let mut g = GrpState::new();
    draw(&mut g);
    grp_page_to_framebuffer(&g.pages[g.display_page as usize], W, H)
}

#[test]
fn page_to_framebuffer_is_deterministic() {
    // The conversion must be a pure function of the page — the whole point of a golden.
    let a = render(|g| g.gpset(1, 2, rgb(255, 0, 0)));
    let b = render(|g| g.gpset(1, 2, rgb(255, 0, 0)));
    assert_eq!(a, b);
}

#[test]
fn color_truncation_round_trips_to_device() {
    // 8→5 truncation then 5→8 left-shift expansion: 128 (0x80) -> 16 (0x10) -> 0x80,
    // 255 -> 31 -> 0xF8, 0 -> 0. So RGB(255,128,0) becomes 0xFFF88000 on the page.
    let fb = render(|g| g.gpset(5, 5, rgb(255, 128, 0)));
    assert_eq!(fb.get_argb(5, 5), 0xFFF8_8000);
    assert_eq!(rgba5551_to_argb8888(0), 0x0000_0000); // unset device pixel = transparent black
}

#[test]
fn gcls_fills_visible_page_with_truncated_color() {
    // gcls_blue.sb3: GCLS RGB(0,0,255) -> every visible pixel opaque blue 0xFF0000F8.
    let fb = render(|g| g.gcls(rgb(0, 0, 255)));
    assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8);
    assert_eq!(fb.get_argb(W - 1, H - 1), 0xFF00_00F8);
    assert_eq!(fb.get_argb(200, 120), 0xFF00_00F8);
}

#[test]
fn gpset_lights_only_the_plotted_pixel() {
    // gpset_corners.sb3: a white center dot on an otherwise transparent page.
    let fb = render(|g| g.gpset(200, 120, rgb(255, 255, 255)));
    assert_eq!(fb.get_argb(200, 120), 0xFFF8_F8F8);
    assert_eq!(fb.get_argb(199, 120), 0x0000_0000); // neighbor untouched
    assert_eq!(fb.get_argb(201, 121), 0x0000_0000);
}

#[test]
fn gfill_covers_its_inclusive_rect_only() {
    // gfill_box.sb3: GFILL 10,20,100,80,RGB(255,128,0).
    let fb = render(|g| g.gfill(10, 20, 100, 80, rgb(255, 128, 0)));
    assert_eq!(fb.get_argb(10, 20), 0xFFF8_8000); // top-left corner inside
    assert_eq!(fb.get_argb(100, 80), 0xFFF8_8000); // bottom-right corner inside (inclusive)
    assert_eq!(fb.get_argb(55, 50), 0xFFF8_8000); // interior
    assert_eq!(fb.get_argb(9, 20), 0x0000_0000); // just left of the rect
    assert_eq!(fb.get_argb(101, 80), 0x0000_0000); // just right of the rect
}

#[test]
fn gline_plots_inclusive_endpoints() {
    // A short HORIZONTAL run: both endpoints lit, one past the end is not. Axis-aligned lines
    // match the device exactly (hw_verified via the scene_mixed GBOX golden); the *diagonal*
    // GLINE/GTRI stepping diverges from SB's fixed-point DDA and is queued (HARVEST_QUEUE.md,
    // M2-T2), so no diagonal-line golden is committed yet.
    let fb = render(|g| g.gline(0, 0, 5, 0, rgb(255, 255, 255)));
    assert_eq!(fb.get_argb(0, 0), 0xFFF8_F8F8);
    assert_eq!(fb.get_argb(5, 0), 0xFFF8_F8F8);
    assert_eq!(fb.get_argb(6, 0), 0x0000_0000);
    assert_eq!(fb.get_argb(0, 1), 0x0000_0000); // stays on its row
}
