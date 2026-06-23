//! Native platform layer — shared library pieces used by the desktop bins.
//!
//! The bins (`sb-run`, `sb`, `sb-play`) keep their own `main`; this crate root exposes the
//! reusable, device-touching host modules that more than one of them needs. Today that is the
//! live audio backend.

// The cpal-backed audio output (M5-T5). cpal links a native device library (ALSA/CoreAudio/
// WASAPI), so this module is desktop-only and behind the off-by-default `audio` feature — see
// `Cargo.toml`. The deterministic streaming core it drives (`PcmRing`/`StereoResampler`) lives
// in `sb-audio` and is always built and unit-tested.
#[cfg(all(feature = "audio", not(target_arch = "wasm32")))]
pub mod audio;
