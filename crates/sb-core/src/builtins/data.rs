//! Array data-ops builtins (M1-T14) — `SORT` / `RSORT`.
//!
//! Unlike the pure math/string builtins, these mutate their array arguments **in
//! place**: each array reaches us as a shared [`ArrayRef`](crate::array::ArrayRef)
//! handle (the compiler pushes a bare array name as its cloned `Rc`), so reordering
//! the backing store is visible through the caller's variable. They take the raw
//! argument values and return no result.
//!
//! ## Contract (`spec/instructions/{sort,rsort}.yaml`, hw_verified)
//!
//! `SORT [start, count,] key[, parallel...]` — ascending; `RSORT ...` — descending.
//! The optional leading `start,count` pair is detected by argument **type**: leading
//! numeric scalars are the pair, the first array operand is the key. The key array
//! (Integer, Double, or String) determines a permutation of its `[start, start+count)`
//! subrange; that **same** permutation is applied to the key and up to 7 parallel
//! arrays (`array2..array8`). `SORT` is a **stable ascending** sort; `RSORT` is the
//! **exact reverse** of `SORT` (NOT a stable descending sort), so equal keys end up in
//! *reverse* original order (hw_verified sb-oracle 2026-06-23; `otya_test.sb3`'s
//! `STABLE[R]SORTTEST` agrees).
//!
//! Errors: wrong argument count / shape → Illegal function call (4); a non-array key
//! or parallel → Type mismatch (8); an out-of-range `start`/`count` or a parallel
//! array shorter than the sorted range → Out of range (10).

use super::{illegal, out_of_range, type_mismatch};
use crate::array::SbArray;
use crate::value::{RuntimeError, Value};
use std::cmp::Ordering;

/// Dispatch `SORT` (`descending = false`) / `RSORT` (`descending = true`).
pub(crate) fn sort(args: &[Value], descending: bool) -> Result<(), RuntimeError> {
    // Peel the optional leading numeric `start,count` pair off the front; everything
    // from the first array onward is a key/parallel array operand.
    let split = args.iter().take_while(|v| is_numeric(v)).count();
    let (nums, arrays) = args.split_at(split);
    let (start, count_opt) = match nums {
        [] => (0usize, None),
        [s, c] => {
            let (s, c) = (s.to_int()?, c.to_int()?);
            if s < 0 || c < 0 {
                return Err(out_of_range());
            }
            (s as usize, Some(c as usize))
        }
        // A lone start without a count (or three+ leading numbers) is malformed.
        _ => return Err(illegal()),
    };
    // The key plus up to 7 parallel arrays (array1..array8).
    if arrays.is_empty() || arrays.len() > 8 {
        return Err(illegal());
    }

    let key_len = array_len(&arrays[0])?;
    let count = count_opt.unwrap_or_else(|| key_len.saturating_sub(start));
    let end = start.checked_add(count).ok_or_else(out_of_range)?;
    if end > key_len {
        return Err(out_of_range());
    }

    // The permutation comes from the key array's `[start, end)` subrange (stable),
    // then is applied to every array's matching subrange.
    let perm = key_perm(&arrays[0], start, end, descending)?;
    for arr in arrays {
        apply_perm(arr, start, end, &perm)?;
    }
    Ok(())
}

/// A leading `start`/`count` operand is a numeric scalar (Integer or Double).
fn is_numeric(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Real(_))
}

/// Element count of an array operand; a non-array raises Type mismatch (8).
fn array_len(v: &Value) -> Result<usize, RuntimeError> {
    match v {
        Value::IntArray(a) => Ok(a.borrow().len()),
        Value::RealArray(a) => Ok(a.borrow().len()),
        Value::StrArray(a) => Ok(a.borrow().len()),
        _ => Err(type_mismatch()),
    }
}

/// Build the stable sort permutation of the key array's `[start, end)` subrange.
/// `perm[k]` is the subrange-local offset of the element that lands at position `k`.
fn key_perm(
    v: &Value,
    start: usize,
    end: usize,
    descending: bool,
) -> Result<Vec<usize>, RuntimeError> {
    match v {
        Value::IntArray(a) => Ok(perm_from(
            &a.borrow().as_slice()[start..end],
            descending,
            i32::cmp,
        )),
        Value::RealArray(a) => Ok(perm_from(
            &a.borrow().as_slice()[start..end],
            descending,
            |x, y| {
                // NaN keys are not expected from SmileBASIC arithmetic; order them as Equal.
                x.partial_cmp(y).unwrap_or(Ordering::Equal)
            },
        )),
        // String arrays sort lexically by UTF-16 code unit (`Vec<u16>` ordering).
        Value::StrArray(a) => Ok(perm_from(
            &a.borrow().as_slice()[start..end],
            descending,
            |x, y| x.cmp(y),
        )),
        _ => Err(type_mismatch()),
    }
}

/// Stable ascending sort of `0..keys.len()` by `cmp`; for `descending` (`RSORT`) the
/// whole permutation is **reversed**. `RSORT` is therefore the exact reverse of `SORT`
/// — equal keys end up in *reverse* original order, NOT preserved (hw_verified
/// sb-oracle 2026-06-23: `SORT A,B` on key `2,3,1,1` → parallel `1,2,3,4`, `RSORT`
/// → `4,3,2,1`, so the two tied `1`s swap; `otya_test.sb3` STABLE[R]SORTTEST agrees).
/// Rust's `slice::sort_by` is stable, so ties hold their order before the reverse.
fn perm_from<T>(keys: &[T], descending: bool, cmp: impl Fn(&T, &T) -> Ordering) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    idx.sort_by(|&i, &j| cmp(&keys[i], &keys[j]));
    if descending {
        idx.reverse();
    }
    idx
}

/// Reorder one array's `[start, end)` subrange by `perm` (a non-array raises Type
/// mismatch (8); a subrange past the end raises Out of range (10)).
fn apply_perm(v: &Value, start: usize, end: usize, perm: &[usize]) -> Result<(), RuntimeError> {
    match v {
        Value::IntArray(a) => reorder(&mut a.borrow_mut(), start, end, perm),
        Value::RealArray(a) => reorder(&mut a.borrow_mut(), start, end, perm),
        Value::StrArray(a) => reorder(&mut a.borrow_mut(), start, end, perm),
        _ => Err(type_mismatch()),
    }
}

/// Permute `arr[start..end]` so that the new element `k` is the old element
/// `start + perm[k]`.
fn reorder<T: Clone + Default + PartialEq>(
    arr: &mut SbArray<T>,
    start: usize,
    end: usize,
    perm: &[usize],
) -> Result<(), RuntimeError> {
    if end > arr.len() {
        return Err(out_of_range());
    }
    let slice = arr.as_mut_slice();
    let reordered: Vec<T> = perm.iter().map(|&k| slice[start + k].clone()).collect();
    slice[start..end].clone_from_slice(&reordered);
    Ok(())
}
