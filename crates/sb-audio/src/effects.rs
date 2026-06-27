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

/// The fixed sample count a *bracketed* `WAVSET` waveform expands to: the handler loads
/// `#0x200` (512) at `@0x1a21c4` and divides it among the bracket groups, tiling each group's
/// bytes to fill its `512 / group_count` slice (`wavset.yaml`).
pub const WAVSET_BRACKET_SAMPLES: usize = 512;

/// Decode a `WAVSET` hexadecimal waveform string into its 8-bit unsigned samples (two hex
/// characters per sample; `&H00`–`&HFF`, with `&H80` = the zero/center level). Returns `None`
/// for any string the handler rejects with errnum 4 (Illegal function call).
///
/// Two forms are accepted (matched to real SB 3.6.0, `wavset.yaml`):
///
/// * **Plain** — all hex, an even length, sample count in [`WAVSET_SAMPLE_COUNTS`].
/// * **Bracketed** — one or more `[evenhex]` repeat groups (each non-empty, whole-byte, not
///   nested) at the *start* of the string, optionally followed by trailing whole-byte hex.
///   The first `[` must be the first character (plain hex *before* a bracket → errnum 4); a
///   `]` with no open bracket, an unclosed `[`, an empty `[]`, an odd-length group, or a
///   nested `[` all → errnum 4. Each group is tiled to fill `512 / group_count` samples (the
///   exact buffer is `disassembled`, not `hw_verified` — the audio output it feeds has no
///   deterministic golden, O-T7).
pub fn decode_waveform(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if bytes.iter().any(|&b| b == b'[' || b == b']') {
        decode_waveform_bracketed(bytes)
    } else {
        decode_waveform_plain(bytes)
    }
}

/// Decode a plain (no-bracket) `WAVSET` waveform: even length, a sample count in
/// [`WAVSET_SAMPLE_COUNTS`], all hex; otherwise `None` (errnum 4).
fn decode_waveform_plain(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let samples = bytes.len() / 2;
    if !WAVSET_SAMPLE_COUNTS.contains(&samples) {
        return None;
    }
    let mut out = Vec::with_capacity(samples);
    for pair in bytes.chunks_exact(2) {
        out.push(hex_byte(pair)?);
    }
    Some(out)
}

/// Decode a bracketed `WAVSET` waveform (see [`decode_waveform`] for the accepted shape).
fn decode_waveform_bracketed(bytes: &[u8]) -> Option<Vec<u8>> {
    // The first `[` must open the string (a leading plain run, or a `]` before any `[`, is
    // rejected at `@0x1a2110` / `@0x1a20fc`).
    if bytes.first() != Some(&b'[') {
        return None;
    }
    let mut groups: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;
    while bytes.get(i) == Some(&b'[') {
        i += 1; // consume '['
        let start = i;
        while let Some(&c) = bytes.get(i) {
            if c == b']' {
                break;
            }
            if c == b'[' {
                return None; // nesting is rejected
            }
            i += 1;
        }
        if bytes.get(i) != Some(&b']') {
            return None; // unclosed '['
        }
        let body = &bytes[start..i];
        i += 1; // consume ']'
        if body.is_empty() || !body.len().is_multiple_of(2) {
            return None; // empty `[]` or odd-length group
        }
        let mut g = Vec::with_capacity(body.len() / 2);
        for pair in body.chunks_exact(2) {
            g.push(hex_byte(pair)?);
        }
        groups.push(g);
    }
    if groups.is_empty() {
        return None;
    }
    // Any content after the final group must be whole-byte hex (no stray `[`/`]`).
    let trailing = &bytes[i..];
    if !trailing.len().is_multiple_of(2) {
        return None;
    }
    let mut trail = Vec::with_capacity(trailing.len() / 2);
    for pair in trailing.chunks_exact(2) {
        trail.push(hex_byte(pair)?);
    }
    // Tile each group across its 512 / group_count slice, then append the trailing samples.
    let per = WAVSET_BRACKET_SAMPLES / groups.len();
    let mut out = Vec::with_capacity(WAVSET_BRACKET_SAMPLES + trail.len());
    for g in &groups {
        for k in 0..per {
            out.push(g[k % g.len()]);
        }
    }
    out.extend_from_slice(&trail);
    Some(out)
}

/// Decode a two-byte ASCII hex pair (`hi`, `lo`) to its 8-bit value, or `None` if either
/// character is not a hex digit.
fn hex_byte(pair: &[u8]) -> Option<u8> {
    Some((hex_digit(pair[0])? << 4) | hex_digit(pair[1])?)
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
        // A bracket group followed by an odd trailing run (`8`) is still rejected.
        assert!(decode_waveform("[7F]8").is_none());
    }

    #[test]
    fn decode_waveform_bracket_repeat_groups() {
        // A single 1-byte group tiles to fill 512 samples (`@0x1a21c4` #0x200, hw_verified
        // accept on SB 3.6.0: harness/harvest/out/cyf_wavset_hex.tsv).
        let w = decode_waveform("[7F]").unwrap();
        assert_eq!(w.len(), 512);
        assert!(w.iter().all(|&b| b == 0x7F));
        // Two groups split 256/256.
        let w = decode_waveform("[7F][FF]").unwrap();
        assert_eq!(w.len(), 512);
        assert_eq!(w[0], 0x7F);
        assert_eq!(w[255], 0x7F);
        assert_eq!(w[256], 0xFF);
        // A 16-byte group and trailing whole-byte hex are both accepted.
        assert!(decode_waveform("[7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF]").is_some());
        assert!(decode_waveform("[7F]7F7F7F7F").is_some());
    }

    #[test]
    fn decode_waveform_bracket_rejects() {
        // All errnum-4 on SB 3.6.0 (harness/harvest/out/cyf_wavset_{hex,brk2}.tsv).
        assert!(decode_waveform("[7F7]").is_none()); // odd hex inside a group
        assert!(decode_waveform("[[7F]]").is_none()); // nested
        assert!(decode_waveform("[]").is_none()); // empty group
        assert!(decode_waveform("[7F").is_none()); // unclosed
        assert!(decode_waveform("7F]").is_none()); // `]` with no open
        assert!(decode_waveform("7F7F[7F]").is_none()); // plain hex before the first bracket
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
