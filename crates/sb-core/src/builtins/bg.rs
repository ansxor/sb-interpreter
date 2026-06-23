//! BG core builtins (M3-T4) — the background-tilemap commands the VM drives over the
//! VM-owned [`BgState`](sb_render::bg::BgState).
//!
//! This slice covers map sizing, tile cells, and the per-layer transforms: `BGSCREEN`
//! (map size + tile pixel size), `BGPAGE` (shared graphic page), `BGPUT`/`BGGET`/`BGFILL`/
//! `BGCLR` (tile cells), `BGOFS` (scroll + depth), `BGROT` (rotation), `BGSCALE` (scale),
//! `BGCOLOR` (tint), `BGSHOW`/`BGHIDE` (visibility), `BGHOME` (origin), and `BGCLIP` (clip
//! area). Animation/coordinate-conversion/load-save (M3-T5) and the composite (M3-T6) build
//! on the same state.
//!
//! ## Form selection by return count
//!
//! Like the M2 graphics / M3 sprite commands, a SET/GET pair's form is chosen first by the
//! **return-value count** (`[r4,#0xc]`): 0 returns = the SET form; a non-zero return count =
//! the GET form (an `OUT`/function spelling). The VM collapses the `OUT` (`out_argc`) and
//! value-returning-function (`wants_value`) spellings into one `ret_count`, so `BGCOLOR 0
//! OUT C`, `C=BGCOLOR(0)`, and `BGOFS 0 OUT X,Y` all route correctly. See
//! `spec/instructions/bg*.yaml`.
//!
//! ## Errors
//!
//! - **Illegal function call** (4): a bad return/argument *count* for the call shape, or a
//!   `BGSCREEN` 4th arg that is not 8/16/32.
//! - **Type mismatch** (8): a `BGFILL`/`BGPUT` screen-data argument that is neither a number
//!   nor a string, or a non-numeric `BGCOLOR` color.
//! - **Out of range** (10): a layer ∉ 0..3, a `BGSCREEN` width/height < 1 or area > 16383, a
//!   `BGPAGE` page ∉ 0..5, or a `BGPUT`/`BGGET` (char-coord) cell off the map.
//! - **String too long** (41): a `BGFILL`/`BGPUT` screen-data string that is too long to
//!   parse.

use sb_render::bg::{BgState, BG_DEFAULT_TILE_SIZE, BG_MAX_CELLS};

use super::{illegal, out_of_range, type_mismatch};
use crate::value::{RuntimeError, Value};

/// errnum 41 — "String too long" (`spec/reference/errors.yaml`).
const ERR_STRING_TOO_LONG: u32 = 41;
/// The `BGFILL` handler's string-length guard (`cmp #0x2000`): longer → errnum 41.
const MAX_DATA_STRING_LEN: usize = 0x2000;

/// Validate + return a BG layer number in 0..3 (else errnum 10).
fn layer(v: &Value) -> Result<usize, RuntimeError> {
    let i = v.to_int()?;
    if BgState::in_range(i) {
        Ok(i as usize)
    } else {
        Err(out_of_range())
    }
}

/// Resolve a `BGPUT`/`BGFILL` screen-data operand to a 16-bit cell value. A number uses its
/// low 16 bits; a string is parsed as a (≤4-digit) hexadecimal value `"0000".."FFFF"`. An
/// over-long string raises errnum 41; a non-number / non-string raises errnum 8. The exact
/// behavior for malformed hex is oracle-pending (here it parses leniently to 0 — see
/// `HARVEST_QUEUE.md`).
fn screen_data(v: &Value) -> Result<u16, RuntimeError> {
    match v {
        Value::Int(_) | Value::Real(_) => Ok((v.to_int()? & 0xFFFF) as u16),
        Value::Str(s) => {
            if s.len() > MAX_DATA_STRING_LEN {
                return Err(RuntimeError::new(ERR_STRING_TOO_LONG));
            }
            let text = String::from_utf16_lossy(s);
            let parsed = i64::from_str_radix(text.trim(), 16).unwrap_or(0);
            Ok((parsed & 0xFFFF) as u16)
        }
        _ => Err(type_mismatch()),
    }
}

/// `BGSCREEN layer, width, height [,tileSize]` — set a layer's map size (3 or 4 args, no
/// return value). Width/height must be ≥ 1 and `width*height ≤ 16383` (errnum 10); the
/// optional tile size must be 8, 16, or 32 (errnum 4, default 16). A return value or a bad
/// argument count raises errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bgscreen(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (l, w, h, tile) = match args {
        [l, w, h] => (l, w, h, None),
        [l, w, h, t] => (l, w, h, Some(t)),
        _ => return Err(illegal()),
    };
    let layer = layer(l)?;
    let (w, h) = (w.to_int()?, h.to_int()?);
    if w < 1 || h < 1 || i64::from(w) * i64::from(h) > i64::from(BG_MAX_CELLS) {
        return Err(out_of_range());
    }
    let tile_size = match tile {
        None => BG_DEFAULT_TILE_SIZE,
        Some(t) => {
            let ts = t.to_int()?;
            if matches!(ts, 8 | 16 | 32) {
                ts
            } else {
                return Err(illegal());
            }
        }
    };
    bg.resize(layer, w, h, tile_size);
    Ok(vec![])
}

/// `BGPAGE page` (set, ret 0, 1 arg) / `BGPAGE()` / `BGPAGE OUT p` (get, ret 1, 0 args) —
/// set or read the shared BG graphic page. The page must be 0..5 (errnum 10). Any other
/// argument/return shape raises errnum 4.
pub fn bgpage(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let p = match args {
                [p] => p.to_int()?,
                _ => return Err(illegal()),
            };
            if !(0..=5).contains(&p) {
                return Err(out_of_range());
            }
            bg.page = p as u8;
            Ok(vec![])
        }
        1 => {
            if !args.is_empty() {
                return Err(illegal());
            }
            Ok(vec![Value::Int(bg.page as i32)])
        }
        _ => Err(illegal()),
    }
}

/// `BGPUT layer, x, y, screenData` — place one BG character (4 args, no return value). X/Y
/// must be inside the layer's map (errnum 10); a return value or a bad argument count is
/// errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bgput(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (l, x, y, data) = match args {
        [l, x, y, d] => (l, x, y, d),
        _ => return Err(illegal()),
    };
    let layer = layer(l)?;
    let (x, y) = (x.to_int()?, y.to_int()?);
    if !bg.layers[layer].in_cell(x, y) {
        return Err(out_of_range());
    }
    let data = screen_data(data)?;
    bg.layers[layer].set_cell(x, y, data);
    Ok(vec![])
}

/// `BGGET(layer, x, y [,coordFlag])` — read one cell's 16-bit screen data (function only, 3
/// or 4 args, 1 return). `coordFlag` 0 (default) = char-unit BG coords (range-checked,
/// errnum 10); 1 = pixel coords (converted to char by flooring `pixel/tileSize`, wrapped).
/// Use as a statement, or a bad argument count, raises errnum 4; the layer must be in 0..3
/// (errnum 10).
pub fn bgget(bg: &BgState, args: &[Value], ret_count: usize) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let (l, x, y, flag) = match args {
        [l, x, y] => (l, x, y, 0),
        [l, x, y, f] => (l, x, y, f.to_int()?),
        _ => return Err(illegal()),
    };
    let layer = layer(l)?;
    let (x, y) = (x.to_int()?, y.to_int()?);
    let cell = if flag == 0 {
        if !bg.layers[layer].in_cell(x, y) {
            return Err(out_of_range());
        }
        bg.layers[layer].cell(x, y)
    } else {
        bg.cell_pixel(layer, x, y)
    };
    Ok(vec![Value::Int(cell as i32)])
}

/// `BGFILL layer, startX, startY, endX, endY, screenData` — fill a rectangle of cells (6
/// args, no return value). The corners are normalized + clamped to the map; the screen data
/// may be a number or a 4-digit hex string (errnum 41 if too long, 8 if a wrong type). A
/// return value or a bad argument count is errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bgfill(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (l, sx, sy, ex, ey, data) = match args {
        [l, sx, sy, ex, ey, d] => (l, sx, sy, ex, ey, d),
        _ => return Err(illegal()),
    };
    let layer = layer(l)?;
    let (sx, sy, ex, ey) = (sx.to_int()?, sy.to_int()?, ex.to_int()?, ey.to_int()?);
    let data = screen_data(data)?;
    bg.fill(layer, sx, sy, ex, ey, data);
    Ok(vec![])
}

/// `BGCLR [layer]` — clear one layer's tilemap (1 arg) or every layer (0 args). A return
/// value or > 1 argument is errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bgclr(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    match args {
        [] => bg.clear_all(),
        [l] => bg.clear(layer(l)?),
        _ => return Err(illegal()),
    }
    Ok(vec![])
}

/// Read an optional integer SET coordinate: a Void (a `,`-skipped slot) keeps the current
/// value `cur`, otherwise the argument's integer value.
fn opt_int(v: &Value, cur: i32) -> Result<i32, RuntimeError> {
    if matches!(v, Value::Void) {
        Ok(cur)
    } else {
        Ok(v.to_int()?)
    }
}

/// `BGOFS layer, x, y [,z]` (set, ret 0, 3/4 args) / `BGOFS layer OUT x, y[, z]` (get,
/// ret 2/3, 1 arg) — set or read a layer's scroll offset (+ optional depth Z). A `,`-skipped
/// SET coordinate keeps its current value. A bad shape is errnum 4; the layer must be in
/// 0..3 (errnum 10).
pub fn bgofs(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count == 0 {
        let (l, rest) = args.split_first().ok_or_else(illegal)?;
        if !matches!(rest.len(), 2 | 3) {
            return Err(illegal());
        }
        let layer = layer(l)?;
        let cur = &bg.layers[layer];
        let x = opt_int(&rest[0], cur.ofs_x)?;
        let y = opt_int(&rest[1], cur.ofs_y)?;
        let z = if rest.len() == 3 {
            Some(opt_int(&rest[2], cur.ofs_z)?)
        } else {
            None
        };
        bg.set_ofs(layer, x, y, z);
        Ok(vec![])
    } else {
        let l = match args {
            [l] => l,
            _ => return Err(illegal()),
        };
        let layer = layer(l)?;
        let lr = &bg.layers[layer];
        match ret_count {
            2 => Ok(vec![Value::Int(lr.ofs_x), Value::Int(lr.ofs_y)]),
            3 => Ok(vec![
                Value::Int(lr.ofs_x),
                Value::Int(lr.ofs_y),
                Value::Int(lr.ofs_z),
            ]),
            _ => Err(illegal()),
        }
    }
}

/// `BGROT layer, angle` (set, ret 0, 2 args) / `BGROT layer OUT r` (get, ret 1, 1 arg) —
/// set or read a layer's rotation. The angle is normalized to 0..359. A bad shape is
/// errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bgrot(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let (l, a) = match args {
                [l, a] => (l, a),
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            bg.set_rot(layer, a.to_int()?);
            Ok(vec![])
        }
        1 => {
            let l = match args {
                [l] => l,
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            Ok(vec![Value::Int(bg.layers[layer].rot)])
        }
        _ => Err(illegal()),
    }
}

/// `BGSCALE layer, scaleX, scaleY` (set, ret 0, 3 args) / `BGSCALE layer OUT sx, sy` (get,
/// ret 2, 1 arg) — set or read a layer's enlargement scale (stored unclamped, returned as
/// Doubles). A bad shape is errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bgscale(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let (l, sx, sy) = match args {
                [l, sx, sy] => (l, sx, sy),
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            bg.set_scale(layer, sx.to_real()?, sy.to_real()?);
            Ok(vec![])
        }
        2 => {
            let l = match args {
                [l] => l,
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            let lr = &bg.layers[layer];
            Ok(vec![Value::Real(lr.scale_x), Value::Real(lr.scale_y)])
        }
        _ => Err(illegal()),
    }
}

/// `BGCOLOR layer, color` (set, ret 0, 2 args) / `BGCOLOR layer OUT c` / `c=BGCOLOR(layer)`
/// (get, ret 1, 1 arg) — set or read a layer's ARGB8888 multiply tint. A non-numeric color
/// in the SET form is errnum 8; a bad shape is errnum 4; the layer must be in 0..3
/// (errnum 10).
pub fn bgcolor(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let (l, c) = match args {
                [l, c] => (l, c),
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            if !matches!(c, Value::Int(_) | Value::Real(_)) {
                return Err(type_mismatch());
            }
            bg.layers[layer].color = c.to_int()? as u32;
            Ok(vec![])
        }
        1 => {
            let l = match args {
                [l] => l,
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            Ok(vec![Value::Int(bg.layers[layer].color as i32)])
        }
        _ => Err(illegal()),
    }
}

/// Shared body of `BGSHOW` (show) and `BGHIDE` (hide): 1 layer argument, no return value
/// (else errnum 4); the layer must be in 0..3 (errnum 10).
fn set_visible(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
    show: bool,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let l = match args {
        [l] => l,
        _ => return Err(illegal()),
    };
    let layer = layer(l)?;
    bg.layers[layer].visible = show;
    Ok(vec![])
}

/// `BGSHOW layer` — make a BG layer visible.
pub fn bgshow(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    set_visible(bg, args, ret_count, true)
}

/// `BGHIDE layer` — make a BG layer invisible.
pub fn bghide(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    set_visible(bg, args, ret_count, false)
}

/// `BGHOME layer, x, y` (set, ret 0, 3 args) / `BGHOME layer OUT hx, hy` (get, ret 2, 1 arg)
/// — set or read a layer's display origin (the rotation/scale pivot). A bad shape is
/// errnum 4; the layer must be in 0..3 (errnum 10).
pub fn bghome(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match ret_count {
        0 => {
            let (l, x, y) = match args {
                [l, x, y] => (l, x, y),
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            bg.set_home(layer, x.to_int()?, y.to_int()?);
            Ok(vec![])
        }
        2 => {
            let l = match args {
                [l] => l,
                _ => return Err(illegal()),
            };
            let layer = layer(l)?;
            let lr = &bg.layers[layer];
            Ok(vec![Value::Int(lr.home_x), Value::Int(lr.home_y)])
        }
        _ => Err(illegal()),
    }
}

/// `BGCLIP layer` (reset to whole layer, 1 arg) / `BGCLIP layer, startX, startY, endX, endY`
/// (pixel rectangle, 5 args; corners normalized min/max) — set a layer's display (clip)
/// area. A return value or an argument count that is neither 1 nor 5 is errnum 4; the layer
/// must be in 0..3 (errnum 10).
pub fn bgclip(
    bg: &mut BgState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    match args {
        [l] => bg.set_clip(layer(l)?, None),
        [l, sx, sy, ex, ey] => {
            let layer = layer(l)?;
            bg.set_clip(
                layer,
                Some((sx.to_int()?, sy.to_int()?, ex.to_int()?, ey.to_int()?)),
            );
        }
        _ => return Err(illegal()),
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: i32) -> Value {
        Value::Int(v)
    }
    fn s(text: &str) -> Value {
        Value::Str(text.encode_utf16().collect())
    }

    #[test]
    fn screen_data_number_and_hex_string() {
        assert_eq!(screen_data(&int(5)).unwrap(), 5);
        assert_eq!(screen_data(&int(0x1_0080)).unwrap(), 0x0080); // low 16 bits
        assert_eq!(screen_data(&s("80FF")).unwrap(), 0x80FF);
        assert_eq!(screen_data(&s("C040")).unwrap(), 0xC040);
        // A wrong type → errnum 8.
        assert_eq!(
            screen_data(&Value::Void).unwrap_err().errnum,
            super::super::ERR_TYPE_MISMATCH
        );
    }

    #[test]
    fn bgscreen_resizes_and_guards() {
        let mut bg = BgState::new();
        bgscreen(&mut bg, &[int(0), int(32), int(32)], 0).unwrap();
        assert_eq!((bg.layers[0].width, bg.layers[0].height), (32, 32));
        // 4-arg tile size.
        bgscreen(&mut bg, &[int(1), int(64), int(64), int(8)], 0).unwrap();
        assert_eq!(bg.layers[1].tile_size, 8);
        // Bad layer / area / tile size.
        assert_eq!(
            bgscreen(&mut bg, &[int(4), int(8), int(8)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            bgscreen(&mut bg, &[int(0), int(128), int(128)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            bgscreen(&mut bg, &[int(0), int(8), int(8), int(10)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        // Used as a function.
        assert_eq!(
            bgscreen(&mut bg, &[int(0), int(8), int(8)], 1)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn bgpage_set_get() {
        let mut bg = BgState::new();
        assert_eq!(
            bgpage(&mut bg, &[], 1).unwrap(),
            vec![int(sb_render::bg::BG_PAGE_DEFAULT as i32)]
        );
        bgpage(&mut bg, &[int(4)], 0).unwrap();
        assert_eq!(bgpage(&mut bg, &[], 1).unwrap(), vec![int(4)]);
        assert_eq!(bgpage(&mut bg, &[int(6)], 0).unwrap_err().errnum, 10);
        assert_eq!(bgpage(&mut bg, &[int(-1)], 0).unwrap_err().errnum, 10);
    }

    #[test]
    fn bgput_get_round_trip() {
        let mut bg = BgState::new();
        bgscreen(&mut bg, &[int(0), int(32), int(32)], 0).unwrap();
        bgput(&mut bg, &[int(0), int(20), int(15), s("80FF")], 0).unwrap();
        assert_eq!(
            bgget(&bg, &[int(0), int(20), int(15)], 1).unwrap(),
            vec![int(0x80FF)]
        );
        // Number write read back.
        bgput(&mut bg, &[int(0), int(0), int(0), int(5)], 0).unwrap();
        assert_eq!(
            bgget(&bg, &[int(0), int(0), int(0), int(0)], 1).unwrap(),
            vec![int(5)]
        );
        // Out-of-range cell.
        assert_eq!(
            bgput(&mut bg, &[int(0), int(32), int(0), int(5)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            bgget(&bg, &[int(4), int(0), int(0)], 1).unwrap_err().errnum,
            10
        );
        // BGGET as a statement.
        assert_eq!(
            bgget(&bg, &[int(0), int(0), int(0)], 0).unwrap_err().errnum,
            4
        );
    }

    #[test]
    fn bgfill_then_get() {
        let mut bg = BgState::new();
        bgscreen(&mut bg, &[int(0), int(32), int(32)], 0).unwrap();
        bgfill(
            &mut bg,
            &[int(0), int(5), int(5), int(10), int(10), int(1024)],
            0,
        )
        .unwrap();
        assert_eq!(
            bgget(&bg, &[int(0), int(7), int(7)], 1).unwrap(),
            vec![int(1024)]
        );
        assert_eq!(
            bgget(&bg, &[int(0), int(0), int(0)], 1).unwrap(),
            vec![int(0)]
        );
        assert_eq!(
            bgfill(
                &mut bg,
                &[int(4), int(0), int(0), int(1), int(1), int(1)],
                0
            )
            .unwrap_err()
            .errnum,
            10
        );
    }

    #[test]
    fn bgofs_set_get_with_z() {
        let mut bg = BgState::new();
        bgofs(&mut bg, &[int(0), int(16), int(-8), int(3)], 0).unwrap();
        assert_eq!(
            bgofs(&mut bg, &[int(0)], 3).unwrap(),
            vec![int(16), int(-8), int(3)]
        );
        assert_eq!(
            bgofs(&mut bg, &[int(0)], 2).unwrap(),
            vec![int(16), int(-8)]
        );
        assert_eq!(bgofs(&mut bg, &[int(0), int(5)], 0).unwrap_err().errnum, 4);
        assert_eq!(
            bgofs(&mut bg, &[int(4), int(0), int(0)], 0)
                .unwrap_err()
                .errnum,
            10
        );
    }

    #[test]
    fn bgrot_normalizes() {
        let mut bg = BgState::new();
        bgrot(&mut bg, &[int(0), int(-90)], 0).unwrap();
        assert_eq!(bgrot(&mut bg, &[int(0)], 1).unwrap(), vec![int(270)]);
        bgrot(&mut bg, &[int(0), int(450)], 0).unwrap();
        assert_eq!(bgrot(&mut bg, &[int(0)], 1).unwrap(), vec![int(90)]);
        assert_eq!(bgrot(&mut bg, &[int(0)], 0).unwrap_err().errnum, 4);
    }

    #[test]
    fn bgscale_unclamped_get() {
        let mut bg = BgState::new();
        bgscale(&mut bg, &[int(0), Value::Real(4.0), Value::Real(0.4)], 0).unwrap();
        assert_eq!(
            bgscale(&mut bg, &[int(0)], 2).unwrap(),
            vec![Value::Real(4.0), Value::Real(0.4)]
        );
        assert_eq!(
            bgscale(&mut bg, &[int(0), int(1)], 0).unwrap_err().errnum,
            4
        );
    }

    #[test]
    fn bgcolor_set_get() {
        let mut bg = BgState::new();
        bgcolor(&mut bg, &[int(0), int(-1)], 0).unwrap(); // &HFFFFFFFF
        assert_eq!(bgcolor(&mut bg, &[int(0)], 1).unwrap(), vec![int(-1)]);
        assert_eq!(
            bgcolor(&mut bg, &[int(0), s("x")], 0).unwrap_err().errnum,
            8
        );
        assert_eq!(
            bgcolor(&mut bg, &[int(4), int(0)], 0).unwrap_err().errnum,
            10
        );
    }

    #[test]
    fn bgshow_hide_clr() {
        let mut bg = BgState::new();
        bghide(&mut bg, &[int(0)], 0).unwrap();
        assert!(!bg.layers[0].visible);
        bgshow(&mut bg, &[int(0)], 0).unwrap();
        assert!(bg.layers[0].visible);
        assert_eq!(bgshow(&mut bg, &[], 0).unwrap_err().errnum, 4);
        assert_eq!(bghide(&mut bg, &[int(4)], 0).unwrap_err().errnum, 10);
        bgclr(&mut bg, &[], 0).unwrap();
        bgclr(&mut bg, &[int(0)], 0).unwrap();
        assert_eq!(bgclr(&mut bg, &[int(4)], 0).unwrap_err().errnum, 10);
    }

    #[test]
    fn bghome_set_get() {
        let mut bg = BgState::new();
        bghome(&mut bg, &[int(0), int(200), int(120)], 0).unwrap();
        assert_eq!(
            bghome(&mut bg, &[int(0)], 2).unwrap(),
            vec![int(200), int(120)]
        );
        assert_eq!(bghome(&mut bg, &[int(0), int(5)], 0).unwrap_err().errnum, 4);
    }

    #[test]
    fn bgclip_reset_and_rect() {
        let mut bg = BgState::new();
        bgclip(&mut bg, &[int(0), int(20), int(20), int(379), int(219)], 0).unwrap();
        assert_eq!(bg.layers[0].clip, Some((20, 20, 379, 219)));
        bgclip(&mut bg, &[int(0)], 0).unwrap();
        assert_eq!(bg.layers[0].clip, None);
        assert_eq!(
            bgclip(&mut bg, &[int(0), int(1), int(2)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        assert_eq!(bgclip(&mut bg, &[int(4)], 0).unwrap_err().errnum, 10);
    }
}
