//! File commands (M6-T2): `SAVE` / `LOAD` / `FILES` / `DELETE` / `RENAME` / `CHKFILE` /
//! `PROJECT` over the device-neutral [`Storage`](crate::storage::Storage) layer (M6-T1).
//!
//! The VM owns a [`Storage`](crate::storage::Storage) (a [`MemStorage`](crate::storage::MemStorage)
//! by default; the native host can inject a real filesystem impl) and a *current project*. The
//! command handlers live on the VM (`vm::Vm::call_files` and friends) where the storage,
//! console and array operands are reachable; this module holds the pure pieces: the resource →
//! `(folder, name)` resolution and the logical DAT/TXT body codecs.
//!
//! Body model (matching the [`Storage`](crate::storage::Storage) contract — the *logical*
//! payload, not the on-disk extdata container):
//! - **TXT** (`TXT:` resources, programs) — UTF-8 bytes of the string.
//! - **DAT** (`DAT:` numeric arrays) — a self-describing `"SBDA"` blob (see [`encode_dat`]).
//!
//! The exact real-SmileBASIC PCBN element-type tagging (how int/double/ushort arrays and their
//! dimensions are laid out so a `DAT:` file round-trips byte-for-byte with real SB / the
//! corpus) is **queued (O-T3)** — see `HARVEST_QUEUE.md`. Until then, loading a non-`SBDA`
//! body (e.g. a real corpus PCBN blob) raises Illegal file format (errnum 35) rather than
//! decoding garbage, and the in-interpreter `SAVE`/`LOAD` round-trip uses the `SBDA` codec.

use crate::builtins::ERR_TYPE_MISMATCH;
use crate::storage::{ResourceKind, ResourceSpec};
use crate::value::{RuntimeError, SbStr, Value};

/// Errnum 35 — Illegal file format (a body SmileBASIC cannot load into the target resource).
pub(crate) const ERR_ILLEGAL_FILE_FORMAT: u32 = 35;
/// Errnum 44 — Can't use in program (DIRECT-mode-only command run from a program).
pub(crate) const ERR_CANT_USE_IN_PROGRAM: u32 = 44;

/// Magic prefix for our self-describing DAT body codec (see [`encode_dat`]).
const DAT_MAGIC: &[u8; 4] = b"SBDA";

/// Resolve a parsed [`ResourceSpec`] to its concrete [`ResourceKind`]. A bare name (no
/// `TYPE:` prefix) defaults to the **current program slot** — `SAVE "NAME"` / `LOAD "NAME"`
/// target the running program slot, which lives in the `TXT` folder alongside `TXT:` and
/// program resources (concept §1).
pub(crate) fn resolve_kind(spec: ResourceSpec, slot: u8) -> ResourceKind {
    match spec {
        ResourceSpec::Bare => ResourceKind::Program(slot),
        ResourceSpec::Kind(k) => k,
    }
}

/// Encode a UTF-16 string into its TXT body (UTF-8 bytes, per the `SAVE "TXT:…"` contract).
pub(crate) fn encode_txt(s: &SbStr) -> Vec<u8> {
    String::from_utf16_lossy(s).into_bytes()
}

/// Decode a TXT body (UTF-8 bytes) back into a UTF-16 string.
pub(crate) fn decode_txt(body: &[u8]) -> SbStr {
    String::from_utf8_lossy(body).encode_utf16().collect()
}

/// Encode a numeric array [`Value`] into a DAT body: `"SBDA"` magic, a 1-byte element tag
/// (`0` = Integer/`i32`, `1` = Double/`f64`), a `u32` LE element count, then the flat
/// row-major elements (LE). A non-numeric-array operand raises Type mismatch (errnum 8) —
/// `DAT:` files hold numeric data only.
pub(crate) fn encode_dat(v: &Value) -> Result<Vec<u8>, RuntimeError> {
    let mut out = Vec::new();
    out.extend_from_slice(DAT_MAGIC);
    match v {
        Value::IntArray(a) => {
            let a = a.borrow();
            out.push(0);
            out.extend_from_slice(&(a.len() as u32).to_le_bytes());
            for &x in a.as_slice() {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::RealArray(a) => {
            let a = a.borrow();
            out.push(1);
            out.extend_from_slice(&(a.len() as u32).to_le_bytes());
            for &x in a.as_slice() {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        _ => return Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    }
    Ok(out)
}

/// Decode a DAT body into the supplied numeric array, in place. A 1-D destination is extended
/// to the stored element count; a fixed-shape (multi-D) destination keeps its shape and
/// receives as many elements as fit. Element values are coerced to the destination's type
/// (Integer ⇄ Double). A body without the `"SBDA"` magic (or truncated) is Illegal file
/// format (errnum 35); a non-numeric-array destination is Type mismatch (errnum 8).
pub(crate) fn decode_dat_into(dest: &Value, body: &[u8]) -> Result<(), RuntimeError> {
    let illegal = || RuntimeError::new(ERR_ILLEGAL_FILE_FORMAT);
    if body.len() < 9 || &body[0..4] != DAT_MAGIC {
        return Err(illegal());
    }
    let tag = body[4];
    let count = u32::from_le_bytes([body[5], body[6], body[7], body[8]]) as usize;
    let mut vals: Vec<f64> = Vec::with_capacity(count);
    let mut p = 9;
    match tag {
        0 => {
            for _ in 0..count {
                let end = p + 4;
                let bytes: [u8; 4] = body.get(p..end).ok_or_else(illegal)?.try_into().unwrap();
                vals.push(i32::from_le_bytes(bytes) as f64);
                p = end;
            }
        }
        1 => {
            for _ in 0..count {
                let end = p + 8;
                let bytes: [u8; 8] = body.get(p..end).ok_or_else(illegal)?.try_into().unwrap();
                vals.push(f64::from_le_bytes(bytes));
                p = end;
            }
        }
        _ => return Err(illegal()),
    }
    match dest {
        Value::IntArray(a) => {
            let mut a = a.borrow_mut();
            let _ = a.resize(count); // 1-D auto-extend; multi-D keeps its shape (Err ignored)
            let n = a.len().min(count);
            let slice = a.as_mut_slice();
            for (i, slot) in slice.iter_mut().take(n).enumerate() {
                *slot = vals[i] as i32;
            }
        }
        Value::RealArray(a) => {
            let mut a = a.borrow_mut();
            let _ = a.resize(count);
            let n = a.len().min(count);
            let slice = a.as_mut_slice();
            slice[..n].copy_from_slice(&vals[..n]);
        }
        _ => return Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    }
    Ok(())
}

/// Map a [`crate::storage::StorageError`] to the SmileBASIC errnum a file command reports.
pub(crate) fn storage_errnum(e: &crate::storage::StorageError) -> RuntimeError {
    RuntimeError::new(e.errnum() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::SbArray;

    fn int_array(vals: &[i32]) -> Value {
        let mut a = SbArray::<i32>::new(&[vals.len() as i32]).unwrap();
        a.as_mut_slice().copy_from_slice(vals);
        Value::IntArray(a.into_ref())
    }

    #[test]
    fn txt_round_trips() {
        let s: SbStr = "héllo".encode_utf16().collect();
        assert_eq!(decode_txt(&encode_txt(&s)), s);
    }

    #[test]
    fn dat_int_round_trips() {
        let src = int_array(&[10, 20, 30]);
        let body = encode_dat(&src).unwrap();
        // A too-small 1-D destination auto-extends to the stored count.
        let dest = int_array(&[0]);
        decode_dat_into(&dest, &body).unwrap();
        if let Value::IntArray(a) = &dest {
            assert_eq!(a.borrow().as_slice(), &[10, 20, 30]);
        } else {
            panic!("dest changed type");
        }
    }

    #[test]
    fn dat_rejects_non_array() {
        assert_eq!(
            encode_dat(&Value::Int(5)).unwrap_err().errnum,
            ERR_TYPE_MISMATCH
        );
    }

    #[test]
    fn dat_rejects_foreign_body() {
        let dest = int_array(&[0, 0]);
        // A non-SBDA body (e.g. a real PCBN blob) is Illegal file format (35), not garbage.
        assert_eq!(
            decode_dat_into(&dest, b"PCBN0001....").unwrap_err().errnum,
            ERR_ILLEGAL_FILE_FORMAT
        );
    }
}
