//! Oscillator / waveform + ADSR-envelope model for the M5-T2 synth.
//!
//! SmileBASIC's real melodic instruments (`@0`–`@127`) are **sampled** GM-equivalent
//! voices baked into the firmware; the PSG sources (`@144`–`@151`) are classic pulse/noise
//! generators and the drum kits (`@128`/`@129`) are one-shot percussion samples. We cannot
//! reproduce the firmware's sample ROM, and per `prd/oracle.md` O-T7 there is **no
//! deterministic audio golden** to verify against, so this module is a *faithful synthetic
//! approximation*: each instrument maps to an analytic oscillator [`Timbre`] whose **shape is
//! a documented placeholder** (confidence `hypothesis`), while the *structure* it plugs into —
//! per-note pitch/duration/gate/velocity timing, ADSR phases, 16-channel mixing — is the
//! `documented`/`disassembled` contract from `spec/concepts/mml-grammar.md` (S-C5).
//!
//! Everything here is pure `f32`/`f64` math (no I/O, no RNG crate — noise uses a seeded LCG)
//! so the render is bit-reproducible and builds for wasm32. The waveform tables are queued in
//! beads (bd:sb-interpreter-i8p) as the M5 deferred fidelity layer.

/// The analytic oscillator an instrument is rendered with. `phase` is in `[0,1)` (one full
/// period), so [`Timbre::sample`] is a pure function of phase except [`Timbre::Noise`], which
/// is driven by an explicit LCG state for reproducibility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timbre {
    /// Pure sine — the cleanest tone; used for soft/flute-like voices.
    Sine,
    /// Triangle — mellow, few harmonics.
    Triangle,
    /// Sawtooth — bright, all harmonics; the default melodic placeholder.
    Saw,
    /// Pulse/square with a duty cycle in `(0,1)`; `0.5` is a square. PSG voices vary duty.
    Pulse { duty: f32 },
    /// White noise (LCG-driven); PSG noise source and percussion.
    Noise,
}

impl Timbre {
    /// Sample the oscillator at `phase` ∈ `[0,1)`. `noise` is a mutable LCG state advanced
    /// only for [`Timbre::Noise`]; callers thread one state per voice so repeated renders are
    /// identical. Output is in `[-1.0, 1.0]`.
    pub fn sample(&self, phase: f32, noise: &mut u32) -> f32 {
        match self {
            Timbre::Sine => (phase * core::f32::consts::TAU).sin(),
            Timbre::Triangle => {
                // 0→1→0→-1→0 across the period.
                let p = phase.rem_euclid(1.0);
                if p < 0.5 {
                    4.0 * p - 1.0
                } else {
                    3.0 - 4.0 * p
                }
            }
            Timbre::Saw => 2.0 * phase.rem_euclid(1.0) - 1.0,
            Timbre::Pulse { duty } => {
                if phase.rem_euclid(1.0) < *duty {
                    1.0
                } else {
                    -1.0
                }
            }
            Timbre::Noise => {
                // Numerical-Recipes LCG; deterministic, period 2^32. Map to [-1,1).
                *noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((*noise >> 8) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
            }
        }
    }
}

/// One-cycle wavetable resolution, in samples. The synth plays a periodic instrument by
/// looping a single cycle of this length through the DSP-style resampler ([`crate::synth`]),
/// so pitch comes from the resample rate exactly as the real hardware derives it from a
/// sample's playback rate. 256 is a balance between table detail and the aliasing the real
/// (linear-interpolating) DSP also exhibits.
pub const WAVETABLE_LEN: usize = 256;

/// One cycle of `timbre` as signed PCM16, ready to feed the resampler. Returns `None` for
/// [`Timbre::Noise`], which has no period and is generated live (LCG) at the output rate.
pub fn wavetable(timbre: Timbre) -> Option<Vec<i16>> {
    if timbre == Timbre::Noise {
        return None;
    }
    let mut noise = 0u32; // unused for periodic timbres
    Some(
        (0..WAVETABLE_LEN)
            .map(|i| {
                let phase = i as f32 / WAVETABLE_LEN as f32;
                let v = timbre.sample(phase, &mut noise);
                (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
            })
            .collect(),
    )
}

/// Map an MML instrument number to its [`Timbre`]. **Placeholder mapping** (confidence
/// `hypothesis`): the real firmware uses sampled voices we can't reproduce; these analytic
/// shapes are a best-effort stand-in chosen so each family is at least audibly distinct.
/// Ranges follow `spec/concepts/mml-grammar.md`:
/// - `@0`–`@127` melodic → [`Timbre::Saw`] (bright, harmonic-rich default).
/// - `@128`/`@129` drum kits → [`Timbre::Noise`] (see [`is_percussion`]).
/// - `@144`–`@150` PSG tones → [`Timbre::Pulse`] with stepped duty cycles.
/// - `@151` PSG noise → [`Timbre::Noise`].
/// - `@224`–`@255` user waveforms → [`Timbre::Saw`] until `WAVSET` feeds real data (M5-T4).
/// - `@256`+ SFX bank → [`Timbre::Noise`] (one-shot effects; M5-T4).
pub fn timbre_for(instrument: u16) -> Timbre {
    match instrument {
        128 | 129 => Timbre::Noise,
        144 => Timbre::Pulse { duty: 0.5 },
        145 => Timbre::Pulse { duty: 0.25 },
        146 => Timbre::Pulse { duty: 0.125 },
        147 => Timbre::Pulse { duty: 0.75 },
        148 => Timbre::Pulse { duty: 0.875 },
        149 => Timbre::Triangle,
        150 => Timbre::Sine,
        151 => Timbre::Noise,
        n if n >= 256 => Timbre::Noise,
        _ => Timbre::Saw,
    }
}

/// `@128`/`@129` (and the SFX bank) are one-shot percussive voices: the note pitch picks a
/// drum from the kit map but the sound is a short noise burst rather than a sustained pitch.
/// We render them as a short fixed envelope regardless of the channel ADSR.
pub fn is_percussion(instrument: u16) -> bool {
    matches!(instrument, 128 | 129) || instrument >= 256
}

/// An ADSR envelope (`@E A,D,S,R`). `a`/`d`/`r` are *times* and `s` is a *level*, each
/// `0..=127`. Per S-C5 the docs say **smaller value = slower** for A/D/R, so a small `a`
/// gives a long attack. The exact firmware curve is unknown (confidence `hypothesis`,
/// queued): we map each time parameter linearly onto `[ENV_MIN_S, ENV_MAX_S]` seconds with
/// `v=127`→fast and `v=0`→slow, and treat `s` as a linear sustain level in `[0,1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adsr {
    pub a: u8,
    pub d: u8,
    pub s: u8,
    pub r: u8,
}

impl Adsr {
    /// The default voice when no `@E` is given: near-instant attack, no decay, full sustain,
    /// a short release. Plain notes are therefore audible at full level with a click-free tail.
    pub const DEFAULT: Adsr = Adsr {
        a: 127,
        d: 127,
        s: 127,
        r: 110,
    };
}

/// Fastest A/D/R time, seconds (parameter value 127).
pub const ENV_MIN_S: f32 = 0.002;
/// Slowest A/D/R time, seconds (parameter value 0).
pub const ENV_MAX_S: f32 = 2.0;

/// Convert a 0..=127 time parameter to seconds (smaller value = slower / longer).
fn time_seconds(v: u8) -> f32 {
    let frac = (127 - v.min(127)) as f32 / 127.0;
    ENV_MIN_S + frac * (ENV_MAX_S - ENV_MIN_S)
}

impl Adsr {
    /// Envelope amplitude in `[0,1]` at sample `i`, given the key is released (gate-off) at
    /// sample `gate_off`. After `gate_off` the level releases from wherever the A/D/S curve
    /// had reached. Pure function of its inputs → deterministic.
    pub fn amplitude(&self, i: u32, gate_off: u32, sample_rate: u32) -> f32 {
        let sr = sample_rate as f32;
        let attack = (time_seconds(self.a) * sr).max(1.0);
        let decay = (time_seconds(self.d) * sr).max(1.0);
        let release = (time_seconds(self.r) * sr).max(1.0);
        let sustain = self.s.min(127) as f32 / 127.0;

        // Level reached by the end of attack+decay if held (used for the release start).
        let level_at = |t: f32| -> f32 {
            if t < attack {
                t / attack
            } else if t < attack + decay {
                let dt = (t - attack) / decay;
                1.0 + dt * (sustain - 1.0)
            } else {
                sustain
            }
        };

        let t = i as f32;
        if i < gate_off {
            level_at(t)
        } else {
            let start = level_at(gate_off as f32);
            let dt = (t - gate_off as f32) / release;
            if dt >= 1.0 {
                0.0
            } else {
                start * (1.0 - dt)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_wavetables_have_one_cycle_noise_has_none() {
        assert_eq!(wavetable(Timbre::Sine).unwrap().len(), WAVETABLE_LEN);
        assert_eq!(wavetable(Timbre::Saw).unwrap().len(), WAVETABLE_LEN);
        assert!(wavetable(Timbre::Noise).is_none());
    }

    #[test]
    fn instrument_ranges_map_to_expected_families() {
        assert_eq!(timbre_for(0), Timbre::Saw); // melodic default
        assert_eq!(timbre_for(127), Timbre::Saw);
        assert_eq!(timbre_for(128), Timbre::Noise); // drum kit
        assert_eq!(timbre_for(144), Timbre::Pulse { duty: 0.5 });
        assert_eq!(timbre_for(151), Timbre::Noise); // PSG noise
        assert_eq!(timbre_for(224), Timbre::Saw); // user waveform (until WAVSET)
        assert_eq!(timbre_for(256), Timbre::Noise); // SFX bank
    }

    #[test]
    fn percussion_is_drums_and_sfx_only() {
        assert!(is_percussion(128));
        assert!(is_percussion(129));
        assert!(is_percussion(300));
        assert!(!is_percussion(0));
        assert!(!is_percussion(150));
    }

    #[test]
    fn sine_wavetable_is_balanced_and_bounded() {
        let wt = wavetable(Timbre::Sine).unwrap();
        // First sample is ~0 (sin 0), quarter-way is the peak.
        assert!(wt[0].abs() < 500);
        assert!(wt[WAVETABLE_LEN / 4] > 32000);
    }

    #[test]
    fn default_envelope_attacks_then_sustains_then_releases() {
        let sr = 32728;
        let e = Adsr::DEFAULT;
        let gate_off = sr; // release after 1 s
        assert!(e.amplitude(0, gate_off, sr) < 0.5); // ramping up at the very start
        assert!(e.amplitude(sr / 2, gate_off, sr) > 0.9); // sustaining
        assert_eq!(e.amplitude(gate_off + sr, gate_off, sr), 0.0); // fully released
    }

    #[test]
    fn lower_envelope_time_param_is_slower() {
        // Smaller A = slower attack: amplitude reached early is lower.
        let sr = 32728;
        let fast = Adsr {
            a: 127,
            d: 127,
            s: 127,
            r: 100,
        };
        let slow = Adsr {
            a: 0,
            d: 127,
            s: 127,
            r: 100,
        };
        let i = sr / 100; // 10 ms in
        assert!(fast.amplitude(i, sr, sr) > slow.amplitude(i, sr, sr));
    }
}
