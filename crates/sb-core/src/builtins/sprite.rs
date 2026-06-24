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
            let z = rest[2].to_real()?;
            // Depth (Z) is range-checked to the documented -256..1024 (inclusive); a value
            // outside raises errnum 10. The 4-arg path reads the depth through the bounded
            // float-arg helper FUN_001eeb9c with the two literal bounds 1024.0 (`s16`=
            // 0x44800000 @0x141458) and -256.0 (`s17`=0xc3800000 @0x14145c): `vcmpe.f32
            // s0,s17; bcc err` / `vcmpe.f32 s0,s16; ble ok` @0x1eebd8-0x1eebf0, raising via
            // `mov r0,#0xa; b 0x1fffdc` @0x1eec04 (cia_3.6.0.lst). hw_verified: SPOFS 0,0,0,
            // 1025 / -257 / 5000 / -2000 -> errnum 10 errline 1; 1024 / -256 stored verbatim.
            if !(-256.0..=1024.0).contains(&z) {
                return Err(out_of_range());
            }
            s.z = z;
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

/// `SPSCALE mgmt, scaleX, scaleY` (set, ret 0, 3 args) / `SPSCALE mgmt OUT SX,SY` (get,
/// ret 2, 1 arg) — set or read a sprite's display magnification. Each axis is stored verbatim
/// as a float with NO *upper* bound (the documented 0.5..2.0 is a guideline only — `4`, `400`,
/// `1000` are all accepted), but each is **range-checked ≥ 0.0**: a negative scale raises
/// errnum 10. X and Y are independent; the fresh-`SPSET` default is 1.0,1.0.
///
/// The sprite must be `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10; the SET form requires
/// exactly 3 arguments (else errnum 4); a void/non-numeric scale is errnum 8 (type mismatch).
/// hw_verified (sb-oracle 2026-06-24): `SPSCALE 0,2,0.5` reads back 2,0.5; `4,4` / `0.4,0.4` /
/// `1000,1000` round-trip unclamped; `0,1` ok; `-1,1` / `1,-1` / `-0.001,1` → errnum 10;
/// `0,,0.5` / `0,0.5,` → errnum 8; default 1,1.
pub fn spscale(
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
        if rest.len() != 2 {
            return Err(illegal());
        }
        // The handler reads each axis through the minimum-bound float getter FUN_001eec18
        // (bl @0x143c6c/@0x143c90) with the literal bound 0.0 (`s16` @0x143d2c=0x00000000):
        // it evaluates the arg (a void/non-numeric value → errnum 8) then `vcmpe.f32 s1,s0;
        // bls ok` (s1=bound 0.0, s0=value → OK iff value ≥ 0), else `mov r0,#0xa; b 0x1fffdc`
        // → errnum 10 (cia_3.6.0.lst). X is fully evaluated+checked before Y.
        let sx = rest[0].to_real()?;
        if sx < 0.0 {
            return Err(out_of_range());
        }
        let sy = rest[1].to_real()?;
        if sy < 0.0 {
            return Err(out_of_range());
        }
        let s = &mut sp.sprites[slot];
        s.scale_x = sx;
        s.scale_y = sy;
        Ok(vec![])
    } else if ret_count == 2 {
        let s = &sp.sprites[slot];
        Ok(vec![Value::Real(s.scale_x), Value::Real(s.scale_y)])
    } else {
        Err(illegal())
    }
}

/// `SPROT mgmt, angle` (set, ret 0, 2 args) / `SPROT mgmt OUT DR` (get, ret 1, 1 arg) /
/// `Variable = SPROT(mgmt)` (function get) — set or read a sprite's rotation angle in degrees
/// (clockwise). The angle is truncated toward zero to an integer then **stored as a SIGNED
/// 16-bit value** and read back VERBATIM — it is NOT normalized into 0..360. The documented
/// 0..360 range is a convention, not a clamp/wrap.
///
/// The sprite must be `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10; the SET form requires
/// exactly 2 arguments (else errnum 4). hw_verified (sb-oracle 2026-06-24): SPROT 0,45→45,
/// 0,-25→-25, 0,450→450 (NOT wrapped to 0..360), 0,11.2→11 / 0,-11.9→-11 (truncate toward
/// zero), fresh default 0. The signed-halfword wrap is decisive: 0,32768→-32768, 0,40000→
/// -25536, 0,65536→0, 0,70000→4464, 0,32767→32767, 0,-32768→-32768.
pub fn sprot(
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
        if rest.len() != 1 {
            return Err(illegal());
        }
        // The angle is read through the integer getter (truncate toward zero), then the apply
        // helper FUN_0017eaa4 sign-extends it to 16 bits (`sxth r1,r1` @0x17eab4) and stores it
        // as a halfword (`strh r1,[r0,#0x28]` @0x17eac4); GET reads it back signed (`ldrsh
        // r1,[r4,#0x28]` @0x141578). So the stored angle wraps mod 2^16 into -32768..32767
        // (cia_3.6.0.lst). The mod-360 normalized angle the renderer uses lives in a separate
        // field [+0x2a] and is NOT what SPROT() returns.
        let stored = rest[0].to_int()? as i16;
        sp.sprites[slot].rot = stored as f64;
        Ok(vec![])
    } else if ret_count == 1 {
        Ok(vec![Value::Int(sp.sprites[slot].rot as i16 as i32)])
    } else {
        Err(illegal())
    }
}

/// `SPHOME mgmt, X, Y` (set, ret 0, 3 args) / `SPHOME mgmt OUT HX,HY` (get, ret 2, 1 arg) —
/// set or read a sprite's reference (home) point: the origin used for `SPOFS` positioning, the
/// center for rotation/scaling, and the collision-area origin. Coordinates are relative to the
/// sprite's top-left and stored as a 32-bit float — negative and fractional offsets are
/// accepted and round-trip VERBATIM (unlike BG's integer grid home). X and Y are independent;
/// the fresh-`SPSET` default is 0.0,0.0.
///
/// The sprite must be `SPSET` (errnum 4); mgmt ∉ 0..511 is errnum 10; the SET form requires
/// exactly 3 arguments (else errnum 4); a void/non-numeric coordinate is errnum 8 (type
/// mismatch). hw_verified (sb-oracle 2026-06-24): `SPHOME 0,16,16`→16,16; `0,-16,-16`→-16,-16;
/// `0,127.5,127.5`→127.5,127.5; `0,16.25,-8.5`→16.25,-8.5; `0,0.1,0.2`→0.1,0.2; default 0,0;
/// per-sprite independence; `0,16` / `0,1,2,3` → errnum 4; `0,,16` → errnum 8.
pub fn sphome(
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
        if rest.len() != 2 {
            return Err(illegal());
        }
        // The handler fetches each coordinate through the slot float getter (a void/non-numeric
        // field → errnum 8) and applies the home via FUN_001ee524 (vldmia sp,{s0,s1} / bl
        // 0x1ee524 @0x142884) — stored as f32 at [r4,#0x18], NO range/sign/integer constraint
        // (cia_3.6.0.lst). X is evaluated before Y.
        let hx = rest[0].to_real()?;
        let hy = rest[1].to_real()?;
        let s = &mut sp.sprites[slot];
        s.home_x = hx;
        s.home_y = hy;
        Ok(vec![])
    } else if ret_count == 2 {
        let s = &sp.sprites[slot];
        Ok(vec![Value::Real(s.home_x), Value::Real(s.home_y)])
    } else {
        Err(illegal())
    }
}

/// `SPPAGE` — select (SET) or read (GET) the global graphic page the sprite system renders
/// onto. Two forms chosen by the return count (cia_3.6.0.lst handler @0x142ad0):
///
/// - `ret_count == 0`, exactly 1 arg → SET: the page is fetched through the bounded-integer
///   getter (lower 0, upper 5 — `mov r1,#0x5` / `mov r3,#0x0` @0x142b1c) so a value < 0 or
///   > 5 raises errnum 10 (Out of range); the accepted page is committed globally.
/// - `ret_count == 1`, 0 args → GET: returns the current page (`[[0x315d60]+0x4c]`,
///   @0x142b54-0x142b70), default GRP4.
///
/// Any other (return-count, arg-count) shape raises errnum 4 (Illegal function call).
pub fn sppage(
    sp: &mut SpriteState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count == 0 {
        if args.len() != 1 {
            return Err(illegal());
        }
        let page = args[0].to_int()?;
        if !(0..=5).contains(&page) {
            return Err(out_of_range());
        }
        sp.page = page as u8;
        Ok(vec![])
    } else if ret_count == 1 {
        if !args.is_empty() {
            return Err(illegal());
        }
        Ok(vec![Value::Int(i32::from(sp.page))])
    } else {
        Err(illegal())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f64) -> Value {
        Value::Real(v)
    }

    /// Create sprite `slot` (SPSET slot,0) so SPOFS can operate on it.
    fn spset0(sp: &mut SpriteState, slot: i32) {
        spset(sp, &[Value::Int(slot), Value::Int(0)], 0).unwrap();
    }

    #[test]
    fn spofs_z_range_guard() {
        // Depth Z is range-checked to -256..1024 inclusive (errnum 10 outside) via the bounded
        // float-arg helper FUN_001eeb9c; X,Y are not range-checked. hw_verified sb-oracle
        // 2026-06-24 (spofs_zerr): 1024 / -256 stored verbatim; 1025 / -257 / 5000 / -2000 → 10.
        let mut sp = SpriteState::new();
        spset0(&mut sp, 0);
        // Inclusive boundaries accepted and stored verbatim.
        spofs(&mut sp, &[n(0.0), n(0.0), n(0.0), n(1024.0)], 0).unwrap();
        assert_eq!(spofs(&mut sp, &[n(0.0)], 3).unwrap()[2], n(1024.0));
        spofs(&mut sp, &[n(0.0), n(0.0), n(0.0), n(-256.0)], 0).unwrap();
        assert_eq!(spofs(&mut sp, &[n(0.0)], 3).unwrap()[2], n(-256.0));
        // Just outside → Out of range.
        for z in [1025.0, -257.0, 5000.0, -2000.0] {
            assert_eq!(
                spofs(&mut sp, &[n(0.0), n(0.0), n(0.0), n(z)], 0)
                    .unwrap_err()
                    .errnum,
                10,
                "Z={z} should be Out of range"
            );
        }
        // A 3-arg SET keeps the already-set Z (no range fire); the verbatim Z survives.
        spofs(&mut sp, &[n(0.0), n(0.0), n(0.0), n(500.0)], 0).unwrap();
        spofs(&mut sp, &[n(0.0), n(9.0), n(9.0)], 0).unwrap();
        assert_eq!(spofs(&mut sp, &[n(0.0)], 3).unwrap()[2], n(500.0));
    }

    #[test]
    fn spofs_value_round_trip() {
        // X,Y stored verbatim incl. fractional + negative; empty-arg skip keeps current coord;
        // per-sprite independence. hw_verified sb-oracle 2026-06-24 (spofs_rt).
        let mut sp = SpriteState::new();
        spset0(&mut sp, 0);
        spofs(&mut sp, &[n(0.0), n(16.5), n(-16.5)], 0).unwrap();
        let xy = spofs(&mut sp, &[n(0.0)], 2).unwrap();
        assert_eq!((xy[0].clone(), xy[1].clone()), (n(16.5), n(-16.5)));
        // Empty-arg skip keeps current X,Y while setting Z.
        spofs(&mut sp, &[n(0.0), n(50.0), n(80.0)], 0).unwrap();
        spofs(&mut sp, &[n(0.0), Value::Void, Value::Void, n(1000.0)], 0).unwrap();
        let xyz = spofs(&mut sp, &[n(0.0)], 3).unwrap();
        assert_eq!(
            (xyz[0].clone(), xyz[1].clone(), xyz[2].clone()),
            (n(50.0), n(80.0), n(1000.0))
        );
        // Per-sprite independence.
        spset0(&mut sp, 1);
        spofs(&mut sp, &[n(1.0), n(33.0), n(44.0)], 0).unwrap();
        let xy0 = spofs(&mut sp, &[n(0.0)], 2).unwrap();
        assert_eq!((xy0[0].clone(), xy0[1].clone()), (n(50.0), n(80.0)));
    }

    #[test]
    fn spscale_round_trip_and_guards() {
        // Scale round-trips verbatim with NO upper clamp; each axis range-checked >= 0.0
        // (negative → errnum 10), void → errnum 8, SET requires exactly 3 args (→ errnum 4),
        // default after SPSET is 1,1, X/Y independent. hw_verified sb-oracle 2026-06-24
        // (spscale_{rt,neg}).
        let mut sp = SpriteState::new();
        spset0(&mut sp, 0);
        // Fresh default 1,1.
        let d = spscale(&mut sp, &[n(0.0)], 2).unwrap();
        assert_eq!((d[0].clone(), d[1].clone()), (n(1.0), n(1.0)));
        // Verbatim, no upper/lower clamp within >= 0: 4,4 / 0.4,0.4 / 1000,1000 / 0,1.
        for (sx, sy) in [(4.0, 4.0), (0.4, 0.4), (1000.0, 1000.0), (0.0, 1.0)] {
            spscale(&mut sp, &[n(0.0), n(sx), n(sy)], 0).unwrap();
            let g = spscale(&mut sp, &[n(0.0)], 2).unwrap();
            assert_eq!((g[0].clone(), g[1].clone()), (n(sx), n(sy)));
        }
        // Negative scale (either axis) → Out of range (errnum 10).
        assert_eq!(
            spscale(&mut sp, &[n(0.0), n(-1.0), n(1.0)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            spscale(&mut sp, &[n(0.0), n(1.0), n(-0.001)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        // Void scale → Type mismatch (errnum 8).
        assert_eq!(
            spscale(&mut sp, &[n(0.0), Value::Void, n(0.5)], 0)
                .unwrap_err()
                .errnum,
            8
        );
        // SET form requires exactly 3 args (else errnum 4).
        assert_eq!(
            spscale(&mut sp, &[n(0.0), n(1.0)], 0).unwrap_err().errnum,
            4
        );
        assert_eq!(
            spscale(&mut sp, &[n(0.0), n(1.0), n(1.0), n(1.0)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        // Used before SPSET → errnum 4; mgmt out of 0..511 → errnum 10.
        assert_eq!(
            spscale(&mut sp, &[n(5.0), n(1.0), n(1.0)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        assert_eq!(
            spscale(&mut sp, &[n(512.0), n(1.0), n(1.0)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        // Per-sprite independence.
        spset0(&mut sp, 1);
        spscale(&mut sp, &[n(0.0), n(2.0), n(2.0)], 0).unwrap();
        spscale(&mut sp, &[n(1.0), n(3.0), n(3.0)], 0).unwrap();
        let g0 = spscale(&mut sp, &[n(0.0)], 2).unwrap();
        assert_eq!((g0[0].clone(), g0[1].clone()), (n(2.0), n(2.0)));
    }

    #[test]
    fn sprot_round_trip_wrap_and_guards() {
        // Angle truncates toward zero then stores as a SIGNED 16-bit halfword (verbatim, no
        // normalization). Magnitudes beyond ±32767 wrap mod 2^16. The function/OUT get returns
        // the stored value as an integer. hw_verified sb-oracle 2026-06-24 (sprot_rt).
        let mut sp = SpriteState::new();
        spset0(&mut sp, 0);
        // Fresh default 0.
        assert_eq!(sprot(&mut sp, &[n(0.0)], 1).unwrap()[0], Value::Int(0));
        // Verbatim, no normalize: 45 / -25 / 450; truncate toward zero: 11.2→11, -11.9→-11.
        // Signed-16-bit wrap: 32767, -32768, 32768→-32768, 40000→-25536, 65536→0, 70000→4464.
        for (input, want) in [
            (45.0, 45),
            (-25.0, -25),
            (450.0, 450),
            (11.2, 11),
            (-11.9, -11),
            (32767.0, 32767),
            (-32768.0, -32768),
            (32768.0, -32768),
            (40000.0, -25536),
            (65536.0, 0),
            (70000.0, 4464),
        ] {
            sprot(&mut sp, &[n(0.0), n(input)], 0).unwrap();
            assert_eq!(
                sprot(&mut sp, &[n(0.0)], 1).unwrap()[0],
                Value::Int(want),
                "SPROT 0,{input} should read back {want}"
            );
        }
        // Per-sprite independence.
        spset0(&mut sp, 1);
        sprot(&mut sp, &[n(0.0), n(30.0)], 0).unwrap();
        sprot(&mut sp, &[n(1.0), n(60.0)], 0).unwrap();
        assert_eq!(sprot(&mut sp, &[n(0.0)], 1).unwrap()[0], Value::Int(30));
        // Used before SPSET → errnum 4; mgmt out of 0..511 → errnum 10; SET form needs exactly
        // 2 args (1-arg-0-return → errnum 4).
        assert_eq!(sprot(&mut sp, &[n(5.0), n(0.0)], 0).unwrap_err().errnum, 4);
        assert_eq!(
            sprot(&mut sp, &[n(512.0), n(0.0)], 0).unwrap_err().errnum,
            10
        );
        assert_eq!(sprot(&mut sp, &[n(0.0)], 0).unwrap_err().errnum, 4);
    }

    #[test]
    fn sphome_round_trip_and_guards() {
        // Home is a verbatim FLOAT round-trip with no range/sign/integer constraint (negative,
        // fractional accepted); a void coord → errnum 8; SET needs exactly 3 args (→ errnum 4);
        // default after SPSET is 0,0, X/Y independent. hw_verified sb-oracle 2026-06-24
        // (sphome_rt).
        let mut sp = SpriteState::new();
        spset0(&mut sp, 0);
        // Fresh default 0,0.
        let d = sphome(&mut sp, &[n(0.0)], 2).unwrap();
        assert_eq!((d[0].clone(), d[1].clone()), (n(0.0), n(0.0)));
        // Verbatim float, any sign/fraction: -16,-16 / 127.5,127.5 / 16.25,-8.5.
        for (hx, hy) in [(-16.0, -16.0), (127.5, 127.5), (16.25, -8.5)] {
            sphome(&mut sp, &[n(0.0), n(hx), n(hy)], 0).unwrap();
            let g = sphome(&mut sp, &[n(0.0)], 2).unwrap();
            assert_eq!((g[0].clone(), g[1].clone()), (n(hx), n(hy)));
        }
        // Void coordinate → Type mismatch (errnum 8).
        assert_eq!(
            sphome(&mut sp, &[n(0.0), Value::Void, n(16.0)], 0)
                .unwrap_err()
                .errnum,
            8
        );
        // SET form requires exactly 3 args (else errnum 4).
        assert_eq!(
            sphome(&mut sp, &[n(0.0), n(16.0)], 0).unwrap_err().errnum,
            4
        );
        assert_eq!(
            sphome(&mut sp, &[n(0.0), n(1.0), n(2.0), n(3.0)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        // Used before SPSET → errnum 4; mgmt out of 0..511 → errnum 10.
        assert_eq!(
            sphome(&mut sp, &[n(5.0), n(0.0), n(0.0)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        assert_eq!(
            sphome(&mut sp, &[n(512.0), n(0.0), n(0.0)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        // Per-sprite independence.
        spset0(&mut sp, 1);
        sphome(&mut sp, &[n(0.0), n(5.0), n(6.0)], 0).unwrap();
        sphome(&mut sp, &[n(1.0), n(7.0), n(8.0)], 0).unwrap();
        let g0 = sphome(&mut sp, &[n(0.0)], 2).unwrap();
        assert_eq!((g0[0].clone(), g0[1].clone()), (n(5.0), n(6.0)));
    }

    #[test]
    fn sppage_round_trip_and_guards() {
        // SPPAGE is the global sprite render page: GET (ret_count 1, 0 args) defaults to GRP4;
        // SET (ret_count 0, 1 arg) round-trips 0..5 verbatim, out of that range → errnum 10;
        // any other (ret,arg) shape → errnum 4. hw_verified sb-oracle 2026-06-24 (sppage_rt).
        let mut sp = SpriteState::new();
        // Fresh default = 4 (GRP4).
        assert_eq!(sppage(&mut sp, &[], 1).unwrap(), vec![Value::Int(4)]);
        // Round-trip every valid page 0..5.
        for p in 0..=5 {
            sppage(&mut sp, &[n(f64::from(p))], 0).unwrap();
            assert_eq!(sppage(&mut sp, &[], 1).unwrap(), vec![Value::Int(p)]);
        }
        // Out of range → errnum 10.
        assert_eq!(sppage(&mut sp, &[n(6.0)], 0).unwrap_err().errnum, 10);
        assert_eq!(sppage(&mut sp, &[n(-1.0)], 0).unwrap_err().errnum, 10);
        // Wrong (return, arg) shape → errnum 4: SET with 0 args, SET with 2 args, GET with 1 arg.
        assert_eq!(sppage(&mut sp, &[], 0).unwrap_err().errnum, 4);
        assert_eq!(sppage(&mut sp, &[n(1.0), n(2.0)], 0).unwrap_err().errnum, 4);
        assert_eq!(sppage(&mut sp, &[n(0.0)], 1).unwrap_err().errnum, 4);
        // A new sprite captures the current global page.
        sppage(&mut sp, &[n(2.0)], 0).unwrap();
        spset0(&mut sp, 0);
        assert_eq!(sp.sprites[0].page, 2);
    }
}
