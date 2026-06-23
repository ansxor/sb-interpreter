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
/// Maximum keyframes accepted per animation target (`cmp r0,#0x20` @0x163a48 →
/// errnum 39 "Animation is too long" past 32).
pub const ANIM_MAX_KEYFRAMES: usize = 32;
/// Highest `SPDEF` template number accepted by `SPSET` form 1 (`&H0FFF`).
pub const SPDEF_MAX: i32 = 4095;
/// Default sheet page a sprite samples from (`SPPAGE` default = GRP4).
pub const SPRITE_PAGE_DEFAULT: u8 = 4;
/// Default `SPSET` source-rectangle size when `W,H` are omitted (16×16).
pub const SPRITE_DEFAULT_WH: i32 = 16;
/// Default `SPSET` attribute when `attr` is omitted: display ON only (`#SPSHOW` &H01).
pub const SPRITE_DEFAULT_ATTR: i32 = 0x01;

/// Why an `SPANIM` keyframe list was rejected. The lifecycle/argument errnums (4/8/10)
/// are decided by the builtin; these are the data-build errnums raised by the keyframe
/// helpers (`FUN_001ee360`/`0x163a00`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimError {
    /// Fewer than one keyframe's worth of data for the channel — errnum 4 ("Illegal
    /// function call", `cmp r6,#0x3 / bge` @0x163a10 / `mov r0,#0x4` @0x163a34).
    TooFew,
    /// More than 32 keyframes — errnum 39 ("Animation is too long", `cmp r4,#0x20` /
    /// `mov r0,#0x27` @0x163a68 / @0x163d54).
    TooLong,
    /// A keyframe time/item is outside the ±32768 fixed-point range — errnum 10 ("Out of
    /// range", `mov r0,#0xa` @0x163960; the items are `sxth`-truncated to 16 bits).
    OutOfRange,
    /// A keyframe has a zero duration — errnum 40 ("Illegal animation data", `cmp r1,#0 /
    /// beq → mov r0,#0x28` @0x1639e4).
    ZeroTime,
}

/// One `SPANIM` keyframe: a `time` (frames) and 1-2 `items`. A positive time HOLDS the
/// item value for that many frames; a negative time LINEARLY INTERPOLATES toward it over
/// `|time|` frames (the smooth form).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimKeyframe {
    /// Per-keyframe duration in frames; sign selects hold (≥0) vs interpolate (<0).
    pub time: i32,
    /// 1 or 2 target values (per the channel's [`ANIM_ITEMS`]).
    pub items: Vec<f64>,
}

/// A running per-channel animation set up by `SPANIM`. Drives one target channel of one
/// sprite across frames; [`SpriteState::tick`] advances it and writes the value back into
/// the slot. Exact interpolation rounding is oracle-pending (no framebuffer harvest yet —
/// see `HARVEST_QUEUE.md`); the structural advancement (hold/interpolate/loop/relative) is
/// deterministic and unit-tested.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteAnim {
    /// Target channel 0..7 (`target & 7`).
    pub channel: usize,
    /// Relative flag (`+8` / trailing `"+"`): items are offsets from `base`.
    pub relative: bool,
    /// The sprite's channel value captured at `SPANIM` time (the relative base / the start
    /// of the first interpolation segment).
    pub base: Vec<f64>,
    /// The keyframe sequence (1..=32).
    pub keyframes: Vec<AnimKeyframe>,
    /// Loop count: run the sequence this many times, or endlessly when 0.
    pub loop_count: i32,
    /// Current keyframe index.
    pub kf: usize,
    /// Frames already applied within the current keyframe.
    pub frame: i32,
    /// Value at the start of the current segment (the interpolation source).
    pub seg_start: Vec<f64>,
    /// The current applied channel value.
    pub cur: Vec<f64>,
    /// Completed loops.
    pub loops_done: i32,
    /// Whether the animation has finished (a non-endless loop ran out).
    pub done: bool,
}

impl SpriteAnim {
    /// Absolute target value of keyframe `i` (adding `base` when relative).
    fn target(&self, i: usize) -> Vec<f64> {
        let items = &self.keyframes[i].items;
        if self.relative {
            items
                .iter()
                .enumerate()
                .map(|(k, v)| self.base.get(k).copied().unwrap_or(0.0) + v)
                .collect()
        } else {
            items.clone()
        }
    }

    /// Advance one frame, updating `cur` (and the keyframe/loop state).
    fn step(&mut self) {
        if self.done || self.keyframes.is_empty() {
            return;
        }
        let i = self.kf;
        let kf_time = self.keyframes[i].time;
        let dur = kf_time.abs();
        let target = self.target(i);
        self.frame += 1;
        if self.frame >= dur {
            // Segment complete: snap to the keyframe target and advance.
            self.cur = target;
            self.advance_keyframe();
        } else if kf_time < 0 {
            // Mid-interpolation: linear from the segment start toward the target.
            let t = self.frame as f64 / dur as f64;
            self.cur = self
                .seg_start
                .iter()
                .zip(target.iter())
                .map(|(s, e)| s + (e - s) * t)
                .collect();
        } else {
            // Hold: the value is the keyframe target for the whole segment.
            self.cur = target;
        }
    }

    /// Move to the next keyframe, wrapping + counting loops at the end of the sequence.
    fn advance_keyframe(&mut self) {
        self.frame = 0;
        self.seg_start = self.cur.clone();
        self.kf += 1;
        if self.kf >= self.keyframes.len() {
            self.kf = 0;
            if self.loop_count != 0 {
                self.loops_done += 1;
                if self.loops_done >= self.loop_count {
                    self.done = true;
                }
            }
        }
    }
}

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
    pub home_x: i32,
    pub home_y: i32,
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
            home_x: 0,
            home_y: 0,
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
    /// the source rectangle `(U,V,W,H)`; `defno` is the `SPDEF` template number, or -1 for
    /// the direct-image forms.
    fn create(&mut self, mgmt: usize, rect: (i32, i32, i32, i32), attr: i32, defno: i32) {
        let (u, v, w, h) = rect;
        let mut sp = Sprite {
            active: true,
            defno,
            u,
            v,
            w,
            h,
            ..Sprite::default()
        };
        sp.set_attr(attr);
        self.sprites[mgmt] = sp;
    }

    /// `SPSET mgmt, U,V,W,H, attr` — create a directly-imaged sprite at an explicit slot.
    pub fn set_direct(&mut self, mgmt: usize, u: i32, v: i32, w: i32, h: i32, attr: i32) {
        self.create(mgmt, (u, v, w, h), attr, -1);
    }

    /// `SPSET mgmt, defn` — create a sprite at an explicit slot from an `SPDEF` template.
    /// The template's `U,V,W,H` are resolved once `SPDEF` lands (M3-T3); for now the slot
    /// records `defno` and uses the default 16×16 rectangle.
    pub fn set_template(&mut self, mgmt: usize, defno: i32, attr: i32) {
        self.create(
            mgmt,
            (0, 0, SPRITE_DEFAULT_WH, SPRITE_DEFAULT_WH),
            attr,
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
        let stride = 1 + ANIM_ITEMS[channel]; // Time + items
                                              // The handler floors the keyframe count to whole keyframes; fewer than one is
                                              // errnum 4.
        let frames = data.len() / stride;
        if frames == 0 {
            return Err(AnimError::TooFew);
        }
        if frames > ANIM_MAX_KEYFRAMES {
            return Err(AnimError::TooLong);
        }
        let used = &data[..frames * stride];
        // Every time/item is a 16-bit fixed-point value (±32768) in the handler.
        if used.iter().any(|&v| !(-32768.0..32768.0).contains(&v)) {
            return Err(AnimError::OutOfRange);
        }
        let mut keyframes = Vec::with_capacity(frames);
        for chunk in used.chunks_exact(stride) {
            let time = chunk[0] as i32;
            // A zero-duration keyframe is illegal animation data (errnum 40).
            if time == 0 {
                return Err(AnimError::ZeroTime);
            }
            keyframes.push(AnimKeyframe {
                time,
                items: chunk[1..].to_vec(),
            });
        }
        let base = self.sprites[mgmt].read_channel(channel);
        self.sprites[mgmt].anims[channel] = Some(SpriteAnim {
            channel,
            relative,
            base: base.clone(),
            keyframes,
            loop_count,
            kf: 0,
            frame: 0,
            seg_start: base.clone(),
            cur: base,
            loops_done: 0,
            done: false,
        });
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
        st.set_template(0, 0, SPRITE_DEFAULT_ATTR);
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
        st.set_template(0, 0, 1);
        st.set_template(5, 0, 1);
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
