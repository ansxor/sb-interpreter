//! BG (background tilemap) system (M3-T4) — SmileBASIC's scrolling tiled layers.
//!
//! SmileBASIC keeps **4 BG layers** (numbers 0..3). Each layer is a tilemap of
//! character (tile) cells plus a set of per-layer display transforms. A cell holds one
//! 16-bit *screen data* value: the character (glyph) number in the low bits plus rotation
//! and horizontal/vertical flip attribute bits. The tiles are sampled from one shared
//! graphic page (`BGPAGE`, default GRP5) and composited into the framebuffer (M3-T6).
//!
//! This module owns the **core** model the VM drives in M3-T4:
//! - map sizing — `BGSCREEN` (width × height in char units, an optional 8/16/32-px tile
//!   size; initial 25×15), and the shared `BGPAGE`;
//! - tile cells — `BGPUT` (write one cell), `BGGET` (read one cell, char- or pixel-coord),
//!   `BGFILL` (fill a rectangle), `BGCLR` (clear one layer or all);
//! - per-layer transforms — `BGOFS` (scroll offset + depth Z), `BGROT` (rotation,
//!   normalized mod 360), `BGSCALE` (X/Y enlargement, unclamped), `BGHOME` (rotation/scale
//!   pivot), `BGCOLOR` (ARGB multiply tint), `BGSHOW`/`BGHIDE` (visibility), and `BGCLIP`
//!   (display-area rectangle, in pixels).
//!
//! Animation/coordinate-conversion/load-save (`BGANIM`/`BGCOORD`/`BGCOPY`/…, M3-T5) and the
//! actual blit into the [`Framebuffer`](crate::Framebuffer) (M3-T6) build on this model.
//!
//! Structural cross-check: `osb/SMILEBASIC/bg.d` (3.5.0). The contract is the
//! `spec/instructions/bg*.yaml` set (disassembled handlers; layer range-check via the
//! shared getter `FUN_001e2504`, errnum 10 when layer ∉ 0..count(=4)). The on-screen
//! *side effects* — the rendered tint, scroll/rotate/scale pixels, clip area — block on the
//! BG framebuffer oracle (O-T6); the tilemap-cell storage, the mod-360 angle normalization,
//! and the transform read-backs are deterministic and unit-tested here.

/// Number of BG layers: 0..3 (the BG-layer count `[[0x315d60]+0x60]` = 4).
pub const BG_LAYER_COUNT: usize = 4;
/// Default graphic page BG tiles are sampled from (`BGPAGE` default = GRP5).
pub const BG_PAGE_DEFAULT: u8 = 5;
/// Initial map width for a layer, in char units (`BGSCREEN` initial 25×15 — sized to fill
/// the top screen with 16×16 tiles).
pub const BG_DEFAULT_WIDTH: i32 = 25;
/// Initial map height for a layer, in char units.
pub const BG_DEFAULT_HEIGHT: i32 = 15;
/// Default tile (character) pixel size when the 4th `BGSCREEN` arg is omitted.
pub const BG_DEFAULT_TILE_SIZE: i32 = 16;
/// Maximum `width * height` (cells) for one layer's map (`BGSCREEN` `cmp #0x3fff`).
pub const BG_MAX_CELLS: i32 = 16383;

/// One BG layer: its tilemap plus the display transforms `BGOFS`/`BGROT`/`BGSCALE`/
/// `BGHOME`/`BGCOLOR`/`BGCLIP`/`BGSHOW`. `cells` is `width * height` 16-bit screen-data
/// values, row-major (`index = y * width + x`); every other field is a display transform
/// the M3-T6 compositor will read. Power-on / reset state is a 25×15 map of empty cells.
#[derive(Debug, Clone, PartialEq)]
pub struct BgLayer {
    /// Map width in char units (`BGSCREEN` width).
    pub width: i32,
    /// Map height in char units (`BGSCREEN` height).
    pub height: i32,
    /// Tile (character) pixel size: 8, 16, or 32 (`BGSCREEN` 4th arg, default 16).
    pub tile_size: i32,
    /// `width * height` cells of 16-bit screen data, row-major. Character number 0 = empty.
    pub cells: Vec<u16>,
    /// Visibility flag (`BGSHOW`/`BGHIDE`). Layers are visible by default (the off-screen
    /// rendered result is oracle-pending — see `HARVEST_QUEUE.md`).
    pub visible: bool,
    /// Display offset / scroll, in pixels (`BGOFS` X,Y).
    pub ofs_x: i32,
    pub ofs_y: i32,
    /// Depth coordinate (`BGOFS` Z): rear 1024 < screen surface 0 < front -256.
    pub ofs_z: i32,
    /// Rotation angle in degrees, normalized to 0..359 (`BGROT`).
    pub rot: i32,
    /// Enlargement scale (`BGSCALE`), 1.0 = 100%. Stored unclamped (no 0.5–2.0 guard).
    pub scale_x: f64,
    pub scale_y: f64,
    /// Display origin / rotation-scale pivot, in pixels (`BGHOME`).
    pub home_x: i32,
    pub home_y: i32,
    /// ARGB8888 multiply tint (`BGCOLOR`); default opaque white (`&HFFFFFFFF`) = unchanged.
    pub color: u32,
    /// Display (clip) area in pixels (`BGCLIP`), normalized `(start_x,start_y,end_x,end_y)`;
    /// `None` = the whole layer.
    pub clip: Option<(i32, i32, i32, i32)>,
}

impl Default for BgLayer {
    fn default() -> Self {
        Self {
            width: BG_DEFAULT_WIDTH,
            height: BG_DEFAULT_HEIGHT,
            tile_size: BG_DEFAULT_TILE_SIZE,
            cells: vec![0; (BG_DEFAULT_WIDTH * BG_DEFAULT_HEIGHT) as usize],
            visible: true,
            ofs_x: 0,
            ofs_y: 0,
            ofs_z: 0,
            rot: 0,
            scale_x: 1.0,
            scale_y: 1.0,
            home_x: 0,
            home_y: 0,
            color: 0xFFFF_FFFF,
            clip: None,
        }
    }
}

impl BgLayer {
    /// Whether `(x, y)` is a valid char-unit cell of this map (`0..width`, `0..height`).
    pub fn in_cell(&self, x: i32, y: i32) -> bool {
        (0..self.width).contains(&x) && (0..self.height).contains(&y)
    }

    /// Read the cell at char coordinate `(x, y)` (the caller has range-checked it).
    pub fn cell(&self, x: i32, y: i32) -> u16 {
        self.cells[(y * self.width + x) as usize]
    }

    /// Write `data` into the cell at char coordinate `(x, y)` (caller range-checked).
    pub fn set_cell(&mut self, x: i32, y: i32, data: u16) {
        self.cells[(y * self.width + x) as usize] = data;
    }
}

/// The BG system state: the 4-layer table plus the shared graphic page. The VM mutates it
/// for the BG commands; the compositor reads it (M3-T6).
#[derive(Debug, Clone, PartialEq)]
pub struct BgState {
    /// The 4 BG layers, indexed by layer number 0..3.
    pub layers: Vec<BgLayer>,
    /// Shared graphic page BG tiles are sampled from (`BGPAGE`), 0..5; default GRP5.
    pub page: u8,
}

impl Default for BgState {
    fn default() -> Self {
        Self::new()
    }
}

impl BgState {
    /// A fresh BG system: 4 layers each 25×15 of empty cells, page GRP5.
    pub fn new() -> Self {
        Self {
            layers: vec![BgLayer::default(); BG_LAYER_COUNT],
            page: BG_PAGE_DEFAULT,
        }
    }

    /// Whether a layer number is in range (0..3).
    pub fn in_range(layer: i32) -> bool {
        (0..BG_LAYER_COUNT as i32).contains(&layer)
    }

    /// `BGSCREEN layer, width, height [,tileSize]` — resize a layer's map (caller has
    /// validated the layer, the `width*height <= 16383` limit, and the tile size). Resizing
    /// reallocates the cell buffer to empty tiles.
    pub fn resize(&mut self, layer: usize, width: i32, height: i32, tile_size: i32) {
        let l = &mut self.layers[layer];
        l.width = width;
        l.height = height;
        l.tile_size = tile_size;
        l.cells = vec![0; (width * height) as usize];
    }

    /// `BGCLR layer` — clear one layer's tilemap to empty cells (transforms untouched).
    pub fn clear(&mut self, layer: usize) {
        for c in &mut self.layers[layer].cells {
            *c = 0;
        }
    }

    /// `BGCLR` (no arg) — clear every layer's tilemap.
    pub fn clear_all(&mut self) {
        for layer in 0..self.layers.len() {
            self.clear(layer);
        }
    }

    /// `BGFILL layer, sx, sy, ex, ey, data` — fill a rectangle of cells with `data`. The
    /// corners are normalized (min/max) and clamped to the map bounds, so an out-of-range
    /// rectangle fills only its in-bounds intersection (the exact OOB behavior is
    /// oracle-pending — see `HARVEST_QUEUE.md`). The caller has validated the layer.
    pub fn fill(&mut self, layer: usize, sx: i32, sy: i32, ex: i32, ey: i32, data: u16) {
        let (w, h) = (self.layers[layer].width, self.layers[layer].height);
        let x0 = sx.min(ex).max(0);
        let y0 = sy.min(ey).max(0);
        let x1 = sx.max(ex).min(w - 1);
        let y1 = sy.max(ey).min(h - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.layers[layer].set_cell(x, y, data);
            }
        }
    }

    /// `BGGET(layer, x, y, 1)` — read a cell addressed by **pixel** coordinates: the pixel is
    /// converted to a char coordinate by flooring `pixel / tileSize`, then wrapped modulo the
    /// map dimensions (so a scrolled / off-map read never panics). The pixel→char rounding
    /// and the wrap are oracle-pending; this models a repeating (wrapping) map. The caller
    /// has validated the layer.
    pub fn cell_pixel(&self, layer: usize, px: i32, py: i32) -> u16 {
        let l = &self.layers[layer];
        let cx = px.div_euclid(l.tile_size).rem_euclid(l.width);
        let cy = py.div_euclid(l.tile_size).rem_euclid(l.height);
        l.cell(cx, cy)
    }

    /// `BGOFS layer, x, y [,z]` — set a layer's scroll offset (and optional depth).
    pub fn set_ofs(&mut self, layer: usize, x: i32, y: i32, z: Option<i32>) {
        let l = &mut self.layers[layer];
        l.ofs_x = x;
        l.ofs_y = y;
        if let Some(z) = z {
            l.ofs_z = z;
        }
    }

    /// `BGROT layer, angle` — set a layer's rotation, normalized to 0..359.
    pub fn set_rot(&mut self, layer: usize, angle: i32) {
        self.layers[layer].rot = normalize_angle(angle);
    }

    /// `BGSCALE layer, sx, sy` — set a layer's enlargement scale (stored unclamped).
    pub fn set_scale(&mut self, layer: usize, sx: f64, sy: f64) {
        let l = &mut self.layers[layer];
        l.scale_x = sx;
        l.scale_y = sy;
    }

    /// `BGHOME layer, x, y` — set a layer's display origin (rotation/scale pivot).
    pub fn set_home(&mut self, layer: usize, x: i32, y: i32) {
        let l = &mut self.layers[layer];
        l.home_x = x;
        l.home_y = y;
    }

    /// `BGCLIP layer` (reset → whole layer) or `BGCLIP layer, sx, sy, ex, ey` (rectangle,
    /// corners normalized min/max). The caller has validated the layer.
    pub fn set_clip(&mut self, layer: usize, rect: Option<(i32, i32, i32, i32)>) {
        self.layers[layer].clip =
            rect.map(|(sx, sy, ex, ey)| (sx.min(ex), sy.min(ey), sx.max(ex), sy.max(ey)));
    }
}

/// Normalize a degree angle into 0..359 (truncating remainder, then a `+360` correction for
/// a negative result), matching the `BGROT` handler's `angle mod 360` reciprocal-multiply
/// with `addmi r0,r0,#0x168`: -90 → 270, 450 → 90, 360 → 0.
pub fn normalize_angle(angle: i32) -> i32 {
    let r = angle % 360;
    if r < 0 {
        r + 360
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_defaults() {
        let st = BgState::new();
        assert_eq!(st.layers.len(), BG_LAYER_COUNT);
        assert_eq!(st.page, BG_PAGE_DEFAULT);
        let l = &st.layers[0];
        assert_eq!((l.width, l.height, l.tile_size), (25, 15, 16));
        assert_eq!(l.cells.len(), 25 * 15);
        assert!(l.cells.iter().all(|&c| c == 0));
        assert!(l.visible);
        assert_eq!((l.scale_x, l.scale_y), (1.0, 1.0));
        assert_eq!(l.color, 0xFFFF_FFFF);
        assert_eq!(l.clip, None);
    }

    #[test]
    fn resize_reallocates_empty() {
        let mut st = BgState::new();
        st.layers[0].set_cell(0, 0, 99);
        st.resize(0, 32, 32, 8);
        let l = &st.layers[0];
        assert_eq!((l.width, l.height, l.tile_size), (32, 32, 8));
        assert_eq!(l.cells.len(), 32 * 32);
        assert!(l.cells.iter().all(|&c| c == 0));
    }

    #[test]
    fn put_get_round_trip() {
        let mut st = BgState::new();
        st.resize(0, 32, 32, 16);
        st.layers[0].set_cell(20, 15, 0x80FF);
        assert_eq!(st.layers[0].cell(20, 15), 0x80FF);
        // A different cell is still empty.
        assert_eq!(st.layers[0].cell(0, 0), 0);
    }

    #[test]
    fn fill_clamps_and_clears() {
        let mut st = BgState::new();
        st.resize(0, 8, 8, 16);
        st.fill(0, 2, 2, 5, 5, 7);
        assert_eq!(st.layers[0].cell(2, 2), 7);
        assert_eq!(st.layers[0].cell(5, 5), 7);
        assert_eq!(st.layers[0].cell(1, 1), 0);
        assert_eq!(st.layers[0].cell(6, 6), 0);
        // Out-of-bounds fill clamps to the in-bounds intersection (no panic).
        st.fill(0, -10, -10, 100, 100, 3);
        assert!(st.layers[0].cells.iter().all(|&c| c == 3));
        // Reversed corners normalize.
        st.fill(0, 4, 4, 1, 1, 9);
        assert_eq!(st.layers[0].cell(1, 1), 9);
        assert_eq!(st.layers[0].cell(4, 4), 9);
    }

    #[test]
    fn clear_one_and_all() {
        let mut st = BgState::new();
        st.resize(0, 4, 4, 16);
        st.resize(1, 4, 4, 16);
        st.layers[0].set_cell(0, 0, 5);
        st.layers[1].set_cell(0, 0, 6);
        st.clear(0);
        assert_eq!(st.layers[0].cell(0, 0), 0);
        assert_eq!(st.layers[1].cell(0, 0), 6);
        st.clear_all();
        assert_eq!(st.layers[1].cell(0, 0), 0);
    }

    #[test]
    fn pixel_coord_converts_and_wraps() {
        let mut st = BgState::new();
        st.resize(0, 4, 4, 16); // 64x64 px map
        st.layers[0].set_cell(1, 2, 42);
        // Pixel (16..31, 32..47) maps to char (1, 2).
        assert_eq!(st.cell_pixel(0, 20, 40), 42);
        // Wrap: pixel one full map width over lands on the same cell.
        assert_eq!(st.cell_pixel(0, 20 + 64, 40), 42);
        // Negative pixel floors then wraps.
        assert_eq!(st.cell_pixel(0, 20 - 64, 40), 42);
    }

    #[test]
    fn angle_normalization() {
        assert_eq!(normalize_angle(0), 0);
        assert_eq!(normalize_angle(180), 180);
        assert_eq!(normalize_angle(360), 0);
        assert_eq!(normalize_angle(450), 90);
        assert_eq!(normalize_angle(-90), 270);
        assert_eq!(normalize_angle(-360), 0);
        assert_eq!(normalize_angle(720 + 45), 45);
    }

    #[test]
    fn transforms_store() {
        let mut st = BgState::new();
        st.set_ofs(0, 16, -8, Some(3));
        assert_eq!(
            (st.layers[0].ofs_x, st.layers[0].ofs_y, st.layers[0].ofs_z),
            (16, -8, 3)
        );
        // Omitting Z leaves it unchanged.
        st.set_ofs(0, 1, 2, None);
        assert_eq!(st.layers[0].ofs_z, 3);
        st.set_scale(0, 1.5, 2.0);
        assert_eq!((st.layers[0].scale_x, st.layers[0].scale_y), (1.5, 2.0));
        st.set_home(0, 200, 120);
        assert_eq!((st.layers[0].home_x, st.layers[0].home_y), (200, 120));
        st.set_rot(0, -90);
        assert_eq!(st.layers[0].rot, 270);
    }

    #[test]
    fn clip_normalizes_and_resets() {
        let mut st = BgState::new();
        st.set_clip(0, Some((379, 219, 20, 20)));
        assert_eq!(st.layers[0].clip, Some((20, 20, 379, 219)));
        st.set_clip(0, None);
        assert_eq!(st.layers[0].clip, None);
    }
}
