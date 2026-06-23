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
/// Highest `SPDEF` template number accepted by `SPSET` form 1 (`&H0FFF`).
pub const SPDEF_MAX: i32 = 4095;
/// Default sheet page a sprite samples from (`SPPAGE` default = GRP4).
pub const SPRITE_PAGE_DEFAULT: u8 = 4;
/// Default `SPSET` source-rectangle size when `W,H` are omitted (16×16).
pub const SPRITE_DEFAULT_WH: i32 = 16;
/// Default `SPSET` attribute when `attr` is omitted: display ON only (`#SPSHOW` &H01).
pub const SPRITE_DEFAULT_ATTR: i32 = 0x01;

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
}
