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

use crate::bg::{BgLayer, BgState, BG_LAYER_COUNT};
use crate::console::Console;
use crate::grp::{rgba5551_to_argb8888, ClipRect, GrpPage, GrpState, GRP_DIM};
use crate::sprite::{Sprite, SpriteState};
use crate::{Framebuffer, TOP_HEIGHT, TOP_WIDTH};

/// Default backdrop: opaque black. The console's default background is transparent, so a
/// visible backdrop is required for the composite to land on a surface.
///
/// FIDELITY: the exact `BACKCOLOR`→backdrop composite (and its default) is oracle-pending —
/// the *composite* framebuffer capture (O-T6) hasn't been harvested; queued in
/// `HARVEST_QUEUE.md`. Callers may pass any ARGB8888 backdrop to [`compose`].
pub const DEFAULT_BACKDROP: u32 = 0xFF00_0000;

/// Per-layer on/off flags for one screen (`VISIBLE console, graphic, bg, sprite`, M4-T4).
/// A cleared flag drops that whole layer group from the composite. The default shows every
/// layer (boot state), so callers that don't model `VISIBLE` keep the full stack.
#[derive(Debug, Clone, Copy)]
pub struct LayerVisibility {
    /// Console (text) layer.
    pub console: bool,
    /// GRP graphics display page.
    pub graphic: bool,
    /// BG tilemap layers.
    pub bg: bool,
    /// Sprite layer.
    pub sprite: bool,
}

impl Default for LayerVisibility {
    fn default() -> Self {
        LayerVisibility {
            console: true,
            graphic: true,
            bg: true,
            sprite: true,
        }
    }
}

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
    /// The font page glyphs are sampled from (the firmware GRPF page); `None` falls back to
    /// the placeholder font.
    pub font: Option<&'a GrpPage>,
    /// This layer's Z depth.
    pub z: i32,
}

impl Layer for ConsoleLayer<'_> {
    fn z(&self) -> i32 {
        self.z
    }

    fn composite(&self, fb: &mut Framebuffer) {
        self.console.render_with_font(fb, self.font);
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

/// Sample one device halfword (RGBA5551) from a sheet GRP page at `(x, y)`. Off-page reads
/// return `0` (a fully-transparent pixel — alpha bit clear), so sprites/BG whose source
/// rectangle runs past the 512×512 sheet edge just leave those texels transparent.
#[inline]
fn sample_sheet(page: &GrpPage, x: i32, y: i32) -> u16 {
    if x < 0 || y < 0 || x >= GRP_DIM as i32 || y >= GRP_DIM as i32 {
        return 0;
    }
    page.pixels[y as usize * GRP_DIM + x as usize]
}

/// Modulate a source ARGB8888 texel by a layer's ARGB8888 multiply color (`SPCOLOR`/
/// `BGCOLOR`). The default opaque-white code (`0xFFFFFFFF`) is the identity (no change), so
/// the common case is a fast no-op. Each channel is `round(src * mod / 255)`.
///
/// FIDELITY: the per-channel rounding (and whether SB modulates alpha) is oracle-pending —
/// the *composite* framebuffer capture (O-T6, screenshot path) hasn't been harvested. Queued
/// in `HARVEST_QUEUE.md` (M3-T6). The default white path — what every committed test uses —
/// is exact regardless.
#[inline]
fn modulate(src: u32, modc: u32) -> u32 {
    if modc == 0xFFFF_FFFF {
        return src;
    }
    let ch = |sh: u32| {
        let s = (src >> sh) & 0xFF;
        let m = (modc >> sh) & 0xFF;
        (s * m + 127) / 255
    };
    (ch(24) << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Additive blend `argb` onto the destination pixel (`#SPADD` attribute): per-channel
/// saturating add of RGB, leaving the result opaque. FIDELITY: the exact additive math is
/// oracle-pending (O-T6 composite, queued); only the non-additive path is exercised by the
/// committed tests.
#[inline]
fn blend_add(fb: &mut Framebuffer, x: usize, y: usize, argb: u32) {
    let d = fb.get_argb(x, y);
    let add = |sh: u32| (((d >> sh) & 0xFF) + ((argb >> sh) & 0xFF)).min(0xFF);
    fb.set_argb(x, y, 0xFF00_0000 | (add(16) << 16) | (add(8) << 8) | add(0));
}

/// One displayed sprite as a composited layer: it samples its source rectangle from a sheet
/// GRP page (`SPPAGE`) and paints it at the sprite's screen position, applying the sprite's
/// home/pivot, scale, 90°-step + free rotation, H/V flip, color modulate and additive flag.
///
/// FIDELITY: the **placement** (identity transform), alpha-keying, H/V flip and 90° rotation
/// are deterministic and pinned by the compositor tests. The exact sub-pixel sampling of
/// *free* rotation / fractional `SPSCALE`, the `SPCHR` sheet offset, the color-modulate
/// rounding and additive math are oracle-pending — they need the composite screenshot capture
/// (O-T6), queued in `HARVEST_QUEUE.md` (M3-T6).
pub struct SpriteLayer<'a> {
    /// The sprite slot to draw (must be `active` + `display`).
    pub sprite: &'a Sprite,
    /// The GRP page the sprite samples (`grp.pages[sprite.page]`).
    pub sheet: &'a GrpPage,
}

impl Layer for SpriteLayer<'_> {
    fn z(&self) -> i32 {
        self.sprite.z.round() as i32
    }

    fn composite(&self, fb: &mut Framebuffer) {
        render_sprite(self.sprite, self.sheet, fb);
    }
}

/// Rasterize one sprite onto `fb` by inverse-mapping every framebuffer pixel in the sprite's
/// transformed bounding box back to a source texel. The forward transform places sprite-local
/// point `l` (in `0..w × 0..h`) at screen `(x,y) + R(angle)·S(scale)·(l − home)`, so the home
/// point lands exactly on `(SPOFS x, y)`; the inverse recovers `l` from each screen pixel.
fn render_sprite(sp: &Sprite, sheet: &GrpPage, fb: &mut Framebuffer) {
    if !sp.active || !sp.display || sp.w <= 0 || sp.h <= 0 {
        return;
    }
    let (w, h) = (sp.w, sp.h);
    let total_deg = sp.rot + 90.0 * sp.rot90 as f64;
    let rad = total_deg.to_radians();
    let (c, s) = (rad.cos(), rad.sin());
    // A zero scale would collapse the sprite to nothing (and divide-by-zero on inverse); SB
    // treats it as not visible. Guard so we simply draw nothing.
    if sp.scale_x == 0.0 || sp.scale_y == 0.0 {
        return;
    }
    let (hx, hy) = (sp.home_x, sp.home_y);
    let fwd = |lx: f64, ly: f64| {
        let ax = (lx - hx) * sp.scale_x;
        let ay = (ly - hy) * sp.scale_y;
        (sp.x + ax * c - ay * s, sp.y + ax * s + ay * c)
    };
    // Screen bounding box of the four source-rect corners.
    let corners = [
        fwd(0.0, 0.0),
        fwd(w as f64, 0.0),
        fwd(0.0, h as f64),
        fwd(w as f64, h as f64),
    ];
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(cx, cy) in &corners {
        minx = minx.min(cx);
        miny = miny.min(cy);
        maxx = maxx.max(cx);
        maxy = maxy.max(cy);
    }
    let x0 = (minx.floor() as i32).max(0);
    let y0 = (miny.floor() as i32).max(0);
    let x1 = (maxx.ceil() as i32).min(fb.width as i32 - 1);
    let y1 = (maxy.ceil() as i32).min(fb.height as i32 - 1);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f64 - sp.x;
            let dy = py as f64 - sp.y;
            // Inverse: undo rotation (Rᵀ), then scale, then re-add the home pivot.
            let rx = dx * c + dy * s;
            let ry = -dx * s + dy * c;
            let lx = rx / sp.scale_x + hx;
            let ly = ry / sp.scale_y + hy;
            let li = lx.round() as i32;
            let lj = ly.round() as i32;
            if li < 0 || lj < 0 || li >= w || lj >= h {
                continue;
            }
            // H/V flip mirror the local lookup within the source rectangle.
            let sx = if sp.flip_h { w - 1 - li } else { li };
            let sy = if sp.flip_v { h - 1 - lj } else { lj };
            let texel = sample_sheet(sheet, sp.u + sx, sp.v + sy);
            if texel & 1 == 0 {
                continue; // 1-bit alpha key: clear bit = transparent.
            }
            let argb = modulate(rgba5551_to_argb8888(texel), sp.color);
            if sp.additive {
                blend_add(fb, px as usize, py as usize, argb);
            } else {
                fb.set_argb(px as usize, py as usize, argb);
            }
        }
    }
}

/// One BG tilemap layer as a composited layer: it tiles its `cells` from a sheet GRP page
/// (`BGPAGE`), honoring scroll (`BGOFS`), home/pivot, rotation, scale and the per-cell
/// character/H/V-flip bits of the 16-bit screen data.
///
/// FIDELITY: the **screen-data decode** (char 0..1023 = bits 0-9, H-flip = bit 10, V-flip =
/// bit 11, palette = bits 12-15) is *documented* (`bgput.md`); the tile **placement**, scroll,
/// wrap, H/V flip and char-0-empty transparency are deterministic and pinned by the compositor
/// tests. The 16-color **palette** remap, the exact rotation/scale sampling, scroll *sign*, and
/// the sheet tile layout are oracle-pending — they need the composite screenshot capture
/// (O-T6), queued in `HARVEST_QUEUE.md` (M3-T6).
pub struct BgRenderLayer<'a> {
    /// The BG layer to draw (skipped when `!visible`).
    pub layer: &'a BgLayer,
    /// The GRP page BG tiles sample (`grp.pages[bg.page]`).
    pub sheet: &'a GrpPage,
}

impl Layer for BgRenderLayer<'_> {
    fn z(&self) -> i32 {
        self.layer.ofs_z
    }

    fn composite(&self, fb: &mut Framebuffer) {
        render_bg(self.layer, self.sheet, fb);
    }
}

/// Rasterize one BG layer onto `fb`. For each framebuffer pixel in the layer's display clip we
/// inverse-map screen→map space (`map = ofs + home + S⁻¹·Rᵀ·(screen − home)`), find the cell
/// covering that map pixel (wrapping modulo the map dimensions), decode the cell's screen data,
/// and sample the corresponding texel from the sheet tile.
fn render_bg(layer: &BgLayer, sheet: &GrpPage, fb: &mut Framebuffer) {
    if !layer.visible || layer.width <= 0 || layer.height <= 0 || layer.tile_size <= 0 {
        return;
    }
    if layer.scale_x == 0.0 || layer.scale_y == 0.0 {
        return;
    }
    // Display clip (pixels) ∩ framebuffer. `None` = the whole layer (the full screen).
    let (cx0, cy0, cx1, cy1) =
        layer
            .clip
            .unwrap_or((0, 0, fb.width as i32 - 1, fb.height as i32 - 1));
    let x0 = cx0.max(0);
    let y0 = cy0.max(0);
    let x1 = cx1.min(fb.width as i32 - 1);
    let y1 = cy1.min(fb.height as i32 - 1);
    let tile = layer.tile_size;
    let tiles_per_row = (GRP_DIM as i32 / tile).max(1);
    let rad = (layer.rot as f64).to_radians();
    let (c, s) = (rad.cos(), rad.sin());
    let (hx, hy) = (layer.home_x as f64, layer.home_y as f64);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f64 - hx;
            let dy = py as f64 - hy;
            let rx = dx * c + dy * s;
            let ry = -dx * s + dy * c;
            let mx = (rx / layer.scale_x + hx + layer.ofs_x as f64).round() as i32;
            let my = (ry / layer.scale_y + hy + layer.ofs_y as f64).round() as i32;
            // Cell covering this map pixel (wrap so a scrolled/off-map read repeats the map).
            let ccx = mx.div_euclid(tile).rem_euclid(layer.width);
            let ccy = my.div_euclid(tile).rem_euclid(layer.height);
            let tx = mx.rem_euclid(tile);
            let ty = my.rem_euclid(tile);
            let data = layer.cell(ccx, ccy);
            let chr = (data & 0x03FF) as i32; // bits 0-9: character 0..1023
            if chr == 0 {
                continue; // character 0 = empty cell (transparent).
            }
            let hflip = data & 0x0400 != 0; // bit 10
            let vflip = data & 0x0800 != 0; // bit 11
            let col = chr % tiles_per_row;
            let row = chr / tiles_per_row;
            let sxp = if hflip { tile - 1 - tx } else { tx };
            let syp = if vflip { tile - 1 - ty } else { ty };
            let texel = sample_sheet(sheet, col * tile + sxp, row * tile + syp);
            if texel & 1 == 0 {
                continue; // transparent sheet texel.
            }
            let argb = modulate(rgba5551_to_argb8888(texel), layer.color);
            fb.set_argb(px as usize, py as usize, argb);
        }
    }
}

/// Compose the scene from the VM's graphics state into a `width`×`height` framebuffer.
/// Back→front the layer stack is: backdrop → the GRP display page (at its `GPRIO` Z, cropped
/// to its display clip) → the four BG layers (each at its `BGOFS` Z) → the displayed sprites
/// (each at its `SPOFS` Z) → console (front). Layers are sorted rear→front by Z ([`compose`]);
/// the documented equal-Z stack `GRP < BG < sprite < console` is encoded by the slice order,
/// and within BG **layer 0 (foreground) draws in front of layer 1+** at equal Z.
///
/// FIDELITY: the per-layer **default Z** values, the exact equal-Z tie-break across kinds, and
/// the sprite-vs-sprite paint order (here ascending management number = rear→front) are
/// oracle-pending — they need the composite screenshot capture (O-T6); queued in
/// `HARVEST_QUEUE.md` (M3-T6).
#[allow(clippy::too_many_arguments)]
pub fn compose_screen(
    width: usize,
    height: usize,
    grp: &GrpState,
    bg: &BgState,
    sprites: &SpriteState,
    console: &Console,
    backdrop: u32,
    vis: LayerVisibility,
) -> Framebuffer {
    // Build the layer list in rear→front order so the stable Z sort keeps the documented
    // layer stack at equal Z. Box the trait objects so the heterogeneous kinds share one list.
    // A `VISIBLE`-hidden layer group (M4-T4) is dropped from the list entirely.
    let mut layers: Vec<Box<dyn Layer + '_>> = Vec::new();
    if vis.graphic {
        layers.push(Box::new(GrpLayer {
            page: &grp.pages[grp.display_page as usize],
            clip: grp.display_clip,
            z: grp.prio,
        }));
    }
    // BG layers: push high→low layer number so layer 0 ends up frontmost among ties.
    if vis.bg {
        for li in (0..BG_LAYER_COUNT).rev() {
            layers.push(Box::new(BgRenderLayer {
                layer: &bg.layers[li],
                sheet: &grp.pages[bg.page as usize],
            }));
        }
    }
    // Sprites: ascending management number (rear→front); exact order oracle-pending.
    if vis.sprite {
        for sp in &sprites.sprites {
            if sp.active && sp.display {
                layers.push(Box::new(SpriteLayer {
                    sprite: sp,
                    sheet: &grp.pages[sp.page as usize],
                }));
            }
        }
    }
    if vis.console {
        layers.push(Box::new(ConsoleLayer {
            console,
            font: Some(&grp.grpf),
            z: CONSOLE_DEFAULT_Z,
        }));
    }
    let refs: Vec<&dyn Layer> = layers.iter().map(|b| b.as_ref()).collect();
    compose(width, height, backdrop, &refs)
}

/// Compose the standard top-screen scene into a 400×240 framebuffer.
/// This is a convenience wrapper around [`compose_screen`].
pub fn compose_top_screen(
    grp: &GrpState,
    bg: &BgState,
    sprites: &SpriteState,
    console: &Console,
    backdrop: u32,
    vis: LayerVisibility,
) -> Framebuffer {
    compose_screen(TOP_WIDTH, TOP_HEIGHT, grp, bg, sprites, console, backdrop, vis)
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
    use crate::bg::BG_PAGE_DEFAULT;
    use crate::grp::{argb8888_to_rgba5551, GrpState};
    use crate::sprite::SPRITE_PAGE_DEFAULT;

    fn full_clip() -> ClipRect {
        ClipRect {
            x0: 0,
            y0: 0,
            x1: TOP_WIDTH as i32 - 1,
            y1: TOP_HEIGHT as i32 - 1,
        }
    }

    /// Compose the top screen with no BG/sprite overlays (the M2 GRP+console scene).
    fn top(grp: &GrpState, console: &Console) -> Framebuffer {
        compose_top_screen(
            grp,
            &BgState::new(),
            &SpriteState::new(),
            console,
            DEFAULT_BACKDROP,
            LayerVisibility::default(),
        )
    }

    #[test]
    fn grp_opaque_pixel_shows_over_backdrop() {
        let mut grp = GrpState::new();
        grp.gcls(0xFFFF_0000); // opaque red fill on the display page
        let fb = top(&grp, &Console::top());
        // Red truncates through the device format to 0xFFF80000.
        assert_eq!(fb.get_argb(0, 0), 0xFFF8_0000);
        assert_eq!(fb.get_argb(399, 239), 0xFFF8_0000);
    }

    #[test]
    fn grp_transparent_pixel_lets_backdrop_through() {
        let grp = GrpState::new(); // blank page = all halfword 0 = alpha bit clear
        let fb = top(&grp, &Console::top());
        // Nothing drawn: the opaque-black backdrop survives.
        assert_eq!(fb.get_argb(10, 10), DEFAULT_BACKDROP);
    }

    #[test]
    fn console_paints_in_front_of_grp_at_equal_z() {
        let mut grp = GrpState::new();
        grp.grpf = crate::assets::default_font_page(); // real font glyphs for the console
        grp.gcls(0xFFFF_0000); // red GRP behind
        let mut console = Console::top();
        console.print_str("A"); // white-on-transparent glyph at (0,0)
                                // GRP prio 0 ties with the console's screen-plane Z; console wins by slice order.
        let fb = top(&grp, &console);
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
        let fb = top(&grp, &Console::top());
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
        let fb = top(&grp, &Console::top());
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8); // blue (5-bit truncated)
    }

    // ---- sprite rasterization (M3-T6) -------------------------------------------------

    /// Full top-screen compose with the given BG + sprite state (default backdrop).
    fn scene(
        grp: &GrpState,
        bg: &BgState,
        sprites: &SpriteState,
        console: &Console,
    ) -> Framebuffer {
        compose_top_screen(
            grp,
            bg,
            sprites,
            console,
            DEFAULT_BACKDROP,
            LayerVisibility::default(),
        )
    }

    /// Compose `z_stack` with custom layer visibility (M4-T4 `VISIBLE`).
    fn scene_vis(
        grp: &GrpState,
        bg: &BgState,
        sprites: &SpriteState,
        console: &Console,
        vis: LayerVisibility,
    ) -> Framebuffer {
        compose_top_screen(grp, bg, sprites, console, DEFAULT_BACKDROP, vis)
    }

    #[test]
    fn sprite_blits_its_source_rect_at_the_home_position() {
        let mut grp = GrpState::new();
        grp.manip_page = SPRITE_PAGE_DEFAULT; // sprites default to sampling GRP4
        grp.gfill(10, 10, 13, 13, 0xFFFF_0000); // a 4×4 opaque-red block on the sheet
        let mut sprites = SpriteState::new();
        sprites.set_direct(0, 10, 10, 4, 4, 0x01); // U,V=10,10 W,H=4, display ON
        sprites.sprites[0].x = 100.0;
        sprites.sprites[0].y = 50.0;
        let fb = scene(&grp, &BgState::new(), &sprites, &Console::top());
        // Home 0,0 → source (U,V) lands exactly on (SPOFS x, y); red truncates to 0xFFF80000.
        assert_eq!(fb.get_argb(100, 50), 0xFFF8_0000);
        assert_eq!(fb.get_argb(103, 53), 0xFFF8_0000); // bottom-right of the 4×4
        assert_eq!(fb.get_argb(104, 54), DEFAULT_BACKDROP); // just past the sprite
        assert_eq!(fb.get_argb(99, 50), DEFAULT_BACKDROP); // just left of it
    }

    #[test]
    fn hidden_sprite_is_not_drawn() {
        let mut grp = GrpState::new();
        grp.manip_page = SPRITE_PAGE_DEFAULT;
        grp.gfill(0, 0, 3, 3, 0xFFFF_0000);
        let mut sprites = SpriteState::new();
        sprites.set_direct(0, 0, 0, 4, 4, 0x00); // attr 0 = display OFF
        let fb = scene(&grp, &BgState::new(), &sprites, &Console::top());
        assert_eq!(fb.get_argb(0, 0), DEFAULT_BACKDROP);
    }

    #[test]
    fn sprite_transparent_texel_lets_layers_behind_through() {
        let mut grp = GrpState::new();
        grp.manip_page = SPRITE_PAGE_DEFAULT; // sheet (page 4) left blank → alpha-clear
        let mut sprites = SpriteState::new();
        sprites.set_direct(0, 0, 0, 8, 8, 0x01);
        let fb = scene(&grp, &BgState::new(), &sprites, &Console::top());
        // Every sampled texel has the alpha bit clear → nothing painted, backdrop survives.
        assert_eq!(fb.get_argb(0, 0), DEFAULT_BACKDROP);
    }

    #[test]
    fn sprite_flip_h_mirrors_the_source_horizontally() {
        let mut grp = GrpState::new();
        grp.manip_page = SPRITE_PAGE_DEFAULT;
        grp.gpset(10, 10, 0xFFFF_0000); // left texel red
        grp.gpset(11, 10, 0xFF00_FF00); // right texel green
        let mut sprites = SpriteState::new();
        sprites.set_direct(0, 10, 10, 2, 1, 0x01);
        sprites.sprites[0].x = 100.0;
        sprites.sprites[0].y = 50.0;
        let fb = scene(&grp, &BgState::new(), &sprites, &Console::top());
        assert_eq!(fb.get_argb(100, 50), 0xFFF8_0000); // red on the left
        assert_eq!(fb.get_argb(101, 50), 0xFF00_F800); // green on the right
        sprites.sprites[0].flip_h = true;
        let fb = scene(&grp, &BgState::new(), &sprites, &Console::top());
        assert_eq!(fb.get_argb(100, 50), 0xFF00_F800); // mirrored: green left
        assert_eq!(fb.get_argb(101, 50), 0xFFF8_0000); // red right
    }

    #[test]
    fn sprite_flip_v_mirrors_the_source_vertically() {
        let mut grp = GrpState::new();
        grp.manip_page = SPRITE_PAGE_DEFAULT;
        grp.gpset(10, 10, 0xFFFF_0000); // top texel red
        grp.gpset(10, 11, 0xFF00_FF00); // bottom texel green
        let mut sprites = SpriteState::new();
        sprites.set_direct(0, 10, 10, 1, 2, 0x01);
        sprites.sprites[0].x = 100.0;
        sprites.sprites[0].y = 50.0;
        sprites.sprites[0].flip_v = true;
        let fb = scene(&grp, &BgState::new(), &sprites, &Console::top());
        assert_eq!(fb.get_argb(100, 50), 0xFF00_F800); // mirrored: green on top
        assert_eq!(fb.get_argb(100, 51), 0xFFF8_0000); // red on bottom
    }

    // ---- BG rasterization (M3-T6) -----------------------------------------------------

    /// Paint sheet pixels for the default 16×16 BG tile `chr` (tiles_per_row = 512/16 = 32).
    fn paint_bg_tile(grp: &mut GrpState, chr: i32, color: u32) {
        grp.manip_page = BG_PAGE_DEFAULT; // BG samples GRP5 by default
        let (col, row) = (chr % 32, chr / 32);
        grp.gfill(col * 16, row * 16, col * 16 + 15, row * 16 + 15, color);
    }

    #[test]
    fn bg_tiles_a_cell_from_the_sheet() {
        let mut grp = GrpState::new();
        paint_bg_tile(&mut grp, 1, 0xFF00_00FF); // char 1 = blue
        let mut bg = BgState::new();
        bg.layers[0].set_cell(0, 0, 1); // place char 1 at cell (0,0)
        let fb = scene(&grp, &bg, &SpriteState::new(), &Console::top());
        // Cell (0,0) covers screen pixels (0..15, 0..15).
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8);
        assert_eq!(fb.get_argb(15, 15), 0xFF00_00F8);
        // Cell (1,0) is char 0 = empty → backdrop shows through.
        assert_eq!(fb.get_argb(16, 0), DEFAULT_BACKDROP);
    }

    #[test]
    fn bg_ofs_scrolls_the_map() {
        let mut grp = GrpState::new();
        paint_bg_tile(&mut grp, 1, 0xFF00_00FF); // char 1 = blue
        let mut bg = BgState::new();
        bg.layers[0].set_cell(1, 0, 1); // char 1 at cell (1,0) → screen (16..31,*)
        let fb = scene(&grp, &bg, &SpriteState::new(), &Console::top());
        assert_eq!(fb.get_argb(0, 0), DEFAULT_BACKDROP); // unscrolled: cell (0,0) empty
        assert_eq!(fb.get_argb(16, 0), 0xFF00_00F8); // the blue cell at its map position
        bg.set_ofs(0, 16, 0, None); // scroll one tile left
        let fb = scene(&grp, &bg, &SpriteState::new(), &Console::top());
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8); // cell (1,0) now at the origin
    }

    #[test]
    fn bg_cell_hflip_mirrors_the_tile() {
        let mut grp = GrpState::new();
        grp.manip_page = BG_PAGE_DEFAULT;
        grp.gfill(16, 0, 23, 15, 0xFFFF_0000); // char 1 left half red
        grp.gfill(24, 0, 31, 15, 0xFF00_FF00); // char 1 right half green
        let mut bg = BgState::new();
        bg.layers[0].set_cell(0, 0, 1); // no flip
        let fb = scene(&grp, &bg, &SpriteState::new(), &Console::top());
        assert_eq!(fb.get_argb(0, 0), 0xFFF8_0000); // red on the left
        assert_eq!(fb.get_argb(15, 0), 0xFF00_F800); // green on the right
        bg.layers[0].set_cell(0, 0, 1 | 0x0400); // bit 10 = H-flip
        let fb = scene(&grp, &bg, &SpriteState::new(), &Console::top());
        assert_eq!(fb.get_argb(0, 0), 0xFF00_F800); // mirrored: green left
        assert_eq!(fb.get_argb(15, 0), 0xFFF8_0000); // red right
    }

    // ---- cross-layer Z interleaving (M3-T6 acceptance) --------------------------------

    /// Build the layered stack used by the Z-order tests: GRP display page red (rear), a blue
    /// BG tile over the top-left 16×16, and an 8×8 green sprite at the origin.
    fn z_stack() -> (GrpState, BgState, SpriteState) {
        let mut grp = GrpState::new();
        grp.grpf = crate::assets::default_font_page(); // real font glyphs for console text
        grp.gcls(0xFFFF_0000); // GRP display page 0 = opaque red
        paint_bg_tile(&mut grp, 1, 0xFF00_00FF); // BG sheet (GRP5): char 1 = blue
        grp.manip_page = SPRITE_PAGE_DEFAULT; // sprite sheet (GRP4)
        grp.gfill(0, 0, 7, 7, 0xFF00_FF00); // 8×8 green sprite image
        let mut bg = BgState::new();
        bg.layers[0].set_cell(0, 0, 1); // blue over screen (0..15, 0..15)
        let mut sprites = SpriteState::new();
        sprites.set_direct(0, 0, 0, 8, 8, 0x01); // 8×8 sprite at the origin
        (grp, bg, sprites)
    }

    #[test]
    fn default_z_orders_grp_bg_sprite_console() {
        let (grp, bg, sprites) = z_stack();
        let fb = scene(&grp, &bg, &sprites, &Console::top());
        // (0,0): sprite (green) is frontmost over BG over GRP.
        assert_eq!(fb.get_argb(0, 0), 0xFF00_F800);
        // (10,0): no sprite here (it's 8×8) → BG blue over the red GRP.
        assert_eq!(fb.get_argb(10, 0), 0xFF00_00F8);
        // (300,200): no sprite, BG cell empty → bare GRP red.
        assert_eq!(fb.get_argb(300, 200), 0xFFF8_0000);
    }

    #[test]
    fn console_paints_in_front_of_sprites_and_bg() {
        let (grp, bg, sprites) = z_stack();
        let mut console = Console::top();
        console.print_str("A"); // white glyph in cell (0,0), over the green sprite
        let fb = scene(&grp, &bg, &sprites, &console);
        let mut saw_white = false;
        for y in 0..8 {
            for x in 0..8 {
                if fb.get_argb(x, y) == 0xFFF8_F8F8 {
                    saw_white = true;
                }
            }
        }
        assert!(saw_white, "console glyph should paint over the sprite/BG");
    }

    #[test]
    fn sprite_z_sends_it_behind_a_nearer_bg_layer() {
        let (grp, bg, mut sprites) = z_stack();
        sprites.sprites[0].z = 1000.0; // push the sprite to the rear (BG ofs_z default 0)
        let fb = scene(&grp, &bg, &sprites, &Console::top());
        // Z now wins over the slice-order default: at (0,0) the nearer BG blue covers the sprite.
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8);
    }

    // ---- VISIBLE layer gating (M4-T4) -------------------------------------------------

    #[test]
    fn hidden_sprite_layer_reveals_the_bg_behind() {
        // The default stack shows the green sprite over the blue BG at (0,0); hiding the
        // sprite layer (VISIBLE _,_,_,0) drops it, so the BG blue shows through.
        let (grp, bg, sprites) = z_stack();
        let vis = LayerVisibility {
            sprite: false,
            ..Default::default()
        };
        let fb = scene_vis(&grp, &bg, &sprites, &Console::top(), vis);
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00F8); // BG blue, sprite hidden
    }

    #[test]
    fn hiding_graphic_and_bg_falls_through_to_the_backdrop() {
        // With the GRP, BG and sprite layers all hidden, an empty console leaves only the
        // backdrop — every overlay layer is gone.
        let (grp, bg, sprites) = z_stack();
        let vis = LayerVisibility {
            graphic: false,
            bg: false,
            sprite: false,
            console: true,
        };
        let fb = scene_vis(&grp, &bg, &sprites, &Console::top(), vis);
        assert_eq!(fb.get_argb(0, 0), DEFAULT_BACKDROP);
        assert_eq!(fb.get_argb(300, 200), DEFAULT_BACKDROP);
    }

    #[test]
    fn hidden_console_layer_drops_the_text() {
        // A printed glyph normally paints white over the scene; hiding the console layer
        // (VISIBLE 0,_,_,_) leaves the sprite green showing instead.
        let (grp, bg, sprites) = z_stack();
        let mut console = Console::top();
        console.print_str("A");
        let vis = LayerVisibility {
            console: false,
            ..Default::default()
        };
        let fb = scene_vis(&grp, &bg, &sprites, &console, vis);
        let mut saw_white = false;
        for y in 0..8 {
            for x in 0..8 {
                if fb.get_argb(x, y) == 0xFFF8_F8F8 {
                    saw_white = true;
                }
            }
        }
        assert!(!saw_white, "hidden console layer must not paint its glyph");
        assert_eq!(fb.get_argb(0, 0), 0xFF00_F800); // sprite green shows instead
    }
}
