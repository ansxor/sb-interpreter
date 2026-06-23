//! Live native audio backend (M5-T5): streams the synth's PCM to the default output device
//! through [cpal].
//!
//! The synth ([`sb_audio::synth`]) renders interleaved stereo PCM16 at a fixed
//! [`SAMPLE_RATE`](sb_audio::synth::SAMPLE_RATE) (32728 Hz). The host device wants its *own*
//! sample rate, channel count, and buffer size, pulling on its own callback thread. This
//! backend bridges the two with the pure primitives from `sb_audio::stream`:
//!
//! - a [`StereoResampler`] converts 32728 Hz → the device rate as PCM is pushed in, keeping
//!   phase continuous across pushes, and
//! - a [`PcmRing`] (shared with the cpal callback) absorbs the jitter between the 60 fps
//!   producer and the device's callback cadence, emitting silence on underrun instead of a
//!   glitch.
//!
//! cpal links a platform device library, so this whole module is desktop-only and behind the
//! `audio` feature (see the crate `Cargo.toml`); the deterministic streaming logic it relies
//! on is tested in `sb-audio`.

use std::error::Error;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use sb_audio::stream::{PcmRing, StereoResampler};
use sb_audio::synth::{Pcm, SAMPLE_RATE};

/// How many 1/60 s frames of audio the device-side ring buffers ahead of the producer. A few
/// frames of headroom let a slightly late `push` still feed the callback without underrunning;
/// too many adds latency. 8 frames ≈ 133 ms is a comfortable desktop default.
const RING_FRAMES_OF_HEADROOM: usize = 8;

/// A live audio output stream fed by [`AudioBackend::push`].
pub struct AudioBackend {
    /// Kept alive so the stream keeps playing; dropping it stops audio.
    _stream: cpal::Stream,
    /// Shared with the cpal callback: the producer pushes device-rate stereo PCM here.
    ring: Arc<Mutex<PcmRing>>,
    /// 32728 Hz → device rate, phase-continuous across pushes.
    resampler: StereoResampler,
    /// The device's output sample rate (Hz).
    device_rate: u32,
}

impl AudioBackend {
    /// Open the default output device and start an (initially silent) stream.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no default audio output device")?;
        let supported = device.default_output_config()?;
        let sample_format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let device_rate = config.sample_rate.0;
        let channels = config.channels as usize;

        // Size the ring to a few 1/60 s frames at the *device* rate (stereo).
        let frame_samples = (device_rate as usize).div_ceil(60) * 2;
        let ring = Arc::new(Mutex::new(PcmRing::with_frames(
            frame_samples / 2 * RING_FRAMES_OF_HEADROOM,
        )));

        let stream = build_stream(&device, &config, sample_format, channels, ring.clone())?;
        stream.play()?;

        Ok(AudioBackend {
            _stream: stream,
            ring,
            resampler: StereoResampler::new(SAMPLE_RATE, device_rate),
            device_rate,
        })
    }

    /// The device's output sample rate (Hz).
    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    /// Resample a chunk of synth PCM (stereo @ 32728 Hz) to the device rate and queue it. Call
    /// once per 1/60 s frame with that frame's rendered audio. Excess that doesn't fit the ring
    /// is dropped (the producer is far ahead); a dry ring plays silence (the producer is late).
    pub fn push(&mut self, pcm: &Pcm) {
        let mut resampled = Vec::new();
        if pcm.sample_rate == self.device_rate {
            resampled.extend_from_slice(&pcm.samples);
        } else {
            self.resampler.process(&pcm.samples, &mut resampled);
        }
        if let Ok(mut ring) = self.ring.lock() {
            ring.push(&resampled);
        }
    }

    /// Samples currently queued ahead of the device (for the demo's drain loop).
    pub fn queued_samples(&self) -> usize {
        self.ring.lock().map(|r| r.available()).unwrap_or(0)
    }
}

/// Build the cpal output stream for the device's sample format, wiring its callback to pull
/// device-rate stereo PCM from `ring` and spread it across the device's channels.
fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    channels: usize,
    ring: Arc<Mutex<PcmRing>>,
) -> Result<cpal::Stream, Box<dyn Error>> {
    match sample_format {
        cpal::SampleFormat::F32 => build_typed::<f32>(device, config, channels, ring),
        cpal::SampleFormat::I16 => build_typed::<i16>(device, config, channels, ring),
        cpal::SampleFormat::U16 => build_typed::<u16>(device, config, channels, ring),
        other => Err(format!("unsupported output sample format: {other:?}").into()),
    }
}

fn build_typed<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    ring: Arc<Mutex<PcmRing>>,
) -> Result<cpal::Stream, Box<dyn Error>>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    // Reused across callbacks: the stereo i16 the ring delivers, before spreading to channels.
    let mut scratch: Vec<i16> = Vec::new();
    let err_fn = |e| eprintln!("audio stream error: {e}");
    let stream = device.build_output_stream(
        config,
        move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = out.len() / channels.max(1);
            scratch.resize(frames * 2, 0);
            if let Ok(mut ring) = ring.lock() {
                ring.pull(&mut scratch);
            } else {
                scratch.iter_mut().for_each(|s| *s = 0);
            }
            for (f, frame) in out.chunks_mut(channels.max(1)).enumerate() {
                let l = scratch[2 * f] as f32 / 32768.0;
                let r = scratch[2 * f + 1] as f32 / 32768.0;
                for (ch, slot) in frame.iter_mut().enumerate() {
                    // First two channels carry L/R; a mono device gets the average; any extra
                    // channels (surround) stay silent.
                    let v = match (channels, ch) {
                        (1, _) => (l + r) * 0.5,
                        (_, 0) => l,
                        (_, 1) => r,
                        _ => 0.0,
                    };
                    *slot = T::from_sample(v);
                }
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

/// Render-and-play a finite clip, blocking until it has fully drained through the device. Used
/// by the `sb-play` demo. Returns once the clip has played (plus a short tail).
pub fn play_blocking(pcm: &Pcm) -> Result<(), Box<dyn Error>> {
    use std::thread::sleep;
    use std::time::Duration;

    let backend = AudioBackend::new()?;
    let resampled = StereoResampler::resample_all(SAMPLE_RATE, backend.device_rate(), &pcm.samples);

    let mut pos = 0;
    loop {
        if pos < resampled.len() {
            if let Ok(mut ring) = backend.ring.lock() {
                pos += ring.push(&resampled[pos..]);
            }
        }
        if pos >= resampled.len() && backend.queued_samples() == 0 {
            break;
        }
        sleep(Duration::from_millis(5));
    }
    // A short tail so the very last buffer reaches the speaker before the stream drops.
    sleep(Duration::from_millis(60));
    Ok(())
}
