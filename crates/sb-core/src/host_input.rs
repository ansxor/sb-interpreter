//! Host-side input accumulation (M4-T5) — the device-neutral half of the platform input
//! layer. The platform receives raw key-down / key-up events from the OS (winit
//! [`KeyCode`](https://docs.rs/winit) on native, DOM `event.code` strings on the web) and
//! turns them into the per-frame `(held mask, stick, stickex)` triple that
//! [`InputState::advance_frame`](crate::input::InputState::advance_frame) consumes.
//!
//! The *physical-key → [`Bind`]* table is the per-platform **default keymap** (each
//! `sb-platform-*` crate owns its own, since the key types differ), but the accumulation —
//! OR-ing button bits, tracking opposing axis directions, collapsing a digital key into a
//! full ±1.0 stick deflection — is identical everywhere, so it lives here once, pure and
//! `wasm32`-safe, and is exercised by the deterministic gate.
//!
//! A keyboard has no analog travel, so a key bound to a stick axis produces a full ±1.0
//! push; holding both the +X and −X keys cancels to 0.0 (the platform may still feed real
//! analog values straight to `advance_frame` for an actual Circle Pad / gamepad stick).

use crate::input::{BTN_A, BTN_B, BTN_DOWN, BTN_L, BTN_LEFT, BTN_R, BTN_RIGHT, BTN_UP};

/// Which Circle Pad a directional binding drives: the left pad (`STICK`) or the right
/// Circle Pad Pro (`STICKEX`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stick {
    /// Left Circle Pad — read by `STICK`.
    Left,
    /// Right Circle Pad Pro — read by `STICKEX`.
    Right,
}

/// One logical action a physical key is bound to in a default keymap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bind {
    /// A digital button: an OR of `BTN_*` bit weights (`crate::input`).
    Button(u16),
    /// A stick X-axis push — `dir` is `+1.0` (right) or `-1.0` (left).
    AxisX(Stick, f64),
    /// A stick Y-axis push — `dir` is `+1.0` (up) or `-1.0` (down).
    AxisY(Stick, f64),
}

/// Accumulates discrete key-up / key-down events into the per-frame hardware snapshot.
///
/// The platform calls [`apply`](Self::apply) on every key event, then reads
/// [`held`](Self::held) / [`stick`](Self::stick) / [`stickex`](Self::stickex) once per
/// frame and forwards them to `InputState::advance_frame`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostInput {
    held: u16,
    /// Left-stick (held-positive, held-negative) flags for the X and Y axes.
    lx: (bool, bool),
    ly: (bool, bool),
    /// Right-stick (held-positive, held-negative) flags for the X and Y axes.
    rx: (bool, bool),
    ry: (bool, bool),
}

impl HostInput {
    /// A fully-released starting state (every button up, both sticks centred).
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one key event: `pressed` is `true` on key-down, `false` on key-up. An unbound
    /// key (`bind == None`) is ignored. Re-applying the same press (key auto-repeat) is
    /// idempotent — the bit / flag is simply re-set.
    pub fn apply(&mut self, bind: Option<Bind>, pressed: bool) {
        match bind {
            Some(Bind::Button(mask)) => {
                if pressed {
                    self.held |= mask;
                } else {
                    self.held &= !mask;
                }
            }
            Some(Bind::AxisX(Stick::Left, dir)) => set_dir(&mut self.lx, dir, pressed),
            Some(Bind::AxisY(Stick::Left, dir)) => set_dir(&mut self.ly, dir, pressed),
            Some(Bind::AxisX(Stick::Right, dir)) => set_dir(&mut self.rx, dir, pressed),
            Some(Bind::AxisY(Stick::Right, dir)) => set_dir(&mut self.ry, dir, pressed),
            None => {}
        }
    }

    /// The currently-held button mask (feature-0 input to `advance_frame`).
    pub fn held(&self) -> u16 {
        self.held
    }

    /// The left Circle Pad deflection `(x, y)` in -1.0..1.0 (`STICK`).
    pub fn stick(&self) -> (f64, f64) {
        (axis(self.lx), axis(self.ly))
    }

    /// The right Circle Pad Pro deflection `(x, y)` in -1.0..1.0 (`STICKEX`).
    pub fn stickex(&self) -> (f64, f64) {
        (axis(self.rx), axis(self.ry))
    }
}

/// Set the positive or negative direction flag of an axis from a key event.
fn set_dir(flags: &mut (bool, bool), dir: f64, pressed: bool) {
    if dir > 0.0 {
        flags.0 = pressed;
    } else {
        flags.1 = pressed;
    }
}

/// Collapse a `(positive_held, negative_held)` flag pair into a -1.0 / 0.0 / +1.0 axis
/// value (both held cancels to 0.0).
fn axis(flags: (bool, bool)) -> f64 {
    f64::from(i32::from(flags.0) - i32::from(flags.1))
}

/// The shared D-pad / face-button column of the default keymap: the eight bindings every
/// platform agrees on so a corpus program's `BUTTON` reads behave the same native and web.
/// (The physical keys that map onto these — arrows, WASD, etc. — are chosen per platform.)
pub mod default_dpad {
    use super::*;

    /// `#UP` button.
    pub const UP: Bind = Bind::Button(BTN_UP);
    /// `#DOWN` button.
    pub const DOWN: Bind = Bind::Button(BTN_DOWN);
    /// `#LEFT` button.
    pub const LEFT: Bind = Bind::Button(BTN_LEFT);
    /// `#RIGHT` button.
    pub const RIGHT: Bind = Bind::Button(BTN_RIGHT);
    /// `#A` button.
    pub const A: Bind = Bind::Button(BTN_A);
    /// `#B` button.
    pub const B: Bind = Bind::Button(BTN_B);
    /// `#L` shoulder.
    pub const L: Bind = Bind::Button(BTN_L);
    /// `#R` shoulder.
    pub const R: Bind = Bind::Button(BTN_R);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{BTN_A, BTN_LEFT, BTN_RIGHT};

    #[test]
    fn button_press_and_release_toggles_the_bit() {
        let mut h = HostInput::new();
        assert_eq!(h.held(), 0);
        h.apply(Some(Bind::Button(BTN_A)), true);
        assert_eq!(h.held(), BTN_A);
        // A second key-down (OS auto-repeat) is idempotent.
        h.apply(Some(Bind::Button(BTN_A)), true);
        assert_eq!(h.held(), BTN_A);
        h.apply(Some(Bind::Button(BTN_A)), false);
        assert_eq!(h.held(), 0);
    }

    #[test]
    fn distinct_buttons_or_together() {
        let mut h = HostInput::new();
        h.apply(Some(Bind::Button(BTN_A)), true);
        h.apply(Some(Bind::Button(BTN_LEFT)), true);
        assert_eq!(h.held(), BTN_A | BTN_LEFT);
        // Releasing one leaves the other held.
        h.apply(Some(Bind::Button(BTN_A)), false);
        assert_eq!(h.held(), BTN_LEFT);
    }

    #[test]
    fn digital_keys_give_full_stick_deflection() {
        let mut h = HostInput::new();
        h.apply(Some(Bind::AxisX(Stick::Left, 1.0)), true);
        h.apply(Some(Bind::AxisY(Stick::Left, -1.0)), true);
        assert_eq!(h.stick(), (1.0, -1.0));
        // Right stick untouched.
        assert_eq!(h.stickex(), (0.0, 0.0));
    }

    #[test]
    fn opposing_axis_keys_cancel() {
        let mut h = HostInput::new();
        h.apply(Some(Bind::AxisX(Stick::Left, 1.0)), true);
        h.apply(Some(Bind::AxisX(Stick::Left, -1.0)), true);
        assert_eq!(h.stick(), (0.0, 0.0));
        // Release the +X key: only −X remains.
        h.apply(Some(Bind::AxisX(Stick::Left, 1.0)), false);
        assert_eq!(h.stick(), (-1.0, 0.0));
    }

    #[test]
    fn left_and_right_sticks_are_independent() {
        let mut h = HostInput::new();
        h.apply(Some(Bind::AxisX(Stick::Left, -1.0)), true);
        h.apply(Some(Bind::AxisY(Stick::Right, 1.0)), true);
        assert_eq!(h.stick(), (-1.0, 0.0));
        assert_eq!(h.stickex(), (0.0, 1.0));
    }

    #[test]
    fn unbound_key_is_ignored() {
        let mut h = HostInput::new();
        h.apply(None, true);
        assert_eq!(h.held(), 0);
        assert_eq!(h.stick(), (0.0, 0.0));
    }

    #[test]
    fn shared_dpad_bindings_match_button_weights() {
        assert_eq!(default_dpad::LEFT, Bind::Button(BTN_LEFT));
        assert_eq!(default_dpad::RIGHT, Bind::Button(BTN_RIGHT));
    }
}
