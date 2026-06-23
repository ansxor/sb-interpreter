//! Hardware-input builtins (M4-T1) — `BUTTON` / `STICK` / `STICKEX` / `BREPEAT`, driven by
//! the VM-owned [`InputState`](crate::input::InputState).
//!
//! Like the graphics / sprite / BG commands, these route through the VM rather than the
//! stateless [`dispatch`](super::dispatch): `BUTTON`/`STICK`/`STICKEX` read the current
//! input snapshot and `BREPEAT` mutates the per-button repeat config. The function
//! (`wants_value`) and `OUT` (`out_argc`) spellings are collapsed into one `ret_count`,
//! exactly the disassembled handlers' `[r0,#0xc]` result-count check.
//!
//! ## Forms (see `spec/instructions/{button,stick,stickex,brepeat}.yaml`)
//!
//! - `BUTTON([feature[,terminal]])` — function, exactly 1 result. feature 0..3 (held /
//!   pressed-with-repeat / pressed-raw / released); a wireless terminal ID is a comms path.
//! - `STICK [terminal] OUT X,Y` / `STICKEX [terminal] OUT X,Y` — statements, exactly 2 OUT
//!   results (the analog axes as Doubles in -1.0..1.0).
//! - `BREPEAT id[,start,interval]` — statement, no result; configures `BUTTON` feature 1.
//!
//! ## Errors
//!
//! - **Illegal function call** (4): a bad result/argument *count* for the call shape, or a
//!   `BREPEAT` management ID of 10 or 13 (rejected despite passing the `< 14` guard).
//! - **Out of range** (10): a `BUTTON` feature ID ∉ 0..3, a `BREPEAT` ID ≥ 14 (or negative),
//!   or a negative `BREPEAT` start / interval.
//! - **Communication error** (52): a wireless terminal ID supplied while multiplayer is not
//!   active (the single-machine interpreter never has active wireless). Undocumented; not in
//!   the official 0..47 error table — kept out of the deterministic golden (queued).

use super::{illegal, out_of_range};
use crate::input::InputState;
use crate::value::{RuntimeError, Value};

/// errnum 52 — the undocumented wireless "Communication error" (`spec/instructions/
/// button.yaml`); raised when a terminal ID is read with no active multiplayer session.
const ERR_COMMUNICATION: u32 = 52;

fn communication_error() -> RuntimeError {
    RuntimeError::new(ERR_COMMUNICATION)
}

/// `BUTTON([feature[,terminal]])` — the hardware button bitmask under the selected feature
/// ID. Always a function with exactly one result (`ret_count == 1`, else errnum 4); the
/// argument count must be 0/1/2 (else errnum 4); feature must be 0..3 (else errnum 10). A
/// terminal ID (2-arg form) hits the wireless comms path (errnum 52 here — no active
/// multiplayer).
pub fn button(
    input: &InputState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let feature = match args {
        [] => 0,
        [f] => f.to_int()?,
        [f, _terminal] => {
            // The 2-arg dispatch tests the wireless-comms flag first; with no active
            // multiplayer it raises errnum 52 before the (otherwise valid) feature read.
            let _ = f.to_int()?;
            return Err(communication_error());
        }
        _ => return Err(illegal()),
    };
    match input.button(feature) {
        Some(mask) => Ok(vec![Value::Int(mask)]),
        None => Err(out_of_range()),
    }
}

/// Shared `STICK` / `STICKEX` body: exactly two OUT results (else errnum 4); argument count
/// 0 (this terminal) or 1 (a wireless terminal ID → errnum 52 here); the axes are Doubles.
fn read_stick(
    axes: (f64, f64),
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 2 {
        return Err(illegal());
    }
    match args {
        [] => Ok(vec![Value::Real(axes.0), Value::Real(axes.1)]),
        [terminal] => {
            // Wireless terminal form: comms inactive on a single machine (errnum 52).
            let _ = terminal.to_int()?;
            Err(communication_error())
        }
        _ => Err(illegal()),
    }
}

/// `STICK [terminal] OUT X,Y` — the Circle Pad analog axes (x right+, y up+).
pub fn stick(
    input: &InputState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    read_stick(input.stick(), args, ret_count)
}

/// `STICKEX [terminal] OUT X,Y` — the Circle Pad Pro (right stick) analog axes.
pub fn stickex(
    input: &InputState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    read_stick(input.stickex(), args, ret_count)
}

/// `BREPEAT id[,start,interval]` — configure `BUTTON` feature 1 key-repeat for one button.
/// Statement (`ret_count == 0`); takes 1 arg (id only → repeat off) or 3 args (id,start,
/// interval). ID must be a valid management number (0..9, 11, 12): ≥14 or negative → errnum
/// 10, the 10/13 reserved slots → errnum 4. start/interval are frame counts, both ≥ 0 (else
/// errnum 10); interval 0 disables repeat.
pub fn brepeat(
    input: &mut InputState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    // Argument-count gate (1 or 3) and the ID operand, matching the handler's order: the ID
    // is validated before the start/interval frame counts.
    let (id_v, timing) = match args {
        [id] => (id, None),
        [id, start, interval] => (id, Some((start, interval))),
        _ => return Err(illegal()),
    };
    let id = id_v.to_int()?;
    // The handler unsigned-compares against 14, so any negative ID reads as ≥ 14 → errnum 10.
    if !(0..14).contains(&id) {
        return Err(out_of_range());
    }
    // IDs 10 (the unused button slot) and 13 (reserved) pass the < 14 guard but are rejected.
    if id == 10 || id == 13 {
        return Err(illegal());
    }
    // The 1-arg form zeros start+interval (repeat off); the 3-arg frame counts must be ≥ 0.
    let (start, interval) = match timing {
        None => (0, 0),
        Some((start, interval)) => {
            let s = start.to_int()?;
            let iv = interval.to_int()?;
            if s < 0 || iv < 0 {
                return Err(out_of_range());
            }
            (s, iv)
        }
    };
    input.set_repeat(id as usize, start as u32, interval as u32);
    Ok(vec![])
}
