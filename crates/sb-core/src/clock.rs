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
//!
//! ## Interactive (host-driven VBlank) model
//!
//! The headless `wait`/`vsync` methods jump the counter instantly — correct for the
//! deterministic runner and all tests. Interactive hosts (the wasm `requestAnimationFrame`
//! loop) need the hardware model instead: VSYNC/WAIT *arm* a target via [`begin_wait`] /
//! [`begin_vsync`], then [`tick`] drives the counter one frame at a time and
//! [`resolve_wait`] clears the pending target once it is reached. The program is blocked
//! (the VM yields at each [`tick`]) until the host's VBlanks accumulate enough frames.
//!
//! [`begin_wait`]: FrameClock::begin_wait
//! [`begin_vsync`]: FrameClock::begin_vsync
//! [`resolve_wait`]: FrameClock::resolve_wait
//! [`tick`]: FrameClock::tick

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
///
/// `wait_target` is non-`None` only in the interactive host-driven VBlank model: it holds the
/// frame number the pending VSYNC/WAIT must reach before the program may resume. The headless
/// `wait`/`vsync` methods never set it; only [`begin_wait`](Self::begin_wait) /
/// [`begin_vsync`](Self::begin_vsync) do.
#[derive(Debug, Clone)]
pub struct FrameClock {
    frame: u64,
    last_vsync: u64,
    /// Pending host-driven wait target (`None` = no active wait / headless mode).
    wait_target: Option<u64>,
}

impl FrameClock {
    /// A fresh clock at frame 0. (On real hardware `MAINCNT` counts from when SmileBASIC was
    /// *launched*, not from `RUN`; in the headless model the VM's clock starts at 0 and is
    /// never reset by program control flow — see the timing-model concept's open question.)
    pub fn new() -> Self {
        FrameClock {
            frame: 0,
            last_vsync: 0,
            wait_target: None,
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

    // -- interactive (host-driven VBlank) API -----------------------------------------

    /// Arm a `WAIT count` in the host-driven VBlank model. Target = **current frame +
    /// count**; the counter is NOT advanced — the host's [`tick`](Self::tick) calls drive
    /// it one frame at a time and [`resolve_wait`](Self::resolve_wait) clears the pending
    /// target once reached. `count == 0` ("0: Ignore") resyncs `last_vsync` immediately
    /// without arming any target, mirroring the headless [`wait`](Self::wait) behaviour.
    pub fn begin_wait(&mut self, count: u64) {
        if count == 0 {
            self.last_vsync = self.frame;
        } else {
            self.wait_target = Some(self.frame.saturating_add(count));
        }
    }

    /// Arm a `VSYNC count` in the host-driven VBlank model. Target = **last_vsync +
    /// count**; the counter is NOT advanced yet. If the current frame already meets or
    /// exceeds the target (body overran its budget), `last_vsync` is resynced immediately
    /// and no target is armed — VSYNC returns at once with 0 frames elapsed, exactly as
    /// the headless [`vsync`](Self::vsync) does.
    pub fn begin_vsync(&mut self, count: u64) {
        let target = self.last_vsync.saturating_add(count);
        if self.frame >= target {
            // Already past the target: resync and don't block.
            self.last_vsync = self.frame;
        } else {
            self.wait_target = Some(target);
        }
    }

    /// Whether a host-driven wait is pending (the program is blocked on VSYNC/WAIT).
    pub fn wait_pending(&self) -> bool {
        self.wait_target.is_some()
    }

    /// Called by [`Vm::tick_frame`](crate::vm::Vm::tick_frame) after each VBlank tick.
    /// If a pending wait target exists and the counter has reached it, the target is
    /// cleared and `last_vsync` is resynced to the current frame (both WAIT and VSYNC
    /// resync on exit). A no-op when no wait is pending.
    pub fn resolve_wait(&mut self) {
        if let Some(target) = self.wait_target {
            if self.frame >= target {
                self.last_vsync = self.frame;
                self.wait_target = None;
            }
        }
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

/// The wall-clock date/time behind the `DATE$` and `TIME$` system variables (M6-T3).
///
/// On real hardware these read the 3DS RTC. `sb-core` keeps no real clock (it must stay
/// deterministic + wasm-safe), so the VM owns a fixed [`WallClock`] that the platform layer
/// can refresh per frame ([`Vm::set_wall_clock`](crate::vm::Vm)). The headless default is a
/// fixed epoch so tests are reproducible without injection — `DATE$`/`TIME$` are deterministic
/// under the injected clock, exactly as M6-T3 requires.
///
/// Fields are stored verbatim and formatted with zero-padding; no calendar arithmetic is done,
/// so whatever the platform injects is what the program reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallClock {
    /// Full year, e.g. `2000`.
    pub year: u16,
    /// Month `1..=12`.
    pub month: u8,
    /// Day of month `1..=31`.
    pub day: u8,
    /// Hour `0..=23`.
    pub hour: u8,
    /// Minute `0..=59`.
    pub minute: u8,
    /// Second `0..=59`.
    pub second: u8,
}

impl WallClock {
    /// The fixed headless epoch: `2000/01/01 00:00:00`. Deterministic, so a test that reads
    /// `DATE$`/`TIME$` without injecting a clock gets a stable value.
    pub const EPOCH: WallClock = WallClock {
        year: 2000,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    };

    /// `DATE$` — the date string `YYYY/MM/DD` (zero-padded fields).
    pub fn date_string(&self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }

    /// `TIME$` — the time string `HH:MM:SS` (zero-padded fields, 24-hour).
    pub fn time_string(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

impl Default for WallClock {
    fn default() -> Self {
        Self::EPOCH
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

    // -- interactive (host-driven VBlank) API -----------------------------------------

    #[test]
    fn begin_wait_arms_target_and_tick_drives_it() {
        let mut clk = FrameClock::new();
        clk.begin_wait(3);
        assert!(clk.wait_pending());
        assert_eq!(clk.maincnt(), 0, "counter not advanced yet");
        for expected in 1..=3u32 {
            clk.tick(1);
            clk.resolve_wait();
            assert_eq!(clk.maincnt(), expected as i32);
        }
        assert!(!clk.wait_pending(), "target reached, wait cleared");
    }

    #[test]
    fn begin_vsync_arms_target_relative_to_last_vsync() {
        let mut clk = FrameClock::new();
        // First VSYNC 2: target = 0 + 2 = 2.
        clk.begin_vsync(2);
        assert!(clk.wait_pending());
        clk.tick(1);
        clk.resolve_wait();
        assert!(clk.wait_pending(), "not there yet after 1 tick");
        clk.tick(1);
        clk.resolve_wait();
        assert!(!clk.wait_pending(), "reached target at frame 2");
        assert_eq!(clk.maincnt(), 2);
        // Second VSYNC 2: target = last_vsync(2) + 2 = 4.
        clk.begin_vsync(2);
        clk.tick(2);
        clk.resolve_wait();
        assert!(!clk.wait_pending());
        assert_eq!(clk.maincnt(), 4);
    }

    #[test]
    fn begin_vsync_immediate_when_already_past_target() {
        // Body overran: frame already past last_vsync + count — no blocking.
        let mut clk = FrameClock::new();
        clk.tick(5); // frame = 5, last_vsync = 0
        clk.begin_vsync(1); // target = 0+1 = 1, already at 5 → immediate
        assert!(!clk.wait_pending(), "already past target, no wait armed");
        assert_eq!(clk.maincnt(), 5);
        // last_vsync resynced to 5, so next VSYNC 1 targets 6.
        clk.begin_vsync(1);
        assert!(clk.wait_pending());
        clk.tick(1);
        clk.resolve_wait();
        assert!(!clk.wait_pending());
        assert_eq!(clk.maincnt(), 6);
    }

    #[test]
    fn begin_wait_zero_resyncs_last_vsync_immediately() {
        let mut clk = FrameClock::new();
        clk.tick(10);
        clk.begin_wait(0); // "0: Ignore" — no target, but resync
        assert!(!clk.wait_pending());
        assert_eq!(clk.maincnt(), 10);
        // last_vsync is now 10, so VSYNC 1 targets 11.
        clk.begin_vsync(1);
        clk.tick(1);
        clk.resolve_wait();
        assert_eq!(clk.maincnt(), 11);
    }

    #[test]
    fn resolve_wait_is_noop_with_no_pending_wait() {
        let mut clk = FrameClock::new();
        clk.tick(5);
        clk.resolve_wait(); // should not panic or mutate
        assert_eq!(clk.maincnt(), 5);
        assert!(!clk.wait_pending());
    }
}
