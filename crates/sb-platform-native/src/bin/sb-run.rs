//! `sb-run` — the headless SmileBASIC runner (M1-T11).
//!
//! Loads a `.sb3` file (plain UTF-8 SmileBASIC source), runs it through the
//! `sb-core` pipeline (`lexer → parser → compiler → VM`, see
//! `spec/concepts/execution-model.md`) with no window, and dumps the resulting
//! text console grid to stdout — exactly what the oracle scrapes from console
//! memory. On a SmileBASIC error it prints `ERRNUM`/`ERRLINE` to stderr.
//!
//! This is the executable `harness/diff/replay.py` shells out to
//! (`target/debug/sb-run`) for the deterministic full-program replay gate.
//!
//! With `--grp <out.png>` it instead renders the GRP **display page** (the visible top-left
//! 400×240 crop, RGBA8888) to a PNG and writes it to `out.png` — the deterministic graphics
//! golden the M2-T5 pixel-diff (`harness/diff/replay.py`) compares against an oracle GRP
//! capture (O-T6). The program still runs through the same VM; only the output differs.
//!
//! With `--composite <out.png> [top|bottom|both]` it renders the FULL composited framebuffer —
//! backdrop → GRP → BG → sprites → console → fader, exactly the layer stack
//! [`sb_render::compositor::compose_screen`] builds — for the selected screen(s) and writes it
//! to `out.png`. This is the deterministic CI counterpart of the oracle composite screenshot
//! (O-T6), so `harness/diff/replay.py` can pixel-diff `golden/composite/*.png` the same way it
//! diffs the GRP goldens. `top` (default) is the 400×240 Upper screen; `bottom` is the Touch
//! screen; `both` stacks Upper over Touch into one frame. Because a composite golden program
//! typically draws a static scene and then spins in `WHILE 1:WEND` (mirroring how the oracle
//! screenshots a running program), the VM is run for a bounded number of frames rather than to
//! completion — see [`COMPOSITE_FRAMES`].
//!
//! Exit codes:
//!   0  — ran to completion (`END`/fell off the end) or stopped via `STOP`.
//!   1  — a SmileBASIC error (parse/compile/runtime): `ERRNUM`/`ERRLINE` on stderr.
//!   2  — a usage / host error (missing arg, unreadable file).

use std::process::ExitCode;

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::{parse, Vm, VmError};
use sb_render::compositor::{apply_fader, compose_screen, grp_page_to_framebuffer};
use sb_render::grp::{GRP_VISIBLE_HEIGHT, GRP_VISIBLE_WIDTH};
use sb_render::Framebuffer;

/// How many displayed frames to run before snapshotting the composite scene. A composite golden
/// program draws its scene up front and then spins in `WHILE 1:WEND` (it never `END`s — it is a
/// running program the oracle screenshots), so the runner can't wait for a halt. Eight frames is
/// well past the setup of every committed golden while staying cheap for an infinite loop.
const COMPOSITE_FRAMES: usize = 8;

/// Per-frame instruction budget for the bounded composite run. A frame that hits `WHILE 1:WEND`
/// (no `VSYNC`) burns this whole budget spinning, so keep it modest — 200k × 8 frames is fast and
/// is far more than any setup needs.
const COMPOSITE_FRAME_BUDGET: usize = 200_000;

/// A SmileBASIC error reduced to its `ERRNUM` + 1-based `ERRLINE`, however it was
/// raised (parse, compile or runtime).
#[derive(Debug)]
struct SbError {
    errnum: u32,
    line: u32,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `--grp <out.png> <program.sb3>`: render the display page to a golden PNG instead of
    // dumping console text. Everything else is the headless text runner.
    if args.first().map(String::as_str) == Some("--grp") {
        let (Some(out), Some(path)) = (args.get(1), args.get(2)) else {
            eprintln!("usage: sb-run --grp <out.png> <program.sb3>");
            return ExitCode::from(2);
        };
        return run_grp(path, out);
    }

    // `--composite <out.png> [top|bottom|both] <program.sb3>`: render the full composited scene.
    // The screen selector is optional and defaults to `top`.
    if args.first().map(String::as_str) == Some("--composite") {
        let usage = "usage: sb-run --composite <out.png> [top|bottom|both] <program.sb3>";
        let Some(out) = args.get(1) else {
            eprintln!("{usage}");
            return ExitCode::from(2);
        };
        let (which, path) = match (args.get(2), args.get(3)) {
            // Three-arg form: explicit screen selector + program.
            (Some(sel), Some(path)) => {
                let Some(which) = CompositeScreen::parse(sel) else {
                    eprintln!("sb-run: unknown screen {sel:?} (want top|bottom|both)\n{usage}");
                    return ExitCode::from(2);
                };
                (which, path)
            }
            // Two-arg form: just the program, defaulting to the top screen.
            (Some(path), None) => (CompositeScreen::Top, path),
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        };
        return run_composite(path, out, which);
    }

    let Some(path) = args.first() else {
        eprintln!(
            "usage: sb-run <program.sb3>  |  sb-run --grp <out.png> <program.sb3>  |  \
             sb-run --composite <out.png> [top|bottom|both] <program.sb3>"
        );
        return ExitCode::from(2);
    };

    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sb-run: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };

    match run(&src) {
        Ok(text) => {
            // The console grid is the deterministic stdout. `println!` adds the final
            // newline so piped output ends cleanly even when the last row is full.
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ERRNUM={} ERRLINE={}", e.errnum, e.line);
            ExitCode::from(1)
        }
    }
}

/// Run `path`'s program and write its GRP display page (visible 400×240 crop) as a PNG to
/// `out`. The VM runs to completion (or to its first error — the partial page is still
/// written, mirroring how the windowed host composites a halted scene); a SmileBASIC error is
/// reported on stderr and yields exit 1, but the PNG is produced either way so the diff sees
/// what the program actually drew.
fn run_grp(path: &str, out: &str) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sb-run: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    // parse → compile share the same `{errnum, loc.line}` error shape; collapse either into a
    // pre-run SbError, otherwise run the VM and snapshot its display page.
    let compiled = parse(&src)
        .map_err(|e| SbError {
            errnum: e.errnum,
            line: e.loc.line,
        })
        .and_then(|ast| {
            compile_with(&ast, &StdBuiltins).map_err(|e| SbError {
                errnum: e.errnum,
                line: e.loc.line,
            })
        });
    let (fb, err) = match compiled {
        Ok(program) => {
            let mut vm = Vm::new(program);
            let err = vm.run().err().and_then(sb_error_of);
            let page = &vm.grp().pages[vm.grp().cur().display_page as usize];
            let fb = grp_page_to_framebuffer(
                page,
                GRP_VISIBLE_WIDTH as usize,
                GRP_VISIBLE_HEIGHT as usize,
            );
            (Some(fb), err)
        }
        Err(e) => (None, Some(e)),
    };
    if let Some(fb) = fb {
        if let Err(e) = std::fs::write(out, sb_render::png::encode(&fb)) {
            eprintln!("sb-run: cannot write {out}: {e}");
            return ExitCode::from(2);
        }
    }
    match err {
        None => ExitCode::SUCCESS,
        Some(e) => {
            eprintln!("ERRNUM={} ERRLINE={}", e.errnum, e.line);
            ExitCode::from(1)
        }
    }
}

/// Which physical screen(s) `--composite` renders.
#[derive(Clone, Copy)]
enum CompositeScreen {
    /// The Upper screen only (screen 0) — the default; matches the 400×240 composite goldens.
    Top,
    /// The Touch screen only (screen 1).
    Bottom,
    /// Upper stacked over Touch in one frame (Upper on top).
    Both,
}

impl CompositeScreen {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "top" | "upper" => Some(Self::Top),
            "bottom" | "touch" | "lower" => Some(Self::Bottom),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// Composite one physical screen's scene into a framebuffer sized for the current `XSCREEN`
/// mode — the same layer stack and per-screen state the wasm/windowed hosts draw
/// (`compose_for_screen`): backdrop → GRP → BG → sprites → console, with the global fader
/// overlaid in front. The backdrop is the live `BACKCOLOR` so a `BACKCOLOR` golden matches.
fn compose_one(vm: &Vm, screen_id: usize) -> Framebuffer {
    let screen = vm.screen_config();
    let (w, h) = screen.display_size_for(screen_id as i32);
    let mut fb = compose_screen(
        w as usize,
        h as usize,
        vm.grp(),
        screen_id,
        vm.bg_for(screen_id),
        vm.sprites_for(screen_id),
        vm.console_for(screen_id),
        vm.backdrop(),
        screen.visibility_for(screen_id as i32),
    );
    if let Some(color) = vm.fader_overlay() {
        apply_fader(&mut fb, color);
    }
    fb
}

/// Stack `top` over `bottom` into one framebuffer (Upper on top). The frame is as wide as the
/// wider screen; a narrower screen is centred in its band (the Touch screen is 320 px, the Upper
/// up to 400). Pixels outside either screen are opaque black, matching the emulator's letterbox.
fn stack_screens(top: &Framebuffer, bottom: &Framebuffer) -> Framebuffer {
    let width = top.width.max(bottom.width);
    let mut fb = Framebuffer::new(width, top.height + bottom.height);
    fb.clear(0xFF00_0000);
    let blit = |fb: &mut Framebuffer, src: &Framebuffer, y_off: usize| {
        let x_off = (width - src.width) / 2;
        for y in 0..src.height {
            for x in 0..src.width {
                fb.set_argb(x_off + x, y_off + y, src.get_argb(x, y));
            }
        }
    };
    blit(&mut fb, top, 0);
    blit(&mut fb, bottom, top.height);
    fb
}

/// Run `path`'s program for a bounded number of frames and write its composited scene (the
/// selected screen(s)) as a PNG to `out`. Mirrors [`run_grp`]'s error handling: a SmileBASIC
/// error is reported on stderr and yields exit 1, but the PNG is produced either way so the diff
/// sees what the program drew before halting.
fn run_composite(path: &str, out: &str, which: CompositeScreen) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sb-run: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let compiled = parse(&src)
        .map_err(|e| SbError {
            errnum: e.errnum,
            line: e.loc.line,
        })
        .and_then(|ast| {
            compile_with(&ast, &StdBuiltins).map_err(|e| SbError {
                errnum: e.errnum,
                line: e.loc.line,
            })
        });
    let (fb, err) = match compiled {
        Ok(program) => {
            let mut vm = Vm::new(program);
            // Run a fixed budget of frames so a never-ending program (the WHILE 1:WEND golden
            // pattern) still snapshots deterministically; stop early on a real halt or error.
            let mut err = None;
            for _ in 0..COMPOSITE_FRAMES {
                vm.tick_frame();
                match vm.run_frame(COMPOSITE_FRAME_BUDGET) {
                    Ok(Some(_)) => break, // END / STOP / ASSERT — scene is final.
                    Ok(None) => {}        // yielded or budget exhausted — keep going.
                    Err(e) => {
                        err = sb_error_of(e);
                        break;
                    }
                }
            }
            let fb = match which {
                CompositeScreen::Top => compose_one(&vm, 0),
                CompositeScreen::Bottom => compose_one(&vm, 1),
                CompositeScreen::Both => stack_screens(&compose_one(&vm, 0), &compose_one(&vm, 1)),
            };
            (Some(fb), err)
        }
        Err(e) => (None, Some(e)),
    };
    if let Some(fb) = fb {
        if let Err(e) = std::fs::write(out, sb_render::png::encode(&fb)) {
            eprintln!("sb-run: cannot write {out}: {e}");
            return ExitCode::from(2);
        }
    }
    match err {
        None => ExitCode::SUCCESS,
        Some(e) => {
            eprintln!("ERRNUM={} ERRLINE={}", e.errnum, e.line);
            ExitCode::from(1)
        }
    }
}

/// Reduce a runtime [`VmError`] to its `ERRNUM`/`ERRLINE` (or `None` for a non-error halt).
fn sb_error_of(e: VmError) -> Option<SbError> {
    match e {
        VmError::Sb { errnum, line } => Some(SbError { errnum, line }),
        VmError::Unsupported(what) => {
            eprintln!("sb-run: unsupported: {what}");
            Some(SbError { errnum: 0, line: 0 })
        }
        VmError::Assert { message, line } => {
            eprintln!("sb-run: ASSERT__ failed at line {line}: {message}");
            Some(SbError { errnum: 0, line })
        }
    }
}

/// Run `src` to completion, returning the console text or the SmileBASIC error.
fn run(src: &str) -> Result<String, SbError> {
    let ast = parse(src).map_err(|e| SbError {
        errnum: e.errnum,
        line: e.loc.line,
    })?;
    let program = compile_with(&ast, &StdBuiltins).map_err(|e| SbError {
        errnum: e.errnum,
        line: e.loc.line,
    })?;
    let mut vm = Vm::new(program);
    match vm.run() {
        // `END`, falling off the end and `STOP` are all non-error halts.
        Ok(_) => Ok(vm.console_text()),
        Err(VmError::Sb { errnum, line }) => Err(SbError { errnum, line }),
        // An opcode whose handler lands in a later milestone — surface it as a
        // generic "Internal error" (errnum 0) rather than masquerading as success.
        Err(VmError::Unsupported(what)) => {
            eprintln!("sb-run: unsupported: {what}");
            Err(SbError { errnum: 0, line: 0 })
        }
        // A failed `ASSERT__` (test-mode builtin) — report and exit as an error.
        Err(VmError::Assert { message, line }) => {
            eprintln!("sb-run: ASSERT__ failed at line {line}: {message}");
            Err(SbError { errnum: 0, line })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sb_core::builtins::StdBuiltins;
    use sb_core::compiler::compile_with;
    use sb_core::{parse, Vm};

    /// Build a VM, run the bounded composite loop, and composite the top screen — the test-side
    /// mirror of `run_composite`'s render path (without the PNG write).
    fn composite_top(src: &str) -> Framebuffer {
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        for _ in 0..COMPOSITE_FRAMES {
            vm.tick_frame();
            if let Ok(Some(_)) = vm.run_frame(COMPOSITE_FRAME_BUDGET) {
                break;
            }
        }
        compose_one(&vm, 0)
    }

    #[test]
    fn prints_console_text() {
        let text = run(r#"?"HI""#).expect("ok");
        assert!(text.starts_with("HI"), "got {text:?}");
    }

    #[test]
    fn fizzbuzz_fixture_runs() {
        // The committed full-program fixture the replay gate uses. `console_text`
        // is the final scrolled grid, so we assert it ends on I=100 → BUZZ and
        // still shows the FIZZBUZZ multiples-of-15 line above it.
        let src = include_str!("../../../../harness/corpus/programs/fizzbuzz.sb3");
        let text = run(src).expect("fizzbuzz runs clean");
        assert!(text.contains("FIZZBUZZ"), "got {text:?}");
        assert!(text.trim_end().ends_with("BUZZ"), "got {text:?}");
    }

    #[test]
    fn runtime_error_carries_errnum_and_line() {
        // SQR(-1) → Out of range (10), on source line 1 (hw_verified, O-T5).
        let err = run("A=SQR(-1)").expect_err("should error");
        assert_eq!(err.errnum, 10);
        assert_eq!(err.line, 1);
    }

    #[test]
    fn parse_error_is_syntax_errnum_3() {
        let err = run("FOR I=").expect_err("should error");
        assert_eq!(err.errnum, 3);
    }

    #[test]
    fn composite_top_is_400x240() {
        let fb = composite_top("ACLS");
        assert_eq!(fb.width, sb_render::TOP_WIDTH);
        assert_eq!(fb.height, sb_render::TOP_HEIGHT);
    }

    #[test]
    fn composite_backcolor_fills_backdrop_even_in_infinite_loop() {
        // The committed composite-golden pattern: set the backdrop, then spin forever. The
        // bounded run must NOT hang, and the backdrop must be the live BACKCOLOR (opaque blue),
        // not the hardcoded DEFAULT_BACKDROP — every pixel here is backdrop (no other layer draws).
        let fb = composite_top("ACLS:BACKCOLOR RGB(0,0,255)\nWHILE 1:WEND");
        assert_eq!(fb.get_argb(0, 0), 0xFF00_00FF);
        assert_eq!(fb.get_argb(200, 120), 0xFF00_00FF);
    }

    #[test]
    fn composite_default_backdrop_is_opaque_black() {
        let fb = composite_top("ACLS:WHILE 1:WEND");
        assert_eq!(fb.get_argb(10, 10), 0xFF00_0000);
    }

    #[test]
    fn composite_gcls_paints_the_grp_page() {
        // GCLS fills the GRP page; the compositor blits it over the backdrop, so the whole top
        // screen reads back as the GCLS color. The GRP page is RGBA5551, so the 8-bit channel
        // expands by left-shift (255 → 5-bit 31 → 0xF8), matching the committed gfx goldens.
        let fb = composite_top("GCLS RGB(255,0,0)");
        assert_eq!(fb.get_argb(0, 0), 0xFFF8_0000);
        assert_eq!(fb.get_argb(399, 239), 0xFFF8_0000);
    }

    #[test]
    fn stack_screens_letterboxes_the_narrower_screen() {
        // Upper 400 wide over Touch 320 wide → 400×480, Touch centred (40 px black margin).
        let top = Framebuffer::new(sb_render::TOP_WIDTH, sb_render::TOP_HEIGHT);
        let mut bottom = Framebuffer::new(sb_render::BOTTOM_WIDTH, sb_render::BOTTOM_HEIGHT);
        bottom.clear(0xFF00_FF00);
        let fb = stack_screens(&top, &bottom);
        assert_eq!(fb.width, sb_render::TOP_WIDTH);
        assert_eq!(fb.height, sb_render::TOP_HEIGHT + sb_render::BOTTOM_HEIGHT);
        // Left margin of the bottom band is black; its centre is the green fill.
        assert_eq!(fb.get_argb(0, sb_render::TOP_HEIGHT + 1), 0xFF00_0000);
        assert_eq!(fb.get_argb(200, sb_render::TOP_HEIGHT + 1), 0xFF00_FF00);
    }
}
