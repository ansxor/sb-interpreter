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
//! - **DAT** (`DAT:` numeric arrays) — either the self-describing `"SBDA"` blob the interpreter
//!   uses for internal `SAVE`/`LOAD` round-trips, or the real-SmileBASIC `"PCBN0001"` numeric-array
//!   layout decoded by [`decode_dat_into`].
//!
//! The real-SmileBASIC `DAT:` body layout (PCBN0001 header + dimensions + row-major `f64`
//! elements) was oracle-verified on SB 3.6.0 — see `spec/concepts/file-and-extdata-format.md` §3
//! and `bd:sb-interpreter-c9d`. The in-memory `SAVE`/`LOAD` round-trip keeps the simpler `SBDA`
//! format; `PCBN` decode is used when loading files from the device/extdata/corpus.

use crate::builtins::ERR_TYPE_MISMATCH;
use crate::storage::{ResourceKind, ResourceSpec};
use crate::value::{RuntimeError, SbStr, Value};

/// Errnum 35 — Illegal file format (a body SmileBASIC cannot load into the target resource).
pub(crate) const ERR_ILLEGAL_FILE_FORMAT: u32 = 35;
/// Errnum 44 — Can't use in program (DIRECT-mode-only command run from a program).
pub(crate) const ERR_CANT_USE_IN_PROGRAM: u32 = 44;

/// Magic prefix for our self-describing DAT body codec (see [`encode_dat`]).
const DAT_MAGIC: &[u8; 4] = b"SBDA";

/// PCBN header for a real-SmileBASIC `DAT:` numeric-array body (O-T3, hw_verified).
const PCBN_MAGIC: &[u8; 4] = b"PCBN";
const PCBN_VERSION: &[u8; 4] = b"0001";
const PCBN_HEADER_LEN: usize = 28;
const PCBN_DAT_TYPE: u16 = 5;
const PCBN_MAX_RANK: usize = 4;

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

/// Decode a real-SmileBASIC `"PCBN0001"` numeric-array body into a value vector.
/// Returns `Illegal file format` (errnum 35) for any malformed header or truncated data.
fn decode_pcbn_values(body: &[u8]) -> Result<Vec<f64>, RuntimeError> {
    let illegal = || RuntimeError::new(ERR_ILLEGAL_FILE_FORMAT);
    if body.len() < PCBN_HEADER_LEN || &body[0..4] != PCBN_MAGIC || &body[4..8] != PCBN_VERSION {
        return Err(illegal());
    }
    let type_tag = u16::from_le_bytes([body[8], body[9]]);
    if type_tag != PCBN_DAT_TYPE {
        return Err(illegal());
    }
    let rank = u16::from_le_bytes([body[10], body[11]]) as usize;
    if rank == 0 || rank > PCBN_MAX_RANK {
        return Err(illegal());
    }
    let mut count: usize = 1;
    for i in 0..rank {
        let d = u32::from_le_bytes([
            body[12 + i * 4],
            body[13 + i * 4],
            body[14 + i * 4],
            body[15 + i * 4],
        ]);
        count = count.checked_mul(d as usize).ok_or_else(illegal)?;
    }
    let data_len = count.checked_mul(8).ok_or_else(illegal)?;
    if PCBN_HEADER_LEN + data_len > body.len() {
        return Err(illegal());
    }
    let mut vals = Vec::with_capacity(count);
    for i in 0..count {
        let off = PCBN_HEADER_LEN + i * 8;
        let bytes: [u8; 8] = body[off..off + 8].try_into().unwrap();
        vals.push(f64::from_le_bytes(bytes));
    }
    Ok(vals)
}

/// Decode a DAT body into the supplied numeric array, in place. A 1-D destination is extended
/// to the stored element count; a fixed-shape (multi-D) destination keeps its shape and
/// receives as many elements as fit. Element values are coerced to the destination's type
/// (Integer ⇄ Double). Accepts both the interpreter's internal `"SBDA"` format and the
/// real-SmileBASIC `"PCBN0001"` format; anything else (or a truncated header/body) raises
/// Illegal file format (errnum 35). A non-numeric-array destination is Type mismatch (errnum 8).
pub(crate) fn decode_dat_into(dest: &Value, body: &[u8]) -> Result<(), RuntimeError> {
    let illegal = || RuntimeError::new(ERR_ILLEGAL_FILE_FORMAT);
    let vals: Vec<f64> = if body.len() >= 9 && &body[0..4] == DAT_MAGIC {
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
        vals
    } else if body.len() >= PCBN_HEADER_LEN && &body[0..4] == PCBN_MAGIC {
        decode_pcbn_values(body)?
    } else {
        return Err(illegal());
    };

    let count = vals.len();
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
        // A truncated/foreign PCBN body still raises Illegal file format (35).
        assert_eq!(
            decode_dat_into(&dest, b"PCBN0001....").unwrap_err().errnum,
            ERR_ILLEGAL_FILE_FORMAT
        );
    }

    fn real_array(vals: &[f64]) -> Value {
        let mut a = SbArray::<f64>::new(&[vals.len() as i32]).unwrap();
        a.as_mut_slice().copy_from_slice(vals);
        Value::RealArray(a.into_ref())
    }

    fn int_2d_array(d0: i32, d1: i32) -> Value {
        let a = SbArray::<i32>::new(&[d0, d1]).unwrap();
        Value::IntArray(a.into_ref())
    }

    fn make_pcbn_1d(vals: &[f64]) -> Vec<u8> {
        let mut body = Vec::with_capacity(PCBN_HEADER_LEN + vals.len() * 8);
        body.extend_from_slice(PCBN_MAGIC);
        body.extend_from_slice(PCBN_VERSION);
        body.extend_from_slice(&PCBN_DAT_TYPE.to_le_bytes());
        body.push(1);
        body.push(0); // rank = 1 u16 LE
        body.extend_from_slice(&(vals.len() as u32).to_le_bytes()); // dim0
                                                                    // Unused dimension slots and the 4-byte reserved field (all zero, matching hw_verified).
        body.extend_from_slice(&[0u8; 12]);
        for &v in vals {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body
    }

    fn make_pcbn_2d(d0: u32, d1: u32, vals: &[f64]) -> Vec<u8> {
        let mut body = Vec::with_capacity(PCBN_HEADER_LEN + vals.len() * 8);
        body.extend_from_slice(PCBN_MAGIC);
        body.extend_from_slice(PCBN_VERSION);
        body.extend_from_slice(&PCBN_DAT_TYPE.to_le_bytes());
        body.push(2);
        body.push(0); // rank = 2
        body.extend_from_slice(&d0.to_le_bytes());
        body.extend_from_slice(&d1.to_le_bytes());
        // Reserved trailing dimension slot + reserved field.
        body.extend_from_slice(&[0u8; 8]);
        for &v in vals {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body
    }

    #[test]
    fn dat_pcbn_1d_decodes_into_int_array() {
        let dest = int_array(&[0]);
        let body = make_pcbn_1d(&[10.0, -20.0, 30.0]);
        decode_dat_into(&dest, &body).unwrap();
        if let Value::IntArray(a) = &dest {
            assert_eq!(a.borrow().as_slice(), &[10, -20, 30]);
        } else {
            panic!("dest changed type");
        }
    }

    #[test]
    fn dat_pcbn_2d_decodes_row_major() {
        // DIM A[2,3] saved by SB 3.6.0 is stored as dims [2,3] and row-major values.
        let dest = int_2d_array(2, 3);
        let body = make_pcbn_2d(2, 3, &[10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
        decode_dat_into(&dest, &body).unwrap();
        if let Value::IntArray(a) = &dest {
            assert_eq!(a.borrow().get(&[0, 0]).unwrap(), 10);
            assert_eq!(a.borrow().get(&[0, 2]).unwrap(), 30);
            assert_eq!(a.borrow().get(&[1, 0]).unwrap(), 40);
            assert_eq!(a.borrow().get(&[1, 2]).unwrap(), 60);
        } else {
            panic!("dest changed type");
        }
    }

    #[test]
    fn dat_pcbn_decodes_into_real_array() {
        let dest = real_array(&[0.0, 0.0]);
        let body = make_pcbn_1d(&[1.5, -2.5]);
        decode_dat_into(&dest, &body).unwrap();
        if let Value::RealArray(a) = &dest {
            assert_eq!(a.borrow().as_slice(), &[1.5, -2.5]);
        } else {
            panic!("dest changed type");
        }
    }

    #[test]
    fn dat_pcbn_gsave_flag1_ushort_reuses_tag5() {
        // bfh (hw_verified, sb-oracle 2026-06-27): a graphics page captured into an array via
        // `GSAVE x,y,w,h,A,1` (16-bit physical RGBA5551 codes) and SAVEd to `DAT:` stores its
        // body with the SAME PCBN tag (0x0005) and f64 elements as a flag-0 (32-bit logical)
        // save — there is NO distinct ushort type tag or size bit. The 16-bit codes are just
        // f64 values (e.g. red = 0xF801 = 63489.0). The real on-disk header for a 4×4 flag-1
        // capture is reproduced byte-for-byte below; note dim1/dim2 hold garbage (0x001D001D)
        // and the reserved field is 0x00000001 — the decoder must ignore them via `rank`.
        #[rustfmt::skip]
        let header: [u8; PCBN_HEADER_LEN] = [
            0x50, 0x43, 0x42, 0x4E, // "PCBN"
            0x30, 0x30, 0x30, 0x31, // "0001"
            0x05, 0x00,             // type tag = 5
            0x01, 0x00,             // rank = 1
            0x10, 0x00, 0x00, 0x00, // dim0 = 16
            0x1D, 0x00, 0x1D, 0x00, // dim1 = garbage (rank<2)
            0x1D, 0x00, 0x1D, 0x00, // dim2 = garbage (rank<3)
            0x01, 0x00, 0x00, 0x00, // reserved = 1 (NOT zero on a real save)
        ];
        let mut body = header.to_vec();
        for _ in 0..16 {
            body.extend_from_slice(&63489.0f64.to_le_bytes()); // 0xF801, RGBA5551 red
        }
        let dest = int_array(&[0]);
        decode_dat_into(&dest, &body).unwrap();
        if let Value::IntArray(a) = &dest {
            assert_eq!(a.borrow().as_slice(), &[63489i32; 16]);
        } else {
            panic!("dest changed type");
        }
    }

    #[test]
    fn dat_pcbn_truncated_data_is_rejected() {
        let dest = int_array(&[0]);
        let mut body = make_pcbn_1d(&[1.0, 2.0]);
        body.truncate(body.len() - 4); // cut last element in half
        assert_eq!(
            decode_dat_into(&dest, &body).unwrap_err().errnum,
            ERR_ILLEGAL_FILE_FORMAT
        );
    }
}
