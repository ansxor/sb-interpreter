//! Sound commands over the VM-owned [`AudioState`]:
//!
//! - BGM (M5-T3): `BGMPLAY` / `BGMSTOP` / `BGMCHK` / `BGMVAR` / `BGMVOL` / `BGMSET` /
//!   `BGMSETD` / `BGMCLEAR`.
//! - SFX / voice (M5-T4): `BEEP` (preset sound effect), `TALK` / `TALKCHK` / `TALKSTOP`
//!   (synthesized speech transport), `EFCSET` / `EFCON` / `EFCOFF` / `EFCWET` (the music
//!   effector, over [`sb_audio::effects::EffectState`]), and `WAVSET` / `WAVSETA` (user MML
//!   instruments `@224`–`@255`, over [`sb_audio::effects::UserInstrument`]).
//!
//! These route over the BGM *transport* state (which user-defined tunes are registered, and
//! per-track playing/volume/internal-variable state), so the VM handles them directly like
//! the graphics/console commands rather than through the stateless `dispatch`. The audible
//! result of playback has **no deterministic emulator golden** (O-T7); what is pinned and
//! tested here is the disassembled call shape — argument counts, the 0..7 track / 0..127
//! volume / 0..42|128..255 tune / 0..7 variable ranges, the MML-compile error (errnum 47),
//! and the form selection (`BGMVAR` write-vs-read by call shape) — plus the deterministic
//! state the commands set (registered tunes, per-track playing flag, internal variables).
//!
//! Specs: `spec/instructions/{bgmplay,bgmstop,bgmchk,bgmvar,bgmvol,bgmset,bgmsetd,
//! bgmclear}.yaml` (S-T10a/b, disassembled handler bodies) and `spec/concepts/mml-grammar.md`
//! (S-C5). MML is compiled through [`sb_audio::mml`] (M5-T1); the parsed [`Song`] is the
//! registered tune, played through the synth (M5-T2) by the live backend (M5-T5).

use std::collections::BTreeMap;

use sb_audio::effects::{
    decode_waveform, Effect, EffectState, ReverbParams, UserInstrument, WetLevels,
    WAVSETA_MAX_SAMPLES,
};
use sb_audio::mml::{self, Song};

use crate::builtins::data::read_values;
use crate::builtins::{illegal, out_of_range, syntax_error, type_mismatch};
use crate::value::{RuntimeError, Value};

/// Number of BGM tracks (0..7). Up to 8 tunes may play simultaneously (`bgmplay.yaml`).
pub const TRACK_COUNT: usize = 8;
/// Number of MML internal variables per track (`$0`..`$7`, `bgmvar.yaml`).
const VAR_COUNT: usize = 8;
/// The user-defined tune band low bound (128, `bgmset.yaml`).
const USER_TUNE_LO: i32 = 128;
/// The user-defined tune band high bound (255).
const USER_TUNE_HI: i32 = 255;
/// The preset tune band high bound (0..42 inclusive, `bgmplay.yaml`).
const PRESET_TUNE_HI: i32 = 42;
/// The default per-track playback volume (full); BGMVOL / the 3-arg BGMPLAY override it.
const DEFAULT_VOLUME: i32 = 127;
/// The MML user tune that a raw `BGMPLAY "MML"` string overwrites (`bgmplay.yaml`).
const MML_SCRATCH_TUNE: u8 = 255;

/// The user-defined instrument band low bound (224, `wavset.yaml`).
const USER_INSTR_LO: i32 = 224;
/// The user-defined instrument band high bound (255).
const USER_INSTR_HI: i32 = 255;
/// The default `WAVSET`/`WAVSETA` reference pitch (69 = O4A) when omitted.
const DEFAULT_REF_PITCH: i32 = 69;
/// The `BEEP` default sound-effect number (preset 0).
const BEEP_DEFAULT_SOUND: i32 = 0;
/// The `BEEP` default volume (-1 = use the preset's own volume).
const BEEP_DEFAULT_VOLUME: i32 = -1;
/// The `BEEP` default pan pot (64 = center).
const BEEP_DEFAULT_PAN: i32 = 64;

/// The four `{sound, frequency, volume, pan}` values a `BEEP` resolves to (after defaults
/// and the pan remap), kept as the last-triggered SFX for test introspection. The audible
/// result has no deterministic golden (O-T7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BeepTrigger {
    /// The range-validated preset sound-effect number (the raw user value, pre-remap).
    pub sound: i32,
    /// The pitch shift in 1/100 of a halftone per unit (-32768..32767).
    pub frequency: i32,
    /// The playback volume 0..127, or -1 to use the preset's own volume.
    pub volume: i32,
    /// The pan pot remapped to a signed offset `pan*2 - 128` (0 → -128, 64 → 0, 127 → +126).
    pub pan: i32,
}

/// Per-track BGM transport state.
#[derive(Debug, Clone)]
struct Track {
    /// Whether the track is currently playing (set by `BGMPLAY`, cleared by `BGMSTOP`).
    playing: bool,
    /// The track mix volume 0..127 (`BGMVOL`, or the 3-arg `BGMPLAY`).
    volume: i32,
    /// The 8 MML internal variables `$0`..`$7` (`BGMVAR`).
    vars: [i32; VAR_COUNT],
}

impl Default for Track {
    fn default() -> Self {
        Track {
            playing: false,
            volume: DEFAULT_VOLUME,
            vars: [0; VAR_COUNT],
        }
    }
}

/// VM-owned BGM state (M5-T3): the registered user-defined tunes (128..255 → compiled
/// [`Song`]) plus the 8 playback tracks' transport state.
#[derive(Debug, Clone, Default)]
pub struct AudioState {
    /// User-defined tunes registered by `BGMSET`/`BGMSETD` (id 128..255 → compiled MML).
    user_tunes: BTreeMap<u8, Song>,
    /// Per-track transport state (tracks 0..7).
    tracks: [Track; TRACK_COUNT],
    /// The music-effector state (M5-T4): `EFCSET`/`EFCON`/`EFCOFF`/`EFCWET`.
    effects: EffectState,
    /// User-defined MML instruments registered by `WAVSET`/`WAVSETA` (id 224..255).
    user_instruments: BTreeMap<u8, UserInstrument>,
    /// Whether synthesized speech (`TALK`) is currently playing (`TALKCHK` reads it,
    /// `TALKSTOP` clears it).
    talk_playing: bool,
    /// The last `BEEP` that was triggered (its resolved {sound, freq, vol, pan}).
    last_beep: Option<BeepTrigger>,
}

/// Evaluate an integer argument and bounds-check it against `min..=max`, mirroring the
/// disassembled shared helper `FUN_001eec7c`: out-of-range → Out of range (errnum 10); a
/// non-numeric value → Type mismatch (errnum 8, from [`Value::to_int`]).
pub(crate) fn ranged(v: &Value, min: i32, max: i32) -> Result<i32, RuntimeError> {
    let n = v.to_int()?;
    if n < min || n > max {
        return Err(out_of_range());
    }
    Ok(n)
}

/// Resolve an optional trailing/skippable argument (`BEEP`): `None` (not given) or
/// `Some(Value::Void)` (an omitted comma slot) takes `default`; any present value is
/// validated through `check`.
fn opt_ranged(
    v: Option<&Value>,
    default: i32,
    check: impl Fn(&Value) -> Result<i32, RuntimeError>,
) -> Result<i32, RuntimeError> {
    match v {
        None | Some(Value::Void) => Ok(default),
        Some(v) => check(v),
    }
}

/// Validate a `BEEP` sound-effect number: 0..133 (documented presets) | 224..255 | 256..383
/// (engine sound banks); anything else (the 134..223 gap or > 383) → Out of range (errnum 10).
fn beep_sound_number(v: &Value) -> Result<i32, RuntimeError> {
    let n = v.to_int()?;
    if (0..=133).contains(&n) || (224..=255).contains(&n) || (256..=383).contains(&n) {
        Ok(n)
    } else {
        Err(out_of_range())
    }
}

/// Validate the four ADSR envelope operands (`WAVSET`/`WAVSETA` args 1-4), each 0..127
/// (a value >= 128, including any negative read unsigned → Out of range, errnum 10).
fn read_adsr(args: &[&Value; 4]) -> Result<[u8; 4], RuntimeError> {
    let mut out = [0u8; 4];
    for (slot, v) in out.iter_mut().zip(args.iter()) {
        *slot = ranged(v, 0, 127)? as u8;
    }
    Ok(out)
}

/// A valid tune number must be a preset (0..42) or user-defined (128..255); anything else
/// (including the 43..127 gap) → Out of range (errnum 10).
fn tune_number(v: &Value) -> Result<i32, RuntimeError> {
    let t = v.to_int()?;
    if (0..=PRESET_TUNE_HI).contains(&t) || (USER_TUNE_LO..=USER_TUNE_HI).contains(&t) {
        Ok(t)
    } else {
        Err(out_of_range())
    }
}

/// Compile an MML string into a [`Song`], mapping the parser's errnum 47 (Illegal MML) into
/// a [`RuntimeError`].
pub(crate) fn compile_mml(src: &str) -> Result<Song, RuntimeError> {
    mml::parse(src).map_err(|e| RuntimeError::new(e.errnum as u32))
}

impl AudioState {
    /// A fresh boot state: no user tunes registered, every track stopped.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a compiled tune under a user-defined id (128..255). Shared by `BGMSET`
    /// (inline MML) and `BGMSETD` (MML gathered from DATA).
    pub(crate) fn register_tune(&mut self, tune: i32, song: Song) {
        self.user_tunes.insert(tune as u8, song);
    }

    /// Start a tune playing on `track`, applying an explicit `volume` when given.
    fn play(&mut self, track: usize, volume: Option<i32>) {
        let t = &mut self.tracks[track];
        t.playing = true;
        if let Some(v) = volume {
            t.volume = v;
        }
    }

    /// `BGMPLAY` — play a registered tune or compile-and-play raw MML. Statement (a return
    /// context → errnum 4); 1..3 arguments (else errnum 4). 1 arg: a numeric value is a tune
    /// number played on track 0; a string is MML compiled into the scratch tune 255 and
    /// played on track 0 (a non-string/non-numeric value → errnum 8). 2 args: `track, tune`.
    /// 3 args: `track, tune, volume`. Track 0..7, tune 0..42|128..255, volume 0..127 (else
    /// errnum 10). See `bgmplay.yaml`.
    pub fn bgmplay(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        match args {
            [a] => match a {
                Value::Str(s) => {
                    let song = compile_mml(&String::from_utf16_lossy(s))?;
                    self.register_tune(MML_SCRATCH_TUNE as i32, song);
                    self.play(0, None);
                }
                Value::Int(_) | Value::Real(_) => {
                    tune_number(a)?;
                    self.play(0, None);
                }
                _ => return Err(type_mismatch()),
            },
            [track, tune] => {
                let track = ranged(track, 0, 7)? as usize;
                tune_number(tune)?;
                self.play(track, None);
            }
            [track, tune, volume] => {
                let track = ranged(track, 0, 7)? as usize;
                tune_number(tune)?;
                let volume = ranged(volume, 0, 127)?;
                self.play(track, Some(volume));
            }
            _ => return Err(illegal()),
        }
        Ok(())
    }

    /// `BGMSTOP` — stop BGM playback. Statement (return context → errnum 4); 0..2 arguments
    /// (else errnum 4). 0 args: stop every track. 1+ args: the special value -1 force-stops
    /// all sound (and clears the scratch tune 255); otherwise arg 0 is a track 0..7 (else
    /// errnum 10) and the optional arg 1 is a fade time in seconds (any number). See
    /// `bgmstop.yaml`.
    pub fn bgmstop(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        match args {
            [] => self.stop_all(false),
            [first, rest @ ..] if rest.len() <= 1 => {
                let v = first.to_int()?;
                if v == -1 {
                    self.stop_all(true);
                } else if (0..TRACK_COUNT as i32).contains(&v) {
                    // The optional fade time (arg 1) is a number; headless it stops at once.
                    if let [fade] = rest {
                        fade.to_real()?;
                    }
                    self.tracks[v as usize].playing = false;
                } else {
                    return Err(out_of_range());
                }
            }
            _ => return Err(illegal()),
        }
        Ok(())
    }

    /// Stop every track; `force` also clears the scratch tune 255 (the `BGMSTOP -1` path).
    fn stop_all(&mut self, force: bool) {
        for t in &mut self.tracks {
            t.playing = false;
        }
        if force {
            self.user_tunes.remove(&MML_SCRATCH_TUNE);
        }
    }

    /// `BGMCHK(track)` — query whether a track is playing. Function (statement use →
    /// errnum 4); 0 args → track 0, 1 arg → track 0..7 (else errnum 10). Returns TRUE (1)
    /// while playing, FALSE (0) when stopped. See `bgmchk.yaml`.
    pub fn bgmchk(&self, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
        if !wants_value {
            return Err(illegal());
        }
        let track = match args {
            [] => 0,
            [t] => ranged(t, 0, 7)? as usize,
            _ => return Err(illegal()),
        };
        Ok(Value::Int(self.tracks[track].playing as i32))
    }

    /// `BGMVAR` — access an MML internal variable. The form is chosen by call shape: a
    /// 3-argument statement `BGMVAR track, varnum, value` WRITES; a 2-argument function
    /// `BGMVAR(track, varnum)` READS. Track and varnum are both 0..7 (else errnum 10); any
    /// other call shape → errnum 4. The read returns the stored value during playback, or
    /// -1 when the track is stopped. Returns `Some(value)` for the read form, `None` for the
    /// write form. See `bgmvar.yaml`.
    pub fn bgmvar(
        &mut self,
        args: &[Value],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        match (wants_value, args) {
            (false, [track, varnum, value]) => {
                let track = ranged(track, 0, 7)? as usize;
                let varnum = ranged(varnum, 0, 7)? as usize;
                let value = value.to_int()?;
                self.tracks[track].vars[varnum] = value;
                Ok(None)
            }
            (true, [track, varnum]) => {
                let track = ranged(track, 0, 7)? as usize;
                let varnum = ranged(varnum, 0, 7)? as usize;
                // Stopped read returns 0 (NOT the docs' -1): the read routine guards on
                // the audio-availability counter [ctx+0x554] >= 0x100 and `movge r0,#0`
                // before touching the var array (disasm @0x1a4ea8). hw_verified 2026-06-24.
                let v = if self.tracks[track].playing {
                    self.tracks[track].vars[varnum]
                } else {
                    0
                };
                Ok(Some(Value::Int(v)))
            }
            _ => Err(illegal()),
        }
    }

    /// `BGMVOL` — set a track's playback volume (0..127). Statement (return context →
    /// errnum 4); 1 arg sets track 0, 2 args set `track, volume` (track 0..7). Out-of-range
    /// track/volume → errnum 10; any other arg count → errnum 4. See `bgmvol.yaml`.
    pub fn bgmvol(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        let (track, volume) = match args {
            [volume] => (0usize, ranged(volume, 0, 127)?),
            [track, volume] => (ranged(track, 0, 7)? as usize, ranged(volume, 0, 127)?),
            _ => return Err(illegal()),
        };
        self.tracks[track].volume = volume;
        Ok(())
    }

    /// `BGMSET tune, "MML"` — compile inline MML and register it under a user-defined tune
    /// (128..255). Statement (return context → errnum 4); exactly 2 args (else errnum 4); a
    /// tune outside 128..255 → errnum 10; a non-string MML arg → errnum 8; MML that fails to
    /// compile → errnum 47. Does not play the tune. See `bgmset.yaml`.
    pub fn bgmset(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        let [tune, mml] = args else {
            return Err(illegal());
        };
        let tune = ranged(tune, USER_TUNE_LO, USER_TUNE_HI)?;
        let song = match mml {
            Value::Str(s) => compile_mml(&String::from_utf16_lossy(s))?,
            _ => return Err(type_mismatch()),
        };
        self.register_tune(tune, song);
        Ok(())
    }

    /// `BGMCLEAR [tune]` — clear user-defined tunes. Statement (return context → errnum 4);
    /// 0 args clears every user tune, 1 arg clears one tune (128..255, else errnum 10); any
    /// other arg count → errnum 4. See `bgmclear.yaml`.
    pub fn bgmclear(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        match args {
            [] => self.user_tunes.clear(),
            [tune] => {
                let tune = ranged(tune, USER_TUNE_LO, USER_TUNE_HI)?;
                self.user_tunes.remove(&(tune as u8));
            }
            _ => return Err(illegal()),
        }
        Ok(())
    }

    // ---- SFX / voice (M5-T4) ----

    /// `BEEP [sound][,freq][,vol][,pan]` — play a preset sound effect. Statement (return
    /// context → errnum 4); 0..4 arguments (5+ → errnum 4); any argument may be omitted with
    /// an empty comma ([`Value::Void`]) to take its default. Ranges (else errnum 10): sound
    /// 0..133 | 224..255 | 256..383 (default 0), frequency -32768..32767 (default 0), volume
    /// 0..127 (default -1 = preset volume), pan 0..127 (default 64 = center), remapped to the
    /// signed offset `pan*2 - 128`. See `beep.yaml`.
    pub fn beep(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        if args.len() > 4 {
            return Err(illegal());
        }
        let sound = opt_ranged(args.first(), BEEP_DEFAULT_SOUND, beep_sound_number)?;
        let frequency = opt_ranged(args.get(1), 0, |v| ranged(v, -32768, 32767))?;
        let volume = opt_ranged(args.get(2), BEEP_DEFAULT_VOLUME, |v| ranged(v, 0, 127))?;
        let pan_pot = opt_ranged(args.get(3), BEEP_DEFAULT_PAN, |v| ranged(v, 0, 127))?;
        self.last_beep = Some(BeepTrigger {
            sound,
            frequency,
            volume,
            // The handler remaps the 0..127 pan pot to a signed offset (0 → -128, 64 → 0).
            pan: pan_pot * 2 - 128,
        });
        Ok(())
    }

    /// `TALK voice$` — generate synthesized speech from a string. Statement (return/function
    /// context → errnum 4); effectively one string operand (0 args = empty string, 2+ args →
    /// errnum 4). The `<S speed>`/`<P pitch>` inline commands are part of the string content.
    /// Speech plays asynchronously; this marks the transport playing (see [`Self::talkchk`]).
    /// See `talk.yaml`.
    pub fn talk(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        match args {
            // 0 or 1 argument is the speak path; a single operand is required to be a string.
            [] => {}
            [v] => {
                v.as_str()?;
            }
            _ => return Err(illegal()),
        }
        self.talk_playing = true;
        Ok(())
    }

    /// `TALKCHK()` — query whether synthesized speech is playing. Function (statement use →
    /// errnum 4 from the handler, after the parser's function-as-statement check); exactly 0
    /// args. Returns TRUE (1) while playing, FALSE (0) when stopped. See `talkchk.yaml`.
    pub fn talkchk(&self, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
        if !wants_value || !args.is_empty() {
            return Err(illegal());
        }
        Ok(Value::Int(self.talk_playing as i32))
    }

    /// `TALKSTOP` — stop the synthesized speech currently playing. Statement (return context
    /// → errnum 4); no arguments (any argument → errnum 4). A no-op when nothing is playing.
    /// See `talkstop.yaml`.
    pub fn talkstop(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value || !args.is_empty() {
            return Err(illegal());
        }
        self.talk_playing = false;
        Ok(())
    }

    /// `EFCSET` — select the music effect. Statement (return context → errnum 3); exactly 1
    /// argument (a preset type 0..3, else errnum 10) OR exactly 7 arguments (the raw reverb
    /// parameters: three ints with ranges 0..2000 / 0..2000 / 1..10000, then four 0.0..1.0
    /// floats). Any other argument count → errnum 3. See `efcset.yaml`.
    pub fn efcset(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(syntax_error());
        }
        match args {
            [ty] => {
                let ty = ty.to_int()?;
                if !(0..=3).contains(&ty) {
                    return Err(out_of_range());
                }
                self.effects.effect = if ty == 0 {
                    Effect::None
                } else {
                    Effect::Preset(ty as u8)
                };
            }
            [a, b, c, d, e, f, g] => {
                let params = ReverbParams {
                    reflect_time: ranged(a, 0, 2000)?,
                    reverb_delay: ranged(b, 0, 2000)?,
                    reverb_decay: ranged(c, 1, 10000)?,
                    filter1: d.to_real()?,
                    filter2: e.to_real()?,
                    reflect_gain: f.to_real()?,
                    reverb_gain: g.to_real()?,
                };
                self.effects.effect = Effect::Raw(params);
            }
            _ => return Err(syntax_error()),
        }
        Ok(())
    }

    /// `EFCON` — turn the music effector ON. Statement (return context → errnum 3); no
    /// arguments (any argument → errnum 3). The effect type is chosen with `EFCSET`. See
    /// `efcon.yaml`.
    pub fn efcon(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value || !args.is_empty() {
            return Err(syntax_error());
        }
        self.effects.enabled = true;
        Ok(())
    }

    /// `EFCOFF` — turn the music effector OFF (without discarding the selected effect type or
    /// the wet amounts). Statement (return context → errnum 3); no arguments (any argument →
    /// errnum 3). See `efcoff.yaml`.
    pub fn efcoff(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value || !args.is_empty() {
            return Err(syntax_error());
        }
        self.effects.enabled = false;
        Ok(())
    }

    /// `EFCWET beep_wet, bgm_wet, talk_wet` — set the per-source effect amounts. Statement
    /// (return context → errnum 3); exactly 3 arguments (else errnum 3), each 0..127 (else
    /// errnum 10). For BEEP/BGM the value is the amount; for TALK only `>= 64` (ON) vs `< 64`
    /// (OFF) matters. See `efcwet.yaml`.
    pub fn efcwet(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(syntax_error());
        }
        let [beep, bgm, talk] = args else {
            return Err(syntax_error());
        };
        self.effects.wet = WetLevels {
            beep: ranged(beep, 0, 127)?,
            bgm: ranged(bgm, 0, 127)?,
            talk: ranged(talk, 0, 127)?,
        };
        Ok(())
    }

    /// `WAVSET defnum, A, D, S, R, waveform$ [,ref_pitch]` — define a user MML instrument
    /// (224..255) from a hexadecimal waveform string. Statement (return context → errnum 4);
    /// exactly 6 or 7 arguments (else errnum 4). defnum 224..255 and ref_pitch 0..127 (else
    /// errnum 10); A/D/S/R each 0..127 (else errnum 10); the waveform must be a string (else
    /// errnum 8) of 16/32/64/128/256/512 hex samples (a non-hex char / bad length → errnum 4,
    /// Illegal function call). See `wavset.yaml`.
    pub fn wavset(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        let (defnum, adsr, waveform, ref_pitch) = match args {
            [defnum, a, d, s, r, waveform] => (defnum, [a, d, s, r], waveform, None),
            [defnum, a, d, s, r, waveform, pitch] => (defnum, [a, d, s, r], waveform, Some(pitch)),
            _ => return Err(illegal()),
        };
        let defnum = ranged(defnum, USER_INSTR_LO, USER_INSTR_HI)?;
        let adsr = read_adsr(&adsr)?;
        let ref_pitch = match ref_pitch {
            Some(p) => ranged(p, 0, 127)?,
            None => DEFAULT_REF_PITCH,
        };
        // The waveform operand must be a string (errnum 8); a bad hex string → errnum 4.
        let Value::Str(s) = waveform else {
            return Err(type_mismatch());
        };
        let samples = decode_waveform(&String::from_utf16_lossy(s)).ok_or_else(illegal)?;
        self.user_instruments.insert(
            defnum as u8,
            UserInstrument {
                samples: samples.into_iter().map(|b| b as i32).collect(),
                adsr,
                ref_pitch: ref_pitch as u8,
            },
        );
        Ok(())
    }

    /// `WAVSETA defnum, A, D, S, R, array [,ref_pitch] [,start] [,end]` — define a user MML
    /// instrument (224..255) from a numeric sample-array slice. Statement (return context →
    /// errnum 4); 6..9 arguments (else errnum 4). defnum 224..255, ref_pitch 0..127, start/end
    /// 0..(last index), all → errnum 10 out of range; A/D/S/R 0..127 (errnum 10); the sample
    /// source must be a numeric array (errnum 8); end < start → errnum 4. Up to 16384 samples
    /// are committed. The defnum/envelope checks precede the array-type check (matching the
    /// disassembled order). See `wavseta.yaml`.
    pub fn wavseta(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value {
            return Err(illegal());
        }
        if !(6..=9).contains(&args.len()) {
            return Err(illegal());
        }
        // defnum + envelope are validated first (before the array-type check, per the handler).
        let defnum = ranged(&args[0], USER_INSTR_LO, USER_INSTR_HI)?;
        let adsr = read_adsr(&[&args[1], &args[2], &args[3], &args[4]])?;
        // The sample source (arg 5) must be a NUMERIC array (a string array → errnum 8).
        let source = &args[5];
        if !matches!(source, Value::IntArray(_) | Value::RealArray(_)) {
            return Err(type_mismatch());
        }
        let len = crate::builtins::data::elem_count(source)?;
        let last = len.saturating_sub(1) as i32;
        let ref_pitch = match args.get(6) {
            Some(p) => ranged(p, 0, 127)?,
            None => DEFAULT_REF_PITCH,
        };
        let start = match args.get(7) {
            Some(v) => ranged(v, 0, last)?,
            None => 0,
        };
        let end = match args.get(8) {
            Some(v) => ranged(v, 0, last)?,
            None => last,
        };
        if end < start {
            return Err(illegal());
        }
        let count = (end - start + 1) as usize;
        let count = count.min(WAVSETA_MAX_SAMPLES);
        let slice = read_values(source, start as usize, count)?;
        let samples: Vec<i32> = slice.iter().map(|v| v.to_int().unwrap_or(0)).collect();
        self.user_instruments.insert(
            defnum as u8,
            UserInstrument {
                samples,
                adsr,
                ref_pitch: ref_pitch as u8,
            },
        );
        Ok(())
    }

    /// Whether a user-defined tune id is currently registered (for tests / introspection).
    #[cfg(test)]
    pub(crate) fn is_registered(&self, tune: i32) -> bool {
        self.user_tunes.contains_key(&(tune as u8))
    }

    /// The configured volume of a track (for tests / introspection).
    #[cfg(test)]
    pub(crate) fn track_volume(&self, track: usize) -> i32 {
        self.tracks[track].volume
    }

    /// The music-effector state (for tests / introspection).
    #[cfg(test)]
    pub(crate) fn effects(&self) -> &EffectState {
        &self.effects
    }

    /// The last `BEEP` that was triggered (for tests / introspection).
    #[cfg(test)]
    pub(crate) fn last_beep(&self) -> Option<BeepTrigger> {
        self.last_beep
    }

    /// A registered user instrument by id (for tests / introspection).
    #[cfg(test)]
    pub(crate) fn user_instrument(&self, defnum: i32) -> Option<&UserInstrument> {
        self.user_instruments.get(&(defnum as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::sb_string;

    fn int(n: i32) -> Value {
        Value::Int(n)
    }

    #[test]
    fn bgmplay_forms_set_playing() {
        let mut a = AudioState::new();
        a.bgmplay(&[int(0)], false).unwrap(); // tune 0 on track 0
        assert!(a.bgmchk(&[int(0)], true).unwrap() == Value::Int(1));
        a.bgmplay(&[int(3), int(27), int(80)], false).unwrap(); // track 3, vol 80
        assert_eq!(a.track_volume(3), 80);
        // A raw MML string compiles into the scratch tune 255 and plays on track 0.
        a.bgmplay(&[sb_string("T120O4L4CDE")], false).unwrap();
        assert!(a.is_registered(255));
    }

    #[test]
    fn bgmplay_range_and_shape_errors() {
        let mut a = AudioState::new();
        assert_eq!(a.bgmplay(&[int(50)], false).unwrap_err().errnum, 10); // gap above 42
        assert_eq!(a.bgmplay(&[int(127)], false).unwrap_err().errnum, 10); // gap below 128
        assert_eq!(a.bgmplay(&[int(8), int(0)], false).unwrap_err().errnum, 10); // track
        assert_eq!(
            a.bgmplay(&[int(0), int(0), int(128)], false)
                .unwrap_err()
                .errnum,
            10
        ); // volume
        assert_eq!(a.bgmplay(&[int(0)], true).unwrap_err().errnum, 4); // function use
        assert_eq!(a.bgmplay(&[], false).unwrap_err().errnum, 4); // 0 args
    }

    #[test]
    fn bgmstop_clears_playing() {
        let mut a = AudioState::new();
        a.bgmplay(&[int(0), int(0)], false).unwrap();
        assert_eq!(a.bgmchk(&[int(0)], true).unwrap(), Value::Int(1));
        a.bgmstop(&[int(0)], false).unwrap();
        assert_eq!(a.bgmchk(&[int(0)], true).unwrap(), Value::Int(0));
        // Force-stop clears all tracks and the scratch tune.
        a.bgmplay(&[sb_string("C")], false).unwrap();
        a.bgmstop(&[int(-1)], false).unwrap();
        assert!(!a.is_registered(255));
        assert_eq!(a.bgmstop(&[int(8)], false).unwrap_err().errnum, 10);
        assert_eq!(a.bgmstop(&[int(0)], true).unwrap_err().errnum, 4);
    }

    #[test]
    fn bgmvar_write_then_read_while_playing() {
        let mut a = AudioState::new();
        a.bgmvar(&[int(0), int(5), int(10)], false).unwrap(); // write
                                                              // Stopped → read returns 0 regardless of the stored value (hw_verified 2026-06-24:
                                                              // the docs' "-1 when stopped" is wrong for 3.6.0; the read routine returns 0 when the
                                                              // availability guard is set, disasm @0x1a4ea8 `cmp r3,#0x100 / movge r0,#0`).
        assert_eq!(
            a.bgmvar(&[int(0), int(5)], true).unwrap(),
            Some(Value::Int(0))
        );
        a.bgmplay(&[int(0)], false).unwrap();
        assert_eq!(
            a.bgmvar(&[int(0), int(5)], true).unwrap(),
            Some(Value::Int(10))
        );
        // Range + shape.
        assert_eq!(
            a.bgmvar(&[int(8), int(0), int(1)], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            a.bgmvar(&[int(0), int(8), int(1)], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(a.bgmvar(&[int(0), int(8)], true).unwrap_err().errnum, 10);
        assert_eq!(a.bgmvar(&[int(0)], true).unwrap_err().errnum, 4); // bad shape
    }

    #[test]
    fn bgmset_compiles_and_registers() {
        let mut a = AudioState::new();
        a.bgmset(&[int(128), sb_string("CDEFG")], false).unwrap();
        assert!(a.is_registered(128));
        assert_eq!(
            a.bgmset(&[int(127), sb_string("C")], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            a.bgmset(&[int(256), sb_string("C")], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(a.bgmset(&[int(128), int(5)], false).unwrap_err().errnum, 8);
        assert_eq!(a.bgmset(&[int(128)], false).unwrap_err().errnum, 4);
        // Malformed MML → Illegal MML (47).
        assert_eq!(
            a.bgmset(&[int(128), sb_string("+R")], false)
                .unwrap_err()
                .errnum,
            47
        );
    }

    #[test]
    fn bgmclear_removes_tunes() {
        let mut a = AudioState::new();
        a.bgmset(&[int(128), sb_string("C")], false).unwrap();
        a.bgmset(&[int(129), sb_string("D")], false).unwrap();
        a.bgmclear(&[int(128)], false).unwrap();
        assert!(!a.is_registered(128));
        assert!(a.is_registered(129));
        a.bgmclear(&[], false).unwrap();
        assert!(!a.is_registered(129));
        assert_eq!(
            a.bgmclear(&[int(128), int(129)], false).unwrap_err().errnum,
            4
        );
    }

    #[test]
    fn bgmvol_sets_volume() {
        let mut a = AudioState::new();
        a.bgmvol(&[int(64)], false).unwrap();
        assert_eq!(a.track_volume(0), 64);
        a.bgmvol(&[int(3), int(100)], false).unwrap();
        assert_eq!(a.track_volume(3), 100);
        assert_eq!(a.bgmvol(&[int(128)], false).unwrap_err().errnum, 10);
        assert_eq!(a.bgmvol(&[int(8), int(64)], false).unwrap_err().errnum, 10);
        assert_eq!(a.bgmvol(&[int(64)], true).unwrap_err().errnum, 4);
    }

    // ---- SFX / voice (M5-T4) ----

    #[test]
    fn beep_resolves_defaults_and_pan_remap() {
        let mut a = AudioState::new();
        // Bare BEEP → sound 0, freq 0, vol -1 (preset), pan center → remapped offset 0.
        a.beep(&[], false).unwrap();
        let b = a.last_beep().unwrap();
        assert_eq!((b.sound, b.frequency, b.volume, b.pan), (0, 0, -1, 0));
        // `BEEP 9,,80` skips freq (default 0), sets vol 80; pan default 64 → offset 0.
        a.beep(&[int(9), Value::Void, int(80)], false).unwrap();
        let b = a.last_beep().unwrap();
        assert_eq!((b.sound, b.frequency, b.volume, b.pan), (9, 0, 80, 0));
        // Pan 127 remaps to +126 (127*2 - 128); pan 0 → -128 (full left).
        a.beep(&[int(0), int(0), int(0), int(127)], false).unwrap();
        assert_eq!(a.last_beep().unwrap().pan, 126);
        a.beep(&[int(0), int(0), int(0), int(0)], false).unwrap();
        assert_eq!(a.last_beep().unwrap().pan, -128);
    }

    #[test]
    fn beep_range_and_shape_errors() {
        let mut a = AudioState::new();
        // The two extended sound banks are legal; the 134..223 gap and >383 are not.
        a.beep(&[int(224)], false).unwrap();
        a.beep(&[int(293)], false).unwrap();
        assert_eq!(a.beep(&[int(134)], false).unwrap_err().errnum, 10);
        assert_eq!(a.beep(&[int(384)], false).unwrap_err().errnum, 10);
        assert_eq!(
            a.beep(&[int(0), int(0), int(200)], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            a.beep(&[int(0), int(0), int(0), int(0), int(0)], false)
                .unwrap_err()
                .errnum,
            4
        ); // 5+ args
        assert_eq!(a.beep(&[int(0)], true).unwrap_err().errnum, 4); // function use
    }

    #[test]
    fn talk_transport_round_trips() {
        let mut a = AudioState::new();
        // Idle → 0; TALK marks playing → 1; TALKSTOP clears → 0.
        assert_eq!(a.talkchk(&[], true).unwrap(), Value::Int(0));
        a.talk(&[sb_string("HELLO")], false).unwrap();
        assert_eq!(a.talkchk(&[], true).unwrap(), Value::Int(1));
        a.talkstop(&[], false).unwrap();
        assert_eq!(a.talkchk(&[], true).unwrap(), Value::Int(0));
        // Shape errors: TALK in a value context, TALKCHK with an arg, TALKSTOP with an arg.
        assert_eq!(a.talk(&[sb_string("HI")], true).unwrap_err().errnum, 4);
        assert_eq!(a.talkchk(&[int(0)], true).unwrap_err().errnum, 4);
        assert_eq!(a.talkstop(&[int(1)], false).unwrap_err().errnum, 4);
    }

    #[test]
    fn efcset_presets_and_raw() {
        let mut a = AudioState::new();
        a.efcset(&[int(2)], false).unwrap();
        assert_eq!(a.effects().effect, Effect::Preset(2));
        a.efcset(&[int(0)], false).unwrap();
        assert_eq!(a.effects().effect, Effect::None);
        // The 7-arg raw form commits the parameters.
        a.efcset(
            &[
                int(997),
                int(113),
                int(1265),
                Value::Real(0.1),
                Value::Real(0.0),
                Value::Real(0.2),
                Value::Real(0.1),
            ],
            false,
        )
        .unwrap();
        assert!(matches!(a.effects().effect, Effect::Raw(_)));
        // Errors: type out of 0..3 → 10; bad arg count → 3; reflect_time > 2000 → 10.
        assert_eq!(a.efcset(&[int(4)], false).unwrap_err().errnum, 10);
        assert_eq!(a.efcset(&[int(1), int(2)], false).unwrap_err().errnum, 3);
        assert_eq!(a.efcset(&[], false).unwrap_err().errnum, 3);
    }

    #[test]
    fn efcon_off_and_wet() {
        let mut a = AudioState::new();
        assert!(!a.effects().enabled);
        a.efcon(&[], false).unwrap();
        assert!(a.effects().enabled);
        a.efcoff(&[], false).unwrap();
        assert!(!a.effects().enabled);
        a.efcwet(&[int(0), int(100), int(64)], false).unwrap();
        assert_eq!(a.effects().wet.bgm, 100);
        assert!(a.effects().wet.talk_on()); // 64 → ON
                                            // Shape / range errors.
        assert_eq!(a.efcon(&[int(1)], false).unwrap_err().errnum, 3);
        assert_eq!(a.efcwet(&[int(0), int(0)], false).unwrap_err().errnum, 3);
        assert_eq!(
            a.efcwet(&[int(128), int(0), int(0)], false)
                .unwrap_err()
                .errnum,
            10
        );
    }

    #[test]
    fn wavset_registers_user_instrument() {
        let mut a = AudioState::new();
        a.wavset(
            &[
                int(224),
                int(3),
                int(10),
                int(30),
                int(5),
                sb_string("7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF"),
            ],
            false,
        )
        .unwrap();
        let instr = a.user_instrument(224).unwrap();
        assert_eq!(instr.samples.len(), 16);
        assert_eq!(instr.adsr, [3, 10, 30, 5]);
        assert_eq!(instr.ref_pitch, 69); // defaulted
                                         // Errors: defnum out of 224..255 → 10; A > 127 → 10; non-string waveform → 8;
                                         // a malformed hex string → 4; wrong arg count → 4.
        let wf = sb_string("7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF");
        assert_eq!(
            a.wavset(
                &[int(223), int(3), int(10), int(30), int(5), wf.clone()],
                false
            )
            .unwrap_err()
            .errnum,
            10
        );
        assert_eq!(
            a.wavset(
                &[int(224), int(128), int(10), int(30), int(5), wf.clone()],
                false
            )
            .unwrap_err()
            .errnum,
            10
        );
        assert_eq!(
            a.wavset(&[int(224), int(3), int(10), int(30), int(5), int(5)], false)
                .unwrap_err()
                .errnum,
            8
        );
        assert_eq!(
            a.wavset(
                &[int(224), int(3), int(10), int(30), int(5), sb_string("ZZ")],
                false
            )
            .unwrap_err()
            .errnum,
            4
        );
        assert_eq!(
            a.wavset(&[int(224), int(3), int(10), int(30), int(5)], false)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn wavseta_non_array_and_arg_count_errors() {
        let mut a = AudioState::new();
        // A non-array sample source is Type mismatch (8), after the defnum/envelope checks.
        assert_eq!(
            a.wavseta(
                &[int(224), int(0), int(95), int(100), int(20), int(12345)],
                false
            )
            .unwrap_err()
            .errnum,
            8
        );
        // defnum out of 224..255 fires before the (scalar) array-type check → 10.
        assert_eq!(
            a.wavseta(
                &[int(223), int(0), int(95), int(100), int(20), int(0)],
                false
            )
            .unwrap_err()
            .errnum,
            10
        );
        // Too few arguments → Illegal function call (4).
        assert_eq!(
            a.wavseta(&[int(224), int(0), int(95), int(100), int(20)], false)
                .unwrap_err()
                .errnum,
            4
        );
    }
}
