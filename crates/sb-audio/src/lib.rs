//! SmileBASIC 3.6.0 audio front-end.
//!
//! Today this crate is just the **MML parser** (`mml`): it turns a `BGMPLAY`-style MML
//! string into a deterministic per-channel note-event stream. The synth (M5-T2) that
//! renders those events to PCM lands later in the same crate; backends (cpal / WebAudio)
//! live in the `sb-platform-*` crates so this stays I/O- and device-free for wasm32.
//!
//! Spec: `spec/concepts/mml-grammar.md` (S-C5).

pub mod mml;
