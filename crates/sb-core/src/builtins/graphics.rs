//! Graphics builtins (M2-T1) — the GRP page-model commands the VM drives over the
//! [`GrpState`](sb_render::grp::GrpState) it owns.
//!
//! This slice covers the **page model + color helpers**: `GPAGE` (display/manipulation
//! page select), `GCLS` (clear the draw page), `GCOLOR` (current draw color), `GPRIO`
//! (screen Z priority), `GCLIP` (clip rectangles), plus `RGB`/`RGBREAD` (build/split a
//! color code) and `GSPOIT` (read a pixel). The drawing primitives that *write* the page
//! (`GPSET`/`GLINE`/…) land in M2-T2.
//!
//! ## Forms and the `ret_count` discriminator
//!
//! Several of these commands choose a SET vs GET form by their **return-value count** —
//! exactly the disassembled handlers' `[r0,#0xc]` check. The VM collapses the two ways a
//! statement can want results — a value-returning function (`C=GCOLOR()`, `ret_count` 1)
//! and an `OUT` statement (`GCOLOR OUT C`, `out_argc` 1) — into a single `ret_count`, so
//! `GPAGE OUT V,W` (`ret_count` 2) and `C=GCOLOR()` (`ret_count` 1) route correctly. Each
//! function returns the values to leave on the stack, in source order (the VM pushes them
//! so the last is topmost, matching the compiler's `OUT`-target pop order).
//!
//! ## Errors (per `spec/instructions/{gpage,gcls,gcolor,gprio,gclip,rgb,rgbread,gspoit}`)
//!
//! - **Illegal function call** (4) for a form mismatch — a bad argument *count* or a wrong
//!   return/`OUT` count.
//! - **Out of range** (10) for a page number ∉ 0..=5 (`GPAGE`) or a Z ∉ -256..=1024
//!   (`GPRIO`).
//! - **Type mismatch** (8) for a string where a number is wanted (via [`Value::to_int`]).

use sb_render::grp::GrpState;

use super::{illegal, out_of_range};
use crate::value::{RuntimeError, Value};

/// `RGB([a,] r, g, b)` — build an ARGB8888 color code. 3 args force opaque alpha (255); 4
/// args take an explicit alpha. Every channel is clamped to 0..=255 (out-of-range
/// saturates, it does not error). Must be used as a value-returning function
/// (`ret_count == 1`). The packed code is returned as an i32 (alpha-set codes read
/// negative, e.g. `RGB(255,255,255) == -1`).
pub fn rgb(args: &[Value], ret_count: usize) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let (a, r, g, b) = match args {
        [r, g, b] => (255, chan(r)?, chan(g)?, chan(b)?),
        [a, r, g, b] => (chan(a)?, chan(r)?, chan(g)?, chan(b)?),
        _ => return Err(illegal()),
    };
    let code = (a << 24) | (r << 16) | (g << 8) | b;
    Ok(vec![Value::Int(code as i32)])
}

/// `RGBREAD color OUT [a,] r, g, b` — split a color code into its 8-bit channels. Takes
/// exactly one input value (the code); the `OUT` receiver count must be 3 (R,G,B) or 4
/// (A,R,G,B). The alpha is the high byte; this is the exact inverse of [`rgb`].
pub fn rgbread(args: &[Value], ret_count: usize) -> Result<Vec<Value>, RuntimeError> {
    let color = match args {
        [c] => c.to_int()? as u32,
        _ => return Err(illegal()),
    };
    let a = ((color >> 24) & 0xff) as i32;
    let r = ((color >> 16) & 0xff) as i32;
    let g = ((color >> 8) & 0xff) as i32;
    let b = (color & 0xff) as i32;
    match ret_count {
        3 => Ok(vec![Value::Int(r), Value::Int(g), Value::Int(b)]),
        4 => Ok(vec![
            Value::Int(a),
            Value::Int(r),
            Value::Int(g),
            Value::Int(b),
        ]),
        _ => Err(illegal()),
    }
}

/// `GSPOIT(x, y)` — read one pixel's color (ARGB8888) from the current draw page. Both
/// coordinates are floored (per the docs). Off-page reads return 0. Must be used as a
/// value-returning function (`ret_count == 1`); exactly 2 args.
pub fn gspoit(
    grp: &GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 1 {
        return Err(illegal());
    }
    let (x, y) = match args {
        [x, y] => (floor_coord(x)?, floor_coord(y)?),
        _ => return Err(illegal()),
    };
    Ok(vec![Value::Int(grp.gspoit(x, y) as i32)])
}

/// `GPAGE display, manip` (SET, `ret_count` 0, 2 args) / `GPAGE OUT VP, WP` (GET,
/// `ret_count` 2, 0 args). Each page must be in 0..=5.
pub fn gpage(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match (ret_count, args.len()) {
        (0, 2) => {
            let d = page(&args[0])?;
            let m = page(&args[1])?;
            grp.display_page = d;
            grp.manip_page = m;
            Ok(vec![])
        }
        (2, 0) => Ok(vec![
            Value::Int(grp.display_page as i32),
            Value::Int(grp.manip_page as i32),
        ]),
        _ => Err(illegal()),
    }
}

/// `GCLS [color]` — clear the current draw page (black with no argument). The color is a
/// full 32-bit ARGB code, not range-checked. No return value (`ret_count` 0).
pub fn gcls(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let color = match args {
        [] => 0,
        [c] => c.to_int()? as u32,
        _ => return Err(illegal()),
    };
    grp.gcls(color);
    Ok(vec![])
}

/// `GCOLOR color` (SET, `ret_count` 0, 1 arg) / `GCOLOR OUT C` or `C=GCOLOR()` (GET,
/// `ret_count` 1, 0 args). The draw color is a full 32-bit ARGB code, not range-checked.
pub fn gcolor(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    match (ret_count, args.len()) {
        (0, 1) => {
            grp.color = args[0].to_int()? as u32;
            Ok(vec![])
        }
        (1, 0) => Ok(vec![Value::Int(grp.color as i32)]),
        _ => Err(illegal()),
    }
}

/// `GPRIO z` — set the screen Z priority. Exactly 1 arg, no return value; `z` must be in
/// -256..=1024.
pub fn gprio(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let z = match args {
        [z] => z.to_int()?,
        _ => return Err(illegal()),
    };
    if !(-256..=1024).contains(&z) {
        return Err(out_of_range());
    }
    grp.prio = z;
    Ok(vec![])
}

/// `GCLIP mode` (reset) / `GCLIP mode, x0, y0, x1, y1` (set rectangle). Mode 0 selects the
/// display clip, non-zero the write clip. 1 or 5 args, no return value.
pub fn gclip(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    match args {
        [mode] => {
            grp.gclip_reset(mode.to_int()? != 0);
            Ok(vec![])
        }
        [mode, x0, y0, x1, y1] => {
            let write = mode.to_int()? != 0;
            grp.gclip_rect(
                write,
                x0.to_int()?,
                y0.to_int()?,
                x1.to_int()?,
                y1.to_int()?,
            );
            Ok(vec![])
        }
        _ => Err(illegal()),
    }
}

/// `GPSET x,y[,color]` — plot one pixel on the draw page in the current or given color.
/// 2 or 3 args, no return value; any other shape → errnum 4 (`gpset.yaml`, hw_verified).
pub fn gpset(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (x, y, color) = match args {
        [x, y] => (floor_coord(x)?, floor_coord(y)?, grp.color),
        [x, y, c] => (floor_coord(x)?, floor_coord(y)?, c.to_int()? as u32),
        _ => return Err(illegal()),
    };
    grp.gpset(x, y, color);
    Ok(vec![])
}

/// `GLINE x1,y1,x2,y2[,color]` — draw a line in the current or given color. 4 or 5 args,
/// no return value; any other shape → errnum 4 (`gline.yaml`, hw_verified).
pub fn gline(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (x1, y1, x2, y2, color) = quad_color(grp, args)?;
    grp.gline(x1, y1, x2, y2, color);
    Ok(vec![])
}

/// `GBOX x1,y1,x2,y2[,color]` — draw a rectangle OUTLINE. 4 or 5 args, no return value;
/// any other shape → errnum 4 (`gbox.yaml`, hw_verified).
pub fn gbox(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (x1, y1, x2, y2, color) = quad_color(grp, args)?;
    grp.gbox(x1, y1, x2, y2, color);
    Ok(vec![])
}

/// `GFILL x1,y1,x2,y2[,color]` — fill a solid rectangle. 4 or 5 args, no return value;
/// any other shape → errnum 4 (`gfill.yaml`, hw_verified).
pub fn gfill(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (x1, y1, x2, y2, color) = quad_color(grp, args)?;
    grp.gfill(x1, y1, x2, y2, color);
    Ok(vec![])
}

/// `GCIRCLE x,y,r[,color]` (full circle) or `GCIRCLE x,y,r,start,end[,flag[,color]]`
/// (arc/sector). Valid arg counts are 3,4,5,6,7; no return value; any other shape →
/// errnum 4 (`gcircle.yaml`, hw_verified). `r <= 0` draws nothing.
pub fn gcircle(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    match args {
        [x, y, r] => {
            let c = grp.color;
            grp.gcircle(floor_coord(x)?, floor_coord(y)?, r.to_int()?, c);
        }
        [x, y, r, c] => {
            grp.gcircle(
                floor_coord(x)?,
                floor_coord(y)?,
                r.to_int()?,
                c.to_int()? as u32,
            );
        }
        // Arc/sector: 5 args (flag=0, default color), 6 args (+flag), 7 args (+flag+color).
        [x, y, r, s, e] => {
            let c = grp.color;
            grp.gcircle_arc(
                floor_coord(x)?,
                floor_coord(y)?,
                r.to_int()?,
                s.to_int()?,
                e.to_int()?,
                false,
                c,
            );
        }
        [x, y, r, s, e, flag] => {
            let c = grp.color;
            grp.gcircle_arc(
                floor_coord(x)?,
                floor_coord(y)?,
                r.to_int()?,
                s.to_int()?,
                e.to_int()?,
                flag.to_int()? == 1,
                c,
            );
        }
        [x, y, r, s, e, flag, c] => {
            grp.gcircle_arc(
                floor_coord(x)?,
                floor_coord(y)?,
                r.to_int()?,
                s.to_int()?,
                e.to_int()?,
                flag.to_int()? == 1,
                c.to_int()? as u32,
            );
        }
        _ => return Err(illegal()),
    }
    Ok(vec![])
}

/// `GTRI x1,y1,x2,y2,x3,y3[,color]` — draw a filled triangle. 6 or 7 args, no return
/// value; any other shape → errnum 4 (`gtri.yaml`, hw_verified).
pub fn gtri(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (x1, y1, x2, y2, x3, y3, color) = match args {
        [x1, y1, x2, y2, x3, y3] => (
            floor_coord(x1)?,
            floor_coord(y1)?,
            floor_coord(x2)?,
            floor_coord(y2)?,
            floor_coord(x3)?,
            floor_coord(y3)?,
            grp.color,
        ),
        [x1, y1, x2, y2, x3, y3, c] => (
            floor_coord(x1)?,
            floor_coord(y1)?,
            floor_coord(x2)?,
            floor_coord(y2)?,
            floor_coord(x3)?,
            floor_coord(y3)?,
            c.to_int()? as u32,
        ),
        _ => return Err(illegal()),
    };
    grp.gtri(x1, y1, x2, y2, x3, y3, color);
    Ok(vec![])
}

/// `GPAINT x,y[,fill[,border]]` — flood-fill from (x,y). 2 args default the fill to the
/// current draw color; 3 args give the fill; 4 args add a border color. No return value;
/// any other shape → errnum 4 (`gpaint.yaml`, hw_verified).
pub fn gpaint(
    grp: &mut GrpState,
    args: &[Value],
    ret_count: usize,
) -> Result<Vec<Value>, RuntimeError> {
    if ret_count != 0 {
        return Err(illegal());
    }
    let (x, y, fill, border) = match args {
        [x, y] => (floor_coord(x)?, floor_coord(y)?, grp.color, None),
        [x, y, f] => (floor_coord(x)?, floor_coord(y)?, f.to_int()? as u32, None),
        [x, y, f, b] => (
            floor_coord(x)?,
            floor_coord(y)?,
            f.to_int()? as u32,
            Some(b.to_int()? as u32),
        ),
        _ => return Err(illegal()),
    };
    grp.gpaint(x, y, fill, border);
    Ok(vec![])
}

/// Shared 4-coordinate (+ optional color) operand fetch for `GLINE`/`GBOX`/`GFILL`:
/// 4 args default the color to the current GCOLOR draw color, 5 args take the explicit
/// color. Any other count → errnum 4.
fn quad_color(grp: &GrpState, args: &[Value]) -> Result<(i32, i32, i32, i32, u32), RuntimeError> {
    match args {
        [x1, y1, x2, y2] => Ok((
            floor_coord(x1)?,
            floor_coord(y1)?,
            floor_coord(x2)?,
            floor_coord(y2)?,
            grp.color,
        )),
        [x1, y1, x2, y2, c] => Ok((
            floor_coord(x1)?,
            floor_coord(y1)?,
            floor_coord(x2)?,
            floor_coord(y2)?,
            c.to_int()? as u32,
        )),
        _ => Err(illegal()),
    }
}

/// One RGB(A) channel: integer-coerced (string → Type mismatch), clamped to 0..=255.
fn chan(v: &Value) -> Result<u32, RuntimeError> {
    Ok(v.to_int()?.clamp(0, 255) as u32)
}

/// A page index argument: integer-coerced, validated to 0..=5 (else Out of range).
fn page(v: &Value) -> Result<u8, RuntimeError> {
    let i = v.to_int()?;
    if (0..=5).contains(&i) {
        Ok(i as u8)
    } else {
        Err(out_of_range())
    }
}

/// A pixel coordinate: floored from a (possibly real) numeric argument, per the docs.
fn floor_coord(v: &Value) -> Result<i32, RuntimeError> {
    Ok(v.to_real()?.floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: &[Value]) -> i32 {
        match v {
            [Value::Int(i)] => *i,
            _ => panic!("expected a single Int, got {v:?}"),
        }
    }

    #[test]
    fn rgb_packs_and_clamps() {
        assert_eq!(
            int(&rgb(&[Value::Int(255), Value::Int(0), Value::Int(0)], 1).unwrap()),
            -65536
        );
        assert_eq!(
            int(&rgb(&[Value::Int(0), Value::Int(0), Value::Int(0)], 1).unwrap()),
            -16777216
        );
        assert_eq!(
            int(&rgb(&[Value::Int(255), Value::Int(255), Value::Int(255)], 1).unwrap()),
            -1
        );
        // 4-arg explicit alpha.
        assert_eq!(
            int(&rgb(
                &[
                    Value::Int(128),
                    Value::Int(255),
                    Value::Int(0),
                    Value::Int(0)
                ],
                1
            )
            .unwrap()),
            -2130771968
        );
        // Saturation, not error.
        assert_eq!(
            int(&rgb(&[Value::Int(999), Value::Int(999), Value::Int(999)], 1).unwrap()),
            -1
        );
        assert_eq!(
            int(&rgb(&[Value::Int(-5), Value::Int(300), Value::Int(0)], 1).unwrap()),
            -16711936
        );
    }

    #[test]
    fn rgb_arg_and_form_errors() {
        assert_eq!(
            rgb(&[Value::Int(1), Value::Int(2)], 1).unwrap_err().errnum,
            4
        );
        assert_eq!(
            rgb(
                &[
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                    Value::Int(4),
                    Value::Int(5)
                ],
                1
            )
            .unwrap_err()
            .errnum,
            4
        );
        // Used as a statement (no return value) → errnum 4.
        assert_eq!(
            rgb(&[Value::Int(1), Value::Int(2), Value::Int(3)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        // String channel → Type mismatch.
        assert_eq!(
            rgb(&[Value::str_from("x"), Value::Int(2), Value::Int(3)], 1)
                .unwrap_err()
                .errnum,
            8
        );
    }

    #[test]
    fn rgbread_splits() {
        let r = rgbread(&[Value::Int(0xFF80_4020u32 as i32)], 3).unwrap();
        assert_eq!(r, vec![Value::Int(128), Value::Int(64), Value::Int(32)]);
        let r = rgbread(&[Value::Int(0x80FF_8040u32 as i32)], 4).unwrap();
        assert_eq!(
            r,
            vec![
                Value::Int(128),
                Value::Int(255),
                Value::Int(128),
                Value::Int(64)
            ]
        );
        // -1 = 0xFFFFFFFF → all 255.
        assert_eq!(
            rgbread(&[Value::Int(-1)], 4).unwrap(),
            vec![Value::Int(255); 4]
        );
        // Bad OUT count → errnum 4.
        assert_eq!(rgbread(&[Value::Int(0)], 2).unwrap_err().errnum, 4);
        assert_eq!(rgbread(&[Value::Int(0)], 5).unwrap_err().errnum, 4);
    }

    #[test]
    fn rgb_rgbread_round_trip() {
        let code = int(&rgb(&[Value::Int(160), Value::Int(128), Value::Int(96)], 1).unwrap());
        assert_eq!(
            rgbread(&[Value::Int(code)], 3).unwrap(),
            vec![Value::Int(160), Value::Int(128), Value::Int(96)]
        );
    }

    #[test]
    fn gpage_set_get_and_errors() {
        let mut g = GrpState::new();
        assert!(gpage(&mut g, &[Value::Int(1), Value::Int(2)], 0)
            .unwrap()
            .is_empty());
        assert_eq!((g.display_page, g.manip_page), (1, 2));
        assert_eq!(
            gpage(&mut g, &[], 2).unwrap(),
            vec![Value::Int(1), Value::Int(2)]
        );
        // Out of range page → errnum 10.
        assert_eq!(
            gpage(&mut g, &[Value::Int(6), Value::Int(0)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(
            gpage(&mut g, &[Value::Int(0), Value::Int(-1)], 0)
                .unwrap_err()
                .errnum,
            10
        );
        // Wrong form → errnum 4.
        assert_eq!(gpage(&mut g, &[Value::Int(0)], 0).unwrap_err().errnum, 4);
        assert_eq!(
            gpage(&mut g, &[Value::Int(0), Value::Int(0), Value::Int(0)], 0)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn gcolor_set_get_and_errors() {
        let mut g = GrpState::new();
        gcolor(&mut g, &[Value::Int(100)], 0).unwrap();
        assert_eq!(g.color, 100);
        assert_eq!(gcolor(&mut g, &[], 1).unwrap(), vec![Value::Int(100)]);
        assert_eq!(gcolor(&mut g, &[], 0).unwrap_err().errnum, 4);
        assert_eq!(
            gcolor(&mut g, &[Value::Int(1), Value::Int(2)], 0)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn gprio_range_and_form() {
        let mut g = GrpState::new();
        gprio(&mut g, &[Value::Int(-256)], 0).unwrap();
        gprio(&mut g, &[Value::Int(1024)], 0).unwrap();
        assert_eq!(g.prio, 1024);
        assert_eq!(
            gprio(&mut g, &[Value::Int(1025)], 0).unwrap_err().errnum,
            10
        );
        assert_eq!(
            gprio(&mut g, &[Value::Int(-257)], 0).unwrap_err().errnum,
            10
        );
        assert_eq!(gprio(&mut g, &[Value::Int(0)], 1).unwrap_err().errnum, 4); // as function
    }

    #[test]
    fn gcls_and_gclip_forms() {
        let mut g = GrpState::new();
        gcls(&mut g, &[], 0).unwrap();
        gcls(&mut g, &[Value::Int(0)], 0).unwrap();
        assert_eq!(gcls(&mut g, &[], 1).unwrap_err().errnum, 4); // as function
        assert_eq!(
            gcls(&mut g, &[Value::Int(0), Value::Int(0)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        gclip(&mut g, &[Value::Int(0)], 0).unwrap();
        gclip(
            &mut g,
            &[
                Value::Int(0),
                Value::Int(100),
                Value::Int(100),
                Value::Int(200),
                Value::Int(200),
            ],
            0,
        )
        .unwrap();
        assert_eq!(
            gclip(&mut g, &[Value::Int(0), Value::Int(1), Value::Int(2)], 0)
                .unwrap_err()
                .errnum,
            4
        );
        assert_eq!(gclip(&mut g, &[Value::Int(0)], 1).unwrap_err().errnum, 4); // as function
    }

    #[test]
    fn primitives_reject_bad_arg_counts_and_return_use() {
        let mut g = GrpState::new();
        // GPSET: 2/3 args ok, others → 4; used as a function (ret_count 1) → 4.
        assert!(gpset(&mut g, &[Value::Int(1), Value::Int(2)], 0).is_ok());
        assert_eq!(gpset(&mut g, &[Value::Int(1)], 0).unwrap_err().errnum, 4);
        assert_eq!(
            gpset(&mut g, &[Value::Int(1), Value::Int(2)], 1)
                .unwrap_err()
                .errnum,
            4
        );
        // GLINE/GBOX/GFILL: 4/5 args.
        let four = [Value::Int(0), Value::Int(0), Value::Int(1), Value::Int(1)];
        for f in [gline, gbox, gfill] {
            assert!(f(&mut g, &four, 0).is_ok());
            assert_eq!(f(&mut g, &four[..3], 0).unwrap_err().errnum, 4);
            assert_eq!(f(&mut g, &four, 1).unwrap_err().errnum, 4);
        }
        // GTRI: 6/7 args.
        let ints = |n: usize| -> Vec<Value> { (0..n).map(|_| Value::Int(1)).collect() };
        assert!(gtri(&mut g, &ints(6), 0).is_ok());
        assert_eq!(gtri(&mut g, &ints(5), 0).unwrap_err().errnum, 4);
        // GPAINT: 2/3/4 args.
        assert!(gpaint(&mut g, &four[..2], 0).is_ok());
        assert!(gpaint(&mut g, &four, 0).is_ok());
        assert_eq!(gpaint(&mut g, &four[..1], 0).unwrap_err().errnum, 4);
        assert_eq!(gpaint(&mut g, &ints(5), 0).unwrap_err().errnum, 4);
        // GCIRCLE: 3..=7 args ok, 2 and 8 → 4.
        assert!(gcircle(&mut g, &ints(3), 0).is_ok());
        assert!(gcircle(&mut g, &ints(7), 0).is_ok());
        assert_eq!(gcircle(&mut g, &ints(2), 0).unwrap_err().errnum, 4);
        assert_eq!(gcircle(&mut g, &ints(8), 0).unwrap_err().errnum, 4);
    }

    #[test]
    fn gpset_uses_default_then_explicit_color() {
        let mut g = GrpState::new();
        gcolor(&mut g, &[Value::Int(0xFF00_FF00u32 as i32)], 0).unwrap(); // opaque green
        gpset(&mut g, &[Value::Int(5), Value::Int(5)], 0).unwrap();
        assert_eq!(g.gspoit(5, 5), 0xFF00_F800); // green, RGBA5551-truncated
        gpset(
            &mut g,
            &[
                Value::Int(6),
                Value::Int(6),
                Value::Int(0xFFFF_0000u32 as i32),
            ],
            0,
        )
        .unwrap();
        assert_eq!(g.gspoit(6, 6), 0xFFF8_0000); // explicit red overrides the draw color
    }

    #[test]
    fn gspoit_reads_manip_page() {
        let mut g = GrpState::new();
        g.gcls(0xFFFF_0000);
        assert_eq!(
            int(&gspoit(&g, &[Value::Int(0), Value::Int(0)], 1).unwrap()),
            0xFFF8_0000u32 as i32
        );
        assert_eq!(
            int(&gspoit(&g, &[Value::Int(-1), Value::Int(-1)], 1).unwrap()),
            0
        );
        assert_eq!(gspoit(&g, &[Value::Int(1)], 1).unwrap_err().errnum, 4);
        assert_eq!(
            gspoit(&g, &[Value::Int(1), Value::Int(2), Value::Int(3)], 1)
                .unwrap_err()
                .errnum,
            4
        );
        assert_eq!(
            gspoit(&g, &[Value::Int(1), Value::Int(1)], 0)
                .unwrap_err()
                .errnum,
            4
        ); // as statement
    }
}
