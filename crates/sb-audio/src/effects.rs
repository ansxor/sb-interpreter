//! Sound-effect / voice support models for the M5-T4 commands (`EFCSET`/`EFCON`/`EFCOFF`/
//! `EFCWET` + `WAVSET`/`WAVSETA`).
//!
//! Two pure, device-free data models live here (no I/O, no threads → wasm32-safe):
//!
//! - [`EffectState`] — the music **effector** (a reverb): which effect is selected
//!   ([`Effect`], `EFCSET`), whether it is enabled (`EFCON`/`EFCOFF`), and the per-source
//!   wet amounts ([`WetLevels`], `EFCWET`).
//! - [`UserInstrument`] — a user-defined MML instrument (slots `@224`–`@255`) defined by
//!   `WAVSET` (a hex waveform string, decoded by [`decode_waveform`]) or `WAVSETA` (a numeric
//!   sample-array slice): its sample table, ADSR envelope and reference pitch.
//!
//! The *audible* result of any of these has **no deterministic emulator golden** (O-T7);
//! what is modeled here is the deterministic state the commands set (the selected effect,
//! the on/off flag, the wet amounts, the registered instrument table), which is what the
//! disassembled handlers' arg-shape/range contract pins (`spec/instructions/{efcset,efcon,
//! efcoff,efcwet,wavset,wavseta}.yaml`, S-T10c/d).

/// The seven raw reverb parameters of the music effector (`EFCSET` form 2, or the
/// resolved parameters of a built-in preset). Ranges per `efcset.yaml`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParams {
    /// Initial reflection time, milliseconds (0..2000).
    pub reflect_time: i32,
    /// Reverberation-sound delay time, milliseconds (0..2000).
    pub reverb_delay: i32,
    /// Reverberation-sound decay time, milliseconds (1..10000).
    pub reverb_decay: i32,
    /// Reverberation filter coefficient 1 (0.0..1.0).
    pub filter1: f64,
    /// Reverberation filter coefficient 2 (0.0..1.0).
    pub filter2: f64,
    /// Initial-reflection gain (0.0..1.0).
    pub reflect_gain: f64,
    /// Reverberation gain (0.0..1.0).
    pub reverb_gain: f64,
}

/// The selected music effect (`EFCSET`). Form 1 picks a built-in preset by number; form 2
/// supplies the raw [`ReverbParams`] directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// Type 0 — no effect (the same audible result as `EFCOFF`).
    None,
    /// Type 1/2/3 — a built-in reverb preset (1 = bathroom, 2 = cave, 3 = space). The exact
    /// reverb parameters of each preset are firmware-internal and have no golden (O-T7); the
    /// preset *number* is the deterministic contract.
    Preset(u8),
    /// `EFCSET` form 2 — the seven raw reverb parameters.
    Raw(ReverbParams),
}

/// Per-source effect (wet) amounts (`EFCWET`). Each is 0..127. For BEEP and BGM the value is
/// the effect amount; for TALK it is a threshold (`< 64` = OFF, `>= 64` = ON), per the docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WetLevels {
    /// Effect amount applied to `BEEP` sounds (0..127).
    pub beep: i32,
    /// Effect amount applied to `BGM` (0..127).
    pub bgm: i32,
    /// Effect setting for `TALK` (0..127; only `>= 64` ON / `< 64` OFF matters).
    pub talk: i32,
}

impl WetLevels {
    /// Whether the effect is applied to `TALK` (the `>= 64` ON threshold).
    pub fn talk_on(&self) -> bool {
        self.talk >= 64
    }
}

/// The music-effector state driven by `EFCSET`/`EFCON`/`EFCOFF`/`EFCWET`. A reverb selected
/// by `EFCSET` only becomes audible while [`enabled`](EffectState::enabled) is set with
/// `EFCON`; `EFCOFF` clears the flag without discarding the selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectState {
    /// Whether the effector is on (`EFCON` sets, `EFCOFF` clears). Off at boot.
    pub enabled: bool,
    /// The selected effect ([`Effect::None`] at boot).
    pub effect: Effect,
    /// The per-source wet amounts (0/0/0 at boot — no effect amount until `EFCWET`).
    pub wet: WetLevels,
}

impl Default for EffectState {
    fn default() -> Self {
        EffectState {
            enabled: false,
            effect: Effect::None,
            wet: WetLevels {
                beep: 0,
                bgm: 0,
                talk: 0,
            },
        }
    }
}

impl EffectState {
    /// A fresh boot state: effector off, no effect selected, all wet amounts 0.
    pub fn new() -> Self {
        Self::default()
    }
}

/// The sample counts a `WAVSET` hex waveform string may hold (`wavset.yaml`): a power of two
/// from 16 to 512. The string length is exactly twice the sample count.
pub const WAVSET_SAMPLE_COUNTS: [usize; 6] = [16, 32, 64, 128, 256, 512];

/// The maximum sample count `WAVSETA` commits from a source array (`wavseta.yaml`, 0x4000).
pub const WAVSETA_MAX_SAMPLES: usize = 16384;

/// Decode a `WAVSET` hexadecimal waveform string into its 8-bit unsigned samples (two hex
/// characters per sample; `&H00`–`&HFF`, with `&H80` = the zero/center level). Returns `None`
/// for a non-hex character or a sample count not in [`WAVSET_SAMPLE_COUNTS`] — the handler
/// maps either to errnum 4 (Illegal function call).
///
/// The disassembled handler also recognises bracketed (`[`/`]`) repeat groups inside the
/// string; that expansion is not modeled here (no committed case exercises it and its exact
/// semantics are unverified — tracked in beads: bd:sb-interpreter-i8p), so a `[`/`]` is treated as a
/// non-hex character.
pub fn decode_waveform(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let samples = bytes.len() / 2;
    if !WAVSET_SAMPLE_COUNTS.contains(&samples) {
        return None;
    }
    let mut out = Vec::with_capacity(samples);
    for pair in bytes.chunks_exact(2) {
        let hi = hex_digit(pair[0])?;
        let lo = hex_digit(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

/// Decode one ASCII hex digit (`0-9`, `A-F`, `a-f`) to its 0..15 value.
fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// A user-defined MML instrument (slots `@224`–`@255`) registered by `WAVSET`/`WAVSETA`.
/// The waveform is selectable in MML with the `@defnum` command (see [`crate::mml`]).
#[derive(Debug, Clone, PartialEq)]
pub struct UserInstrument {
    /// The waveform sample table. `WAVSET` stores its decoded 8-bit hex samples (0..255,
    /// `&H80` = center); `WAVSETA` stores the selected slice of the source array (raw sample
    /// values). The *audible* synthesis from these has no deterministic golden (O-T7).
    pub samples: Vec<i32>,
    /// The ADSR envelope (attack, decay, sustain, release), each 0..127.
    pub adsr: [u8; 4],
    /// The reference pitch (MIDI note), 0..127; 69 = O4A. Defaults to 69 when omitted.
    pub ref_pitch: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_waveform_basic_16_samples() {
        // 32 hex chars = 16 samples.
        let w = decode_waveform("7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF").unwrap();
        assert_eq!(w.len(), 16);
        assert_eq!(w[0], 0x7F);
        assert_eq!(w[4], 0xFF);
    }

    #[test]
    fn decode_waveform_lowercase_hex() {
        let w = decode_waveform(&"ab".repeat(16)).unwrap();
        assert_eq!(w.len(), 16);
        assert!(w.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn decode_waveform_rejects_bad_length() {
        // 10 samples is not one of 16/32/64/128/256/512.
        assert!(decode_waveform(&"7F".repeat(10)).is_none());
        // Odd number of hex characters.
        assert!(decode_waveform("7F7").is_none());
    }

    #[test]
    fn decode_waveform_rejects_non_hex() {
        assert!(decode_waveform(&"GG".repeat(16)).is_none());
        // The bracketed-repeat form is not modeled — treated as non-hex.
        assert!(decode_waveform("[7F]8").is_none());
    }

    #[test]
    fn effect_state_defaults() {
        let e = EffectState::new();
        assert!(!e.enabled);
        assert_eq!(e.effect, Effect::None);
        assert!(!e.wet.talk_on());
    }

    #[test]
    fn wet_talk_threshold() {
        let on = WetLevels {
            beep: 0,
            bgm: 0,
            talk: 64,
        };
        let off = WetLevels {
            beep: 0,
            bgm: 0,
            talk: 63,
        };
        assert!(on.talk_on());
        assert!(!off.talk_on());
    }
}
