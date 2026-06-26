//! synth.rs — render a parsed MML [`Song`] to PCM through a 3DS-DSP-style voice/mixer.
//!
//! ## What is and isn't grounded
//! Per `prd/oracle.md` O-T7 there is **no deterministic emulator audio golden** (SB cannot
//! render audio to a file; real device audio is real-time/timing-dependent). So this engine's
//! *output fidelity* is the M5 **deferred refining layer** — not e2e-verifiable. What we *can*
//! and *do* ground:
//!
//! - **The signal path** is modeled on the real 3DS DSP, as implemented by the
//!   citra/azahar emulator's `audio_core` (the same emulator the `sb-oracle` skill drives):
//!     * output is **interleaved stereo PCM16 at 32728 Hz**, the DSP's native rate
//!       (`audio_core/audio_types.h`: `native_sample_rate = 32728`, `samples_per_frame = 160`);
//!     * a voice is a **sample buffer played at a fractional rate** (pitch = resample rate),
//!       resampled with the DSP's **linear interpolation**: 24-bit fixed-point fraction and a
//!       **saturated delta** — `out = x0 + frac·clamp(x1−x0, −32768, 32767) / 2²⁴`
//!       (`audio_core/interpolate.cpp` `Linear`, "verified by black-box fuzzing").
//!       So instrument *voices* are single-cycle wavetables ([`crate::instruments::wavetable`])
//!       resampled exactly the way the hardware resamples its sample ROM.
//! - **Timing** follows `spec/concepts/mml-grammar.md` (S-C5): 192 ticks/whole note (48/quarter),
//!   tempo `T` = quarter-notes per minute; samples-per-tick = `32728·60 / (T·48)`.
//!
//! What stays `hypothesis` (tracked in beads: bd:sb-interpreter-i8p): the actual instrument sample ROM
//! (firmware data we don't have — analytic wavetables stand in), the exact `@E` envelope
//! curve, `@V`/`(`/`)` volume scaling, and the LFO (`@MP`/`@MA`/`@ML`) depth/speed mapping.
//!
//! The render is fully deterministic (pure integer/`f32` math, noise via a seeded LCG), so the
//! same MML always yields byte-identical PCM — that reproducibility *is* the testable contract.

use crate::instruments::{is_percussion, timbre_for, wavetable, Adsr};
use crate::mml::{Event, Song};

/// DSP native output rate (`audio_core/audio_types.h`: `native_sample_rate = 32728`).
pub const SAMPLE_RATE: u32 = 32728;
/// DSP audio frame size (`audio_core/audio_types.h`: `samples_per_frame = 160`).
pub const SAMPLES_PER_FRAME: u32 = 160;
/// Ticks per quarter note (192 per whole / 4); see S-C5.
pub const TICKS_PER_QUARTER: f64 = 48.0;
/// Default tempo when no `T` is given (quarter-notes/min).
pub const DEFAULT_TEMPO: u16 = 120;
/// Default channel volume / note velocity (0–127).
pub const DEFAULT_VOLUME: u8 = 127;
/// Default pan (64 = center).
pub const DEFAULT_PAN: u8 = 64;
/// `(`/`)` adjust channel volume by `n × VOLUME_STEP` units of the 0–127 `V` scale. Step size
/// is **oracle-pending** (S-C5); 1 unit/step is the placeholder.
pub const VOLUME_STEP: u8 = 1;

// DSP linear-interpolation fixed point: 24 fractional bits (audio_core/interpolate.cpp).
const FRAC_BITS: u32 = 24;
const FRAC_ONE: u64 = 1 << FRAC_BITS;
const FRAC_MASK: u64 = FRAC_ONE - 1;

/// Rendered PCM: interleaved stereo (L, R, L, R, …) signed 16-bit at [`SAMPLE_RATE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

impl Pcm {
    /// Number of stereo frames (each frame = one L + one R sample).
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }
}

/// MIDI key → frequency (Hz). A440 = key 69; `O4C` = key 60 (middle C, S-C5).
fn midi_to_freq(key: f32) -> f32 {
    440.0 * 2f32.powf((key - 69.0) / 12.0)
}

/// One step of the DSP's linear interpolation between two PCM16 samples, with a Q24 `frac`.
/// Reimplemented from citra/azahar `audio_core/interpolate.cpp` `Linear`: the delta is a
/// **saturated subtraction** and the multiply-accumulate is done in fixed point (integer
/// division truncates toward zero, matching the C `/`).
fn interp_linear(x0: i16, x1: i16, frac: u64) -> i16 {
    let delta = (x1 as i64 - x0 as i64).clamp(-32768, 32767);
    (x0 as i64 + (frac as i64 * delta) / FRAC_ONE as i64) as i16
}

/// Active low-frequency-oscillator on a channel (`@MP`/`@MA`/`@ML`, mutually exclusive per
/// S-C5). Engaged only while modulation is on (`@MON`). Depth/speed→amount mapping is a
/// documented placeholder (`hypothesis`, queued).
#[derive(Debug, Clone, Copy)]
enum Lfo {
    /// Pitch LFO (`@MP`) — vibrato. `depth` in semitones at full swing.
    Vibrato { depth: f32, speed: f32 },
    /// Amplitude LFO (`@MA`) — tremolo. `depth` in [0,1] of gain swing.
    Tremolo { depth: f32, speed: f32 },
    /// Pan LFO (`@ML`) — autopan. `depth` in [0,1] of pan swing around center.
    AutoPan { depth: f32, speed: f32 },
}

/// Per-channel synthesis state, evolved as the event timeline is consumed.
struct ChannelState {
    tempo: u16,
    volume: u8,
    pan: u8,
    instrument: u16,
    wavetable: Option<Vec<i16>>,
    percussion: bool,
    adsr: Adsr,
    detune_semis: f32,
    prev_freq: Option<f32>,
    modulation_on: bool,
    lfo: Option<Lfo>,
    noise: u32,
    /// Absolute output-sample counter (for continuous LFO phase).
    abs_sample: u64,
}

impl ChannelState {
    fn new(channel: usize) -> Self {
        let instrument = 0;
        ChannelState {
            tempo: DEFAULT_TEMPO,
            volume: DEFAULT_VOLUME,
            pan: DEFAULT_PAN,
            instrument,
            wavetable: wavetable(timbre_for(instrument)),
            percussion: is_percussion(instrument),
            adsr: Adsr::DEFAULT,
            detune_semis: 0.0,
            prev_freq: None,
            modulation_on: false,
            lfo: None,
            // Distinct per-channel noise seed so channels' noise streams differ but stay
            // reproducible run to run.
            noise: 0x1234_5678u32.wrapping_add((channel as u32).wrapping_mul(0x9E37_79B9)),
            abs_sample: 0,
        }
    }

    fn samples_per_tick(&self) -> f64 {
        // seconds/tick = 60 / (T · ticksPerQuarter); samples/tick = SR · seconds/tick.
        SAMPLE_RATE as f64 * 60.0 / (self.tempo as f64 * TICKS_PER_QUARTER)
    }
}

/// The MML synthesizer.
#[derive(Debug, Default, Clone)]
pub struct Synth;

impl Synth {
    pub fn new() -> Self {
        Synth
    }

    /// Render the full timeline of every channel once, mixing to stereo PCM. Endless-loop
    /// markers (`[ … ]`) are played through a single time (the timeline plays once). Use
    /// [`Synth::render_frames`] to render a fixed length, expanding endless loops as needed.
    pub fn render(&self, song: &Song) -> Pcm {
        let buffers: Vec<Vec<f32>> = song
            .channels
            .iter()
            .enumerate()
            .map(|(ch, events)| render_channel(ch, events, None))
            .collect();
        mix(&buffers)
    }

    /// Render exactly `frames` 1/60 s frames worth of samples (`frames · SAMPLE_RATE / 60`,
    /// rounded), expanding endless loops to fill the budget and truncating finite material.
    pub fn render_frames(&self, song: &Song, frames: u32) -> Pcm {
        let target = (frames as u64 * SAMPLE_RATE as u64).div_ceil(60) as usize;
        let buffers: Vec<Vec<f32>> = song
            .channels
            .iter()
            .enumerate()
            .map(|(ch, events)| render_channel(ch, events, Some(target)))
            .collect();
        let mut pcm = mix(&buffers);
        // Pad (or truncate) to exactly the requested frame budget.
        pcm.samples.resize(target * 2, 0);
        pcm
    }
}

/// Equal-power pan gains for `pan` (0 = hard left, 64 = center, 127 = hard right).
fn pan_gains(pan: u8) -> (f32, f32) {
    let p = (pan.min(127) as f32 / 127.0) * core::f32::consts::FRAC_PI_2;
    (p.cos(), p.sin())
}

/// Render one channel's event stream to an interleaved stereo `f32` buffer. `limit` (in mono
/// samples) caps the output and enables endless-loop expansion; `None` plays the timeline once.
fn render_channel(channel: usize, events: &[Event], limit: Option<usize>) -> Vec<f32> {
    let mut st = ChannelState::new(channel);
    let mut out: Vec<f32> = Vec::new();
    // Loop stack of (event index just after LoopStart). Endless loops only matter when a frame
    // budget bounds the render; without a limit we play straight through (markers are no-ops).
    let mut loop_stack: Vec<usize> = Vec::new();
    let mut cursor: f64 = 0.0; // next event start, in mono samples (fractional → no drift)

    let mut i = 0;
    while i < events.len() {
        if let Some(lim) = limit {
            if (cursor.round() as usize) >= lim {
                break;
            }
        }
        match &events[i] {
            Event::Tempo(t) => st.tempo = (*t).clamp(1, 512),
            Event::Volume(v) => st.volume = (*v).min(127),
            Event::VolumeUp(n) => {
                st.volume = st
                    .volume
                    .saturating_add(n.saturating_mul(VOLUME_STEP))
                    .min(127);
            }
            Event::VolumeDown(n) => {
                st.volume = st.volume.saturating_sub(n.saturating_mul(VOLUME_STEP));
            }
            Event::Pan(p) => st.pan = (*p).min(127),
            Event::Instrument(inst) => {
                st.instrument = *inst;
                st.wavetable = wavetable(timbre_for(*inst));
                st.percussion = is_percussion(*inst);
            }
            Event::Envelope { a, d, s, r } => {
                st.adsr = Adsr {
                    a: *a,
                    d: *d,
                    s: *s,
                    r: *r,
                }
            }
            Event::EnvelopeReset => st.adsr = Adsr::DEFAULT,
            Event::Detune(n) => {
                // @D −128..127 ≈ ∓ one tone (2 semitones); ±64 ≈ ±1 semitone (S-C5, hypothesis).
                st.detune_semis = *n as f32 / 64.0;
            }
            Event::Vibrato { depth, speed, .. } => {
                st.lfo = Some(Lfo::Vibrato {
                    depth: *depth as f32 / 127.0 * 2.0, // up to ±2 semitones (placeholder)
                    speed: *speed as f32 / 8.0,         // → Hz (placeholder)
                });
            }
            Event::Tremolo { depth, speed, .. } => {
                st.lfo = Some(Lfo::Tremolo {
                    depth: *depth as f32 / 127.0,
                    speed: *speed as f32 / 8.0,
                });
            }
            Event::AutoPan { depth, speed, .. } => {
                st.lfo = Some(Lfo::AutoPan {
                    depth: *depth as f32 / 127.0,
                    speed: *speed as f32 / 8.0,
                });
            }
            Event::ModulationOn => st.modulation_on = true,
            Event::ModulationOff => st.modulation_on = false,
            Event::Rest { duration } => {
                cursor += *duration as f64 * st.samples_per_tick();
            }
            Event::Note {
                pitch,
                duration,
                gate,
                velocity,
                slur,
                portamento,
            } => {
                render_note(
                    &mut st,
                    &mut out,
                    &mut cursor,
                    limit,
                    *pitch,
                    *duration,
                    *gate,
                    *velocity,
                    *slur,
                    *portamento,
                );
            }
            Event::LoopStart => {
                if limit.is_some() {
                    loop_stack.push(i + 1);
                }
            }
            Event::LoopEnd => {
                if let Some(lim) = limit {
                    if let Some(&back) = loop_stack.last() {
                        if (cursor.round() as usize) < lim {
                            i = back;
                            continue;
                        }
                        loop_stack.pop();
                    }
                } else {
                    // No budget: the endless section has played once — stop the channel here.
                    break;
                }
            }
        }
        i += 1;
    }
    out
}

/// Render a single note into `out` at the current `cursor`, advancing `cursor` by the full note
/// duration (channels are monophonic, so the next note begins exactly at the slot boundary).
#[allow(clippy::too_many_arguments)]
fn render_note(
    st: &mut ChannelState,
    out: &mut Vec<f32>,
    cursor: &mut f64,
    limit: Option<usize>,
    pitch: u8,
    duration: u32,
    gate: u8,
    velocity: u8,
    slur: bool,
    portamento: bool,
) {
    let spt = st.samples_per_tick();
    let start = cursor.round() as usize;
    *cursor += duration as f64 * spt;
    let end = cursor.round() as usize;
    let note_len = end.saturating_sub(start);
    if note_len == 0 {
        return;
    }

    // Gate: fraction of the slot actually sounded before release. Q8 (or a slur) = full legato.
    let gate_off = if slur || gate >= 8 {
        note_len as u32
    } else {
        ((note_len as u64 * gate as u64) / 8).max(1) as u32
    };

    let target_freq = midi_to_freq(pitch as f32 + st.detune_semis);
    let from_freq = if portamento {
        st.prev_freq.unwrap_or(target_freq)
    } else {
        target_freq
    };
    st.prev_freq = Some(target_freq);

    let amp_scale = (st.volume as f32 / 127.0) * (velocity as f32 / 127.0);
    let wt_len = st.wavetable.as_ref().map(|w| w.len()).unwrap_or(0);
    let mut position: u64 = 0; // Q24 index into the wavetable

    // Ensure the output buffer covers this note (interleaved stereo → ×2).
    let need = (start + note_len) * 2;
    let cap = limit.map(|l| l * 2);
    let need = cap.map(|c| need.min(c)).unwrap_or(need);
    if out.len() < need {
        out.resize(need, 0.0);
    }

    for n in 0..note_len {
        let idx = start + n;
        if let Some(l) = limit {
            if idx >= l {
                break;
            }
        }

        // Envelope amplitude (percussion uses a short fixed decay).
        let env = if st.percussion {
            percussion_env(n as u32, note_len as u32)
        } else {
            st.adsr.amplitude(n as u32, gate_off, SAMPLE_RATE)
        };

        // Frequency for this sample (portamento glide + optional vibrato).
        let glide = if portamento && note_len > 1 {
            from_freq + (target_freq - from_freq) * (n as f32 / note_len as f32)
        } else {
            target_freq
        };
        let (freq, gain_mod, pan_mod) = apply_lfo(st, glide);

        // Source sample: wavetable resample (DSP linear interp) or live noise.
        let s16 = if let Some(wt) = &st.wavetable {
            let step =
                ((freq as f64 * wt_len as f64 / SAMPLE_RATE as f64) * FRAC_ONE as f64) as u64;
            let table_idx = (position >> FRAC_BITS) as usize % wt_len;
            let x0 = wt[table_idx];
            let x1 = wt[(table_idx + 1) % wt_len];
            let frac = position & FRAC_MASK;
            position = (position + step) % ((wt_len as u64) << FRAC_BITS);
            interp_linear(x0, x1, frac)
        } else {
            // Noise: live LCG (Timbre::Noise has no period to resample).
            st.noise = st.noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (((st.noise >> 8) as i32) - (1 << 23)) as i16
        };

        let val = (s16 as f32 / 32768.0) * env * amp_scale * gain_mod;
        let pan = (st.pan as f32 + pan_mod).clamp(0.0, 127.0) as u8;
        let (lg, rg) = pan_gains(pan);
        out[idx * 2] += val * lg;
        out[idx * 2 + 1] += val * rg;
        st.abs_sample += 1;
    }
}

/// Apply the active LFO (if modulation is on) to frequency/gain/pan for one sample.
/// Returns `(freq, gain_multiplier, pan_offset)`.
fn apply_lfo(st: &ChannelState, base_freq: f32) -> (f32, f32, f32) {
    if !st.modulation_on {
        return (base_freq, 1.0, 0.0);
    }
    let Some(lfo) = st.lfo else {
        return (base_freq, 1.0, 0.0);
    };
    let t = st.abs_sample as f32 / SAMPLE_RATE as f32;
    match lfo {
        Lfo::Vibrato { depth, speed } => {
            let cents = depth * (core::f32::consts::TAU * speed * t).sin();
            (base_freq * 2f32.powf(cents / 12.0), 1.0, 0.0)
        }
        Lfo::Tremolo { depth, speed } => {
            let m = 1.0 - depth * 0.5 * (1.0 - (core::f32::consts::TAU * speed * t).sin());
            (base_freq, m.clamp(0.0, 1.0), 0.0)
        }
        Lfo::AutoPan { depth, speed } => {
            let off = depth * 63.0 * (core::f32::consts::TAU * speed * t).sin();
            (base_freq, 1.0, off)
        }
    }
}

/// Short fixed percussion envelope (drum hits): quick attack, exponential-ish decay over at
/// most ~120 ms, regardless of the channel ADSR.
fn percussion_env(n: u32, note_len: u32) -> f32 {
    let decay = (SAMPLE_RATE / 8).min(note_len.max(1)); // ≤125 ms
    if n >= decay {
        return 0.0;
    }
    let x = n as f32 / decay as f32;
    (1.0 - x) * (1.0 - x)
}

/// Sum the per-channel stereo buffers and clamp to PCM16 (the DSP's final mix saturates).
fn mix(buffers: &[Vec<f32>]) -> Pcm {
    let len = buffers.iter().map(|b| b.len()).max().unwrap_or(0);
    let mut acc = vec![0f32; len];
    for b in buffers {
        for (a, v) in acc.iter_mut().zip(b.iter()) {
            *a += *v;
        }
    }
    let samples = acc
        .into_iter()
        .map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    Pcm {
        sample_rate: SAMPLE_RATE,
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mml::parse;

    fn render(mml: &str) -> Pcm {
        Synth::new().render(&parse(mml).unwrap())
    }

    /// Rising zero-crossings on the left channel over `[from, to)` frames — a cheap pitch
    /// estimate (≈ one per waveform cycle).
    fn rising_crossings(pcm: &Pcm, from: usize, to: usize) -> usize {
        let mut count = 0;
        let mut prev = 0.0f32;
        for f in from..to.min(pcm.frames()) {
            let s = pcm.samples[f * 2] as f32;
            if prev <= 0.0 && s > 0.0 {
                count += 1;
            }
            prev = s;
        }
        count
    }

    fn left_energy(pcm: &Pcm, from: usize, to: usize) -> u64 {
        (from..to.min(pcm.frames()))
            .map(|f| (pcm.samples[f * 2] as i64).unsigned_abs())
            .sum()
    }

    fn right_energy(pcm: &Pcm, from: usize, to: usize) -> u64 {
        (from..to.min(pcm.frames()))
            .map(|f| (pcm.samples[f * 2 + 1] as i64).unsigned_abs())
            .sum()
    }

    #[test]
    fn empty_song_is_empty() {
        let pcm = render("");
        assert!(pcm.samples.is_empty());
        assert_eq!(pcm.sample_rate, SAMPLE_RATE);
    }

    #[test]
    fn render_is_deterministic() {
        // Identical MML must yield byte-identical PCM (the M5 testable contract).
        assert_eq!(
            render(":0@1V100O4CDEFG:1@150C8C8"),
            render(":0@1V100O4CDEFG:1@150C8C8")
        );
    }

    #[test]
    fn note_produces_sound() {
        let pcm = render("C");
        assert!(pcm.samples.iter().any(|&s| s != 0));
    }

    #[test]
    fn quarter_note_is_half_second_at_t120() {
        // T120 → a quarter (default L4) is 0.5 s = 32728/2 = 16364 samples.
        assert_eq!(render("C").frames(), 16364);
    }

    #[test]
    fn tempo_scales_duration() {
        // Doubling the tempo halves the note length.
        assert_eq!(render("T240C").frames(), 8182);
        // A whole note (L1) at T120 is 2 s.
        assert_eq!(render("C1").frames(), 32728 * 2);
    }

    #[test]
    fn rest_then_note_has_leading_silence() {
        let pcm = render("R4C4");
        assert_eq!(pcm.frames(), 16364 * 2);
        // The first quarter (the rest) is pure silence.
        assert_eq!(left_energy(&pcm, 0, 16364), 0);
        // The second quarter (the note) sounds.
        assert!(left_energy(&pcm, 16364, 16364 * 2) > 0);
    }

    #[test]
    fn gate_creates_a_staccato_tail() {
        // Q1 sounds only ~1/8 of the slot; the last quarter should be (near) silent, while a
        // legato Q8 note sustains through it.
        let staccato = render("Q1C");
        let legato = render("Q8C");
        let tail = (12273, 16364); // last quarter of the slot
        assert!(left_energy(&staccato, tail.0, tail.1) * 4 < left_energy(&legato, tail.0, tail.1));
    }

    #[test]
    fn pan_hard_left_silences_right() {
        let pcm = render("P0C");
        assert!(left_energy(&pcm, 0, pcm.frames()) > 0);
        assert_eq!(right_energy(&pcm, 0, pcm.frames()), 0);
    }

    #[test]
    fn pan_hard_right_silences_left() {
        let pcm = render("P127C");
        assert_eq!(left_energy(&pcm, 0, pcm.frames()), 0);
        assert!(right_energy(&pcm, 0, pcm.frames()) > 0);
    }

    #[test]
    fn a_channel_other_than_zero_plays() {
        // Material written only to channel 3 must still reach the mix.
        let pcm = render(":3 C");
        assert!(pcm.samples.iter().any(|&s| s != 0));
    }

    #[test]
    fn channels_mix_additively() {
        // Two channels playing the same in-phase note sum louder than one alone.
        let one = render(":0 V60 @150 C");
        let two = render(":0 V60 @150 C :1 V60 @150 C");
        let peak = |p: &Pcm| {
            p.samples
                .iter()
                .map(|s| s.unsigned_abs())
                .max()
                .unwrap_or(0)
        };
        assert!(peak(&two) > peak(&one));
    }

    #[test]
    fn render_frames_has_exact_length() {
        let song = parse("CDEFG").unwrap();
        let pcm = Synth::new().render_frames(&song, 60); // 60 frames = 1 s
        assert_eq!(pcm.samples.len(), SAMPLE_RATE as usize * 2);
    }

    #[test]
    fn endless_loop_fills_the_frame_budget() {
        // `[C]` loops forever; render_frames must keep producing sound to the end of the budget.
        let song = parse("@150[C]").unwrap();
        let pcm = Synth::new().render_frames(&song, 120); // 2 s
        let n = pcm.frames();
        assert!(
            left_energy(&pcm, n - 2000, n) > 0,
            "endless loop went silent before the budget end"
        );
    }

    #[test]
    fn one_shot_render_plays_an_endless_loop_once() {
        // Without a frame budget, `[C]` plays its body a single time (one quarter note).
        assert_eq!(render("@150[C]").frames(), 16364);
    }

    #[test]
    fn pitch_matches_the_note_frequency() {
        // O4C = MIDI 60 ≈ 261.6 Hz; a 0.5 s sine has ≈131 cycles.
        let pcm = render("@150 O4 C");
        let cycles = rising_crossings(&pcm, 0, pcm.frames());
        let hz = cycles as f32 / 0.5;
        assert!(
            (245.0..280.0).contains(&(hz)),
            "estimated {hz} Hz, expected ~261.6"
        );
    }

    #[test]
    fn octave_up_doubles_the_frequency() {
        let lo = render("@150 O4 C");
        let hi = render("@150 O5 C");
        let clo = rising_crossings(&lo, 0, lo.frames());
        let chi = rising_crossings(&hi, 0, hi.frames());
        // One octave up ≈ 2× the cycles (allow slack for the attack ramp / rounding).
        assert!(chi > clo * 9 / 5, "expected ~2× ({clo} → {chi})");
    }

    #[test]
    fn detune_raises_pitch() {
        // @D64 ≈ +1 semitone, so more cycles than the plain note.
        let plain = render("@150 C");
        let sharp = render("@150 @D64 C");
        assert!(
            rising_crossings(&sharp, 0, sharp.frames())
                > rising_crossings(&plain, 0, plain.frames())
        );
    }

    #[test]
    fn interp_linear_endpoints_and_midpoint() {
        assert_eq!(interp_linear(100, 200, 0), 100); // fraction 0 → x0
        assert_eq!(interp_linear(100, 200, FRAC_ONE / 2), 150); // halfway
        assert_eq!(interp_linear(-100, 100, FRAC_ONE / 2), 0);
    }
}
