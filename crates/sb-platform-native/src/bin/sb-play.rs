//! `sb-play` — the audio backend demo (M5-T5).
//!
//! Parses an MML string with the `sb-audio` MML parser, renders it to PCM with the synth, and
//! plays it through the live cpal output device. It is the runnable end-to-end exercise of the
//! native audio backend (`sb_platform_native::audio`): MML → events → PCM → resample → device.
//!
//! ```text
//!   sb-play "T120 O4 CDEFGAB<C"        # play the tune once
//!   sb-play "[O4CEG]4" 240             # render 240 frames (4 s) of an endless loop
//! ```
//!
//! It needs a real output device, so it is behind the `audio` feature and excluded from the
//! default / wasm builds (see `Cargo.toml`).

use std::process::ExitCode;

use sb_audio::mml;
use sb_audio::synth::Synth;
use sb_platform_native::audio;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(src) = args.next() else {
        eprintln!("usage: sb-play \"<MML>\" [frames]");
        return ExitCode::from(2);
    };
    let frames: Option<u32> = match args.next() {
        Some(s) => match s.parse() {
            Ok(n) => Some(n),
            Err(_) => {
                eprintln!("frames must be a non-negative integer");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    let song = match mml::parse(&src) {
        Ok(song) => song,
        Err(e) => {
            eprintln!(
                "MML error (errnum {}): {} at offset {}",
                e.errnum, e.message, e.offset
            );
            return ExitCode::from(1);
        }
    };

    let synth = Synth::new();
    let pcm = match frames {
        Some(n) => synth.render_frames(&song, n),
        None => synth.render(&song),
    };

    println!(
        "playing {} stereo frames ({:.2}s) @ {} Hz…",
        pcm.frames(),
        pcm.frames() as f64 / pcm.sample_rate as f64,
        pcm.sample_rate
    );

    if let Err(e) = audio::play_blocking(&pcm) {
        eprintln!("audio backend error: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
