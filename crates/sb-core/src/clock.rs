//! Frame clock (M4-T3) — the single 60 fps frame counter behind `MAINCNT`, `VSYNC`, and
//! `WAIT`.
//!
//! SmileBASIC runs at a fixed **60 fps**; almost every duration in the language (VSYNC/WAIT
//! counts, `BREPEAT` timing, `FADE` time, animation keyframes) is a **frame count** of
//! 1/60th of a second. The whole language hangs off one free-running counter — the value at
//! pointer `[0x315ec0]` in the 3.6.0 binary — which is exactly what `MAINCNT` returns, what
//! `WAIT` counts forward from, and what `VSYNC` compares against. `disasm.py xref 0x315ec0`
//! finds *only* the WAIT handler, the VSYNC handler, and the MAINCNT getter, so there is one
//! clock, not three (see `spec/concepts/frame-and-timing-model.md`).
//!
//! This module owns that counter as [`FrameClock`]. It is pure state with no real-time
//! dependency, so it stays wasm-safe and deterministic: in the headless runner there is no
//! VBlank, and the clock only advances when something explicitly drives it —
//! [`FrameClock::tick`] (the platform's per-frame heartbeat) or [`FrameClock::wait`] /
//! [`FrameClock::vsync`] (a program blocking on frames). The real-time pacing that turns
//! [`FRAME_DURATION`] into wall-clock 60 fps lives in the native platform crate.

use core::time::Duration;

/// Frames per second. The 3DS displays at a fixed 60 Hz and SmileBASIC ties every timed
/// behavior to that rate; there is no sub-frame timer in the language.
pub const FPS: u32 = 60;

/// Wall-clock duration of one displayed frame (1/60 s), for the native host's frame pacer.
/// `Duration` is from `core::time`, so this constant is available on `wasm32` too even
/// though `sb-core` never reads a real clock itself.
pub const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / FPS as u64);

/// The single global frame counter plus the per-program "last VSYNC" anchor.
///
/// `frame` is the free-running counter (`[0x315ec0]`); `last_vsync` is the per-program frame
/// stamp (`[0x315ee8]`) updated by both VSYNC and WAIT on exit — the *only* state that
/// distinguishes the two instructions. Kept as `u64` so the wait arithmetic never overflows;
/// [`maincnt`](Self::maincnt) truncates to `i32`, modelling the hardware's 32-bit wrap.
#[derive(Debug, Clone)]
pub struct FrameClock {
    frame: u64,
    last_vsync: u64,
}

impl FrameClock {
    /// A fresh clock at frame 0. (On real hardware `MAINCNT` counts from when SmileBASIC was
    /// *launched*, not from `RUN`; in the headless model the VM's clock starts at 0 and is
    /// never reset by program control flow — see the timing-model concept's open question.)
    pub fn new() -> Self {
        FrameClock {
            frame: 0,
            last_vsync: 0,
        }
    }

    /// The raw frame counter as `MAINCNT` reports it. `MAINCNT` is an Integer (`i32`); the
    /// `u64` counter is truncated, so it wraps through negative at `0x7FFFFFFF` exactly as
    /// the hardware's 32-bit value does (≈414 days of uptime — not reachable in practice).
    pub fn maincnt(&self) -> i32 {
        self.frame as i32
    }

    /// Advance the global counter by `frames` displayed frames — the platform's per-VBlank
    /// heartbeat. Does **not** touch `last_vsync` (only VSYNC/WAIT resync that anchor), so a
    /// later `VSYNC` measures from the previous VSYNC across these ticks. Used by the native
    /// 60 fps loop and by the per-frame background machinery; a no-op when `frames == 0`.
    pub fn tick(&mut self, frames: u64) {
        self.frame = self.frame.saturating_add(frames);
    }

    /// `WAIT count` — block until the counter reaches **current + count**, anchored at the
    /// instant WAIT runs (`add r5, current, count` off `[0x315ec0]` @0x14b020). `count <= 0`
    /// resolves to 0 here and does not wait ("0: Ignore"); on exit `last_vsync` is set to the
    /// current frame (@0x14b078) so a following `VSYNC` measures from the end of the WAIT.
    /// Returns the number of frames actually elapsed.
    pub fn wait(&mut self, count: u64) -> u64 {
        let before = self.frame;
        self.frame = self.frame.saturating_add(count);
        self.last_vsync = self.frame;
        self.frame - before
    }

    /// `VSYNC count` — block until the counter reaches **last_vsync + count**, anchored at the
    /// *previous* VSYNC (`add r5, last_vsync, count` @0x14563c), then resync `last_vsync` to
    /// the current frame (@0x145690). Because the target is measured from the last VSYNC, a
    /// `VSYNC 1` loop holds a steady 60 fps and absorbs body jitter; if the body already
    /// overran the target the wait returns immediately (0 elapsed) and `last_vsync` catches up
    /// to the current frame. `count <= 0` (resolved to 0) skips the wait but still resyncs.
    /// Returns the number of frames actually elapsed.
    pub fn vsync(&mut self, count: u64) -> u64 {
        let before = self.frame;
        let target = self.last_vsync.saturating_add(count);
        self.frame = self.frame.max(target);
        self.last_vsync = self.frame;
        self.frame - before
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let clk = FrameClock::new();
        assert_eq!(clk.maincnt(), 0);
    }

    #[test]
    fn wait_advances_maincnt_from_now() {
        let mut clk = FrameClock::new();
        assert_eq!(clk.wait(60), 60);
        assert_eq!(clk.maincnt(), 60);
        // A second WAIT keeps counting from the (new) current frame.
        assert_eq!(clk.wait(1), 1);
        assert_eq!(clk.maincnt(), 61);
    }

    #[test]
    fn vsync_loop_advances_one_frame_each() {
        let mut clk = FrameClock::new();
        for expected in 1..=5 {
            assert_eq!(clk.vsync(1), 1);
            assert_eq!(clk.maincnt(), expected);
        }
    }

    #[test]
    fn tick_advances_the_counter_without_moving_the_vsync_anchor() {
        let mut clk = FrameClock::new();
        clk.tick(5);
        assert_eq!(clk.maincnt(), 5);
        // `tick` left `last_vsync` at 0, so a `VSYNC 1` (target = 0 + 1 = 1) is already past:
        // it returns immediately, 0 frames elapsed, and catches `last_vsync` up to current.
        assert_eq!(clk.vsync(1), 0);
        assert_eq!(clk.maincnt(), 5);
    }

    #[test]
    fn wait_counts_from_now_even_after_a_tick() {
        // The VSYNC/WAIT contrast: after the counter has advanced underneath us (jitter),
        // WAIT still adds its full count from the present instant, unlike the VSYNC above.
        let mut clk = FrameClock::new();
        clk.tick(5);
        assert_eq!(clk.wait(1), 1);
        assert_eq!(clk.maincnt(), 6);
    }

    #[test]
    fn zero_count_does_not_wait_but_resyncs_the_anchor() {
        let mut clk = FrameClock::new();
        clk.tick(10);
        // VSYNC 0 / WAIT 0 ("0: Ignore"): no advance, but `last_vsync` resyncs to current,
        // so the next `VSYNC 1` (target = 10 + 1) genuinely advances by one.
        assert_eq!(clk.vsync(0), 0);
        assert_eq!(clk.maincnt(), 10);
        assert_eq!(clk.vsync(1), 1);
        assert_eq!(clk.maincnt(), 11);
    }
}
