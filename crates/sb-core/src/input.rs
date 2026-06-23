//! Hardware input state (M4-T1) — the per-frame button / Circle-Pad snapshot the
//! [`BUTTON`](crate::builtins)/`STICK`/`STICKEX` builtins read and `BREPEAT` configures.
//!
//! `sb-core` stays device- and I/O-free (it must build for `wasm32`), so this is a pure
//! data holder: the platform layer (M4-T5) fills it each frame from a real keyboard /
//! gamepad, and deterministic tests drive it through a **scripted input timeline** via
//! [`InputState::advance_frame`]. The builtins only ever *read* the precomputed masks /
//! axes — exactly the disassembled `BUTTON` handler, which reads precomputed fields
//! `[r1,#0x78]` (held) / `[r1,#0x80]` (pressed) / `[r1,#0x7c]` (released) rather than
//! re-sampling the hardware (see `spec/instructions/button.yaml`).
//!
//! ## Button bit layout
//!
//! A button mask is 13 meaningful bits; bit 10 is unused (`spec/reference/constants.yaml`,
//! hw_verified): b00 `#UP`(1) b01 `#DOWN`(2) b02 `#LEFT`(4) b03 `#RIGHT`(8) b04 `#A`(16)
//! b05 `#B`(32) b06 `#X`(64) b07 `#Y`(128) b08 `#L`(256) b09 `#R`(512) b10 unused b11
//! `#ZR`(2048) b12 `#ZL`(4096). The same bit positions are used for `BUTTON`'s four
//! feature IDs (held / pressed-with-repeat / pressed-raw / released).
//!
//! ## Key-repeat ([[brepeat]])
//!
//! `BUTTON` feature ID 1 (moment-pressed-with-repeat) is the raw press edge plus, for any
//! button configured via `BREPEAT start,interval`, a periodic re-fire while held: after
//! the button has been held `start` frames the press re-fires, then every `interval`
//! frames (`interval == 0` disables repeat, so feature 1 == feature 2 for that button).
//! `BREPEAT`'s **management ID is the bit index** (0=up … 9=R, 11=ZR, 12=ZL; 10 unused),
//! NOT the bit weight. The exact default timing + whether SB pre-seeds a non-zero repeat
//! are not deterministically harvestable (no input injection in the oracle); repeat is
//! modelled OFF until `BREPEAT` sets it and queued in `HARVEST_QUEUE.md`.

/// The number of button bits SmileBASIC tracks (b00..b12). Bit 10 is unused.
pub const BUTTON_BITS: usize = 13;

/// Bit index of the single unused button bit (b10).
pub const UNUSED_BIT: u32 = 10;

/// Per-button key-repeat configuration (`BREPEAT start,interval`, both in 1/60 s frames).
/// `interval == 0` disables repeat (the boot default and the 1-arg `BREPEAT id` form).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RepeatCfg {
    start: u32,
    interval: u32,
}

/// The current-frame hardware input snapshot: the four `BUTTON` feature masks, the two
/// analog sticks, and the per-button repeat configuration + hold counters that drive the
/// pressed-with-repeat mask.
#[derive(Debug, Clone)]
pub struct InputState {
    /// Buttons held down this frame (feature 0).
    held: u16,
    /// Buttons held the previous frame (for edge detection).
    prev_held: u16,
    /// Raw press edge this frame: held & !prev_held (feature 2).
    pressed: u16,
    /// Press edge plus key-repeat re-fires this frame (feature 1).
    pressed_repeat: u16,
    /// Release edge this frame: !held & prev_held (feature 3).
    released: u16,
    /// Consecutive frames each button has been held (0 on the press frame, then 1,2,…).
    hold_frames: [u32; BUTTON_BITS],
    /// Key-repeat timing per button (indexed by bit / `BREPEAT` management ID).
    repeat: [RepeatCfg; BUTTON_BITS],
    /// Circle Pad axes, already scaled + clamped to -1.0..1.0 (x right+, y up+).
    stick: (f64, f64),
    /// Circle Pad Pro (right stick) axes, same convention.
    stickex: (f64, f64),
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    /// A centred, all-released input state (the headless / boot default): every feature
    /// mask 0, both sticks (0.0, 0.0), repeat off for all buttons.
    pub fn new() -> Self {
        InputState {
            held: 0,
            prev_held: 0,
            pressed: 0,
            pressed_repeat: 0,
            released: 0,
            hold_frames: [0; BUTTON_BITS],
            repeat: [RepeatCfg::default(); BUTTON_BITS],
            stick: (0.0, 0.0),
            stickex: (0.0, 0.0),
        }
    }

    /// Advance one frame of the input timeline: record the newly-held button mask and the
    /// two stick positions, then recompute the press / pressed-with-repeat / release edge
    /// masks. The platform calls this once per VSYNC; deterministic tests call it to step a
    /// scripted timeline. Stick axes are clamped to the inclusive range -1.0..1.0 (the
    /// disassembled `STICK` clamp); the unused button bit (b10) is masked off.
    pub fn advance_frame(&mut self, held: u16, stick: (f64, f64), stickex: (f64, f64)) {
        let held = held & !(1 << UNUSED_BIT);
        self.prev_held = self.held;
        self.held = held;
        self.pressed = held & !self.prev_held;
        self.released = !held & self.prev_held;

        let mut repeat_fire = self.pressed;
        for bit in 0..BUTTON_BITS {
            let mask = 1u16 << bit;
            if held & mask != 0 {
                if self.prev_held & mask != 0 {
                    self.hold_frames[bit] = self.hold_frames[bit].saturating_add(1);
                } else {
                    // Fresh press: the raw edge (already in `pressed`) starts the timer.
                    self.hold_frames[bit] = 0;
                }
                let cfg = self.repeat[bit];
                let h = self.hold_frames[bit];
                if cfg.interval != 0
                    && h > 0
                    && h >= cfg.start
                    && (h - cfg.start).is_multiple_of(cfg.interval)
                {
                    repeat_fire |= mask;
                }
            } else {
                self.hold_frames[bit] = 0;
            }
        }
        self.pressed_repeat = repeat_fire;
        self.stick = (clamp_axis(stick.0), clamp_axis(stick.1));
        self.stickex = (clamp_axis(stickex.0), clamp_axis(stickex.1));
    }

    /// Set the key-repeat timing for one button (`BREPEAT id,start,interval`). `id` is a
    /// validated bit index in `0..13` (never the unused 10); `interval == 0` turns repeat
    /// off (the `BREPEAT id` 1-arg form). Caller validates the ID + non-negativity.
    pub fn set_repeat(&mut self, id: usize, start: u32, interval: u32) {
        self.repeat[id] = RepeatCfg { start, interval };
    }

    /// The button mask for a `BUTTON` feature ID: 0 held, 1 pressed-with-repeat, 2
    /// pressed-raw (no repeat), 3 released. Returned as `i32` (the SmileBASIC Integer).
    pub fn button(&self, feature: i32) -> Option<i32> {
        let mask = match feature {
            0 => self.held,
            1 => self.pressed_repeat,
            2 => self.pressed,
            3 => self.released,
            _ => return None,
        };
        Some(i32::from(mask))
    }

    /// The Circle Pad axes (x right+, y up+), each a Double in -1.0..1.0.
    pub fn stick(&self) -> (f64, f64) {
        self.stick
    }

    /// The Circle Pad Pro (right stick) axes, same convention as [`stick`](Self::stick).
    pub fn stickex(&self) -> (f64, f64) {
        self.stickex
    }
}

/// Clamp a stick axis to the inclusive range -1.0..1.0 (the disassembled `STICK`
/// `vcmpe`/`vmovge`/`vmovls` clamp). NaN clamps to 0.0.
fn clamp_axis(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Button bit weights (the BUTTON return mask / BREPEAT management ID = bit index).
    const UP: u16 = 1 << 0;
    const A: u16 = 1 << 4;
    const ZR: u16 = 1 << 11;
    const ZL: u16 = 1 << 12;

    #[test]
    fn held_pressed_released_edges() {
        let mut s = InputState::new();
        // Press A: held + raw-pressed set, released clear.
        s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(0), Some(A as i32)); // held
        assert_eq!(s.button(2), Some(A as i32)); // pressed (raw)
        assert_eq!(s.button(3), Some(0)); // released
                                          // Keep holding A: still held, no longer a press edge.
        s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(0), Some(A as i32));
        assert_eq!(s.button(2), Some(0));
        assert_eq!(s.button(3), Some(0));
        // Release A: not held, release edge set.
        s.advance_frame(0, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(0), Some(0));
        assert_eq!(s.button(2), Some(0));
        assert_eq!(s.button(3), Some(A as i32));
    }

    #[test]
    fn feature1_equals_feature2_without_brepeat() {
        // With no BREPEAT, pressed-with-repeat (1) == pressed-raw (2) every frame.
        let mut s = InputState::new();
        for f in 0..5 {
            let held = if f < 3 { A } else { 0 };
            s.advance_frame(held, (0.0, 0.0), (0.0, 0.0));
            assert_eq!(s.button(1), s.button(2), "frame {f}");
        }
    }

    #[test]
    fn brepeat_refires_pressed_with_repeat() {
        // BREPEAT A=4 -> start=15, interval=4: re-fire at hold 15, 19, 23, …
        let mut s = InputState::new();
        s.set_repeat(4, 15, 4);
        // Frame 0: fresh press (hold 0) -> feature 1 fires the raw edge.
        s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(1), Some(A as i32));
        assert_eq!(s.button(2), Some(A as i32));
        // Frames with hold 1..14: no re-fire (feature 1 clears with feature 2).
        for _ in 0..14 {
            s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
            assert_eq!(s.button(1), Some(0));
        }
        // hold == 15: repeat begins.
        s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(1), Some(A as i32));
        assert_eq!(s.button(2), Some(0)); // raw edge stays clear
                                          // hold 16,17,18: quiet. hold 19: next interval.
        for _ in 0..3 {
            s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
            assert_eq!(s.button(1), Some(0));
        }
        s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(1), Some(A as i32));
    }

    #[test]
    fn brepeat_interval_zero_disables() {
        let mut s = InputState::new();
        s.set_repeat(4, 0, 0); // explicit off
        for _ in 0..30 {
            s.advance_frame(A, (0.0, 0.0), (0.0, 0.0));
        }
        // Still held, but no repeat re-fire after the initial press.
        assert_eq!(s.button(1), Some(0));
        assert_eq!(s.button(0), Some(A as i32));
    }

    #[test]
    fn unused_bit_is_masked_off() {
        let mut s = InputState::new();
        s.advance_frame(1 << UNUSED_BIT, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(0), Some(0));
    }

    #[test]
    fn exact_bit_weights() {
        // The named-constant weights must read back exactly (#ZR=2048 b11, #ZL=4096 b12).
        let mut s = InputState::new();
        s.advance_frame(UP | ZR | ZL, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(s.button(0), Some(1 + 2048 + 4096));
    }

    #[test]
    fn invalid_feature_is_none() {
        let s = InputState::new();
        assert_eq!(s.button(4), None);
        assert_eq!(s.button(-1), None);
    }

    #[test]
    fn stick_clamps_to_unit_range() {
        let mut s = InputState::new();
        s.advance_frame(0, (2.0, -3.0), (0.5, f64::NAN));
        assert_eq!(s.stick(), (1.0, -1.0));
        assert_eq!(s.stickex(), (0.5, 0.0));
    }
}
