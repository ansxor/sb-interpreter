//! Device-independent PCM streaming primitives for the live audio backends (M5-T5).
//!
//! The synth ([`crate::synth`]) produces interleaved stereo PCM16 at a fixed
//! [`SAMPLE_RATE`](crate::synth::SAMPLE_RATE) (32728 Hz), one 1/60 s frame at a time. A host
//! audio device pulls samples on its *own* callback, at its own buffer size and sample rate.
//! Those two clocks don't line up, so the backends sit two pure primitives between them:
//!
//! - [`PcmRing`] — a fixed-capacity ring buffer. The 60 fps producer pushes a frame's PCM;
//!   the device callback pulls whatever it needs. An empty ring yields **silence plus a
//!   counted underrun**, never a panic or a glitchy read of stale data — that's how the
//!   backend "no buffer underruns at 60 fps" guarantee is observed and tested.
//! - [`StereoResampler`] — converts the synth's 32728 Hz to whatever rate the device wants,
//!   carrying fractional phase across chunk boundaries so successive 1/60 s frames join
//!   seamlessly (no click at each frame seam).
//!
//! Everything here is pure: no device, no threads, no `std::time` — deterministic integer/
//! `f64` math only. It builds for wasm32 and is unit-tested in CI. The cpal / WebAudio glue
//! that opens an actual output stream lives in the `sb-platform-*` crates and feeds these.

/// A fixed-capacity ring buffer of interleaved stereo `i16` samples that decouples the
/// 60 fps synth producer from a device's pull-based callback.
///
/// Capacities and counts below are in **samples** (one stereo frame = two samples). Push
/// what the synth rendered this frame; the device callback pulls its buffer size each time.
#[derive(Debug, Clone)]
pub struct PcmRing {
    buf: Vec<i16>,
    /// Read cursor into `buf`.
    head: usize,
    /// Number of valid (unread) samples currently buffered.
    len: usize,
    /// Total silence samples emitted on underrun over the ring's lifetime.
    underrun_samples: u64,
}

impl PcmRing {
    /// Create a ring sized to hold `frames` stereo frames (`frames * 2` samples). A device
    /// typically wants a few frames of headroom (e.g. 4–8 × the 1/60 s frame) so a late
    /// producer push doesn't starve the callback.
    pub fn with_frames(frames: usize) -> Self {
        PcmRing {
            buf: vec![0; frames.saturating_mul(2)],
            head: 0,
            len: 0,
            underrun_samples: 0,
        }
    }

    /// Total capacity in samples.
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Samples currently buffered and ready to pull.
    pub fn available(&self) -> usize {
        self.len
    }

    /// Free space in samples (`capacity - available`).
    pub fn free(&self) -> usize {
        self.buf.len() - self.len
    }

    /// Whether the ring holds no samples (the next pull would underrun).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Lifetime count of silence samples emitted because a pull outran the producer. A
    /// healthy 60 fps stream keeps this at zero; a rising count means the producer is late.
    pub fn underrun_samples(&self) -> u64 {
        self.underrun_samples
    }

    /// Push interleaved stereo samples, writing as many as fit. Returns the number written;
    /// a result `< data.len()` means the ring was full and the overflow tail was dropped
    /// (preferring to keep already-buffered audio over the newest material).
    pub fn push(&mut self, data: &[i16]) -> usize {
        let n = data.len().min(self.free());
        if n == 0 {
            return 0;
        }
        let cap = self.buf.len();
        let tail = (self.head + self.len) % cap;
        let first = n.min(cap - tail);
        self.buf[tail..tail + first].copy_from_slice(&data[..first]);
        if n > first {
            self.buf[..n - first].copy_from_slice(&data[first..n]);
        }
        self.len += n;
        n
    }

    /// Fill `out` with buffered samples, front to back. Any shortfall (the ring ran dry) is
    /// written as silence and added to [`underrun_samples`](Self::underrun_samples). Returns
    /// the number of *real* (non-silence) samples delivered.
    pub fn pull(&mut self, out: &mut [i16]) -> usize {
        let n = out.len().min(self.len);
        if n > 0 {
            let cap = self.buf.len();
            let first = n.min(cap - self.head);
            out[..first].copy_from_slice(&self.buf[self.head..self.head + first]);
            if n > first {
                out[first..n].copy_from_slice(&self.buf[..n - first]);
            }
            self.head = (self.head + n) % cap;
            self.len -= n;
        }
        if n < out.len() {
            out[n..].fill(0);
            self.underrun_samples += (out.len() - n) as u64;
        }
        n
    }

    /// Drop all buffered samples (e.g. on `BGMSTOP`/track change). Leaves the underrun tally.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}

/// One interleaved stereo frame (`[left, right]`).
type Frame = [i16; 2];

/// Linear-interpolate between two `i16` samples at fraction `fr` ∈ [0, 1], rounded to nearest
/// and clamped to the `i16` range.
fn lerp_i16(a: i16, b: i16, fr: f64) -> i16 {
    let v = a as f64 + (b as f64 - a as f64) * fr;
    v.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

/// A stateful linear stereo resampler that converts a continuous stream of interleaved PCM16
/// from one sample rate to another, chunk by chunk.
///
/// It carries the fractional read position and the last input frame across [`process`](Self::process)
/// calls, so feeding it successive 1/60 s synth frames produces the same output as resampling
/// the whole stream at once — no discontinuity at the seams. Linear interpolation matches the
/// synth's own DSP-style resampler family (see `synth::interp_linear`); it is deterministic.
#[derive(Debug, Clone)]
pub struct StereoResampler {
    /// Input frames consumed per output frame (`from / to`). `1.0` is identity.
    ratio: f64,
    /// Read position in input frames, relative to the start of the *current* chunk. May be
    /// negative after a carry, in which case index −1 refers to [`prev`](Self::prev).
    pos: f64,
    /// The last input frame of the previous chunk (index −1 of the current chunk).
    prev: Frame,
}

impl StereoResampler {
    /// Create a resampler from `from_rate` to `to_rate` (both Hz, must be > 0).
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        assert!(
            from_rate > 0 && to_rate > 0,
            "sample rates must be positive"
        );
        StereoResampler {
            ratio: from_rate as f64 / to_rate as f64,
            pos: 0.0,
            prev: [0, 0],
        }
    }

    /// Input frames consumed per output frame.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Resample one chunk of interleaved stereo input, appending output frames to `out`.
    /// Output length depends on the rate ratio and the carried phase; for a steady stream the
    /// average is `input_frames / ratio` frames per call.
    pub fn process(&mut self, input: &[i16], out: &mut Vec<i16>) {
        let n = input.len() / 2; // input frame count
        if n == 0 {
            return;
        }
        let frame = |i: i64| -> Frame {
            if i < 0 {
                self.prev
            } else {
                let i = i as usize;
                [input[2 * i], input[2 * i + 1]]
            }
        };
        // Emit every output frame whose interpolation interval [idx, idx+1] lies within the
        // data available this chunk (idx may be −1, using `prev`).
        loop {
            let idx = self.pos.floor() as i64;
            if idx + 1 > n as i64 - 1 {
                break;
            }
            let fr = self.pos - idx as f64;
            let a = frame(idx);
            let b = frame(idx + 1);
            out.push(lerp_i16(a[0], b[0], fr));
            out.push(lerp_i16(a[1], b[1], fr));
            self.pos += self.ratio;
        }
        // Re-base position onto the next chunk: index N becomes index 0, so index N−1 (this
        // chunk's last frame) becomes the next chunk's index −1.
        self.prev = frame(n as i64 - 1);
        self.pos -= n as f64;
    }

    /// Convenience one-shot: resample a whole buffer in a single call (fresh phase). Used by
    /// the native backend's blocking "play this finite clip" path.
    pub fn resample_all(from_rate: u32, to_rate: u32, input: &[i16]) -> Vec<i16> {
        let mut r = StereoResampler::new(from_rate, to_rate);
        let mut out = Vec::with_capacity((input.len() as f64 / r.ratio).ceil() as usize + 2);
        r.process(input, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_push_pull_roundtrip() {
        let mut ring = PcmRing::with_frames(4); // 8 samples
        assert_eq!(ring.capacity(), 8);
        assert_eq!(ring.push(&[1, 2, 3, 4]), 4);
        assert_eq!(ring.available(), 4);
        let mut out = [0i16; 4];
        assert_eq!(ring.pull(&mut out), 4);
        assert_eq!(out, [1, 2, 3, 4]);
        assert!(ring.is_empty());
        assert_eq!(ring.underrun_samples(), 0);
    }

    #[test]
    fn ring_wraps_around_capacity() {
        let mut ring = PcmRing::with_frames(3); // 6 samples
        ring.push(&[1, 2, 3, 4]);
        let mut out = [0i16; 4];
        ring.pull(&mut out); // head now at 4
                             // Push 4 more: 2 fit before the end, 2 wrap to the front.
        assert_eq!(ring.push(&[5, 6, 7, 8]), 4);
        let mut out2 = [0i16; 4];
        assert_eq!(ring.pull(&mut out2), 4);
        assert_eq!(out2, [5, 6, 7, 8]);
    }

    #[test]
    fn ring_push_overflow_drops_tail() {
        let mut ring = PcmRing::with_frames(2); // 4 samples
        assert_eq!(ring.push(&[1, 2, 3, 4, 5, 6]), 4); // only 4 fit
        assert_eq!(ring.free(), 0);
        let mut out = [0i16; 4];
        ring.pull(&mut out);
        assert_eq!(out, [1, 2, 3, 4]);
    }

    #[test]
    fn ring_underrun_fills_silence_and_counts() {
        let mut ring = PcmRing::with_frames(4);
        ring.push(&[7, 8]);
        let mut out = [99i16; 6];
        assert_eq!(ring.pull(&mut out), 2); // 2 real, 4 silence
        assert_eq!(out, [7, 8, 0, 0, 0, 0]);
        assert_eq!(ring.underrun_samples(), 4);
        // A fully dry pull is all silence.
        let mut out2 = [42i16; 2];
        assert_eq!(ring.pull(&mut out2), 0);
        assert_eq!(out2, [0, 0]);
        assert_eq!(ring.underrun_samples(), 6);
    }

    #[test]
    fn ring_clear_empties() {
        let mut ring = PcmRing::with_frames(4);
        ring.push(&[1, 2, 3, 4]);
        ring.clear();
        assert!(ring.is_empty());
        assert_eq!(ring.free(), 8);
    }

    #[test]
    fn resample_identity_preserves_signal() {
        // from == to: ratio 1.0; output should match input (modulo at most one boundary frame).
        let input: Vec<i16> = (0..20).collect();
        let out = StereoResampler::resample_all(32728, 32728, &input);
        // First frames must be exactly the input frames (fr == 0 at integer positions).
        assert_eq!(&out[..16], &input[..16]);
    }

    #[test]
    fn resample_downsample_halves_length() {
        // 2:1 downsample → about half as many output frames.
        let input: Vec<i16> = vec![0; 200]; // 100 frames
        let out = StereoResampler::resample_all(64000, 32000, &input);
        let frames = out.len() / 2;
        assert!((49..=51).contains(&frames), "got {frames} frames");
    }

    #[test]
    fn resample_upsample_roughly_doubles_length() {
        let input: Vec<i16> = vec![0; 200]; // 100 frames
        let out = StereoResampler::resample_all(32000, 64000, &input);
        let frames = out.len() / 2;
        assert!((198..=202).contains(&frames), "got {frames} frames");
    }

    #[test]
    fn resample_streaming_matches_oneshot() {
        // Feeding the stream in chunks must equal resampling it whole — that's the cross-chunk
        // phase-continuity guarantee the device callback relies on.
        let input: Vec<i16> = (0..400).map(|i| ((i * 137) % 4000 - 2000) as i16).collect();
        let oneshot = StereoResampler::resample_all(32728, 44100, &input);

        let mut r = StereoResampler::new(32728, 44100);
        let mut chunked = Vec::new();
        for chunk in input.chunks(2 * 7) {
            // odd 7-frame chunks
            r.process(chunk, &mut chunked);
        }
        assert_eq!(chunked, oneshot);
    }

    #[test]
    fn resample_interpolates_midpoint() {
        // Exact 2× upsample of a three-frame ramp: the inserted samples are linear midpoints.
        // (The final frame at integer position n−1 defers to the next chunk by design, so feed
        // one extra frame to observe the 100 endpoint.)
        let input: Vec<i16> = vec![0, 0, 100, 100, 200, 200];
        let out = StereoResampler::resample_all(1000, 2000, &input);
        // frames at input positions 0, 0.5, 1.0, 1.5 → 0, 50, 100, 150
        assert_eq!(out[0], 0);
        assert_eq!(out[2], 50);
        assert_eq!(out[4], 100);
        assert_eq!(out[6], 150);
    }
}
