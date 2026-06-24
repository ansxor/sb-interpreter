//! Sprite table (M3-T1) — SmileBASIC's movable graphic objects.
//!
//! SmileBASIC keeps **512 sprite slots** (management numbers 0..511). Each slot, once
//! created with `SPSET`, holds everything the compositor needs to draw it: a source
//! rectangle on the sprite sheet (`U,V,W,H`), a position (`SPOFS`, incl. Z), a home/origin
//! offset (`SPHOME`), uniform/non-uniform scale (`SPSCALE`), free rotation (`SPROT`), a
//! modulate color (`SPCOLOR`), the sheet page it samples (`SPPAGE`, default GRP4), a
//! display/rotation/flip/additive attribute bitfield (the `SPSET`/`SPCHR` `attr`), and 8
//! per-sprite variables (`SPVAR`). The sheet page is one of the M2 GRP pages.
//!
//! This module owns the table + the **lifecycle** operations the VM drives in M3-T1:
//! create (`SPSET`, all six forms — explicit number or auto-allocated, from an `SPDEF`
//! template or a direct image), release (`SPCLR`, one or all), show/hide (`SPSHOW`/
//! `SPHIDE`), and the in-use query (`SPUSED`). Animation/link/vars (M3-T2), collision
//! (M3-T3), and the actual blit into the [`Framebuffer`](crate::Framebuffer) (M3-T6) build
//! on this model; the transform/char/color **setters** (`SPOFS`/`SPCHR`/…) land with them.
//!
//! Structural cross-check: `osb/SMILEBASIC/sprite.d` `SpriteData` (3.5.0). FIDELITY notes:
//! `SPSET` (re)initializes the slot — offset 0, rotation 0, scale 1, all `SPVAR` 0, color
//! default white, attribute per the `attr` argument (display ON by default) — per
//! `spec/instructions/spset.yaml` (disassembled handler @0x1415a0).

/// Number of sprite management slots: 0..511 (the sprite count `[[0x315d60]+0x48]`).
pub const SPRITE_COUNT: usize = 512;
/// Number of `SPANIM` target channels (`target & 7`): 0 XY, 1 Z, 2 UV, 3 I (definition),
/// 4 R (rotation), 5 S (scale), 6 C (color), 7 V (internal variable 7).
pub const ANIM_CHANNELS: usize = 8;
/// Items per keyframe for each channel: XY/UV/S take 2, the rest take 1
/// (`spec/instructions/spanim.yaml`).
pub const ANIM_ITEMS: [usize; ANIM_CHANNELS] = [2, 1, 2, 1, 1, 2, 1, 1];
/// Highest `SPDEF` template number accepted by `SPSET` form 1 (`&H0FFF`).
pub const SPDEF_MAX: i32 = 4095;
/// Default sheet page a sprite samples from (`SPPAGE` default = GRP4).
pub const SPRITE_PAGE_DEFAULT: u8 = 4;
/// Default `SPSET` source-rectangle size when `W,H` are omitted (16×16).
pub const SPRITE_DEFAULT_WH: i32 = 16;
/// Default `SPSET` attribute when `attr` is omitted: display ON only (`#SPSHOW` &H01).
pub const SPRITE_DEFAULT_ATTR: i32 = 0x01;
/// Number of `SPDEF` definition-template slots: 0..4095 (the `cmp r8,#0x1000` clamp
/// @0x13ff48 in the `SPDEF` handler).
pub const SPDEF_TEMPLATE_COUNT: usize = 4096;

/// One `SPDEF` definition template: the pre-set source rectangle, home/origin point, and
/// attribute that `SPSET` copies from when creating a sprite by definition number. The
/// template table is seeded from `spdef.csv` on the real machine; absent that resource we
/// model the initial/reset state as the documented defaults (16×16 at the sheet origin,
/// home 0,0, attribute display-ON). The exact `spdef.csv` per-template rectangles are
/// oracle-pending (no framebuffer harvest yet — see `HARVEST_QUEUE.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpdefEntry {
    /// Source-image X,Y on the sprite sheet.
    pub u: i32,
    pub v: i32,
    /// Source-image width/height (default 16×16).
    pub w: i32,
    pub h: i32,
    /// Reference (home) point for the sprite's coordinates (default 0,0).
    pub origin_x: i32,
    pub origin_y: i32,
    /// Display/rotation/flip/blend attribute bits (default &H01 = display ON).
    pub attr: i32,
}

impl Default for SpdefEntry {
    fn default() -> Self {
        Self {
            u: 0,
            v: 0,
            w: SPRITE_DEFAULT_WH,
            h: SPRITE_DEFAULT_WH,
            origin_x: 0,
            origin_y: 0,
            attr: SPRITE_DEFAULT_ATTR,
        }
    }
}

/// The shared collision-result record written by the most recent `SPHIT*` and read back by
/// `SPHITINFO`: a swept-frame collision `time` (0..1) plus the collision-time coordinates
/// and velocities of the two colliding objects (object 1 = the tested sprite/rectangle,
/// object 2 = the sprite it hit). `collision_coordinate = position_at_detection +
/// velocity * time`. The default (no collision yet) is all-zero. The non-zero swept `time`
/// is oracle-pending; with the velocities we model (static, vel 0 unless `SPCOLVEC` set)
/// it stays 0, matching the documented "position at detection" coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HitInfo {
    pub time: f64,
    pub x1: f64,
    pub y1: f64,
    pub vx1: f64,
    pub vy1: f64,
    pub x2: f64,
    pub y2: f64,
    pub vx2: f64,
    pub vy2: f64,
}

/// The keyframe-animation engine sprites share with BG layers (`SPANIM`/`BGANIM`). The
/// `AnimError`/`AnimKeyframe`/[`ANIM_MAX_KEYFRAMES`] data-build pieces and the
/// hold/interpolate/loop/relative advancement live in [`crate::anim`]; a sprite animation is
/// one of those engines (`SpriteAnim`), with the sprite's [`Sprite::read_channel`]/
/// [`Sprite::write_channel`] supplying the per-channel base/write-back. Exact interpolation
/// rounding is oracle-pending (no framebuffer harvest yet — see `HARVEST_QUEUE.md`).
pub use crate::anim::{AnimError, AnimKeyframe, KeyframeAnim, ANIM_MAX_KEYFRAMES};

/// A running per-channel `SPANIM` animation: a [`KeyframeAnim`] driving one sprite channel.
pub type SpriteAnim = KeyframeAnim;

/// One sprite slot. `active` distinguishes a live sprite (created by `SPSET`, not yet
/// `SPCLR`'d) from a free slot; every other field is meaningful only while `active`.
///
/// The fields mirror what the M3-T6 compositor will read. Angles are degrees (`SPROT`),
/// `color` is an ARGB8888 modulate code (default opaque white), and the attribute is kept
/// decoded into its component flags (see [`Sprite::set_attr`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// Live (created by `SPSET`, not yet released by `SPCLR`).
    pub active: bool,
    /// Display flag — attribute b00 (`#SPSHOW` &H01). `SPSHOW`/`SPHIDE` toggle it.
    pub display: bool,
    /// 90° rotation step — attribute b01-b02 (`#SPROT0/90/180/270` = 0/1/2/3).
    pub rot90: u8,
    /// Horizontal flip — attribute b03 (`#SPREVH` &H08).
    pub flip_h: bool,
    /// Vertical flip — attribute b04 (`#SPREVV` &H10).
    pub flip_v: bool,
    /// Additive blending — attribute b05 (`#SPADD` &H20).
    pub additive: bool,
    /// `SPDEF` template number this slot was created from (form 1/3/5), else -1 (direct
    /// image). The template's `U,V,W,H` are resolved when `SPDEF` lands (M3-T3); the slot
    /// keeps its own copy so a later `SPDEF` does not retroactively change it.
    pub defno: i32,
    /// Source rectangle on the sheet (`U,V` top-left, `W,H` size).
    pub u: i32,
    pub v: i32,
    pub w: i32,
    pub h: i32,
    /// Position (`SPOFS`), in screen pixels; `z` is the depth/priority.
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Home/origin offset (`SPHOME`), the rotation/scale pivot relative to the sprite.
    /// Stored as a 32-bit float on hardware (the handler writes f32 at slot+0x18); negative
    /// and fractional offsets are accepted and round-trip verbatim, so this is `f64` here, not
    /// the integer grid-cell home of BG. hw_verified (sb-oracle 2026-06-24): `SPHOME 0,127.5,
    /// 127.5` / `0,16.25,-8.5` / `0,-16,-16` read back unchanged.
    pub home_x: f64,
    pub home_y: f64,
    /// Character offset added to the sheet sampling (`SPCHR`).
    pub chr: i32,
    /// Scale factors (`SPSCALE`), 1.0 = unscaled.
    pub scale_x: f64,
    pub scale_y: f64,
    /// Free rotation in degrees (`SPROT`).
    pub rot: f64,
    /// Modulate color (`SPCOLOR`), ARGB8888. Default opaque white (`-1`).
    pub color: u32,
    /// Sheet page the sprite samples (`SPPAGE`), 0..=5 (default GRP4).
    pub page: u8,
    /// The 8 per-sprite variables (`SPVAR`), all 0 on `SPSET`.
    pub var: [f64; 8],
    /// Animation paused flag (`SPSTOP` sets it, `SPSTART` clears it) — slot flag bit
    /// 0x2000000. A freshly `SPANIM`'d sprite is running (flag clear).
    pub anim_stopped: bool,
    /// Parent sprite this slot is linked to (`SPLINK`), or `None` when unlinked. The parent
    /// number is always strictly lower than this slot's number.
    pub parent: Option<usize>,
    /// Bound callback process name (`SPFUNC`), without the leading `@`. Invoked by
    /// `CALL SPRITE`; survives `SPSET` (binding does not require the slot to be active).
    pub func: Option<String>,
    /// The per-channel `SPANIM` animations (one optional animation per target 0..7).
    pub anims: [Option<SpriteAnim>; ANIM_CHANNELS],
    /// Collision detection enabled (`SPCOL`). A sprite only participates in `SPHIT*` once
    /// `SPCOL` has been called; a fresh sprite does not collide.
    pub col_enabled: bool,
    /// Detection rectangle start, relative to `SPHOME` (the home point is the area origin
    /// 0,0). Defaults to 0,0 (the top-left of the sprite).
    pub col_sx: i32,
    pub col_sy: i32,
    /// Detection rectangle size. When `SPCOL` is enabled without an explicit range these
    /// default to the sprite's full `W,H`.
    pub col_w: i32,
    pub col_h: i32,
    /// Synchronize the detection area with `SPSCALE` (the `SPCOL` scale-adjust flag).
    pub col_scale_adjust: bool,
    /// 32-bit collision mask (`SPCOL`). Two objects collide only when `maskA AND maskB`
    /// is non-zero. Default all bits set (`&HFFFFFFFF`).
    pub col_mask: u32,
    /// Per-frame movement vector carried into swept collision (`SPCOLVEC`), reported by
    /// `SPHITINFO` as VX/VY. Default 0,0.
    pub col_vx: f64,
    pub col_vy: f64,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            active: false,
            display: false,
            rot90: 0,
            flip_h: false,
            flip_v: false,
            additive: false,
            defno: -1,
            u: 0,
            v: 0,
            w: SPRITE_DEFAULT_WH,
            h: SPRITE_DEFAULT_WH,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            home_x: 0.0,
            home_y: 0.0,
            chr: 0,
            scale_x: 1.0,
            scale_y: 1.0,
            rot: 0.0,
            color: 0xFFFF_FFFF,
            page: SPRITE_PAGE_DEFAULT,
            var: [0.0; 8],
            anim_stopped: false,
            parent: None,
            func: None,
            anims: std::array::from_fn(|_| None),
            col_enabled: false,
            col_sx: 0,
            col_sy: 0,
            col_w: 0,
            col_h: 0,
            col_scale_adjust: false,
            col_mask: 0xFFFF_FFFF,
            col_vx: 0.0,
            col_vy: 0.0,
        }
    }
}

impl Sprite {
    /// Decode an `SPSET`/`SPCHR` attribute bitfield into the display/rotation/flip/additive
    /// flags: b00 display, b01-b02 90° rotation, b03 H-flip, b04 V-flip, b05 additive.
    pub fn set_attr(&mut self, attr: i32) {
        self.display = attr & 0x01 != 0;
        self.rot90 = ((attr >> 1) & 0x03) as u8;
        self.flip_h = attr & 0x08 != 0;
        self.flip_v = attr & 0x10 != 0;
        self.additive = attr & 0x20 != 0;
    }

    /// Read an `SPANIM` channel's current value(s) as floats (the relative/interpolation
    /// base). Channels: 0 XY, 1 Z, 2 UV, 3 I (definition), 4 R, 5 S, 6 C, 7 V.
    fn read_channel(&self, channel: usize) -> Vec<f64> {
        match channel {
            0 => vec![self.x, self.y],
            1 => vec![self.z],
            2 => vec![self.u as f64, self.v as f64],
            3 => vec![self.defno as f64],
            4 => vec![self.rot],
            5 => vec![self.scale_x, self.scale_y],
            6 => vec![self.color as f64],
            _ => vec![self.var[7]],
        }
    }

    /// Write an `SPANIM` channel's animated value(s) back into the slot. Integer channels
    /// (UV/I/C) round to the nearest integer; the exact rounding is oracle-pending.
    fn write_channel(&mut self, channel: usize, v: &[f64]) {
        let g = |i: usize| v.get(i).copied().unwrap_or(0.0);
        match channel {
            0 => {
                self.x = g(0);
                self.y = g(1);
            }
            1 => self.z = g(0),
            2 => {
                self.u = g(0).round() as i32;
                self.v = g(1).round() as i32;
            }
            3 => self.defno = g(0).round() as i32,
            4 => self.rot = g(0),
            5 => {
                self.scale_x = g(0);
                self.scale_y = g(1);
            }
            6 => self.color = g(0).round() as i64 as u32,
            _ => self.var[7] = g(0),
        }
    }
}

/// The sprite system state: the 512-slot table the VM mutates for the lifecycle
/// commands. Rendering reads it (M3-T6); the transform setters extend it (M3-T2/T3).
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteState {
    /// The 512 sprite slots, indexed by management number.
    pub sprites: Vec<Sprite>,
    /// The 4096 `SPDEF` definition templates `SPSET` (form 1) copies from.
    pub spdef: Vec<SpdefEntry>,
    /// The shared `SPHIT*` collision-result record `SPHITINFO` reads back.
    pub hit: HitInfo,
    /// The global graphic page the sprite system renders onto (`SPPAGE`, 0..=5; default
    /// GRP4). One value for the whole system — the disassembled GET reads it back from
    /// `[[0x315d60]+0x4c]`. New sprites capture it at `SPSET` time into their per-slot
    /// [`Sprite::page`].
    pub page: u8,
}

impl Default for SpriteState {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteState {
    /// A fresh sprite system: every slot free (no sprite created yet).
    pub fn new() -> Self {
        Self {
            sprites: vec![Sprite::default(); SPRITE_COUNT],
            spdef: vec![SpdefEntry::default(); SPDEF_TEMPLATE_COUNT],
            hit: HitInfo::default(),
            page: SPRITE_PAGE_DEFAULT,
        }
    }

    /// Whether a management number is in range (0..511).
    pub fn in_range(mgmt: i32) -> bool {
        (0..SPRITE_COUNT as i32).contains(&mgmt)
    }

    /// `SPUSED(mgmt)` — is the slot currently allocated? (TRUE after `SPSET`, until
    /// `SPCLR`). No active-bit *guard* — querying a free slot is valid and returns false.
    pub fn is_used(&self, mgmt: usize) -> bool {
        self.sprites[mgmt].active
    }

    /// Initialise a slot as a live sprite, resetting transform/vars to defaults. `rect` is
    /// the source rectangle `(U,V,W,H)`; `home` is the home/origin offset; `defno` is the
    /// `SPDEF` template number, or -1 for the direct-image forms.
    fn create(
        &mut self,
        mgmt: usize,
        rect: (i32, i32, i32, i32),
        home: (i32, i32),
        attr: i32,
        defno: i32,
    ) {
        let (u, v, w, h) = rect;
        let mut sp = Sprite {
            active: true,
            defno,
            u,
            v,
            w,
            h,
            home_x: home.0 as f64,
            home_y: home.1 as f64,
            // New sprites sample the page the sprite system is currently rendering onto
            // (the global `SPPAGE`, default GRP4).
            page: self.page,
            ..Sprite::default()
        };
        sp.set_attr(attr);
        self.sprites[mgmt] = sp;
    }

    /// `SPSET mgmt, U,V,W,H, attr` — create a directly-imaged sprite at an explicit slot
    /// (home/origin 0,0).
    pub fn set_direct(&mut self, mgmt: usize, u: i32, v: i32, w: i32, h: i32, attr: i32) {
        self.create(mgmt, (u, v, w, h), (0, 0), attr, -1);
    }

    /// `SPSET mgmt, defn` — create a sprite at an explicit slot from an `SPDEF` template:
    /// the slot copies the template's source rectangle, home/origin, and attribute (so a
    /// later `SPDEF` does not retroactively change a created sprite). The caller has already
    /// range-checked `defno` (0..4095).
    pub fn set_template(&mut self, mgmt: usize, defno: i32) {
        let t = self.spdef[defno as usize];
        self.create(
            mgmt,
            (t.u, t.v, t.w, t.h),
            (t.origin_x, t.origin_y),
            t.attr,
            defno,
        );
    }

    /// Find a free slot scanning the inclusive `[start, end]` range in the given order
    /// (`start` toward `end`), returning the chosen management number or `None` if every
    /// slot in the range is in use. Forms 3/4 pass the whole range `0..=511`.
    pub fn alloc(&self, start: usize, end: usize) -> Option<usize> {
        if start <= end {
            (start..=end).find(|&i| !self.sprites[i].active)
        } else {
            (end..=start).rev().find(|&i| !self.sprites[i].active)
        }
    }

    /// `SPCLR mgmt` — release one slot (harmless if already free).
    pub fn clear(&mut self, mgmt: usize) {
        self.sprites[mgmt] = Sprite::default();
    }

    /// `SPCLR` (no argument) — release every user sprite at once.
    pub fn clear_all(&mut self) {
        for sp in &mut self.sprites {
            *sp = Sprite::default();
        }
    }

    /// `SPSTOP`/`SPSTART` — pause (`stop`=true) or resume the animation of every sprite at
    /// once (the no-argument forms). The disassembly walks all slots unconditionally and
    /// raises no error.
    pub fn set_anim_stopped_all(&mut self, stop: bool) {
        for sp in &mut self.sprites {
            sp.anim_stopped = stop;
        }
    }

    /// `SPSTOP mgmt`/`SPSTART mgmt` — pause/resume one sprite's animation.
    pub fn set_anim_stopped(&mut self, mgmt: usize, stop: bool) {
        self.sprites[mgmt].anim_stopped = stop;
    }

    /// `SPFUNC mgmt, name` — bind a callback process name to a slot (no `SPSET` required).
    pub fn set_func(&mut self, mgmt: usize, name: Option<String>) {
        self.sprites[mgmt].func = name;
    }

    /// The callback process name bound to a slot by `SPFUNC`, or `None` if unbound — read by
    /// `CALL SPRITE` dispatch (M6-T6).
    pub fn func(&self, mgmt: usize) -> Option<String> {
        self.sprites[mgmt].func.clone()
    }

    /// `SPLINK child, parent` — link `child` to `parent` (only coordinates are inherited).
    /// The caller has already validated the ordering and active bits.
    pub fn link(&mut self, child: usize, parent: usize) {
        self.sprites[child].parent = Some(parent);
    }

    /// `SPUNLINK mgmt` — break a slot's parent link (a no-op when unlinked).
    pub fn unlink(&mut self, mgmt: usize) {
        self.sprites[mgmt].parent = None;
    }

    /// `=SPLINK(mgmt)` — the undocumented function form: the slot's parent management
    /// number, or -1 when unlinked.
    pub fn parent_of(&self, mgmt: usize) -> i32 {
        self.sprites[mgmt].parent.map_or(-1, |p| p as i32)
    }

    /// The composited display position of a sprite: its own `SPOFS` plus every ancestor's
    /// position (rotation/scale are NOT inherited — only coordinates). A parent always has a
    /// lower management number, so the chain terminates.
    pub fn display_pos(&self, mgmt: usize) -> (f64, f64) {
        let sp = &self.sprites[mgmt];
        let (mut x, mut y) = (sp.x, sp.y);
        if let Some(p) = sp.parent {
            let (px, py) = self.display_pos(p);
            x += px;
            y += py;
        }
        (x, y)
    }

    /// Install an `SPANIM` animation on a channel of a slot from an already-flattened
    /// `data` list (`Time, Item[, Item], …`) and a `loop_count` (0 = endless). Validates the
    /// per-channel keyframe shape, the 32-keyframe cap, and the ±32768 item range, then
    /// captures the slot's current channel value as the relative/interpolation base and
    /// starts the animation. The caller validated mgmt/active/target.
    pub fn set_anim(
        &mut self,
        mgmt: usize,
        channel: usize,
        relative: bool,
        data: &[f64],
        loop_count: i32,
    ) -> Result<(), AnimError> {
        let base = self.sprites[mgmt].read_channel(channel);
        let anim = KeyframeAnim::build(
            channel,
            relative,
            ANIM_ITEMS[channel],
            base,
            data,
            loop_count,
        )?;
        self.sprites[mgmt].anims[channel] = Some(anim);
        // A fresh animation runs immediately (the stop bit is per-sprite; SPANIM does not
        // touch it). Leave anim_stopped as-is.
        Ok(())
    }

    /// Advance every running animation by `frames` frames (the frame clock — driven by
    /// `VSYNC`/`WAIT`). Stopped or inactive sprites are skipped; each advanced channel value
    /// is written back into its slot.
    pub fn tick(&mut self, frames: u64) {
        for _ in 0..frames {
            for i in 0..self.sprites.len() {
                if !self.sprites[i].active || self.sprites[i].anim_stopped {
                    continue;
                }
                // Take the channels out so we can write the value back into the slot.
                let mut anims = std::mem::take(&mut self.sprites[i].anims);
                for (ch, slot) in anims.iter_mut().enumerate() {
                    if let Some(anim) = slot {
                        if !anim.done {
                            anim.step();
                            let cur = anim.cur.clone();
                            self.sprites[i].write_channel(ch, &cur);
                        }
                    }
                }
                self.sprites[i].anims = anims;
            }
        }
    }

    // -- collision (M3-T3) -----------------------------------------------------

    /// `SPCOL` — enable collision on a sprite. `rect` is the explicit detection rectangle
    /// `(sx,sy,w,h)` relative to `SPHOME`; when `None` the detection area defaults to the
    /// sprite's full `W,H` (origin 0,0). The caller has validated mgmt/active.
    pub fn set_col(
        &mut self,
        mgmt: usize,
        rect: Option<(i32, i32, i32, i32)>,
        scale_adjust: bool,
        mask: u32,
    ) {
        let (w, h) = (self.sprites[mgmt].w, self.sprites[mgmt].h);
        let sp = &mut self.sprites[mgmt];
        sp.col_enabled = true;
        sp.col_scale_adjust = scale_adjust;
        sp.col_mask = mask;
        match rect {
            Some((sx, sy, rw, rh)) => {
                sp.col_sx = sx;
                sp.col_sy = sy;
                sp.col_w = rw;
                sp.col_h = rh;
            }
            None => {
                sp.col_sx = 0;
                sp.col_sy = 0;
                sp.col_w = w;
                sp.col_h = h;
            }
        }
    }

    /// `SPCOLVEC` — set the per-frame collision movement vector. `v` is the explicit vector;
    /// `None` is the auto-calculated form (the `SPANIM` "XY" linear-interpolation delta when
    /// running, otherwise 0,0 — the running-delta case is oracle-pending, so we model the
    /// documented stationary default 0,0). The caller has validated mgmt/active.
    pub fn set_colvec(&mut self, mgmt: usize, v: Option<(f64, f64)>) {
        let (vx, vy) = v.unwrap_or((0.0, 0.0));
        self.sprites[mgmt].col_vx = vx;
        self.sprites[mgmt].col_vy = vy;
    }

    /// World-space AABB detection rectangle `(x,y,w,h)` of a collision-enabled sprite, or
    /// `None` when the sprite is inactive or has not had `SPCOL` called (such a sprite does
    /// not collide). The rectangle is the sprite's display position (incl. `SPLINK`
    /// inheritance) plus the detection start, sized by the detection `W,H` (scaled by
    /// `SPSCALE` when the scale-adjust flag is set).
    fn col_aabb(&self, mgmt: usize) -> Option<(f64, f64, f64, f64)> {
        let sp = &self.sprites[mgmt];
        if !sp.active || !sp.col_enabled {
            return None;
        }
        let (dx, dy) = self.display_pos(mgmt);
        let (mut w, mut h) = (sp.col_w as f64, sp.col_h as f64);
        if sp.col_scale_adjust {
            w *= sp.scale_x.abs();
            h *= sp.scale_y.abs();
        }
        Some((dx + sp.col_sx as f64, dy + sp.col_sy as f64, w, h))
    }

    /// Record a sprite-vs-sprite hit into the shared `SPHITINFO` record (object 1 = the
    /// tested sprite `a`, object 2 = the sprite it hit `b`); `time` is 0 (static collision —
    /// the swept time is oracle-pending).
    fn record_hit_sp(&mut self, a: usize, b: usize) {
        let (ax, ay) = self.display_pos(a);
        let (bx, by) = self.display_pos(b);
        let (avx, avy) = (self.sprites[a].col_vx, self.sprites[a].col_vy);
        let (bvx, bvy) = (self.sprites[b].col_vx, self.sprites[b].col_vy);
        self.hit = HitInfo {
            time: 0.0,
            x1: ax,
            y1: ay,
            vx1: avx,
            vy1: avy,
            x2: bx,
            y2: by,
            vx2: bvx,
            vy2: bvy,
        };
    }

    /// Record a rectangle-vs-sprite hit (object 1 = the tested rectangle, object 2 = the
    /// sprite it hit).
    fn record_hit_rc(&mut self, rect: (f64, f64, f64, f64), mv: (f64, f64), b: usize) {
        let (bx, by) = self.display_pos(b);
        let (bvx, bvy) = (self.sprites[b].col_vx, self.sprites[b].col_vy);
        self.hit = HitInfo {
            time: 0.0,
            x1: rect.0,
            y1: rect.1,
            vx1: mv.0,
            vy1: mv.1,
            x2: bx,
            y2: by,
            vx2: bvx,
            vy2: bvy,
        };
    }

    /// `SPHITSP(mgmt[,first,last])` — test one sprite against a management-number range
    /// (inclusive, defaulting to the whole table). Returns the first colliding sprite's
    /// management number (skipping the tested sprite), or -1 when none collide.
    pub fn hit_sp_range(&mut self, mgmt: usize, first: usize, last: usize) -> i32 {
        let Some(a) = self.col_aabb(mgmt) else {
            return -1;
        };
        let amask = self.sprites[mgmt].col_mask;
        let (lo, hi) = if first <= last {
            (first, last)
        } else {
            (last, first)
        };
        for opp in lo..=hi {
            if opp == mgmt {
                continue;
            }
            let Some(b) = self.col_aabb(opp) else {
                continue;
            };
            if amask & self.sprites[opp].col_mask == 0 {
                continue;
            }
            if aabb_overlap(a, b) {
                self.record_hit_sp(mgmt, opp);
                return opp as i32;
            }
        }
        -1
    }

    /// `SPHITSP(mgmt, opponent)` — test two specific sprites. Returns true on collision.
    pub fn hit_sp_pair(&mut self, mgmt: usize, opp: usize) -> bool {
        let (Some(a), Some(b)) = (self.col_aabb(mgmt), self.col_aabb(opp)) else {
            return false;
        };
        if self.sprites[mgmt].col_mask & self.sprites[opp].col_mask == 0 {
            return false;
        }
        if aabb_overlap(a, b) {
            self.record_hit_sp(mgmt, opp);
            true
        } else {
            false
        }
    }

    /// `SPHITRC` — test a (moving) rectangle against a management-number range. Returns the
    /// first colliding sprite's management number, or -1 when none collide.
    pub fn hit_rc_range(
        &mut self,
        rect: (f64, f64, f64, f64),
        mask: u32,
        mv: (f64, f64),
        first: usize,
        last: usize,
    ) -> i32 {
        let (lo, hi) = if first <= last {
            (first, last)
        } else {
            (last, first)
        };
        for opp in lo..=hi {
            let Some(b) = self.col_aabb(opp) else {
                continue;
            };
            if mask & self.sprites[opp].col_mask == 0 {
                continue;
            }
            if aabb_overlap(rect, b) {
                self.record_hit_rc(rect, mv, opp);
                return opp as i32;
            }
        }
        -1
    }

    /// `SPHITRC(mgmt, …)` — test a (moving) rectangle against one specific sprite. Returns
    /// true on collision.
    pub fn hit_rc_one(
        &mut self,
        rect: (f64, f64, f64, f64),
        mask: u32,
        mv: (f64, f64),
        opp: usize,
    ) -> bool {
        let Some(b) = self.col_aabb(opp) else {
            return false;
        };
        if mask & self.sprites[opp].col_mask == 0 {
            return false;
        }
        if aabb_overlap(rect, b) {
            self.record_hit_rc(rect, mv, opp);
            true
        } else {
            false
        }
    }

    /// `SPCHK(mgmt)` — the 8-bit animation-status bitmask: bit `c` is set when channel `c`
    /// has a running (not finished) `SPANIM`. A stopped sprite (`SPSTOP`) reads 0.
    pub fn anim_status(&self, mgmt: usize) -> i32 {
        let sp = &self.sprites[mgmt];
        if sp.anim_stopped {
            return 0;
        }
        let mut bits = 0;
        for (ch, slot) in sp.anims.iter().enumerate() {
            if let Some(a) = slot {
                if !a.done {
                    bits |= 1 << ch;
                }
            }
        }
        bits
    }

    // -- SPDEF definition templates (M3-T3) ------------------------------------

    /// `SPDEF` (no args) — reset every definition template to its initial default.
    pub fn spdef_reset(&mut self) {
        for e in &mut self.spdef {
            *e = SpdefEntry::default();
        }
    }

    /// Read a definition template (the caller has range-checked `defnum` 0..4095).
    pub fn spdef_get(&self, defnum: usize) -> SpdefEntry {
        self.spdef[defnum]
    }

    /// Write a definition template (the caller has range-checked `defnum` and the fields).
    pub fn spdef_set(&mut self, defnum: usize, entry: SpdefEntry) {
        self.spdef[defnum] = entry;
    }
}

/// Standard AABB overlap test for two `(x,y,w,h)` rectangles (touching edges do not
/// count as overlap).
fn aabb_overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_is_all_free() {
        let st = SpriteState::new();
        assert_eq!(st.sprites.len(), SPRITE_COUNT);
        assert!(st.sprites.iter().all(|s| !s.active));
        assert!(!st.is_used(0));
    }

    #[test]
    fn create_then_clear_round_trips_used() {
        let mut st = SpriteState::new();
        st.set_template(0, 0);
        assert!(st.is_used(0));
        // Default attr = display ON only.
        assert!(st.sprites[0].display);
        assert_eq!(st.sprites[0].rot90, 0);
        assert!(!st.sprites[0].flip_h);
        // SPSET defaults: scale 1, no rotation, white, page GRP4, vars 0.
        assert_eq!(st.sprites[0].scale_x, 1.0);
        assert_eq!(st.sprites[0].rot, 0.0);
        assert_eq!(st.sprites[0].color, 0xFFFF_FFFF);
        assert_eq!(st.sprites[0].page, SPRITE_PAGE_DEFAULT);
        assert_eq!(st.sprites[0].var, [0.0; 8]);
        st.clear(0);
        assert!(!st.is_used(0));
    }

    #[test]
    fn attr_bits_decode() {
        let mut sp = Sprite::default();
        // display + 180° + V-flip + additive = 0x01 | 0x04 | 0x10 | 0x20.
        sp.set_attr(0x01 | 0x04 | 0x10 | 0x20);
        assert!(sp.display);
        assert_eq!(sp.rot90, 2);
        assert!(!sp.flip_h);
        assert!(sp.flip_v);
        assert!(sp.additive);
    }

    #[test]
    fn alloc_picks_lowest_free_then_returns_none_when_full() {
        let mut st = SpriteState::new();
        assert_eq!(st.alloc(0, SPRITE_COUNT - 1), Some(0));
        // Fill every slot.
        for i in 0..SPRITE_COUNT {
            st.set_direct(i, 0, 0, 16, 16, 1);
        }
        assert_eq!(st.alloc(0, SPRITE_COUNT - 1), None);
        // Freeing one makes it allocatable again.
        st.clear(100);
        assert_eq!(st.alloc(0, SPRITE_COUNT - 1), Some(100));
    }

    #[test]
    fn alloc_respects_range_and_direction() {
        let mut st = SpriteState::new();
        // Range [10,20] returns the low end first.
        assert_eq!(st.alloc(10, 20), Some(10));
        st.set_direct(10, 0, 0, 16, 16, 1);
        assert_eq!(st.alloc(10, 20), Some(11));
        // Reversed range scans from the high end down.
        assert_eq!(st.alloc(20, 10), Some(20));
    }

    #[test]
    fn clear_all_frees_every_slot() {
        let mut st = SpriteState::new();
        st.set_template(0, 0);
        st.set_template(5, 0);
        st.clear_all();
        assert!(st.sprites.iter().all(|s| !s.active));
    }

    #[test]
    fn link_inherits_only_coordinates() {
        let mut st = SpriteState::new();
        st.set_direct(4, 0, 0, 16, 16, 1);
        st.set_direct(15, 0, 0, 16, 16, 1);
        st.sprites[4].x = 100.0;
        st.sprites[4].y = 50.0;
        st.sprites[15].x = 10.0;
        st.sprites[15].y = 5.0;
        st.sprites[4].rot = 90.0; // rotation is NOT inherited
        assert_eq!(st.parent_of(15), -1);
        st.link(15, 4);
        assert_eq!(st.parent_of(15), 4);
        // Child display position = own offset + parent's (coords only).
        assert_eq!(st.display_pos(15), (110.0, 55.0));
        assert_eq!(st.sprites[15].rot, 0.0);
        st.unlink(15);
        assert_eq!(st.parent_of(15), -1);
        assert_eq!(st.display_pos(15), (10.0, 5.0));
    }

    #[test]
    fn nested_links_chain_coordinates() {
        let mut st = SpriteState::new();
        for i in [0usize, 1, 2] {
            st.set_direct(i, 0, 0, 16, 16, 1);
            st.sprites[i].x = 10.0;
            st.sprites[i].y = 1.0;
        }
        st.link(1, 0);
        st.link(2, 1);
        // 2 -> 1 -> 0: 10 + 10 + 10 = 30.
        assert_eq!(st.display_pos(2), (30.0, 3.0));
    }

    #[test]
    fn anim_hold_then_interpolate_xy() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 16, 16, 1);
        // Hold (200,100) for 2 frames, then interpolate to (50,20) over 4 frames.
        let data = [2.0, 200.0, 100.0, -4.0, 50.0, 20.0];
        st.set_anim(0, 0, false, &data, 1).unwrap();
        // Hold keyframe: position is the target for its whole duration.
        st.tick(1);
        assert_eq!((st.sprites[0].x, st.sprites[0].y), (200.0, 100.0));
        st.tick(1); // still holding (2-frame hold)
        assert_eq!((st.sprites[0].x, st.sprites[0].y), (200.0, 100.0));
        // Interpolation begins from (200,100) toward (50,20) over 4 frames.
        st.tick(2); // 2/4 of the way
        assert_eq!(st.sprites[0].x, 200.0 + (50.0 - 200.0) * 0.5);
        assert_eq!(st.sprites[0].y, 100.0 + (20.0 - 100.0) * 0.5);
        st.tick(2); // reaches the target
        assert_eq!((st.sprites[0].x, st.sprites[0].y), (50.0, 20.0));
    }

    #[test]
    fn anim_relative_adds_base() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 16, 16, 1);
        st.sprites[0].x = 30.0;
        st.sprites[0].y = 7.0;
        // Relative XY: hold (+5,+2) for 1 frame.
        st.set_anim(0, 0, true, &[1.0, 5.0, 2.0], 1).unwrap();
        st.tick(1);
        assert_eq!((st.sprites[0].x, st.sprites[0].y), (35.0, 9.0));
    }

    #[test]
    fn anim_loop_count_then_stops() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 16, 16, 1);
        // Single 1-frame hold of Z=5, looped twice.
        st.set_anim(0, 1, false, &[1.0, 5.0], 2).unwrap();
        st.tick(2); // two loops consumed
        assert!(st.sprites[0].anims[1].as_ref().unwrap().done);
        assert_eq!(st.sprites[0].z, 5.0);
    }

    #[test]
    fn anim_stop_pauses_advance() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 16, 16, 1);
        st.set_anim(0, 1, false, &[10.0, 9.0], 1).unwrap();
        st.set_anim_stopped(0, true);
        st.tick(5);
        assert_eq!(st.sprites[0].anims[1].as_ref().unwrap().frame, 0);
        st.set_anim_stopped(0, false);
        st.tick(1);
        assert_eq!(st.sprites[0].anims[1].as_ref().unwrap().frame, 1);
    }

    /// Place a default 16×16 collision-enabled sprite at `(x,y)`.
    fn place_col(st: &mut SpriteState, mgmt: usize, x: f64, y: f64) {
        st.set_direct(mgmt, 0, 0, 16, 16, 1);
        st.sprites[mgmt].x = x;
        st.sprites[mgmt].y = y;
        st.set_col(mgmt, None, false, 0xFFFF_FFFF);
    }

    #[test]
    fn col_default_rect_is_full_sprite() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 24, 32, 1);
        st.set_col(0, None, false, 0xFFFF_FFFF);
        let s = &st.sprites[0];
        assert!(s.col_enabled);
        assert_eq!((s.col_sx, s.col_sy, s.col_w, s.col_h), (0, 0, 24, 32));
    }

    #[test]
    fn hit_sp_overlap_and_separation() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 100.0, 100.0);
        place_col(&mut st, 1, 100.0, 100.0);
        // Overlapping: vs-all finds sprite 1, the pair test is true.
        assert_eq!(st.hit_sp_range(0, 0, SPRITE_COUNT - 1), 1);
        assert!(st.hit_sp_pair(0, 1));
        // Move sprite 1 fully clear (16px sprites, 200px apart): no collision.
        st.sprites[1].x = 200.0;
        st.sprites[1].y = 200.0;
        assert_eq!(st.hit_sp_range(0, 0, SPRITE_COUNT - 1), -1);
        assert!(!st.hit_sp_pair(0, 1));
    }

    #[test]
    fn hit_sp_edge_touch_does_not_collide() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 0.0, 0.0);
        // Exactly adjacent (sprite 1 starts where sprite 0 ends): touching edges ≠ overlap.
        place_col(&mut st, 1, 16.0, 0.0);
        assert!(!st.hit_sp_pair(0, 1));
        st.sprites[1].x = 15.0;
        assert!(st.hit_sp_pair(0, 1));
    }

    #[test]
    fn hit_sp_skips_disabled_and_self() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 10.0, 10.0);
        // An overlapping sprite that never had SPCOL does not collide.
        st.set_direct(2, 0, 0, 16, 16, 1);
        st.sprites[2].x = 10.0;
        st.sprites[2].y = 10.0;
        assert_eq!(st.hit_sp_range(0, 0, SPRITE_COUNT - 1), -1);
        // A disabled test sprite never collides.
        st.set_direct(3, 0, 0, 16, 16, 1);
        st.sprites[3].x = 10.0;
        st.sprites[3].y = 10.0;
        assert_eq!(st.hit_sp_range(3, 0, SPRITE_COUNT - 1), -1);
    }

    #[test]
    fn hit_sp_mask_filters() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 0.0, 0.0);
        place_col(&mut st, 1, 0.0, 0.0);
        // Disjoint masks (0b01 vs 0b10): AND == 0 → no collision despite overlap.
        st.sprites[0].col_mask = 0b01;
        st.sprites[1].col_mask = 0b10;
        assert!(!st.hit_sp_pair(0, 1));
        // Overlapping bit set → collision.
        st.sprites[1].col_mask = 0b11;
        assert!(st.hit_sp_pair(0, 1));
    }

    #[test]
    fn hit_sp_range_returns_lowest_in_range() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 0.0, 0.0);
        place_col(&mut st, 5, 0.0, 0.0);
        place_col(&mut st, 8, 0.0, 0.0);
        // Restricting the range skips sprite 5, finds sprite 8.
        assert_eq!(st.hit_sp_range(0, 6, 10), 8);
    }

    #[test]
    fn hit_rc_against_sprites() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 8.0, 8.0);
        // A 16×16 quad at the origin overlaps the sprite at (8,8).
        assert_eq!(
            st.hit_rc_range(
                (0.0, 0.0, 16.0, 16.0),
                0xFFFF_FFFF,
                (0.0, 0.0),
                0,
                SPRITE_COUNT - 1
            ),
            0
        );
        assert!(st.hit_rc_one((0.0, 0.0, 16.0, 16.0), 0xFFFF_FFFF, (0.0, 0.0), 0));
        // A far-away quad misses.
        assert_eq!(
            st.hit_rc_range(
                (100.0, 100.0, 4.0, 4.0),
                0xFFFF_FFFF,
                (0.0, 0.0),
                0,
                SPRITE_COUNT - 1
            ),
            -1
        );
    }

    #[test]
    fn hit_records_info_for_sphitinfo() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 50.0, 60.0);
        place_col(&mut st, 1, 55.0, 65.0);
        st.set_colvec(0, Some((2.0, -1.0)));
        st.set_colvec(1, Some((0.0, 3.0)));
        assert!(st.hit_sp_pair(0, 1));
        assert_eq!(st.hit.time, 0.0);
        assert_eq!((st.hit.x1, st.hit.y1), (50.0, 60.0));
        assert_eq!((st.hit.vx1, st.hit.vy1), (2.0, -1.0));
        assert_eq!((st.hit.x2, st.hit.y2), (55.0, 65.0));
        assert_eq!((st.hit.vx2, st.hit.vy2), (0.0, 3.0));
    }

    #[test]
    fn col_aabb_inherits_link_position() {
        let mut st = SpriteState::new();
        place_col(&mut st, 0, 100.0, 50.0);
        place_col(&mut st, 1, 10.0, 5.0);
        st.link(1, 0);
        // Sprite 1's detection box is offset by its parent's position.
        assert_eq!(st.col_aabb(1), Some((110.0, 55.0, 16.0, 16.0)));
    }

    #[test]
    fn anim_status_bits() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 16, 16, 1);
        assert_eq!(st.anim_status(0), 0);
        // Z (channel 1) + R (channel 4) running → bits 0b10010 = 18.
        st.set_anim(0, 1, false, &[10.0, 5.0], 0).unwrap();
        st.set_anim(0, 4, false, &[10.0, 90.0], 0).unwrap();
        assert_eq!(st.anim_status(0), (1 << 1) | (1 << 4));
        // Stopped → 0.
        st.set_anim_stopped(0, true);
        assert_eq!(st.anim_status(0), 0);
    }

    #[test]
    fn spdef_define_read_reset_copy() {
        let mut st = SpriteState::new();
        // Default template is 16×16, attr display-ON.
        let d = st.spdef_get(0);
        assert_eq!((d.w, d.h, d.attr), (16, 16, 1));
        // Define template 1, then SPSET copies its rect/home/attr.
        st.spdef_set(
            1,
            SpdefEntry {
                u: 32,
                v: 48,
                w: 24,
                h: 24,
                origin_x: 12,
                origin_y: 12,
                attr: 1,
            },
        );
        st.set_template(0, 1);
        let sp = &st.sprites[0];
        assert_eq!((sp.u, sp.v, sp.w, sp.h), (32, 48, 24, 24));
        assert_eq!((sp.home_x, sp.home_y), (12.0, 12.0));
        // Reset restores defaults.
        st.spdef_reset();
        assert_eq!(st.spdef_get(1), SpdefEntry::default());
    }

    #[test]
    fn anim_errors() {
        let mut st = SpriteState::new();
        st.set_direct(0, 0, 0, 16, 16, 1);
        // XY needs 3 values per keyframe; 2 values is fewer than one keyframe.
        assert_eq!(
            st.set_anim(0, 0, false, &[1.0, 2.0], 1),
            Err(AnimError::TooFew)
        );
        // > 32 keyframes (Z = 2 values each → 33 keyframes = 66 values; times are nonzero).
        let too_many: Vec<f64> = (0..33).flat_map(|_| [1.0, 0.0]).collect();
        assert_eq!(
            st.set_anim(0, 1, false, &too_many, 1),
            Err(AnimError::TooLong)
        );
        // Item out of the ±32768 range.
        assert_eq!(
            st.set_anim(0, 1, false, &[1.0, 40000.0], 1),
            Err(AnimError::OutOfRange)
        );
        // A zero-duration keyframe is illegal animation data.
        assert_eq!(
            st.set_anim(0, 0, false, &[0.0, 100.0, 50.0], 1),
            Err(AnimError::ZeroTime)
        );
    }
}
