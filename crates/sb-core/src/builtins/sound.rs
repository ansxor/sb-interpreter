//! BGM commands (M5-T3) — `BGMPLAY` / `BGMSTOP` / `BGMCHK` / `BGMVAR` / `BGMVOL` /
//! `BGMSET` / `BGMSETD` / `BGMCLEAR` over the VM-owned [`AudioState`].
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

use sb_audio::mml::{self, Song};

use crate::builtins::{illegal, out_of_range, type_mismatch};
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
                let v = if self.tracks[track].playing {
                    self.tracks[track].vars[varnum]
                } else {
                    -1
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
                                                              // Stopped → read returns -1 regardless of the stored value.
        assert_eq!(
            a.bgmvar(&[int(0), int(5)], true).unwrap(),
            Some(Value::Int(-1))
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
}
