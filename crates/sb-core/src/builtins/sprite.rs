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
    AnimError, SpdefEntry, SpriteState, ANIM_ITEMS, SPDEF_MAX, SPRITE_COUNT, SPRITE_DEFAULT_ATTR,
    SPRITE_DEFAULT_WH,
};

use super::{illegal, out_of_range, type_mismatch};
use crate::value::{RuntimeError, Value};

/// errnum 39 — "Animation is too long" (`spec/reference/errors.yaml`).
const ERR_ANIM_TOO_LONG: u32 = 39;
/// errnum 40 — "Illegal animation data".
const ERR_ANIM_ILLEGAL: u32 = 40;

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
            sp.set_template(slot, d);
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
                Image::Template(d) => sp.set_template(slot, d),
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

/// Validate a sprite internal-variable number in 0..7 (`SPVAR`). Out of range → errnum 10
/// (the documented range; the exact errnum for a bad variable number is oracle-pending —
/// see `HARVEST_QUEUE.md`).
fn varnum(v: &Value) -> Result<usize, RuntimeError> {
    let i = v.to_int()?;
    if (0..8).contains(&i) {
        Ok(i as usize)
    } else {
        Err(out_of_range())
    }
}

/// Resolve an `SPANIM`/animation `target` operand to a `(channel 0..7, relative)` pair. A
/// number adds 8 (bit 3) for relative; a string is one of `XY/Z/UV/I/R/S/C/V` with an
/// optional trailing `"+"` for relative. A negative numeric target or an unknown string
/// raises errnum 4 (the disassembly validates the resolved target `>= 0`); a non-number /
/// non-string raises errnum 8.
pub(crate) fn parse_target(v: &Value) -> Result<(usize, bool), RuntimeError> {
    match v {
        Value::Str(s) => {
            let mut name = String::from_utf16_lossy(s).trim().to_ascii_uppercase();
            let relative = name.ends_with('+');
            if relative {
                name.pop();
            }
            let channel = match name.as_str() {
                "XY" => 0,
                "Z" => 1,
                "UV" => 2,
                "I" => 3,
                "R" => 4,
                "S" => 5,
                "C" => 6,
                "V" => 7,
                _ => return Err(illegal()),
            };
            Ok((channel, relative))
        }
        Value::Int(_) | Value::Real(_) => {
            let t = v.to_int()?;
            if t < 0 {
                return Err(illegal());
            }
            Ok(((t & 7) as usize, t & 8 != 0))
        }
        _ => Err(type_mismatch()),
    }
}

/// Map an [`AnimError`] from the keyframe builder to its SmileBASIC errnum.
pub(crate) fn anim_err(e: AnimError) -> RuntimeError {
    match e {
        AnimError::TooFew => illegal(),
        AnimError::TooLong => RuntimeError::new(ERR_ANIM_TOO_LONG),
        AnimError::OutOfRange => out_of_range(),
        AnimError::ZeroTime => RuntimeError::new(ERR_ANIM_ILLEGAL),
    }
}

/// Items per keyframe for an already-resolved animation channel (the inline `loop`
/// disambiguator and the form-1/2 data-stride).
pub(crate) fn anim_items(channel: usize) -> usize {
    ANIM_ITEMS[channel]
}

/// `SPVAR` — read/write one of a sprite's eight internal variables. The form is the
/// return count: 0 = setter (`SPVAR m,n,v`, 3 args), 1 = reader (`v=SPVAR(m,n)` or
/// `SPVAR m,n OUT v`, 2 args). SPVAR works before `SPSET` (the storage exists for every
/// slot), so there is no active-bit guard. A bad shape → errnum 4; mgmt ∉ 0..511 →
/// errnum 10.
pub fn spvar(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let (m, n, val) = match args {
                [m, n, val] => (m, n, val),
                _ => return Err(illegal()),
            };
            let slot = mgmt(m)?;
            let vn = varnum(n)?;
            sp.sprites[slot].var[vn] = val.to_real()?;
            Ok(vec![])
        }
        1 => {
            let (m, n) = match args {
                [m, n] => (m, n),
                _ => return Err(illegal()),
            };
            let slot = mgmt(m)?;
            let vn = varnum(n)?;
            Ok(vec![Value::Real(sp.sprites[slot].var[vn])])
        }
        _ => Err(illegal()),
    }
}

/// Shared body of `SPSTART` (resume, `stop`=false) and `SPSTOP` (pause, `stop`=true). The
/// no-argument form toggles every sprite (no error); the one-argument form requires the
/// slot to be in range (errnum 10) and already `SPSET` (errnum 4). A return value or >1
/// argument is errnum 4.
fn set_anim_run(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
    stop: bool,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    match args {
        [] => sp.set_anim_stopped_all(stop),
        [m] => {
            let slot = mgmt(m)?;
            if !sp.is_used(slot) {
                return Err(illegal());
            }
            sp.set_anim_stopped(slot, stop);
        }
        _ => return Err(illegal()),
    }
    Ok(vec![])
}

/// `SPSTART [mgmt]` — resume animation (clear the stop flag) for one sprite or all.
pub fn spstart(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    set_anim_run(sp, args, ret_count, false)
}

/// `SPSTOP [mgmt]` — pause animation (set the stop flag) for one sprite or all.
pub fn spstop(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    set_anim_run(sp, args, ret_count, true)
}

/// `SPLINK` — link a child to a parent (SET form, ret 0, 2 args) or read a sprite's parent
/// (GET form, ret 1, 1 arg → parent number or -1). The parent must be strictly lower than
/// the child and both must be `SPSET` (errnum 4); a management number ∉ 0..511 is errnum 10.
pub fn splink(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let (c, p) = match args {
                [c, p] => (c, p),
                _ => return Err(illegal()),
            };
            let child = mgmt(c)?;
            let parent = mgmt(p)?;
            // Parent must be strictly lower than the child (errline 1 in the handler).
            if child <= parent {
                return Err(illegal());
            }
            if !sp.is_used(child) || !sp.is_used(parent) {
                return Err(illegal());
            }
            sp.link(child, parent);
            Ok(vec![])
        }
        1 => {
            let m = match args {
                [m] => m,
                _ => return Err(illegal()),
            };
            let slot = mgmt(m)?;
            if !sp.is_used(slot) {
                return Err(illegal());
            }
            Ok(vec![Value::Int(sp.parent_of(slot))])
        }
        _ => Err(illegal()),
    }
}

/// `SPUNLINK mgmt` — break a sprite's parent link (1 arg, no return). The sprite must be
/// `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10. Unlinking an unlinked sprite is a no-op.
pub fn spunlink(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let m = match args {
        [m] => m,
        _ => return Err(illegal()),
    };
    let slot = mgmt(m)?;
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    sp.unlink(slot);
    Ok(vec![])
}

/// Install an `SPANIM` animation: validate the management number (errnum 10) and active
/// bit (errnum 4 "used before SPSET"), resolve the `target` (errnum 8/4), then build the
/// keyframe list from the already-flattened `data` (errnum 39/40/10). The caller (the VM)
/// has already gated argcount>=3 and return-count==0 and flattened `data`/`loop_count`
/// from the array / `@label` / inline form.
pub fn spanim(
    sp: &mut SpriteState,
    mgmt_v: &Value,
    target_v: &Value,
    data: &[f64],
    loop_count: i32,
) -> Result<(), RuntimeError> {
    let slot = mgmt(mgmt_v)?;
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    let (channel, relative) = parse_target(target_v)?;
    sp.set_anim(slot, channel, relative, data, loop_count)
        .map_err(anim_err)
}

/// `SPOFS mgmt, X, Y [,Z]` (set) / `SPOFS mgmt OUT X,Y[,Z]` (get) — move or read a sprite's
/// screen position. Implemented here as the positioning glue M3-T3 collision needs (the
/// other transform setters land later). The set form takes 3 or 4 arguments; an empty
/// (`,`-skipped) coordinate keeps its current value. The get form returns X,Y (2 OUT) or
/// X,Y,Z (3 OUT). The sprite must be `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10; a bad
/// argument/OUT count is errnum 4.
pub fn spofs(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if args.is_empty() {
        return Err(illegal());
    }
    let slot = mgmt(&args[0])?;
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    if ret_count == 0 {
        let rest = &args[1..];
        if !matches!(rest.len(), 2 | 3) {
            return Err(illegal());
        }
        let s = &mut sp.sprites[slot];
        if !matches!(rest[0], Value::Void) {
            s.x = rest[0].to_real()?;
        }
        if !matches!(rest[1], Value::Void) {
            s.y = rest[1].to_real()?;
        }
        if rest.len() == 3 && !matches!(rest[2], Value::Void) {
            s.z = rest[2].to_real()?;
        }
        Ok(vec![])
    } else {
        let s = &sp.sprites[slot];
        let out = match ret_count {
            2 => vec![Value::Real(s.x), Value::Real(s.y)],
            3 => vec![Value::Real(s.x), Value::Real(s.y), Value::Real(s.z)],
            _ => return Err(illegal()),
        };
        Ok(out)
    }
}

// -- collision (M3-T3) --------------------------------------------------------

/// The `SPCOL` scale-adjustment flag: a Void (skipped `,,`) field is the default FALSE,
/// otherwise it is truthy/falsy (non-zero).
fn scale_arg(v: &Value) -> Result<bool, RuntimeError> {
    if matches!(v, Value::Void) {
        Ok(false)
    } else {
        Ok(v.to_int()? != 0)
    }
}

/// A 32-bit collision mask argument: a Void (skipped) field is the default all-bits mask,
/// otherwise the value reinterpreted as 32 bits (`&HFFFFFFFF` parses to the i32 `-1`).
fn mask_arg(v: &Value) -> Result<u32, RuntimeError> {
    if matches!(v, Value::Void) {
        Ok(0xFFFF_FFFF)
    } else {
        Ok(v.to_int()? as u32)
    }
}

/// An explicit `SPCOL` detection rectangle `(sx,sy,w,h)`.
fn rect_arg(
    sx: &Value,
    sy: &Value,
    w: &Value,
    h: &Value,
) -> Result<(i32, i32, i32, i32), RuntimeError> {
    Ok((sx.to_int()?, sy.to_int()?, w.to_int()?, h.to_int()?))
}

/// `SPCOL` — configure collision (setters, `ret_count` 0, forms 1-3) or read it back
/// (`OUT` getters, `ret_count` 1/2/4/5/6, forms 4-7). The first argument is the management
/// number; the form is then chosen by the return count and the remaining argument count.
/// The sprite must be `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10.
///
/// `OUT` getters return their fields in declaration order; intermediate-slot skipping
/// (`SPCOL m OUT ,mask`) is not yet supported (the read-back values are oracle-pending —
/// see `HARVEST_QUEUE.md`).
pub fn spcol(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if args.is_empty() {
        return Err(illegal());
    }
    let slot = mgmt(&args[0])?;
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    if ret_count == 0 {
        match &args[1..] {
            [] => sp.set_col(slot, None, false, 0xFFFF_FFFF),
            [scale] => sp.set_col(slot, None, scale_arg(scale)?, 0xFFFF_FFFF),
            [scale, mask] => sp.set_col(slot, None, scale_arg(scale)?, mask_arg(mask)?),
            [sx, sy, w, h] => sp.set_col(slot, Some(rect_arg(sx, sy, w, h)?), false, 0xFFFF_FFFF),
            [sx, sy, w, h, scale] => sp.set_col(
                slot,
                Some(rect_arg(sx, sy, w, h)?),
                scale_arg(scale)?,
                0xFFFF_FFFF,
            ),
            [sx, sy, w, h, scale, mask] => sp.set_col(
                slot,
                Some(rect_arg(sx, sy, w, h)?),
                scale_arg(scale)?,
                mask_arg(mask)?,
            ),
            _ => return Err(illegal()),
        }
        Ok(vec![])
    } else {
        let s = &sp.sprites[slot];
        let scale = Value::Int(s.col_scale_adjust as i32);
        let mask = Value::Int(s.col_mask as i32);
        let (sx, sy, w, h) = (
            Value::Int(s.col_sx),
            Value::Int(s.col_sy),
            Value::Int(s.col_w),
            Value::Int(s.col_h),
        );
        let out = match ret_count {
            1 => vec![scale],
            2 => vec![scale, mask],
            4 => vec![sx, sy, w, h],
            5 => vec![sx, sy, w, h, scale],
            6 => vec![sx, sy, w, h, scale, mask],
            _ => return Err(illegal()),
        };
        Ok(out)
    }
}

/// `SPCOLVEC mgmt [,mvx,mvy]` — set the per-frame collision movement vector (auto-calc when
/// omitted). 1 or 3 args, no return value (else errnum 4); the sprite must be `SPSET`
/// (errnum 4); mgmt ∉ 0..511 is errnum 10.
pub fn spcolvec(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 || args.is_empty() {
        return Err(illegal());
    }
    let slot = mgmt(&args[0])?;
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    match &args[1..] {
        [] => sp.set_colvec(slot, None),
        [vx, vy] => sp.set_colvec(slot, Some((vx.to_real()?, vy.to_real()?))),
        _ => return Err(illegal()),
    }
    Ok(vec![])
}

/// `SPCHK(mgmt)` — the sprite's 8-bit animation-status bitmask (which `SPANIM` channels are
/// running; 0 when stopped). Requires (1 arg, 1 result) — else errnum 4; the sprite must be
/// `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10.
pub fn spchk(
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
    if !sp.is_used(slot) {
        return Err(illegal());
    }
    Ok(vec![Value::Int(sp.anim_status(slot))])
}

/// `SPHITSP(mgmt[,first,last])` / `SPHITSP(mgmt,opponent)` — sprite-sprite collision. The
/// form is chosen by argument count: 1 → vs all sprites (returns the first colliding
/// management number, or -1); 2 → vs one opponent (returns TRUE/FALSE); 3 → vs a range
/// (first colliding number, or -1). Requires a return value (errnum 4); a management number
/// ∉ 0..511 is errnum 10. A sprite that is not `SPSET`/`SPCOL`'d simply does not collide.
pub fn sphitsp(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let result = match args {
        [m] => {
            let s = mgmt(m)?;
            sp.hit_sp_range(s, 0, SPRITE_COUNT - 1)
        }
        [m, opp] => {
            let s = mgmt(m)?;
            let o = mgmt(opp)?;
            sp.hit_sp_pair(s, o) as i32
        }
        [m, first, last] => {
            let s = mgmt(m)?;
            let f = mgmt(first)?;
            let l = mgmt(last)?;
            sp.hit_sp_range(s, f, l)
        }
        _ => return Err(illegal()),
    };
    Ok(vec![Value::Int(result)])
}

/// A parsed `SPHITRC` quadrangle: the rectangle `(sx,sy,w,h)`, the 32-bit mask, and the
/// per-frame movement vector `(mvx,mvy)`.
type RcQuad = ((f64, f64, f64, f64), u32, (f64, f64));

/// Parse a `SPHITRC` rectangle tail: `[sx,sy,w,h]` (mask default, no movement) or
/// `[sx,sy,w,h,mask,mvx,mvy]` (mask skippable with `,,`).
fn rc_tail(t: &[Value]) -> Result<RcQuad, RuntimeError> {
    let rect = (
        t[0].to_real()?,
        t[1].to_real()?,
        t[2].to_real()?,
        t[3].to_real()?,
    );
    if t.len() >= 7 {
        Ok((rect, mask_arg(&t[4])?, (t[5].to_real()?, t[6].to_real()?)))
    } else {
        Ok((rect, 0xFFFF_FFFF, (0.0, 0.0)))
    }
}

/// `SPHITRC` — (moving) rectangle vs sprites. The form is chosen by argument count:
/// 4/7 → vs all sprites (first colliding management number, or -1); 5/8 → vs one sprite
/// (TRUE/FALSE); 6/9 → vs a range (first colliding number, or -1). The 7/8/9 counts add the
/// optional `mask,mvx,mvy`. Requires a return value (errnum 4); a referenced management
/// number ∉ 0..511 is errnum 10.
pub fn sphitrc(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let result = match args.len() {
        4 | 7 => {
            let (rect, mask, mv) = rc_tail(args)?;
            sp.hit_rc_range(rect, mask, mv, 0, SPRITE_COUNT - 1)
        }
        5 | 8 => {
            let opp = mgmt(&args[0])?;
            let (rect, mask, mv) = rc_tail(&args[1..])?;
            sp.hit_rc_one(rect, mask, mv, opp) as i32
        }
        6 | 9 => {
            let first = mgmt(&args[0])?;
            let last = mgmt(&args[1])?;
            let (rect, mask, mv) = rc_tail(&args[2..])?;
            sp.hit_rc_range(rect, mask, mv, first, last)
        }
        _ => return Err(illegal()),
    };
    Ok(vec![Value::Int(result)])
}

/// `SPHITINFO OUT …` — read back the most recent `SPHIT*` collision (time, then the
/// collision coordinates/velocities of the two objects). Takes NO input arguments (else
/// errnum 4); the form is the number of `OUT` variables (1/3/5/9 — any other count is
/// errnum 4). Intermediate-slot skipping (`OUT ,X1,…`) is not yet supported.
pub fn sphitinfo(
    sp: &SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if !args.is_empty() {
        return Err(illegal());
    }
    let h = &sp.hit;
    let r = Value::Real;
    let out = match ret_count {
        1 => vec![r(h.time)],
        3 => vec![r(h.time), r(h.x1), r(h.y1)],
        5 => vec![r(h.time), r(h.x1), r(h.y1), r(h.x2), r(h.y2)],
        9 => vec![
            r(h.time),
            r(h.x1),
            r(h.y1),
            r(h.vx1),
            r(h.vy1),
            r(h.x2),
            r(h.y2),
            r(h.vx2),
            r(h.vy2),
        ],
        _ => return Err(illegal()),
    };
    Ok(out)
}

// -- SPDEF definition templates (M3-T3) ---------------------------------------

/// Validate a definition template's fields against the documented ranges (errnum 10 on any
/// violation): `U,V` 0..512, `U+W`/`V+H` ≤ 512, `W,H` ≥ 0, attribute 0..&H3F, origin
/// -32768..32767.
pub(crate) fn validate_spdef(e: &SpdefEntry) -> Result<(), RuntimeError> {
    let ok = (0..=512).contains(&e.u)
        && (0..=512).contains(&e.v)
        && e.w >= 0
        && e.h >= 0
        && i64::from(e.u) + i64::from(e.w) <= 512
        && i64::from(e.v) + i64::from(e.h) <= 512
        && (0..=0x3f).contains(&e.attr)
        && (-32768..=32767).contains(&e.origin_x)
        && (-32768..=32767).contains(&e.origin_y);
    if ok {
        Ok(())
    } else {
        Err(out_of_range())
    }
}

/// Build a definition template from the `SPDEF defnum,U,V…` (form 2) argument tail. Omitted
/// `W,H` default to 16×16, origin to 0,0, attribute to &H01 (the documented defaults).
fn parse_define(rest: &[Value]) -> Result<SpdefEntry, RuntimeError> {
    let g = |i: usize| rest[i].to_int();
    let e = match rest.len() {
        2 => SpdefEntry {
            u: g(0)?,
            v: g(1)?,
            ..SpdefEntry::default()
        },
        3 => SpdefEntry {
            u: g(0)?,
            v: g(1)?,
            attr: g(2)?,
            ..SpdefEntry::default()
        },
        4 => SpdefEntry {
            u: g(0)?,
            v: g(1)?,
            w: g(2)?,
            h: g(3)?,
            ..SpdefEntry::default()
        },
        5 => SpdefEntry {
            u: g(0)?,
            v: g(1)?,
            w: g(2)?,
            h: g(3)?,
            attr: g(4)?,
            ..SpdefEntry::default()
        },
        6 => SpdefEntry {
            u: g(0)?,
            v: g(1)?,
            w: g(2)?,
            h: g(3)?,
            origin_x: g(4)?,
            origin_y: g(5)?,
            ..SpdefEntry::default()
        },
        7 => SpdefEntry {
            u: g(0)?,
            v: g(1)?,
            w: g(2)?,
            h: g(3)?,
            origin_x: g(4)?,
            origin_y: g(5)?,
            attr: g(6)?,
        },
        _ => return Err(illegal()),
    };
    Ok(e)
}

/// Apply one optional `SPDEF` copy-form (form 6) override: a present value replaces the
/// field, a Void (`,`-skipped) or missing slot keeps the source template's value.
fn override_field(field: &mut i32, v: Option<&Value>) -> Result<(), RuntimeError> {
    match v {
        None | Some(Value::Void) => Ok(()),
        Some(val) => {
            *field = val.to_int()?;
            Ok(())
        }
    }
}

/// `SPDEF` with a numeric scalar first argument: the single-template define (form 2) or the
/// copy-with-adjust (form 6). Copy is selected when the second argument is the only one
/// (`SPDEF dst,src`) or when any override field is skipped (a `,,` Void) — otherwise it is a
/// define. The defined template is range-validated (errnum 10); `defnum`/`srcnum` ∉ 0..4095
/// is errnum 10.
pub(crate) fn spdef_scalar(sp: &mut SpriteState, args: &[Value]) -> Result<(), RuntimeError> {
    let defnum = defn(&args[0])? as usize;
    let rest = &args[1..];
    let is_copy = rest.len() == 1 || rest.iter().any(|v| matches!(v, Value::Void));
    let entry = if is_copy {
        let src = defn(&rest[0])? as usize;
        let mut e = sp.spdef_get(src);
        let ov = &rest[1..];
        override_field(&mut e.u, ov.first())?;
        override_field(&mut e.v, ov.get(1))?;
        override_field(&mut e.w, ov.get(2))?;
        override_field(&mut e.h, ov.get(3))?;
        override_field(&mut e.origin_x, ov.get(4))?;
        override_field(&mut e.origin_y, ov.get(5))?;
        override_field(&mut e.attr, ov.get(6))?;
        e
    } else {
        parse_define(rest)?
    };
    validate_spdef(&entry)?;
    sp.spdef_set(defnum, entry);
    Ok(())
}

/// Build a definition template from a flat 7-element `[U,V,W,H,OX,OY,Attr]` slice (the bulk
/// array / DATA forms 3/4). The caller has range-checked the element count.
pub(crate) fn spdef_entry_from_slice(vals: &[f64]) -> SpdefEntry {
    SpdefEntry {
        u: vals[0] as i32,
        v: vals[1] as i32,
        w: vals[2] as i32,
        h: vals[3] as i32,
        origin_x: vals[4] as i32,
        origin_y: vals[5] as i32,
        attr: vals[6] as i32,
    }
}
