//! Faithful "limitation stub" builtins (M6-T5): the special-hardware feature gate
//! (`XON`/`XOFF`), the microphone family (`MICSTART`/`MICSTOP`/`MICDATA`/`MICSAVE`), the
//! motion sensors (`GYROA`/`GYROV`/`GYROSYNC`/`ACCEL`), the wireless-multiplayer family
//! (`MPSTART`/`MPEND`/`MPSET`/`MPSTAT`/`MPSEND`/`MPRECV`/`MPGET`/`MPNAME$`), and the
//! `DIALOG` modal box.
//!
//! These features are NOT available in the headless interpreter (no microphone, no motion
//! sensors, no wireless peers, no Touch-Screen UI), so we reproduce their *observable*
//! behavior — the disassembled argument-shape / range / type guards and the availability
//! errors — rather than the device itself ("faithful stub", `prd/M6.md`). The pure logic
//! lives here; the VM (`call_device`) owns the [`DeviceState`] enable flags and the
//! `RESULT` system variable the routing mutates.
//!
//! ## Availability gating (the heart of the stubs)
//! - The microphone must be enabled with `XON MIC` first: until then `MICSTART`/`MICSTOP`/
//!   `MICDATA` raise **errnum 36** ("Mic is not available"). `MICSAVE` is special — its
//!   arg-count / array-type guards run BEFORE any mic-state check, so it never raises 36
//!   (hw_verified sb-oracle 2026-06-22, s_t11c).
//! - The motion sensors must be enabled with `XON MOTION` first: until then `GYROA`/`GYROV`/
//!   `GYROSYNC`/`ACCEL` raise **errnum 37** ("Motion sensor is not available")
//!   (hw_verified, s_t11b).
//! - The MP-restriction flag (`@0x305612`) gates every MP command with errnum 52 in a
//!   restricted context; on real SB 3.6.0 in DIRECT/program mode that flag is 0 (the oracle
//!   ran every MP command past it to its arg-count guard, 2026-06-23), so the stub treats MP
//!   as *reachable* and reproduces the body-pinned arg-shape/range/type errors. With no real
//!   wireless peers a session is never established: `MPSTAT()` → 0, `MPRECV` → SID −1, and
//!   the peer-indexed reads (`MPSTAT(id)`/`MPGET`/`MPNAME$`) are out of range (errnum 10),
//!   since the connected-terminal count is 0.

use super::{illegal, out_of_range, syntax_error, type_mismatch};
use crate::value::{RuntimeError, Value};

/// errnum 41 — "String too long" (`MPSEND` payload over 128 UTF-16 code units).
const ERR_STRING_TOO_LONG: u32 = 41;
/// errnum 36 — "Mic is not available" (a microphone instruction without `XON MIC`).
const ERR_MIC_UNAVAILABLE: u32 = 36;
/// errnum 37 — "Motion sensor is not available" (a motion instruction without `XON MOTION`).
const ERR_MOTION_UNAVAILABLE: u32 = 37;

fn mic_unavailable() -> RuntimeError {
    RuntimeError::new(ERR_MIC_UNAVAILABLE)
}

fn motion_unavailable() -> RuntimeError {
    RuntimeError::new(ERR_MOTION_UNAVAILABLE)
}

/// A special hardware feature toggled by `XON` / `XOFF`. The numeric codes are the synthetic
/// operands the parser emits for the `XON feature` / `XOFF feature` keyword form.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feature {
    Motion,
    Expad,
    Mic,
}

impl Feature {
    /// Map the parser-emitted feature code (0 = MOTION, 1 = EXPAD, 2 = MIC) to a [`Feature`].
    pub fn from_code(code: i32) -> Option<Feature> {
        match code {
            0 => Some(Feature::Motion),
            1 => Some(Feature::Expad),
            2 => Some(Feature::Mic),
            _ => None,
        }
    }
}

/// VM-owned enable flags for the gated hardware features (`XON`/`XOFF`). All boot disabled —
/// a fresh program has not yet declared any special feature, matching real SB where each
/// `XON` shows a one-time confirmation and flips the flag (`xon.yaml`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DeviceState {
    /// `XON MOTION` — the motion/gyro sensors (GYROA/GYROV/GYROSYNC/ACCEL).
    pub motion: bool,
    /// `XON EXPAD` — the Circle Pad Pro. Enabling it sets `RESULT` TRUE (handled in the VM).
    pub expad: bool,
    /// `XON MIC` — the microphone (MICSTART/MICSTOP/MICDATA).
    pub mic: bool,
}

impl DeviceState {
    /// Apply an `XON`/`XOFF feature` toggle.
    pub fn set(&mut self, feature: Feature, on: bool) {
        match feature {
            Feature::Motion => self.motion = on,
            Feature::Expad => self.expad = on,
            Feature::Mic => self.mic = on,
        }
    }
}

/// An integer operand range-checked to `lo..=hi` (errnum 10 outside; errnum 8 if non-numeric).
fn int_in(v: &Value, lo: i32, hi: i32) -> Result<i32, RuntimeError> {
    let n = v.to_int()?;
    if n < lo || n > hi {
        return Err(out_of_range());
    }
    Ok(n)
}

// ---- microphone (MIC) -------------------------------------------------------------------

/// `MICSTART rate, bits, seconds` — exactly 3 args, no return value; requires `XON MIC`.
/// Faithful stub: validates the disassembled arg-count (→ 4), XON-MIC (→ 36) and
/// rate/bits/seconds range (→ 10) guards, then no-ops (there is no real sampling buffer).
pub fn micstart(dev: &DeviceState, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || args.len() != 3 {
        return Err(illegal());
    }
    if !dev.mic {
        return Err(mic_unavailable());
    }
    int_in(&args[0], 0, 3)?; // rate 0..3
    int_in(&args[1], 0, 3)?; // bits 0..3 (depth = low bit)
    if args[2].to_real()? < 0.0 {
        return Err(out_of_range()); // seconds must be >= 0
    }
    Ok(())
}

/// `MICSTOP` — no arguments, no return value; requires `XON MIC`. Stub: arg-count (→ 4) and
/// XON-MIC (→ 36) guards, then a no-op (no live sampler to stop).
pub fn micstop(dev: &DeviceState, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || !args.is_empty() {
        return Err(illegal());
    }
    if !dev.mic {
        return Err(mic_unavailable());
    }
    Ok(())
}

/// `MICDATA([position])` — a function returning one waveform sample; requires `XON MIC`.
/// Stub: the result-count (→ 4), XON-MIC (→ 36) and arg-count (0/1 else → 4) guards. With no
/// microphone there is no recorded waveform, so a reachable read yields 0 (oracle-pending).
pub fn micdata(
    dev: &DeviceState,
    args: &[Value],
    wants_value: bool,
) -> Result<Value, RuntimeError> {
    if !wants_value {
        return Err(illegal()); // result-count must be 1
    }
    if !dev.mic {
        return Err(mic_unavailable());
    }
    match args.len() {
        0 => {} // current recording position
        1 => {
            args[0].to_int()?;
        } // explicit position (type-checked)
        _ => return Err(illegal()),
    }
    Ok(Value::Int(0))
}

/// `MICSAVE [[position,] count,] array` — copy the sampling buffer into a numeric array.
/// The arg-count (1/2/3 else → 4) and array-type (last arg must be an array, else → 8) guards
/// run BEFORE any mic-state check, so this never raises errnum 36 (hw_verified, s_t11c). With
/// no recorded samples (count 0), requesting a positive count/position is out of range (→ 10).
pub fn micsave(args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value {
        return Err(illegal()); // result-count must be 0 (statement)
    }
    let (lead, dest): (&[Value], &Value) = match args {
        [a] => (&[], a),
        [c, a] => (std::slice::from_ref(c), a),
        [p, c, a] => {
            // (position, count, array)
            let _ = (p, c);
            (&args[..2], a)
        }
        _ => return Err(illegal()),
    };
    if !dest.is_array() {
        return Err(type_mismatch());
    }
    // recorded sample count is 0 (no mic): any positive position/count is out of range.
    if lead.iter().any(|v| v.to_int().is_ok_and(|n| n > 0)) {
        return Err(out_of_range());
    }
    Ok(())
}

// ---- motion sensors (MOTION) ------------------------------------------------------------

/// `GYROA`/`GYROV`/`ACCEL OUT a,b,c` — no value args, exactly 3 OUT variables; requires
/// `XON MOTION`. Stub: the shape guard (→ 4) and XON-MOTION guard (→ 37), then three zeroed
/// axes (live sensor values need motion hardware, oracle-pending).
pub fn motion_read(
    dev: &DeviceState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if !args.is_empty() || ret_count != 3 {
        return Err(illegal());
    }
    if !dev.motion {
        return Err(motion_unavailable());
    }
    Ok(vec![Value::Real(0.0), Value::Real(0.0), Value::Real(0.0)])
}

/// `GYROSYNC` — no args, no return value; requires `XON MOTION`. Stub: shape guard (→ 4) and
/// XON-MOTION guard (→ 37), then a no-op (the recalibration is a hardware side-effect).
pub fn gyrosync(dev: &DeviceState, args: &[Value], ret_count: usize) -> Result<(), RuntimeError> {
    if !args.is_empty() || ret_count != 0 {
        return Err(illegal());
    }
    if !dev.motion {
        return Err(motion_unavailable());
    }
    Ok(())
}

// ---- wireless multiplayer (MP) ----------------------------------------------------------

/// `MPSTART max_users, "identifier"` — exactly 2 args, no return value. Validates the
/// max-users range (2..4 → 10), the string identifier type (→ 8) and length (1..16 → 10).
/// Offline no session is established; the VM sets `RESULT` FALSE afterwards.
pub fn mpstart(args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || args.len() != 2 {
        return Err(illegal());
    }
    int_in(&args[0], 2, 4)?; // max_users 2..4
    let len = mp_string_len(&args[1])?;
    if len == 0 || len > 16 {
        return Err(out_of_range()); // identifier length 1..16
    }
    Ok(())
}

/// `MPEND` — no arguments, no return value. Offline: arg-count guard (→ 4), then a no-op.
pub fn mpend(args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || !args.is_empty() {
        return Err(illegal());
    }
    Ok(())
}

/// `MPSET mgmt_number, value` — exactly 2 args, no return value. Validates the management
/// number range (0..8 → 10); offline the write is discarded (no peers read it).
pub fn mpset(args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || args.len() != 2 {
        return Err(illegal());
    }
    int_in(&args[0], 0, 8)?; // mgmt_number 0..8
    args[1].to_int()?; // value (integer)
    Ok(())
}

/// `MPSTAT([terminal_id])` — connection status (function, one return value). Offline: the
/// whole-session form returns 0 (never established); a peer-indexed form is out of range
/// (→ 10), since 0 terminals are connected.
pub fn mpstat(args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
    if !wants_value {
        return Err(illegal()); // result-count must be 1
    }
    match args.len() {
        0 => Ok(Value::Int(0)), // whole session: not established
        1 => {
            args[0].to_int()?;
            Err(out_of_range()) // no connected terminals → any id out of range
        }
        _ => Err(illegal()),
    }
}

/// `MPSEND "string"` — broadcast one string. Validates the arg shape (→ 4), string type
/// (→ 8), non-empty (→ 4) and length <= 128 code units (→ 41). Offline the send is a no-op
/// (delivery/overflow behavior needs real peers, oracle-pending).
pub fn mpsend(args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || args.len() != 1 {
        return Err(illegal());
    }
    let len = mp_string_len(&args[0])?;
    if len == 0 {
        return Err(illegal()); // empty string -> errnum 4
    }
    if len > 128 {
        return Err(RuntimeError::new(ERR_STRING_TOO_LONG));
    }
    Ok(())
}

/// `MPRECV OUT sid, rcv$` — pop one queued message (no value args, 2 OUT variables). Offline
/// there is never any data: SID is set to −1 and RCV$ to the empty string.
pub fn mprecv(args: &[Value], ret_count: usize) -> Result<Vec<Value>, RuntimeError> {
    if !args.is_empty() || ret_count != 2 {
        return Err(illegal());
    }
    Ok(vec![Value::Int(-1), Value::str_from("")])
}

/// `MPGET(terminal_id, management_number)` — read a peer's data slot (function, one return).
/// Offline: arg shape (→ 4), then the peer-id range check fails (→ 10) since 0 terminals are
/// connected.
pub fn mpget(args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
    if !wants_value || args.len() != 2 {
        return Err(illegal());
    }
    args[0].to_int()?;
    args[1].to_int()?;
    Err(out_of_range()) // no connected terminals
}

/// `MPNAME$(terminal_id)` — a peer's terminal-name string (function, one return). Offline:
/// arg shape (→ 4), then the peer-id range check fails (→ 10) since 0 terminals are connected.
pub fn mpname(args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
    if !wants_value || args.len() != 1 {
        return Err(illegal());
    }
    args[0].to_int()?;
    Err(out_of_range()) // no connected terminals
}

/// The UTF-16 code-unit length of a required string operand (errnum 8 if not a string).
fn mp_string_len(v: &Value) -> Result<usize, RuntimeError> {
    match v {
        Value::Str(s) => Ok(s.len()),
        _ => Err(type_mismatch()),
    }
}

// ---- DIALOG -----------------------------------------------------------------------------

/// The outcome of a [`dialog`] call: the value to store in `RESULT`, and (for the function
/// forms) the value to push as the call's result.
#[derive(Debug)]
pub struct DialogOutcome {
    /// The value written to the `RESULT` system variable.
    pub result: i32,
    /// The pushed return value for a function-form call (`None` for the statement form).
    pub push: Option<Value>,
}

/// `DIALOG text[,seltype,caption,timeout]` (statement) or its function forms. Validates the
/// arg-count (1..4 else → errnum 3) and the leading string operand (→ 8). Headless there is
/// no Touch Screen and no user, so the modal cannot be answered: the statement / confirm
/// forms resolve to RESULT 0 (Time out) and the file-name input form to RESULT −1 (Canceled)
/// with an empty string. The interactive return values are oracle-pending.
pub fn dialog(args: &[Value], wants_value: bool) -> Result<DialogOutcome, RuntimeError> {
    if args.is_empty() || args.len() > 4 {
        return Err(syntax_error()); // errnum 3
    }
    if !matches!(args[0], Value::Str(_)) {
        return Err(type_mismatch()); // the text/initial operand must be a string
    }
    // The file-name input form is distinguished by a STRING second argument.
    let filename_form = args.len() >= 2 && matches!(args[1], Value::Str(_));
    if !wants_value {
        // Statement form: shows the box, stores the outcome in RESULT (no return value).
        return Ok(DialogOutcome {
            result: 0,
            push: None,
        });
    }
    if filename_form {
        // S$ = DIALOG(initial, caption[, maxchars]) — canceled headless: RESULT -1, "".
        Ok(DialogOutcome {
            result: -1,
            push: Some(Value::str_from("")),
        })
    } else {
        // R = DIALOG(text[,seltype,...]) — timed out headless: RESULT 0, returns 0.
        Ok(DialogOutcome {
            result: 0,
            push: Some(Value::Int(0)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::str_from(text)
    }

    #[test]
    fn feature_codes_round_trip() {
        assert_eq!(Feature::from_code(0), Some(Feature::Motion));
        assert_eq!(Feature::from_code(1), Some(Feature::Expad));
        assert_eq!(Feature::from_code(2), Some(Feature::Mic));
        assert_eq!(Feature::from_code(3), None);
    }

    #[test]
    fn mic_requires_xon_mic() {
        let off = DeviceState::default();
        let on = DeviceState {
            mic: true,
            ..Default::default()
        };
        // No XON MIC -> errnum 36 (after the arg-count guard passes).
        assert_eq!(
            micstart(&off, &[Value::Int(0), Value::Int(0), Value::Int(1)], false)
                .unwrap_err()
                .errnum,
            36
        );
        assert_eq!(micstop(&off, &[], false).unwrap_err().errnum, 36);
        assert_eq!(
            micdata(&off, &[Value::Int(0)], true).unwrap_err().errnum,
            36
        );
        // With XON MIC the validated forms succeed / read 0.
        assert!(micstart(&on, &[Value::Int(0), Value::Int(0), Value::Int(1)], false).is_ok());
        assert!(micstop(&on, &[], false).is_ok());
        assert_eq!(micdata(&on, &[], true).unwrap(), Value::Int(0));
        // Arg-count guard fires before the XON-MIC check.
        assert_eq!(
            micstart(&off, &[Value::Int(0), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            4
        );
        // Range guard (XON MIC on): rate 4 is out of 0..3.
        assert_eq!(
            micstart(&on, &[Value::Int(4), Value::Int(0), Value::Int(1)], false)
                .unwrap_err()
                .errnum,
            10
        );
    }

    #[test]
    fn micsave_shape_and_array_guards() {
        // 0 / 4 args -> errnum 4; a scalar in the array slot -> errnum 8.
        assert_eq!(micsave(&[], false).unwrap_err().errnum, 4);
        assert_eq!(
            micsave(
                &[Value::Int(0), Value::Int(1), Value::Int(2), Value::Int(3)],
                false
            )
            .unwrap_err()
            .errnum,
            4
        );
        assert_eq!(
            micsave(&[Value::Int(0), Value::Int(1), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            8
        );
    }

    #[test]
    fn motion_requires_xon_motion() {
        let off = DeviceState::default();
        let on = DeviceState {
            motion: true,
            ..Default::default()
        };
        assert_eq!(motion_read(&off, &[], 3).unwrap_err().errnum, 37);
        assert_eq!(gyrosync(&off, &[], 0).unwrap_err().errnum, 37);
        // Shape guard before the XON-MOTION check.
        assert_eq!(motion_read(&off, &[], 2).unwrap_err().errnum, 4);
        assert_eq!(gyrosync(&off, &[Value::Int(0)], 0).unwrap_err().errnum, 4);
        // Enabled: three zeroed axes / a no-op resync.
        assert_eq!(motion_read(&on, &[], 3).unwrap(), vec![Value::Real(0.0); 3]);
        assert!(gyrosync(&on, &[], 0).is_ok());
    }

    #[test]
    fn mp_offline_behavior() {
        // Arg-shape / range / type guards.
        assert_eq!(mpend(&[Value::Int(1)], false).unwrap_err().errnum, 4);
        assert_eq!(mpset(&[Value::Int(5)], false).unwrap_err().errnum, 4);
        assert_eq!(
            mpset(&[Value::Int(-1), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            mpset(&[Value::Int(9), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(mpname(&[], true).unwrap_err().errnum, 4);
        assert_eq!(
            mpstart(&[Value::Int(1), s("ID")], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            mpstart(&[Value::Int(2), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            8
        );
        assert_eq!(mpsend(&[Value::Int(0)], false).unwrap_err().errnum, 8);
        // Offline session reads.
        assert_eq!(mpstat(&[], true).unwrap(), Value::Int(0)); // whole session: down
        assert_eq!(mpstat(&[Value::Int(0)], true).unwrap_err().errnum, 10);
        assert_eq!(
            mpget(&[Value::Int(0), Value::Int(0)], true)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            mprecv(&[], 2).unwrap(),
            vec![Value::Int(-1), Value::str_from("")]
        );
        // Valid no-op sends / session control.
        assert!(mpstart(&[Value::Int(2), s("ROOM")], false).is_ok());
        assert!(mpsend(&[s("HI")], false).is_ok());
        assert!(mpend(&[], false).is_ok());
    }

    #[test]
    fn dialog_arg_count_and_forms() {
        // 0 args / >4 args -> errnum 3 (Syntax error).
        assert_eq!(dialog(&[], false).unwrap_err().errnum, 3);
        assert_eq!(
            dialog(
                &[s("a"), Value::Int(0), s("b"), Value::Int(0), Value::Int(9)],
                true
            )
            .unwrap_err()
            .errnum,
            3
        );
        // Statement form -> RESULT 0, no push.
        let o = dialog(&[s("hi")], false).unwrap();
        assert_eq!(o.result, 0);
        assert!(o.push.is_none());
        // Confirm function form -> returns 0, RESULT 0.
        let o = dialog(&[s("ok?"), Value::Int(1)], true).unwrap();
        assert_eq!((o.result, o.push), (0, Some(Value::Int(0))));
        // File-name input form (string 2nd arg) -> RESULT -1, empty string.
        let o = dialog(&[s(""), s("Name?")], true).unwrap();
        assert_eq!(o.result, -1);
        assert_eq!(o.push, Some(Value::str_from("")));
    }
}
