//! `sb-audio` — SmileBASIC sound (milestone M5).
//!
//! Houses the MML (Music Macro Language) parser and synthesizer behind BGMPLAY/
//! BGMSET/BEEP/TALK, plus the audio backend (cpal native / WebAudio wasm). The full
//! MML grammar is specified in `spec/reference/mml.yaml` (from
//! `sb-docs/smilebasic-3/reference/mml.md`). This is the area where `osb` is weakest
//! (audio stubbed), so the docs + disassembly carry the implementation.

/// SmileBASIC outputs audio at 32_728? — exact rate TBD from the disassembly during M5.
pub const PLACEHOLDER_SAMPLE_RATE: u32 = 32_768;

// TODO(M5): MmlParser, channels/envelopes/instruments, synth, golden-WAV harvesting.
