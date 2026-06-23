//! SmileBASIC 3.6.0 audio front-end.
//!
//! - [`mml`] — the **MML parser** (M5-T1): a `BGMPLAY`-style MML string → a deterministic
//!   per-channel note-event stream ([`mml::Song`]).
//! - [`instruments`] — oscillator/wavetable + ADSR-envelope model (M5-T2).
//! - [`synth`] — the **synth engine** (M5-T2): renders a [`mml::Song`] to interleaved stereo
//!   PCM16 through a 3DS-DSP-style voice/resampler/mixer ([`synth::Synth`]).
//!
//! Everything here is I/O- and device-free (pure integer/`f32` math, no threads) so it builds
//! for wasm32; the live audio backends (cpal / WebAudio) live in the `sb-platform-*` crates.
//! The render is fully deterministic — identical MML always yields byte-identical PCM.
//!
//! Specs: `spec/concepts/mml-grammar.md` (S-C5); signal path modeled on the 3DS DSP as in
//! citra/azahar `audio_core`. Audio *output fidelity* is the M5 deferred refining layer
//! (`prd/oracle.md` O-T7) — not e2e-verifiable; the testable contract is MML→events and
//! deterministic, structurally-correct PCM.

pub mod instruments;
pub mod mml;
pub mod synth;
