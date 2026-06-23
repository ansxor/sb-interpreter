//! SmileBASIC arrays (M1-T4) — 1–4D, row-major, reference types.
//!
//! SmileBASIC arrays come in three element flavours — Integer (`i32`), Double
//! (`f64`) and String — declared with `DIM`/`VAR` in 1 to 4 dimensions
//! (`spec/instructions/dim.yaml`). They are **reference types**: assigning an
//! array variable shares the underlying storage, and OUT/by-ref args pass the same
//! buffer. We model that with [`ArrayRef<T>`] = `Rc<RefCell<SbArray<T>>>`, which is
//! single-threaded and wasm-safe (no atomics, no threads — required by prd/M1.md).
//!
//! ## Layout
//!
//! Storage is a flat `Vec<T>` with **row-major** addressing: for declared sizes
//! `d0,d1,d2,d3` the element `[i0,i1,i2,i3]` lives at
//! `((i0*d1 + i1)*d2 + i2)*d3 + i3`. This matches real SmileBASIC 3.6.0: the
//! `hw_verified` `DIM POS[3,2]` case has `POS[2,1]` in range (last element, flat
//! index 5), which only holds if the **first** subscript is the slowest-varying
//! axis. (osb's `type.d` `Array` reverses this — `opIndex(i1,i2)` bounds-checks
//! `i1 >= dim[1]` and indexes `i1*dim[0]+i2`, a 3.5.0 quirk we do **not** inherit;
//! the docs/oracle win, per prd/M1.md.)
//!
//! ## Errors
//!
//! Out-of-range / negative subscripts raise **`Subscript out of range`** (errnum
//! 31, `spec/reference/errors.yaml`). Indexing with the **wrong number of
//! subscripts** (a rank mismatch) raises **`Syntax error`** (errnum 3) — hw_verified
//! on real SB 3.6.0 (`DIM Z[3,2]:A=Z[1]`→3, `DIM Z[3]:A=Z[1,1]`→3, `Z[3]`→31; see
//! `dim.yaml`). osb agrees here (`type.d` `opIndex` throws `SyntaxError` on a rank
//! mismatch).

use crate::value::RuntimeError;
use std::cell::RefCell;
use std::rc::Rc;

/// `Subscript out of range` — an index is `< 0` or `>=` the dimension size.
const ERR_SUBSCRIPT: u32 = 31;
/// `Syntax error` — used here for an array **rank** mismatch (wrong subscript count).
const ERR_SYNTAX: u32 = 3;
/// `Out of memory` — a `DIM` too large to allocate.
const ERR_OUT_OF_MEMORY: u32 = 11;
/// `Illegal function call` — e.g. `POP` on an empty array, or a bad dim count.
const ERR_ILLEGAL_FN: u32 = 4;

/// A shared, mutable SmileBASIC array. Cloning the `Rc` shares the storage, which
/// is exactly the by-reference semantics SmileBASIC arrays have.
pub type ArrayRef<T> = Rc<RefCell<SbArray<T>>>;

/// A 1–4D SmileBASIC array with a flat row-major backing store.
#[derive(Debug, Clone, PartialEq)]
pub struct SbArray<T> {
    /// Row-major element storage (`len()` == product of the active dims).
    data: Vec<T>,
    /// Declared size of each dimension; only the first [`dim_count`](Self::dim_count)
    /// entries are meaningful, the rest are `0`.
    dim: [i32; 4],
    /// Number of declared dimensions, 1–4.
    dim_count: usize,
}

impl<T: Clone + Default + PartialEq> SbArray<T> {
    /// Allocate an array from declared dimension sizes (`DIM name[d0[,d1[,d2[,d3]]]]`).
    ///
    /// `dims` must hold 1–4 entries, each `>= 0`. Elements default to `T::default()`
    /// (0 / 0.0 / "" — per `dim.yaml`: "All numeric elements default to 0; all string
    /// elements default to \"\""). A negative size or a count outside 1–4 raises
    /// `Illegal function call` (errnum 4); an allocation that overflows `usize`
    /// raises `Out of memory` (errnum 11).
    pub fn new(dims: &[i32]) -> Result<Self, RuntimeError> {
        if dims.is_empty() || dims.len() > 4 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FN));
        }
        let mut dim = [0i32; 4];
        let mut len: usize = 1;
        for (slot, &d) in dim.iter_mut().zip(dims) {
            if d < 0 {
                return Err(RuntimeError::new(ERR_ILLEGAL_FN));
            }
            *slot = d;
            len = len
                .checked_mul(d as usize)
                .ok_or_else(|| RuntimeError::new(ERR_OUT_OF_MEMORY))?;
        }
        Ok(SbArray {
            data: vec![T::default(); len],
            dim,
            dim_count: dims.len(),
        })
    }

    /// Wrap an existing 1D buffer (used by literal array construction / tests).
    pub fn from_vec(data: Vec<T>) -> Self {
        let dim = [data.len() as i32, 0, 0, 0];
        SbArray {
            data,
            dim,
            dim_count: 1,
        }
    }

    /// Number of declared dimensions (1–4).
    pub fn dim_count(&self) -> usize {
        self.dim_count
    }

    /// Declared sizes of the active dimensions.
    pub fn dims(&self) -> &[i32] {
        &self.dim[..self.dim_count]
    }

    /// Total element count (`LEN` of a 1D array; product of dims otherwise).
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the array holds no elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Resolve a subscript tuple to a flat row-major offset, bounds-checking each
    /// axis. The number of subscripts must equal [`dim_count`](Self::dim_count).
    fn flat_index(&self, idx: &[i32]) -> Result<usize, RuntimeError> {
        if idx.len() != self.dim_count {
            // rank mismatch is a Syntax error (3), not Subscript out of range (31).
            return Err(RuntimeError::new(ERR_SYNTAX));
        }
        let mut flat: usize = 0;
        for (axis, &i) in idx.iter().enumerate() {
            let size = self.dim[axis];
            if i < 0 || i >= size {
                return Err(RuntimeError::new(ERR_SUBSCRIPT));
            }
            // flat = flat * size + i, computed left (slowest axis) to right.
            flat = flat * size as usize + i as usize;
        }
        Ok(flat)
    }

    /// Read element `idx` (a 1–4 entry subscript tuple). Out-of-range → errnum 31.
    pub fn get(&self, idx: &[i32]) -> Result<T, RuntimeError> {
        let flat = self.flat_index(idx)?;
        Ok(self.data[flat].clone())
    }

    /// Write `v` to element `idx`. Out-of-range → errnum 31.
    pub fn set(&mut self, idx: &[i32], v: T) -> Result<(), RuntimeError> {
        let flat = self.flat_index(idx)?;
        self.data[flat] = v;
        Ok(())
    }

    /// Borrow the flat backing store (row-major) — used by `COPY`/`FILL`/`SORT`.
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Mutably borrow the flat backing store.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    // --- stack/queue ops (PUSH/POP/SHIFT/UNSHIFT) — 1D arrays only -------------
    //
    // SmileBASIC's PUSH/POP/SHIFT/UNSHIFT operate on one-dimensional arrays
    // (`spec/instructions/{push,pop,shift,unshift}.yaml`). Applying them to a
    // multi-dimensional array raises Illegal function call (errnum 4); the exact
    // errnum is oracle-queued. POP/SHIFT of an empty array also raise errnum 4.

    /// Append `v` to the end and grow dimension 0 by one (`PUSH`). 1D only.
    pub fn push(&mut self, v: T) -> Result<(), RuntimeError> {
        self.require_1d()?;
        self.data.push(v);
        self.dim[0] += 1;
        Ok(())
    }

    /// Remove and return the last element, shrinking dimension 0 (`POP`). 1D only;
    /// empty → errnum 4.
    pub fn pop(&mut self) -> Result<T, RuntimeError> {
        self.require_1d()?;
        let v = self
            .data
            .pop()
            .ok_or_else(|| RuntimeError::new(ERR_ILLEGAL_FN))?;
        self.dim[0] -= 1;
        Ok(v)
    }

    /// Remove and return the first element, shifting the rest down (`SHIFT`). 1D
    /// only; empty → errnum 4.
    pub fn shift(&mut self) -> Result<T, RuntimeError> {
        self.require_1d()?;
        if self.data.is_empty() {
            return Err(RuntimeError::new(ERR_ILLEGAL_FN));
        }
        let v = self.data.remove(0);
        self.dim[0] -= 1;
        Ok(v)
    }

    /// Insert `v` at the front, shifting the rest up (`UNSHIFT`). 1D only.
    pub fn unshift(&mut self, v: T) -> Result<(), RuntimeError> {
        self.require_1d()?;
        self.data.insert(0, v);
        self.dim[0] += 1;
        Ok(())
    }

    /// Grow or shrink a 1D array to `new_len` (truncating, or padding with
    /// defaults). Backs the stack ops; not directly an instruction.
    pub fn resize(&mut self, new_len: usize) -> Result<(), RuntimeError> {
        self.require_1d()?;
        self.data.resize(new_len, T::default());
        self.dim[0] = new_len as i32;
        Ok(())
    }

    /// Wrap into a shared [`ArrayRef`].
    pub fn into_ref(self) -> ArrayRef<T> {
        Rc::new(RefCell::new(self))
    }

    fn require_1d(&self) -> Result<(), RuntimeError> {
        if self.dim_count == 1 {
            Ok(())
        } else {
            Err(RuntimeError::new(ERR_ILLEGAL_FN))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_d_declare_defaults_zero() {
        // DIM B[3] -> B[0..2] all 0 (dim.yaml hw_verified array_default_zero).
        let a = SbArray::<i32>::new(&[3]).unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!(a.dim_count(), 1);
        assert_eq!(a.get(&[0]).unwrap(), 0);
        assert_eq!(a.get(&[2]).unwrap(), 0);
    }

    #[test]
    fn one_d_set_get() {
        // DIM A[5]: A[2]=7 -> 7 (dim.yaml hw_verified array_1d_declare).
        let mut a = SbArray::<i32>::new(&[5]).unwrap();
        a.set(&[2], 7).unwrap();
        assert_eq!(a.get(&[2]).unwrap(), 7);
    }

    #[test]
    fn two_d_row_major() {
        // DIM POS[3,2]: POS[2,1]=9 -> 9 (dim.yaml hw_verified array_2d). Natural
        // row-major: [2,1] is flat 5, the last element of a 3*2 buffer.
        let mut a = SbArray::<i32>::new(&[3, 2]).unwrap();
        assert_eq!(a.len(), 6);
        assert_eq!(a.dim_count(), 2);
        a.set(&[2, 1], 9).unwrap();
        assert_eq!(a.get(&[2, 1]).unwrap(), 9);
        // Verify the exact flat slot to lock in row-major addressing.
        assert_eq!(a.as_slice()[5], 9);
    }

    #[test]
    fn three_and_four_d_layout() {
        // 3D: flat = (i0*d1 + i1)*d2 + i2.  [1,2,3] in dims [2,3,4] -> (1*3+2)*4+3 = 23.
        let (i0, i1, i2) = (1usize, 2usize, 3usize);
        let (d1, d2) = (3usize, 4usize);
        let mut a = SbArray::<i32>::new(&[2, 3, 4]).unwrap();
        assert_eq!(a.len(), 24);
        a.set(&[i0 as i32, i1 as i32, i2 as i32], 42).unwrap();
        assert_eq!(a.as_slice()[(i0 * d1 + i1) * d2 + i2], 42);
        // 4D: DIM MAP[8,8,4,2] style (dim.yaml 4D example).
        let mut m = SbArray::<i32>::new(&[8, 8, 4, 2]).unwrap();
        assert_eq!(m.len(), 8 * 8 * 4 * 2);
        m.set(&[7, 7, 3, 1], 5).unwrap();
        assert_eq!(m.get(&[7, 7, 3, 1]).unwrap(), 5);
    }

    #[test]
    fn subscript_out_of_range_is_errnum_31() {
        let a = SbArray::<i32>::new(&[3]).unwrap();
        assert_eq!(a.get(&[3]).unwrap_err().errnum, 31); // == size -> OOR
        assert_eq!(a.get(&[-1]).unwrap_err().errnum, 31); // negative -> OOR
        let mut b = SbArray::<i32>::new(&[3, 2]).unwrap();
        assert_eq!(b.get(&[0, 2]).unwrap_err().errnum, 31);
        assert_eq!(b.set(&[3, 0], 1).unwrap_err().errnum, 31);
    }

    #[test]
    fn wrong_subscript_count_is_errnum_3() {
        // hw_verified: a rank mismatch is Syntax error (3), not Subscript OOR (31).
        let a = SbArray::<i32>::new(&[3, 2]).unwrap();
        assert_eq!(a.get(&[1]).unwrap_err().errnum, 3);
        assert_eq!(a.get(&[1, 1, 1]).unwrap_err().errnum, 3);
    }

    #[test]
    fn bad_dim_count_or_negative_size() {
        assert_eq!(SbArray::<i32>::new(&[]).unwrap_err().errnum, 4);
        assert_eq!(SbArray::<i32>::new(&[1, 1, 1, 1, 1]).unwrap_err().errnum, 4);
        assert_eq!(SbArray::<i32>::new(&[-1]).unwrap_err().errnum, 4);
    }

    #[test]
    fn push_pop_lifo() {
        let mut a = SbArray::<i32>::new(&[0]).unwrap();
        assert!(a.is_empty());
        a.push(10).unwrap();
        a.push(20).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a.dims(), &[2]);
        assert_eq!(a.pop().unwrap(), 20);
        assert_eq!(a.pop().unwrap(), 10);
        assert_eq!(a.pop().unwrap_err().errnum, 4); // empty POP
    }

    #[test]
    fn shift_unshift_fifo() {
        let mut a = SbArray::<i32>::new(&[0]).unwrap();
        a.push(1).unwrap();
        a.push(2).unwrap();
        a.unshift(0).unwrap(); // [0,1,2]
        assert_eq!(a.as_slice(), &[0, 1, 2]);
        assert_eq!(a.shift().unwrap(), 0); // [1,2]
        assert_eq!(a.as_slice(), &[1, 2]);
        assert_eq!(a.dims(), &[2]);
    }

    #[test]
    fn stack_ops_reject_multidim() {
        let mut a = SbArray::<i32>::new(&[2, 2]).unwrap();
        assert_eq!(a.push(1).unwrap_err().errnum, 4);
        assert_eq!(a.pop().unwrap_err().errnum, 4);
        assert_eq!(a.shift().unwrap_err().errnum, 4);
        assert_eq!(a.resize(3).unwrap_err().errnum, 4);
    }

    #[test]
    fn string_array_defaults_empty() {
        // DIM S$[3] -> "" defaults; S$[1]="hi" (dim.yaml string_array).
        let mut s = SbArray::<Vec<u16>>::new(&[3]).unwrap();
        assert_eq!(s.get(&[0]).unwrap(), Vec::<u16>::new());
        let hi: Vec<u16> = "hi".encode_utf16().collect();
        s.set(&[1], hi.clone()).unwrap();
        assert_eq!(s.get(&[1]).unwrap(), hi);
    }

    #[test]
    fn array_ref_shares_storage() {
        // Arrays are reference types: a cloned Rc sees writes through the other.
        let a = SbArray::<i32>::new(&[3]).unwrap().into_ref();
        let b = Rc::clone(&a);
        a.borrow_mut().set(&[0], 99).unwrap();
        assert_eq!(b.borrow().get(&[0]).unwrap(), 99);
    }
}
