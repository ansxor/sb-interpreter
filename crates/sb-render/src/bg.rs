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
//!   normalized mod 360), `BGSCALE` (X/Y enlargement, unclamped), `BGHOME` (display origin —
//!   an additive translation, not a scale/rotation pivot, per `BGCOORD`),
//!   `BGCOLOR` (ARGB multiply tint), `BGSHOW`/`BGHIDE` (visibility), and `BGCLIP`
//!   (display-area rectangle, in pixels).
//!
//! Animation/coordinate-conversion/load-save (`BGANIM`/`BGCOORD`/`BGCOPY`/…, M3-T5) build on
//! this model: each layer gains eight internal variables (`BGVAR`), a per-channel keyframe
//! animation (`BGANIM`, sharing the sprite [`KeyframeAnim`](crate::anim::KeyframeAnim)
//! engine), an animation-stopped flag (`BGSTART`/`BGSTOP`/`BGCHK`), and a bound callback
//! name (`BGFUNC`). The actual blit into the [`Framebuffer`](crate::Framebuffer) (M3-T6)
//! builds on top.
//!
//! Structural cross-check: `osb/SMILEBASIC/bg.d` (3.5.0). The contract is the
//! `spec/instructions/bg*.yaml` set (disassembled handlers; layer range-check via the
//! shared getter `FUN_001e2504`, errnum 10 when layer ∉ 0..count(=4)). The on-screen
//! *side effects* — the rendered tint, scroll/rotate/scale pixels, clip area — block on the
//! BG framebuffer oracle (O-T6); the tilemap-cell storage, the mod-360 angle normalization,
//! and the transform read-backs are deterministic and unit-tested here.

use crate::anim::{AnimError, KeyframeAnim};
use crate::sprite::ANIM_ITEMS;

/// Number of BG layers: 0..3 (the BG-layer count `[[0x315d60]+0x60]` = 4).
pub const BG_LAYER_COUNT: usize = 4;
/// Number of per-layer internal variables (`BGVAR`), 0..7. Variable 7 doubles as the
/// `BGANIM` "V" channel.
pub const BG_VAR_COUNT: usize = 8;
/// Number of `BGANIM` target channels (`target & 7`). BG uses 0 XY, 1 Z, 4 R, 5 S, 6 C,
/// 7 V; unlike sprites it has NO UV(2) or definition-I(3) channel (those indices stay None).
pub const BG_ANIM_CHANNELS: usize = 8;

/// Map a raw `BGANIM` channel (`target & 7`: 0 XY, 1 Z, 4 R, 5 S, 6 C, 7 V) to its
/// **compacted** `BGCHK` flags-word bit index. BG packs its 6 channels into bits 0..5
/// (XY=0, Z=1, R=2, S=3, C=4, V=5 → values 1/2/4/8/16/32), unlike the sprite `1<<channel`
/// layout — see [`BgState::anim_status`] for the disassembly cites. UV(2)/I(3) are rejected
/// upstream (errnum 4) so they never reach here.
fn bg_anim_bit(channel: usize) -> u32 {
    match channel {
        0 => 0, // XY
        1 => 1, // Z
        4 => 2, // R
        5 => 3, // S
        6 => 4, // C
        7 => 5, // V
        _ => channel as u32,
    }
}
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
    /// Display origin, in pixels (`BGHOME`) — an additive translation of the converted
    /// display coordinate (NOT a scale/rotation pivot; the pivot is the layer origin 0,0,
    /// hw_verified via `BGCOORD`, M7-T2).
    pub home_x: i32,
    pub home_y: i32,
    /// ARGB8888 multiply tint (`BGCOLOR`); default opaque white (`&HFFFFFFFF`) = unchanged.
    pub color: u32,
    /// Display (clip) area in pixels (`BGCLIP`), normalized `(start_x,start_y,end_x,end_y)`;
    /// `None` = the whole layer.
    pub clip: Option<(i32, i32, i32, i32)>,
    /// The eight per-layer internal variables (`BGVAR`), all 0 by default. Variable 7 is
    /// also the `BGANIM` "V" channel target.
    pub var: [f64; BG_VAR_COUNT],
    /// Animation paused flag (`BGSTOP` sets it, `BGSTART` clears it) — layer flag bit 0x40.
    /// A freshly `BGANIM`'d layer is running (flag clear).
    pub anim_stopped: bool,
    /// The per-channel `BGANIM` animations (one optional animation per target). Channels
    /// 2 (UV) and 3 (I) are unused on BG and always `None`.
    pub anims: [Option<KeyframeAnim>; BG_ANIM_CHANNELS],
    /// Bound callback process name (`BGFUNC`), without the leading `@`. Invoked by
    /// `CALL BG`; inside it `CALLIDX` is the layer number. Binding does not require a setup.
    pub func: Option<String>,
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
            var: [0.0; BG_VAR_COUNT],
            anim_stopped: false,
            anims: std::array::from_fn(|_| None),
            func: None,
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

    /// Read a `BGANIM` channel's current value(s) as floats (the relative/interpolation
    /// base). Channels: 0 XY (scroll), 1 Z (depth), 4 R (rotation), 5 S (scale), 6 C (color),
    /// 7 V (internal variable 7). Channels 2/3 are unused on BG.
    fn read_channel(&self, channel: usize) -> Vec<f64> {
        match channel {
            0 => vec![self.ofs_x as f64, self.ofs_y as f64],
            1 => vec![self.ofs_z as f64],
            4 => vec![self.rot as f64],
            5 => vec![self.scale_x, self.scale_y],
            6 => vec![self.color as f64],
            _ => vec![self.var[7]],
        }
    }

    /// Write a `BGANIM` channel's animated value(s) back into the layer. The integer-valued
    /// channels (XY scroll, Z, rotation, color) round to the nearest integer; rotation is
    /// re-normalized to 0..359. Exact rounding is oracle-pending (no BG framebuffer harvest).
    fn write_channel(&mut self, channel: usize, v: &[f64]) {
        let g = |i: usize| v.get(i).copied().unwrap_or(0.0);
        match channel {
            0 => {
                self.ofs_x = g(0).round() as i32;
                self.ofs_y = g(1).round() as i32;
            }
            1 => self.ofs_z = g(0).round() as i32,
            4 => self.rot = normalize_angle(g(0).round() as i32),
            5 => {
                self.scale_x = g(0);
                self.scale_y = g(1);
            }
            6 => self.color = g(0).round() as i64 as u32,
            _ => self.var[7] = g(0),
        }
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
    /// converted to a char coordinate by flooring `pixel / tileSize` (no wrap, no error). A
    /// resulting char coordinate outside the map reads as the empty cell **0** — pixel mode is
    /// NOT range-checked (unlike char mode), so an off-map read returns 0 instead of raising.
    /// hw_verified (sb-oracle 2026-06-24): on a 32×32 / tile-16 layer `BGGET(0,512,16,1)` → 0
    /// (char col 32 off-map) and `BGGET(0,-1,16,1)` → 0, while px 496..511 read char col 31.
    /// The caller has validated the layer.
    pub fn cell_pixel(&self, layer: usize, px: i32, py: i32) -> u16 {
        let l = &self.layers[layer];
        let cx = px.div_euclid(l.tile_size);
        let cy = py.div_euclid(l.tile_size);
        if l.in_cell(cx, cy) {
            l.cell(cx, cy)
        } else {
            0
        }
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

    /// `BGHOME layer, x, y` — set a layer's display origin (an additive translation of the
    /// converted display coordinate; NOT a scale/rotation pivot — see `coord`).
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

    // -- internal variables (BGVAR, M3-T5) -------------------------------------

    /// `BGVAR layer, n, value` — write one of a layer's eight internal variables (the caller
    /// has range-checked the layer and `n` 0..7).
    pub fn set_var(&mut self, layer: usize, n: usize, value: f64) {
        self.layers[layer].var[n] = value;
    }

    /// `BGVAR(layer, n)` — read one of a layer's eight internal variables (0 if never
    /// written; caller range-checked).
    pub fn get_var(&self, layer: usize, n: usize) -> f64 {
        self.layers[layer].var[n]
    }

    // -- animation (BGANIM / BGSTART / BGSTOP / BGCHK, M3-T5) -------------------

    /// Install a `BGANIM` animation on a channel of a layer from an already-flattened `data`
    /// list (`Time, Item[, Item], …`) and a `loop_count` (0 = endless). Captures the layer's
    /// current channel value as the relative/interpolation base. The caller validated the
    /// layer and resolved the channel (BG has no UV/I channel).
    pub fn set_anim(
        &mut self,
        layer: usize,
        channel: usize,
        relative: bool,
        data: &[f64],
        loop_count: i32,
    ) -> Result<(), AnimError> {
        let base = self.layers[layer].read_channel(channel);
        let anim = KeyframeAnim::build(
            channel,
            relative,
            ANIM_ITEMS[channel],
            base,
            data,
            loop_count,
        )?;
        self.layers[layer].anims[channel] = Some(anim);
        Ok(())
    }

    /// Advance every running BG animation by `frames` frames (the frame clock — driven by
    /// `VSYNC`/`WAIT`). Stopped layers are skipped; each advanced channel value is written
    /// back into its layer.
    pub fn tick(&mut self, frames: u64) {
        for _ in 0..frames {
            for i in 0..self.layers.len() {
                if self.layers[i].anim_stopped {
                    continue;
                }
                let mut anims = std::mem::take(&mut self.layers[i].anims);
                for (ch, slot) in anims.iter_mut().enumerate() {
                    if let Some(anim) = slot {
                        if !anim.done {
                            anim.step();
                            let cur = anim.cur.clone();
                            self.layers[i].write_channel(ch, &cur);
                        }
                    }
                }
                self.layers[i].anims = anims;
            }
        }
    }

    /// `BGCHK(layer)` — the animation-status bitmask: a bit is set when the corresponding
    /// channel has a running (not finished) `BGANIM`. A stopped layer (`BGSTOP`) reads 0.
    ///
    /// BG uses a **compacted** bit layout, NOT the sprite `1 << channel`: the BGANIM handler's
    /// per-channel switch (jump table @0x164dbc) ORs a packed bit into the layer flags word —
    /// XY→0x1, Z→0x2, R→0x4, S→0x8, C→0x10, V→0x20 (the `orr r0,r0,#…` sites in cases
    /// @0x164ddc/4e5c/4eb0/4ef4/4f44/4f84), and BGCHK returns that word raw. So R/S/C/V are
    /// bits 2/3/4/5 (4/8/16/32), NOT the documented #CHKR/S/C/V 16/32/64/128 (those are the
    /// SPRITE layout, where channels are contiguous 0..7). hw_verified (M7-T2 run 53,
    /// `bganim.tsv`); the run-35 `R=16…` extrapolation was wrong (it only confirmed XY=1).
    pub fn anim_status(&self, layer: usize) -> i32 {
        let l = &self.layers[layer];
        if l.anim_stopped {
            return 0;
        }
        let mut bits = 0;
        for (ch, slot) in l.anims.iter().enumerate() {
            if let Some(a) = slot {
                if !a.done {
                    bits |= 1 << bg_anim_bit(ch);
                }
            }
        }
        bits
    }

    /// `BGSTOP`/`BGSTART` — pause/resume the animation of every layer at once (the
    /// no-argument forms). The handler walks all layers and raises no error.
    pub fn set_anim_stopped_all(&mut self, stop: bool) {
        for l in &mut self.layers {
            l.anim_stopped = stop;
        }
    }

    /// `BGSTOP layer`/`BGSTART layer` — pause/resume one layer's animation (caller
    /// range-checked the layer).
    pub fn set_anim_stopped(&mut self, layer: usize, stop: bool) {
        self.layers[layer].anim_stopped = stop;
    }

    /// `BGFUNC layer, @label` — bind a callback process name to a layer (caller
    /// range-checked + resolved the name).
    pub fn set_func(&mut self, layer: usize, name: Option<String>) {
        self.layers[layer].func = name;
    }

    /// The callback process name bound to a layer by `BGFUNC`, or `None` if unbound — read by
    /// `CALL BG` dispatch (M6-T6).
    pub fn func(&self, layer: usize) -> Option<String> {
        self.layers[layer].func.clone()
    }

    // -- block copy (BGCOPY, M3-T5) --------------------------------------------

    /// `BGCOPY layer, sx, sy, ex, ey, dx, dy` — copy a rectangular block of the layer's
    /// tilemap (inclusive of both source corners, character units) from `src` `(sx,sy)-(ex,ey)`
    /// to a destination whose top-left is `dest` `(dx, dy)`, within the same layer. The source
    /// rectangle is read into a buffer first so overlapping source/destination copy correctly;
    /// cells whose source or destination falls outside the map are skipped (the exact OOB
    /// behavior is oracle-pending — see `HARVEST_QUEUE.md`). The caller has validated the layer.
    pub fn copy(&mut self, layer: usize, src: (i32, i32, i32, i32), dest: (i32, i32)) {
        let (sx, sy, ex, ey) = src;
        let (dx, dy) = dest;
        let l = &self.layers[layer];
        let (x0, x1) = (sx.min(ex), sx.max(ex));
        let (y0, y1) = (sy.min(ey), sy.max(ey));
        // Capture the source block (row-major) before writing, so overlap is safe.
        let w = (x1 - x0 + 1).max(0);
        let h = (y1 - y0 + 1).max(0);
        let mut block = Vec::with_capacity((w * h) as usize);
        for ry in 0..h {
            for rx in 0..w {
                let (sxx, syy) = (x0 + rx, y0 + ry);
                block.push(if l.in_cell(sxx, syy) {
                    Some(l.cell(sxx, syy))
                } else {
                    None
                });
            }
        }
        // The destination keeps the source rectangle's orientation: cell (rx,ry) of the
        // block lands at (dx+rx, dy+ry).
        for ry in 0..h {
            for rx in 0..w {
                if let Some(data) = block[(ry * w + rx) as usize] {
                    let (dxx, dyy) = (dx + rx, dy + ry);
                    if self.layers[layer].in_cell(dxx, dyy) {
                        self.layers[layer].set_cell(dxx, dyy, data);
                    }
                }
            }
        }
    }

    // -- array load/save (BGLOAD / BGSAVE, M3-T5) ------------------------------

    /// Read a `width × height` block of a layer's tilemap (row-major, starting at
    /// `(start_x, start_y)`) as 16-bit cell values (`BGSAVE`). Off-map cells read as 0. The
    /// caller has validated the layer; the cell packing matches what [`Self::load_cells`]
    /// writes back, so a save/load round-trips.
    pub fn save_cells(
        &self,
        layer: usize,
        start_x: i32,
        start_y: i32,
        width: i32,
        height: i32,
    ) -> Vec<u16> {
        let l = &self.layers[layer];
        let mut out = Vec::with_capacity((width.max(0) * height.max(0)) as usize);
        for ry in 0..height {
            for rx in 0..width {
                let (x, y) = (start_x + rx, start_y + ry);
                out.push(if l.in_cell(x, y) { l.cell(x, y) } else { 0 });
            }
        }
        out
    }

    /// Write a `width × height` block of 16-bit cell values into a layer's tilemap (row-major,
    /// starting at `(start_x, start_y)`) — the inverse of [`Self::save_cells`] (`BGLOAD`).
    /// Off-map destination cells are skipped. `cells` is consumed in row-major order; a short
    /// list leaves the remaining destination cells untouched. The caller validated the layer.
    pub fn load_cells(
        &mut self,
        layer: usize,
        start_x: i32,
        start_y: i32,
        width: i32,
        height: i32,
        cells: &[u16],
    ) {
        let mut k = 0;
        for ry in 0..height {
            for rx in 0..width {
                let Some(&data) = cells.get(k) else { return };
                k += 1;
                let (x, y) = (start_x + rx, start_y + ry);
                if self.layers[layer].in_cell(x, y) {
                    self.layers[layer].set_cell(x, y, data);
                }
            }
        }
    }

    // -- coordinate conversion (BGCOORD, M3-T5) --------------------------------

    /// `BGCOORD layer, srcX, srcY, mode OUT dx, dy` — convert between a layer's BG-screen
    /// space and display space, applying its scroll (`BGOFS`), rotation (`BGROT`), scale
    /// (`BGSCALE`) and origin (`BGHOME`). Mode 0: BG-screen → display. Mode 1: display →
    /// BG-screen in character units. Mode 2: display → BG-screen in pixel units. The caller
    /// has validated the layer and mode 0..2.
    ///
    /// The forward map (mode 0) matches the disassembled handler structure
    /// `display = offset + scale·rotate(bg)` where the scale/rotation pivot is the layer
    /// ORIGIN (0,0) — NOT `BGHOME` — and the additive `offset` is `home - ofs`
    /// (`FUN_0028defc` @0x28df48: `out = [obj+0x24] + scale·(rot·(src-pivot))`, pivot
    /// `[obj+0x34]`=0). This was hw_verified for the non-rotation contract via the oracle
    /// (M7-T2): scroll subtracts in mode 0 / adds in the inverse, `BGHOME` is a pure
    /// translation (not a pivot), and `BGSCALE` enlarges about the origin. The EXACT rotation
    /// matrix/table (`[0x306de0]`, indexed non-trivially by the angle) is NOT yet pinned — a
    /// `BGROT 90` of `(16,0)` returns `(-1,15)`, not a clean 90° rotation — so the rotated
    /// path stays a best-effort standard rotation about the origin, oracle-pending with the
    /// BG framebuffer effect (O-T6 — see `HARVEST_QUEUE.md`). The continuous (`f64`) result is
    /// returned here; the handler truncates each component toward zero to an integer before it
    /// reaches the OUT variables (done by the caller, `builtins::bg::bgcoord`).
    pub fn coord(&self, layer: usize, src_x: f64, src_y: f64, mode: i32) -> (f64, f64) {
        let l = &self.layers[layer];
        let (hx, hy) = (l.home_x as f64, l.home_y as f64);
        let (ox, oy) = (l.ofs_x as f64, l.ofs_y as f64);
        let rad = (l.rot as f64).to_radians();
        let (sin, cos) = rad.sin_cos();
        match mode {
            // BG-screen pixel -> display pixel: rotate + scale about the ORIGIN, then add the
            // display origin (BGHOME) and subtract the scroll (BGOFS).
            0 => {
                let rx = src_x * cos - src_y * sin;
                let ry = src_x * sin + src_y * cos;
                (rx * l.scale_x + hx - ox, ry * l.scale_y + hy - oy)
            }
            // display pixel -> BG-screen: inverse of mode 0. Mode 1 reports character units
            // (pixel / tile size), mode 2 reports pixel units.
            _ => {
                // Undo the display origin (add back the scroll, subtract the home origin).
                let (px, py) = (src_x - hx + ox, src_y - hy + oy);
                // Inverse scale (guard a zero scale), then inverse rotation about the origin.
                let sx = if l.scale_x != 0.0 {
                    px / l.scale_x
                } else {
                    0.0
                };
                let sy = if l.scale_y != 0.0 {
                    py / l.scale_y
                } else {
                    0.0
                };
                let bx = sx * cos + sy * sin;
                let by = -sx * sin + sy * cos;
                if mode == 1 {
                    let ts = l.tile_size.max(1) as f64;
                    ((bx / ts).floor(), (by / ts).floor())
                } else {
                    (bx, by)
                }
            }
        }
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
    fn pixel_coord_floors_no_wrap_offmap_zero() {
        // hw_verified (sb-oracle 2026-06-24): pixel mode floors px/tile with NO wrap; an
        // off-map char coordinate reads the empty cell 0 (no range error).
        let mut st = BgState::new();
        st.resize(0, 4, 4, 16); // 64x64 px map
        st.layers[0].set_cell(1, 2, 42);
        // Pixel (16..31, 32..47) maps to char (1, 2).
        assert_eq!(st.cell_pixel(0, 20, 40), 42);
        assert_eq!(st.cell_pixel(0, 16, 32), 42);
        assert_eq!(st.cell_pixel(0, 31, 47), 42);
        // No wrap: one full map width over is off-map -> 0 (real SB does NOT repeat here).
        assert_eq!(st.cell_pixel(0, 20 + 64, 40), 0);
        // Negative pixel floors to a negative char -> off-map -> 0.
        assert_eq!(st.cell_pixel(0, -1, 40), 0);
        assert_eq!(st.cell_pixel(0, 20 - 64, 40), 0);
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
    fn set_rot_normalizes_mod_360() {
        // hw_verified sb-oracle 2026-06-24 (M7-T2 run 16, harness/harvest/out/bgrot_rt.tsv):
        // BGROT normalizes via a truncated-toward-zero remainder + a +360 fixup for a
        // negative remainder, so the stored/returned angle is always in 0..359.
        let mut st = BgState::new();
        for (input, want) in [
            (45, 45),
            (180, 180),
            (359, 359),
            (360, 0),
            (361, 1),
            (450, 90),
            (720, 0),
            (-1, 359),
            (-90, 270),
            (-360, 0),
            (-450, 270),
            (100000, 280),
        ] {
            st.set_rot(0, input);
            assert_eq!(
                st.layers[0].rot, want,
                "BGROT 0,{input} should read back {want}"
            );
        }
    }

    #[test]
    fn clip_normalizes_and_resets() {
        let mut st = BgState::new();
        st.set_clip(0, Some((379, 219, 20, 20)));
        assert_eq!(st.layers[0].clip, Some((20, 20, 379, 219)));
        st.set_clip(0, None);
        assert_eq!(st.layers[0].clip, None);
    }

    // -- M3-T5 BG extras -------------------------------------------------------

    #[test]
    fn internal_variables_default_zero_and_store() {
        let mut st = BgState::new();
        assert_eq!(st.get_var(0, 0), 0.0);
        assert_eq!(st.get_var(3, 7), 0.0);
        st.set_var(2, 5, 42.0);
        assert_eq!(st.get_var(2, 5), 42.0);
        // Variable 7 doubles as the BGANIM "V" channel but is just storage here.
        st.set_var(0, 7, -3.5);
        assert_eq!(st.layers[0].var[7], -3.5);
    }

    #[test]
    fn anim_drives_scroll_offset() {
        let mut st = BgState::new();
        // XY channel: hold (16,-8) for 2 frames, then interpolate to (0,0) over 4 frames.
        let data = [2.0, 16.0, -8.0, -4.0, 0.0, 0.0];
        st.set_anim(0, 0, false, &data, 1).unwrap();
        st.tick(1);
        assert_eq!((st.layers[0].ofs_x, st.layers[0].ofs_y), (16, -8));
        st.tick(1); // still holding
        assert_eq!((st.layers[0].ofs_x, st.layers[0].ofs_y), (16, -8));
        st.tick(2); // halfway through the interpolation
        assert_eq!(
            st.layers[0].ofs_x,
            (16.0_f64 + (0.0 - 16.0) * 0.5).round() as i32
        );
        st.tick(2); // reaches the target
        assert_eq!((st.layers[0].ofs_x, st.layers[0].ofs_y), (0, 0));
    }

    #[test]
    fn anim_rotation_channel_normalizes() {
        let mut st = BgState::new();
        // R channel: hold -90 for 1 frame -> normalized to 270.
        st.set_anim(0, 4, false, &[1.0, -90.0], 1).unwrap();
        st.tick(1);
        assert_eq!(st.layers[0].rot, 270);
    }

    #[test]
    fn anim_relative_adds_base_var7() {
        let mut st = BgState::new();
        st.set_var(0, 7, 10.0);
        // Relative V channel: hold (+5) for 1 frame.
        st.set_anim(0, 7, true, &[1.0, 5.0], 1).unwrap();
        st.tick(1);
        assert_eq!(st.layers[0].var[7], 15.0);
    }

    #[test]
    fn anim_status_bits_and_stop() {
        let mut st = BgState::new();
        assert_eq!(st.anim_status(0), 0);
        // Z (channel 1) + R (channel 4) running -> compacted bits Z=1<<1, R=1<<2 = 6
        // (NOT the sprite 1<<channel = 18 — hw_verified M7-T2 run 53, BGANIM bganim_multi_zr=6).
        st.set_anim(0, 1, false, &[10.0, 5.0], 0).unwrap();
        st.set_anim(0, 4, false, &[10.0, 90.0], 0).unwrap();
        assert_eq!(st.anim_status(0), (1 << 1) | (1 << 2));
        // Stopping a layer freezes the advance and reads 0.
        st.set_anim_stopped(0, true);
        assert_eq!(st.anim_status(0), 0);
        st.tick(5);
        assert_eq!(st.layers[0].anims[1].as_ref().unwrap().frame, 0);
        st.set_anim_stopped(0, false);
        st.tick(1);
        assert_eq!(st.layers[0].anims[1].as_ref().unwrap().frame, 1);
        // The no-argument forms toggle every layer.
        st.set_anim_stopped_all(true);
        assert!(st.layers.iter().all(|l| l.anim_stopped));
    }

    #[test]
    fn copy_block_within_layer() {
        let mut st = BgState::new();
        st.resize(0, 16, 16, 16);
        st.layers[0].set_cell(0, 0, 11);
        st.layers[0].set_cell(1, 0, 22);
        st.layers[0].set_cell(0, 1, 33);
        st.layers[0].set_cell(1, 1, 44);
        // Copy the 2x2 block at (0,0)-(1,1) to top-left (8,8).
        st.copy(0, (0, 0, 1, 1), (8, 8));
        assert_eq!(st.layers[0].cell(8, 8), 11);
        assert_eq!(st.layers[0].cell(9, 8), 22);
        assert_eq!(st.layers[0].cell(8, 9), 33);
        assert_eq!(st.layers[0].cell(9, 9), 44);
        // The source is unchanged.
        assert_eq!(st.layers[0].cell(0, 0), 11);
    }

    #[test]
    fn copy_overlapping_source_dest() {
        let mut st = BgState::new();
        st.resize(0, 8, 1, 16);
        for x in 0..4 {
            st.layers[0].set_cell(x, 0, (x + 1) as u16);
        }
        // Copy [0..3] one cell to the right (overlapping): captured first, so no smearing.
        st.copy(0, (0, 0, 3, 0), (1, 0));
        assert_eq!(st.layers[0].cell(1, 0), 1);
        assert_eq!(st.layers[0].cell(2, 0), 2);
        assert_eq!(st.layers[0].cell(3, 0), 3);
        assert_eq!(st.layers[0].cell(4, 0), 4);
        // (0,0) keeps its original value (not part of the destination).
        assert_eq!(st.layers[0].cell(0, 0), 1);
    }

    #[test]
    fn save_load_round_trips() {
        let mut st = BgState::new();
        st.resize(0, 8, 8, 16);
        for y in 0..8 {
            for x in 0..8 {
                st.layers[0].set_cell(x, y, (y * 8 + x) as u16 | 0x8000);
            }
        }
        let saved = st.save_cells(0, 0, 0, 8, 8);
        assert_eq!(saved.len(), 64);
        // Clear, then load it back into a different layer of the same size.
        st.resize(1, 8, 8, 16);
        st.load_cells(1, 0, 0, 8, 8, &saved);
        assert_eq!(st.layers[1].cells, st.layers[0].cells);
        // A ranged save/load round-trips a sub-rectangle.
        let region = st.save_cells(0, 2, 2, 3, 3);
        st.resize(2, 8, 8, 16);
        st.load_cells(2, 5, 5, 3, 3, &region);
        assert_eq!(st.layers[2].cell(5, 5), st.layers[0].cell(2, 2));
        assert_eq!(st.layers[2].cell(7, 7), st.layers[0].cell(4, 4));
    }

    #[test]
    fn coord_modes_round_trip_with_transforms() {
        let mut st = BgState::new();
        // A scrolled, scaled layer (no rotation keeps the round-trip exact).
        st.set_ofs(0, 40, 20, None);
        st.set_scale(0, 2.0, 2.0);
        st.set_home(0, 8, 8);
        // BG-screen -> display (mode 0), then display -> BG-screen pixels (mode 2) inverts.
        let (dx, dy) = st.coord(0, 100.0, 60.0, 0);
        let (bx, by) = st.coord(0, dx, dy, 2);
        assert!((bx - 100.0).abs() < 1e-9, "bx={bx}");
        assert!((by - 60.0).abs() < 1e-9, "by={by}");
        // Mode 1 reports the same in character (tile) units.
        let (cx, cy) = st.coord(0, dx, dy, 1);
        assert_eq!(cx, (bx / 16.0).floor());
        assert_eq!(cy, (by / 16.0).floor());
    }

    #[test]
    fn coord_value_contract_no_rotation() {
        // hw_verified non-rotation value contract (M7-T2, bgcoord_rt.tsv).
        let mut st = BgState::new();
        // Default state: mode 0 is identity, mode 1 floors display/tile, mode 2 is identity.
        assert_eq!(st.coord(0, 16.0, 16.0, 0), (16.0, 16.0));
        assert_eq!(st.coord(0, 200.0, 120.0, 1), (12.0, 7.0));
        assert_eq!(st.coord(0, 15.0, 15.0, 1), (0.0, 0.0));
        assert_eq!(st.coord(0, 16.0, 16.0, 2), (16.0, 16.0));
        // BGOFS: scroll SUBTRACTS in mode 0, ADDS back in the inverse.
        st.set_ofs(0, 8, 8, None);
        assert_eq!(st.coord(0, 0.0, 0.0, 0), (-8.0, -8.0));
        assert_eq!(st.coord(0, 16.0, 16.0, 0), (8.0, 8.0));
        assert_eq!(st.coord(0, 8.0, 8.0, 2), (16.0, 16.0));
        // BGHOME: a pure additive translation (not a pivot).
        let mut st = BgState::new();
        st.set_home(0, 16, 16);
        assert_eq!(st.coord(0, 16.0, 16.0, 0), (32.0, 32.0));
        assert_eq!(st.coord(0, 32.0, 32.0, 0), (48.0, 48.0));
        assert_eq!(st.coord(0, 16.0, 16.0, 2), (0.0, 0.0));
        // BGSCALE: enlarges about the ORIGIN (0,0), not BGHOME.
        let mut st = BgState::new();
        st.set_scale(0, 2.0, 2.0);
        assert_eq!(st.coord(0, 16.0, 16.0, 0), (32.0, 32.0));
        assert_eq!(st.coord(0, 32.0, 32.0, 2), (16.0, 16.0));
    }
}
