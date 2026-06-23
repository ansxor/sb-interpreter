//! Sprite lifecycle builtins (M3-T1) — the management commands the VM drives over the
//! VM-owned [`SpriteState`](sb_render::sprite::SpriteState).
//!
//! This slice covers **creation/release/visibility/query**: `SPSET` (all six forms —
//! explicit management number or auto-allocated, from an `SPDEF` template or a direct
//! image), `SPCLR` (release one or all), `SPSHOW`/`SPHIDE` (toggle the display flag of a
//! live sprite), and `SPUSED` (is a slot in use?). The transform/char/color setters
//! (`SPOFS`/`SPCHR`/`SPSCALE`/…), animation/link/vars (M3-T2), and collision (M3-T3) build
//! on the same table.
//!
//! ## Form selection by return count
//!
//! Like the M2 graphics commands, `SPSET`'s form is chosen first by the **return-value
//! count** (`[r0,#0xc]`): 0 returns = an explicit-management-number form (1 or 2), 1
//! return = an auto-allocate `OUT`/function form (3-6). The VM collapses the `OUT`
//! (`out_argc`) and value-returning-function (`wants_value`) spellings into one `ret_count`,
//! so `SPSET 0 OUT IX`, `IX=SPSET(0)`, and `PRINT SPUSED(0)` all route correctly. See
//! `spec/instructions/{spset,spclr,spshow,sphide,spused}.yaml`.
//!
//! ## Errors
//!
//! - **Illegal function call** (4): a bad return/argument *count* for the call shape, or
//!   `SPSHOW`/`SPHIDE` on a slot that was never `SPSET` (inactive).
//! - **Out of range** (10): a management number ∉ 0..511, a definition number ∉ 0..4095,
//!   or a source rectangle that runs off the 512-pixel sheet (`U+W` / `V+H` > 512).

use sb_render::sprite::{
    SpriteState, SPDEF_MAX, SPRITE_COUNT, SPRITE_DEFAULT_ATTR, SPRITE_DEFAULT_WH,
};

use super::{illegal, out_of_range};
use crate::value::{RuntimeError, Value};

/// Validate + return a management number in 0..511 (else errnum 10).
fn mgmt(v: &Value) -> Result<usize, RuntimeError> {
    let i = v.to_int()?;
    if SpriteState::in_range(i) {
        Ok(i as usize)
    } else {
        Err(out_of_range())
    }
}

/// Validate a definition (`SPDEF` template) number in 0..4095 (else errnum 10).
fn defn(v: &Value) -> Result<i32, RuntimeError> {
    let i = v.to_int()?;
    if (0..=SPDEF_MAX).contains(&i) {
        Ok(i)
    } else {
        Err(out_of_range())
    }
}

/// Validate a direct-image source rectangle: `U,V` in 0..511, `W,H` in 1..512, and the
/// rectangle must fit on the 512-pixel sheet (`U+W <= 512`, `V+H <= 512`). Out of range →
/// errnum 10 (the documented assumption; the exact errnum is oracle-pending, see
/// `HARVEST_QUEUE.md`).
fn rect(u: i32, v: i32, w: i32, h: i32) -> Result<(), RuntimeError> {
    let ok = (0..SPRITE_COUNT as i32).contains(&u)
        && (0..SPRITE_COUNT as i32).contains(&v)
        && (1..=SPRITE_COUNT as i32).contains(&w)
        && (1..=SPRITE_COUNT as i32).contains(&h)
        && u + w <= SPRITE_COUNT as i32
        && v + h <= SPRITE_COUNT as i32;
    if ok {
        Ok(())
    } else {
        Err(out_of_range())
    }
}

/// `SPSET` — register a sprite. Six forms, chosen by `ret_count` then argument count:
///
/// - `ret_count == 0` (explicit slot, 2..6 args): form 1 `mgmt,defn`; form 2
///   `mgmt,U,V[,W,H][,attr]` (`W,H` default 16, `attr` default &H01).
/// - `ret_count == 1` (auto-allocate, returns the chosen slot or -1): form 3 `defn`; form 4
///   `U,V,W,H,attr`; form 5 `upper,lower,defn`; form 6 `upper,lower,U,V,W,H,attr`.
///
/// Any other (`ret_count`, argc) combination raises errnum 4.
pub fn spset(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            spset_explicit(sp, args)?;
            Ok(vec![])
        }
        1 => {
            let slot = spset_alloc(sp, args)?;
            Ok(vec![Value::Int(slot)])
        }
        _ => Err(illegal()),
    }
}

/// The explicit-management-number forms (1 and 2). The argument *count* is validated
/// first (2..6 args, matching the disassembled `sub argcount,#2 / cmp #5` guard — an
/// argc outside that range is errnum 4 *before* the management number is range-checked),
/// then the management number, then the form-specific params.
fn spset_explicit(sp: &mut SpriteState, args: &[Value]) -> Result<(), RuntimeError> {
    match args {
        // Form 1 — from an SPDEF template.
        [m, d] => {
            let slot = mgmt(m)?;
            let d = defn(d)?;
            sp.set_template(slot, d, SPRITE_DEFAULT_ATTR);
        }
        // Form 2 — direct image, W,H default 16, attr default &H01.
        [m, u, v] => {
            let slot = mgmt(m)?;
            let (u, v) = (u.to_int()?, v.to_int()?);
            rect(u, v, SPRITE_DEFAULT_WH, SPRITE_DEFAULT_WH)?;
            sp.set_direct(
                slot,
                u,
                v,
                SPRITE_DEFAULT_WH,
                SPRITE_DEFAULT_WH,
                SPRITE_DEFAULT_ATTR,
            );
        }
        // Form 2 — direct image with explicit attr, W,H default 16.
        [m, u, v, a] => {
            let slot = mgmt(m)?;
            let (u, v, a) = (u.to_int()?, v.to_int()?, a.to_int()?);
            rect(u, v, SPRITE_DEFAULT_WH, SPRITE_DEFAULT_WH)?;
            sp.set_direct(slot, u, v, SPRITE_DEFAULT_WH, SPRITE_DEFAULT_WH, a);
        }
        // Form 2 — direct image with W,H, attr default &H01.
        [m, u, v, w, h] => {
            let slot = mgmt(m)?;
            let (u, v, w, h) = (u.to_int()?, v.to_int()?, w.to_int()?, h.to_int()?);
            rect(u, v, w, h)?;
            sp.set_direct(slot, u, v, w, h, SPRITE_DEFAULT_ATTR);
        }
        // Form 2 — direct image, full.
        [m, u, v, w, h, a] => {
            let slot = mgmt(m)?;
            let (u, v, w, h, a) = (
                u.to_int()?,
                v.to_int()?,
                w.to_int()?,
                h.to_int()?,
                a.to_int()?,
            );
            rect(u, v, w, h)?;
            sp.set_direct(slot, u, v, w, h, a);
        }
        _ => return Err(illegal()),
    }
    Ok(())
}

/// The auto-allocate `OUT`/function forms (3-6). Returns the chosen management number, or
/// -1 if no slot in the search range is free.
fn spset_alloc(sp: &mut SpriteState, args: &[Value]) -> Result<i32, RuntimeError> {
    // (search range, image spec). The range is the whole table for forms 3/4 and the
    // inclusive [upper,lower] for forms 5/6.
    let (start, end, image) = match args {
        // Form 3 — defn, whole range.
        [d] => (0, SPRITE_COUNT - 1, Image::Template(defn(d)?)),
        // Form 4 — U,V,W,H,attr, whole range.
        [u, v, w, h, a] => {
            let img = direct_image(u, v, w, h, a)?;
            (0, SPRITE_COUNT - 1, img)
        }
        // Form 5 — upper,lower,defn.
        [up, lo, d] => (mgmt(up)?, mgmt(lo)?, Image::Template(defn(d)?)),
        // Form 6 — upper,lower,U,V,W,H,attr.
        [up, lo, u, v, w, h, a] => {
            let img = direct_image(u, v, w, h, a)?;
            (mgmt(up)?, mgmt(lo)?, img)
        }
        _ => return Err(illegal()),
    };
    match sp.alloc(start, end) {
        Some(slot) => {
            match image {
                Image::Template(d) => sp.set_template(slot, d, SPRITE_DEFAULT_ATTR),
                Image::Direct { u, v, w, h, a } => sp.set_direct(slot, u, v, w, h, a),
            }
            Ok(slot as i32)
        }
        // No free slot — the documented -1 "no available number" result.
        None => Ok(-1),
    }
}

/// A resolved image specification for an auto-allocated sprite.
enum Image {
    Template(i32),
    Direct {
        u: i32,
        v: i32,
        w: i32,
        h: i32,
        a: i32,
    },
}

/// Validate + bundle the direct-image params shared by forms 4 and 6.
fn direct_image(
    u: &Value,
    v: &Value,
    w: &Value,
    h: &Value,
    a: &Value,
) -> Result<Image, RuntimeError> {
    let (u, v, w, h, a) = (
        u.to_int()?,
        v.to_int()?,
        w.to_int()?,
        h.to_int()?,
        a.to_int()?,
    );
    rect(u, v, w, h)?;
    Ok(Image::Direct { u, v, w, h, a })
}

/// `SPCLR [mgmt]` — release one sprite (1 arg) or every user sprite (0 args). A return
/// value or more than 1 argument raises errnum 4; an out-of-range slot raises errnum 10.
/// Releasing a slot that is not in use is harmless.
pub fn spclr(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    match args {
        [] => sp.clear_all(),
        [m] => sp.clear(mgmt(m)?),
        _ => return Err(illegal()),
    }
    Ok(vec![])
}

/// `SPSHOW mgmt` — turn a live sprite's display flag ON. Exactly 1 arg, no return value
/// (else errnum 4); the slot must be in range (else errnum 10) and already created with
/// `SPSET` (else errnum 4 — the documented "used before SPSET" guard).
pub fn spshow(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    set_display(sp, args, ret_count, true)
}

/// `SPHIDE mgmt` — turn a live sprite's display flag OFF (the sprite keeps existing).
/// Same call-shape / active-bit guards as [`spshow`].
pub fn sphide(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    set_display(sp, args, ret_count, false)
}

fn set_display(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
    show: bool,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let slot = match args {
        [m] => mgmt(m)?,
        _ => return Err(illegal()),
    };
    // The sprite must have been created with SPSET (the active bit) — else errnum 4.
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    sp.sprites[slot].display = show;
    Ok(vec![])
}

/// `SPUSED(mgmt)` / `SPUSED mgmt OUT v` — is the slot in use? Returns TRUE(1)/FALSE(0).
/// Requires (1 arg, 1 result) — any other shape raises errnum 4; an out-of-range slot
/// raises errnum 10. There is no active-bit guard: querying a free slot is valid.
pub fn spused(
    sp: &SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let slot = match args {
        [m] => mgmt(m)?,
        _ => return Err(illegal()),
    };
    Ok(vec![Value::Int(sp.is_used(slot) as i32)])
}
