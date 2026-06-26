//! `sb-platform-wasm` — the browser host for the SmileBASIC interpreter (M1-T12, M4-T5).
//!
//! A `wasm-bindgen` module that runs a program through the `sb-core` pipeline (the same
//! `parse → compile → VM` as the headless `sb-run` and the desktop `sb` window), renders the
//! resulting text console into an [`sb_render::Framebuffer`], and blits that RGBA8888 buffer
//! straight onto an HTML `<canvas>` via `CanvasRenderingContext2d::put_image_data`.
//!
//! The framebuffer model is shared with the native host; only the final blit differs
//! (canvas `ImageData` here, a softbuffer window there). The browser bindings are gated to
//! `wasm32`, so this crate compiles to a thin rlib (just [`render_program`] + [`keymap`]) on
//! the desktop and the workspace's native `cargo build`/`cargo test` never pull in `web-sys`.
//!
//! Two entry points (both `wasm32`-only):
//! * [`web::run_program`] — one-shot: run to completion, paint the final scene once.
//! * [`web::run_interactive`] — run to completion, then a `requestAnimationFrame` loop that
//!   feeds live keyboard / mouse input into the VM each frame (M4-T5) and re-paints, the
//!   browser counterpart of the native 60 fps host loop.
//!
//! The default browser keymap ([`keymap`]) matches the native one — keyed on DOM
//! `KeyboardEvent.code` physical codes — so a program reads the same `BUTTON`/`STICK` on
//! both hosts:
//!
//! ```text
//!   Arrow keys ........ D-pad   (#UP #DOWN #LEFT #RIGHT)
//!   U I J K ........... face    (#Y #X #B #A)
//!   Q E / 1 2 ......... shoulders (#L #R) / (#ZL #ZR)
//!   W A S D ........... left Circle Pad  (STICK)
//!   Numpad 8 4 2 6 .... right Circle Pad Pro (STICKEX)
//!   Mouse on canvas ... touch screen (TOUCH); left button = touching
//! ```

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::{parse, Vm};
use sb_render::compositor::{apply_fader, compose_screen, DEFAULT_BACKDROP};
use sb_render::Framebuffer;

// The browser IndexedDB storage backend (M6-T1): backs the `sb-core` `Storage` trait with an
// in-memory mirror persisted to IndexedDB. `wasm32`-only (the native host uses the filesystem);
// the storage *logic* lives in the wasm-safe, gate-tested `sb-core` core.
#[cfg(target_arch = "wasm32")]
pub mod storage;

/// The default browser keymap: DOM `KeyboardEvent.code` → logical
/// [`Bind`](sb_core::host_input::Bind). Kept platform-side (the native host keys the same
/// layout off winit `KeyCode`) and device-neutral so it builds + is tested on the desktop.
pub mod keymap {
    use sb_core::host_input::{Bind, Stick};
    use sb_core::input::{
        BTN_A, BTN_B, BTN_DOWN, BTN_L, BTN_LEFT, BTN_R, BTN_RIGHT, BTN_UP, BTN_X, BTN_Y, BTN_ZL,
        BTN_ZR,
    };

    /// Map one DOM physical key code to its default binding, or `None` if unbound.
    pub fn bind(code: &str) -> Option<Bind> {
        Some(match code {
            // D-pad — arrow keys.
            "ArrowUp" => Bind::Button(BTN_UP),
            "ArrowDown" => Bind::Button(BTN_DOWN),
            "ArrowLeft" => Bind::Button(BTN_LEFT),
            "ArrowRight" => Bind::Button(BTN_RIGHT),
            // Face buttons — U/I/J/K diamond (#Y top, #A bottom, #B left, #X right).
            "KeyU" => Bind::Button(BTN_Y),
            "KeyI" => Bind::Button(BTN_X),
            "KeyJ" => Bind::Button(BTN_B),
            "KeyK" => Bind::Button(BTN_A),
            // Shoulders.
            "KeyQ" => Bind::Button(BTN_L),
            "KeyE" => Bind::Button(BTN_R),
            "Digit1" => Bind::Button(BTN_ZL),
            "Digit2" => Bind::Button(BTN_ZR),
            // Left Circle Pad (STICK) — WASD.
            "KeyW" => Bind::AxisY(Stick::Left, 1.0),
            "KeyS" => Bind::AxisY(Stick::Left, -1.0),
            "KeyA" => Bind::AxisX(Stick::Left, -1.0),
            "KeyD" => Bind::AxisX(Stick::Left, 1.0),
            // Right Circle Pad Pro (STICKEX) — numpad arrows.
            "Numpad8" => Bind::AxisY(Stick::Right, 1.0),
            "Numpad2" => Bind::AxisY(Stick::Right, -1.0),
            "Numpad4" => Bind::AxisX(Stick::Right, -1.0),
            "Numpad6" => Bind::AxisX(Stick::Right, 1.0),
            _ => return None,
        })
    }
}

/// Parse → compile → run `src` to completion, returning the live VM. The halt result is
/// ignored — a halted program's partial scene is still worth showing / animating.
fn build_vm(src: &str) -> Result<Vm, String> {
    let mut vm = prepare_vm(src)?;
    let _ = vm.run();
    Ok(vm)
}

/// Parse → compile → create a fresh VM without running it. Used by the interactive wasm host
/// so the platform's `requestAnimationFrame` loop can drive execution with live input.
fn prepare_vm(src: &str) -> Result<Vm, String> {
    let ast = parse(src).map_err(|e| e.to_string())?;
    let program = compile_with(&ast, &StdBuiltins).map_err(|e| e.to_string())?;
    Ok(Vm::new(program))
}

/// Composite one named screen (`screen_id`: 0 = Upper, 1 = Touch) from ITS OWN per-screen
/// context — the GRP display page + clip for that screen, plus the per-screen BG / sprite
/// tables and `VISIBLE` flags — into a framebuffer sized for that screen under the current
/// `XSCREEN` mode (via [`ScreenConfig::display_size_for`]). This is the per-screen primitive
/// every other entry point routes through.
pub fn compose_for_screen(vm: &Vm, screen_id: usize) -> Framebuffer {
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
        DEFAULT_BACKDROP,
        screen.visibility_for(screen_id as i32),
    );
    // The screen fader is a global overlay drawn in front of both screens.
    if let Some(color) = vm.fader_overlay() {
        apply_fader(&mut fb, color);
    }
    fb
}

/// Composite the VM's current scene into a single framebuffer sized for the active `XSCREEN`
/// mode and `DISPLAY` target. Modes 2/3 with `DISPLAY 1` render the 320×240 Touch Screen;
/// mode 4 renders the one shared 320×480 vertical screen (screen 0); everything else renders
/// the Upper screen. Delegates to [`compose_for_screen`] for whichever screen it selects.
fn compose(vm: &Vm) -> Framebuffer {
    let screen = vm.screen_config();
    match screen.mode {
        2 | 3 if screen.display == 1 => compose_for_screen(vm, 1),
        // Mode 4 is a single combined surface driven by screen 0's context;
        // `display_size_for(0)` returns the full 320×480 area in mode 4.
        _ => compose_for_screen(vm, 0),
    }
}

/// Composite BOTH physical screens for the current `XSCREEN` mode: always the Upper-screen
/// framebuffer, plus `Some(touch_fb)` ONLY when the mode (2 or 3) exposes the Touch Screen as
/// an independent second graphics screen. Modes 0/1 (Upper only) and mode 4 (one combined
/// surface) return `None`. This is the API the dual-canvas player (#81) consumes — each
/// framebuffer comes from [`compose_for_screen`] with the matching screen id.
pub fn compose_both(vm: &Vm) -> (Framebuffer, Option<Framebuffer>) {
    let upper = compose_for_screen(vm, 0);
    let touch = match vm.screen_config().mode {
        2 | 3 => Some(compose_for_screen(vm, 1)),
        _ => None,
    };
    (upper, touch)
}

/// Run `src` to completion and return the final scene composited into a top-screen
/// framebuffer (RGBA8888). On a SmileBASIC error the partial scene is composited anyway — the
/// canvas shows whatever the program drew before it halted. Backdrop is opaque black so the
/// (transparent-by-default) GRP/console pixels blit to a visible surface.
pub fn render_program(src: &str) -> Framebuffer {
    match build_vm(src) {
        Ok(vm) => compose(&vm),
        Err(_) => blank(),
    }
}

/// An opaque-black top-screen framebuffer (used when the program fails to parse/compile).
fn blank() -> Framebuffer {
    let mut fb = Framebuffer::top();
    fb.clear(DEFAULT_BACKDROP);
    fb
}

/// Wall-clock duration of one displayed frame in milliseconds (1/60 s). The browser rAF
/// loop paces VM frames to this interval so a high-refresh display (120/144 Hz) doesn't
/// run the program too fast — the web counterpart of the native host's
/// [`FRAME_DURATION`](sb_core::clock::FRAME_DURATION) pacer. Compiled on `wasm32` (where the
/// rAF loop uses it) and under `test` (so the pacing math is unit-testable on the desktop).
#[cfg(any(target_arch = "wasm32", test))]
const FRAME_MS: f64 = 1000.0 / sb_core::clock::FPS as f64;

/// Maximum VM frames to run in one rAF tick when catching up after a stall (e.g. the tab
/// was backgrounded and rAF paused). Beyond this the pacer resyncs to "now" and drops the
/// backlog rather than firing a burst that would freeze the tab — the web analog of the
/// native host's "if we fell far behind, resync to now" rule.
#[cfg(any(target_arch = "wasm32", test))]
const MAX_CATCHUP: u32 = 3;

/// Accumulate whole 1/60 s frames elapsed since `*next_frame` and advance the deadline,
/// returning how many VM frames the rAF loop should run this tick. Catches up at most
/// [`MAX_CATCHUP`] frames; if more than that is due (a long stall), resyncs the deadline to
/// `now + FRAME_MS` and drops the backlog. Pure arithmetic over wall-clock milliseconds —
/// no DOM dependency — so the pacing logic is unit-testable independent of the browser.
#[cfg(any(target_arch = "wasm32", test))]
fn frames_due(now: f64, next_frame: &mut f64) -> u32 {
    let mut due = 0u32;
    while *next_frame <= now && due < MAX_CATCHUP {
        due += 1;
        *next_frame += FRAME_MS;
    }
    // Fell further behind than the cap absorbs: resync to now + one frame instead of
    // leaving the deadline in the past (which would force a burst on every later tick).
    if *next_frame <= now {
        *next_frame = now + FRAME_MS;
    }
    due
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::{blank, build_vm, compose_both, frames_due, keymap, prepare_vm, Vm, FRAME_MS};
    use js_sys::Function;
    use sb_core::host_input::HostInput;
    use sb_core::VmError;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::{Clamped, JsCast};
    use web_sys::{
        CanvasRenderingContext2d, CompositionEvent, Event, HtmlCanvasElement, HtmlInputElement,
        ImageData, InputEvent, KeyboardEvent, MouseEvent, Performance,
    };

    /// A canvas together with its 2D context — the unit `paint` blits a framebuffer onto.
    type Surface = (HtmlCanvasElement, CanvasRenderingContext2d);

    /// The bottom (touch) screen is 320×240; `TOUCH` reports a coordinate in that space.
    const TOUCH_WIDTH: f64 = 320.0;
    const TOUCH_HEIGHT: f64 = 240.0;

    /// Look up the `<canvas>` with `canvas_id` and its 2D context.
    fn canvas_and_ctx(canvas_id: &str) -> Result<Surface, JsValue> {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| JsValue::from_str("no document"))?;
        let canvas: HtmlCanvasElement = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas element not found"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("element is not a <canvas>"))?;
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("not a 2d context"))?;
        Ok((canvas, ctx))
    }

    /// Blit a framebuffer onto the canvas (sizing the canvas to it on the first paint).
    fn paint(
        canvas: &HtmlCanvasElement,
        ctx: &CanvasRenderingContext2d,
        fb: &super::Framebuffer,
    ) -> Result<(), JsValue> {
        if canvas.width() != fb.width as u32 || canvas.height() != fb.height as u32 {
            canvas.set_width(fb.width as u32);
            canvas.set_height(fb.height as u32);
        }
        // Framebuffer pixels are already RGBA8888 row-major — exactly ImageData's layout.
        let image = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&fb.pixels),
            fb.width as u32,
            fb.height as u32,
        )?;
        ctx.put_image_data(&image, 0.0, 0.0)
    }

    /// Show or hide a canvas by toggling its `hidden` attribute, writing only when the state
    /// actually flips so the per-frame `paint_both` doesn't churn the attribute (and trigger
    /// layout) every tick.
    fn set_visible(canvas: &HtmlCanvasElement, visible: bool) {
        if canvas.hidden() == visible {
            canvas.set_hidden(!visible);
        }
    }

    /// Paint BOTH physical screens for the VM's current `XSCREEN` mode (#81). The `top` canvas
    /// always shows the Upper screen (or, in mode 4, the single combined 320×480 surface); the
    /// `bottom` canvas shows the Touch Screen as an independent graphics screen only in the
    /// modes that expose it (`XSCREEN` 2/3), and is hidden otherwise. This is what makes the
    /// player dual-screen instead of painting only whichever screen `DISPLAY` points at.
    fn paint_both(top: &Surface, bottom: &Surface, vm: &Vm) -> Result<(), JsValue> {
        let (upper, touch) = compose_both(vm);
        paint(&top.0, &top.1, &upper)?;
        match touch {
            Some(fb) => {
                paint(&bottom.0, &bottom.1, &fb)?;
                set_visible(&bottom.0, true);
            }
            None => set_visible(&bottom.0, false),
        }
        Ok(())
    }

    /// Run a SmileBASIC program and paint its final scene onto the two `<canvas>` elements
    /// (one-shot, no input). `top_id` is the Upper screen; `bottom_id` is the Touch Screen,
    /// shown only when the program's `XSCREEN` mode exposes it. The framebuffers are drawn 1:1
    /// (use CSS `image-rendering: pixelated` + a transform to scale them up).
    #[wasm_bindgen]
    pub fn run_program(top_id: &str, bottom_id: &str, src: &str) -> Result<(), JsValue> {
        let top = canvas_and_ctx(top_id)?;
        let bottom = canvas_and_ctx(bottom_id)?;
        match build_vm(src) {
            Ok(vm) => paint_both(&top, &bottom, &vm),
            Err(_) => {
                paint(&top.0, &top.1, &blank())?;
                set_visible(&bottom.0, false);
                Ok(())
            }
        }
    }

    /// Live touch state shared between the canvas mouse listeners and the frame loop: whether
    /// the (left) button is down and the last `(x, y)` in 320×240 touch coordinates.
    #[derive(Default)]
    struct Touch {
        down: bool,
        x: i32,
        y: i32,
    }

    /// Map a mouse event's canvas-relative offset to a 320×240 `TOUCH` coordinate, using the
    /// canvas's displayed (CSS) size so it works regardless of how the canvas is scaled up.
    fn touch_coords(canvas: &HtmlCanvasElement, e: &MouseEvent) -> (i32, i32) {
        let (cw, ch) = (canvas.client_width(), canvas.client_height());
        if cw <= 0 || ch <= 0 {
            return (0, 0);
        }
        let x =
            (f64::from(e.offset_x()) / f64::from(cw) * TOUCH_WIDTH).clamp(0.0, TOUCH_WIDTH - 1.0);
        let y =
            (f64::from(e.offset_y()) / f64::from(ch) * TOUCH_HEIGHT).clamp(0.0, TOUCH_HEIGHT - 1.0);
        (x as i32, y as i32)
    }

    /// Schedule the next `requestAnimationFrame` tick.
    fn request_frame(f: &Closure<dyn FnMut()>) {
        if let Some(w) = web_sys::window() {
            let _ = w.request_animation_frame(f.as_ref().unchecked_ref());
        }
    }

    /// The high-resolution monotonic clock (`performance.now()`) the rAF pacer measures
    /// deadlines against, in milliseconds. Returns `0.0` if the clock is unavailable (the
    /// pacer then runs one VM frame per tick — the pre-fix behavior — rather than stalling).
    fn now_ms(perf: &Performance) -> f64 {
        perf.now()
    }

    /// Shared flag used to stop a running interactive loop from JavaScript.
    static STOP: AtomicBool = AtomicBool::new(false);

    /// Request that the running `run_interactive` loop stop after its current frame.
    /// The next call to `run_interactive` resets this flag automatically.
    #[wasm_bindgen]
    pub fn stop_interactive() {
        STOP.store(true, Ordering::Relaxed);
    }

    /// Format a VM failure for the browser error banner.
    fn format_vm_error(e: &VmError) -> String {
        match e {
            VmError::Sb { errnum, line } => {
                let msg = sb_core::error::error_message(*errnum);
                format!(
                    "Runtime error (errnum {}) at line {}: {}",
                    errnum, line, msg
                )
            }
            VmError::Unsupported(what) => format!("Unsupported operation: {}", what),
            VmError::Assert { message, line } => {
                format!("ASSERT failed at line {}: {}", line, message)
            }
        }
    }

    /// Call a JS error callback with `message`. The callback is a JS function value so the
    /// rAF loop can keep invoking it after `run_interactive` has returned.
    fn report_error(callback: &JsValue, message: &str) -> Result<(), JsValue> {
        let f: &Function = callback.unchecked_ref();
        let args = js_sys::Array::new();
        args.push(&JsValue::from_str(message));
        f.apply(&JsValue::NULL, &args)?;
        Ok(())
    }

    /// Maximum number of bytecode instructions the VM may execute inside one
    /// `requestAnimationFrame` callback before returning to the host. Large enough that
    /// simple programs finish in a single frame, small enough that a runaway loop cannot
    /// freeze the browser tab for a noticeable period.
    const FRAME_BUDGET: usize = 50_000;

    /// Run a SmileBASIC program under live host input in a `requestAnimationFrame` loop.
    /// Unlike the one-shot `run_program`, this uses `Vm::run_frame` so the VM yields at
    /// `VSYNC`/`WAIT` and the host can refresh `BUTTON`/`STICK`/`STICKEX`/`TOUCH` each frame.
    /// `on_error` is called with a human-readable message whenever parse/compile fails up front
    /// or a runtime error halts the program, so the player UI can make failures visible.
    #[wasm_bindgen]
    pub fn run_interactive(
        top_id: &str,
        bottom_id: &str,
        src: &str,
        on_error: Function,
    ) -> Result<(), JsValue> {
        let top = canvas_and_ctx(top_id)?;
        let bottom = canvas_and_ctx(bottom_id)?;
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| JsValue::from_str("no document"))?;

        STOP.store(false, Ordering::Relaxed);

        // Keep the JS callback as a generic value so the rAF closure can invoke it after this
        // function returns.
        let on_error = JsValue::from(on_error);

        let vm = prepare_vm(src);
        // Paint the initial scene once up front. On a compile failure show a blank Upper screen,
        // hide the Touch canvas, and report the error to JS.
        match &vm {
            Ok(vm) => paint_both(&top, &bottom, vm)?,
            Err(msg) => {
                paint(&top.0, &top.1, &blank())?;
                set_visible(&bottom.0, false);
                report_error(&on_error, msg)?;
                return Ok(());
            }
        }
        let vm = Rc::new(RefCell::new(vm.unwrap()));

        let input = Rc::new(RefCell::new(HostInput::new()));
        let touch = Rc::new(RefCell::new(Touch::default()));
        let awaiting: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let input_overlay: Rc<RefCell<Option<HtmlInputElement>>> = Rc::new(RefCell::new(
            document
                .get_element_by_id("sb-input-line")
                .and_then(|e| e.dyn_into().ok()),
        ));

        // Keyboard → button / stick masks. keydown sets, keyup clears.
        // While the VM is awaiting interactive text input, swallow game-bound keys and
        // use Enter to submit the line.
        {
            let input = input.clone();
            let vm = vm.clone();
            let overlay = input_overlay.clone();
            let awaiting = awaiting.clone();
            let cb = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                if awaiting.get() {
                    let code = e.code();
                    if keymap::bind(&code).is_some() {
                        e.prevent_default();
                        return;
                    }
                    if code == "Enter" {
                        e.prevent_default();
                        vm.borrow_mut().input_enter();
                        if let Some(el) = overlay.borrow().as_ref() {
                            el.set_value("");
                            let _ = el.blur();
                        }
                        awaiting.set(false);
                        return;
                    }
                }
                input.borrow_mut().apply(keymap::bind(&e.code()), true);
            });
            document.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }
        {
            let input = input.clone();
            let cb = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                input.borrow_mut().apply(keymap::bind(&e.code()), false);
            });
            document.add_event_listener_with_callback("keyup", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }

        // Text input overlay: the browser handles typing/IME/cursor; we mirror the value
        // into the VM's input buffer.
        if let Some(el) = input_overlay.borrow().as_ref() {
            // `beforeinput` lets us intercept simple typing and backspace before the DOM changes.
            let vm2 = vm.clone();
            let overlay2 = input_overlay.clone();
            let awaiting2 = awaiting.clone();
            let cb = Closure::<dyn FnMut(InputEvent)>::new(move |e: InputEvent| {
                if !awaiting2.get() {
                    return;
                }
                let composing = e.is_composing();
                match e.input_type().as_str() {
                    "insertText" | "insertFromPaste" if !composing => {
                        if let Some(data) = e.data() {
                            if !data.is_empty() {
                                e.prevent_default();
                                {
                                    let mut vm = vm2.borrow_mut();
                                    for ch in data.chars() {
                                        vm.input_char(ch as u32);
                                    }
                                }
                                if let Some(el) = overlay2.borrow().as_ref() {
                                    el.set_value(&vm2.borrow().input_current_line());
                                }
                            }
                        }
                    }
                    "deleteContentBackward" if !composing => {
                        e.prevent_default();
                        vm2.borrow_mut().input_backspace();
                        if let Some(el) = overlay2.borrow().as_ref() {
                            el.set_value(&vm2.borrow().input_current_line());
                        }
                    }
                    _ => {}
                }
            });
            el.add_event_listener_with_callback("beforeinput", cb.as_ref().unchecked_ref())?;
            cb.forget();

            // `input` and `compositionend` catch IME, paste, and any edits we didn't prevent.
            let vm3 = vm.clone();
            let awaiting3 = awaiting.clone();
            let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                if !awaiting3.get() {
                    return;
                }
                if let Some(target) = e.target() {
                    if let Ok(el) = target.dyn_into::<HtmlInputElement>() {
                        vm3.borrow_mut().input_set_current(&el.value());
                    }
                }
            });
            el.add_event_listener_with_callback("input", cb.as_ref().unchecked_ref())?;
            cb.forget();

            let vm4 = vm.clone();
            let awaiting4 = awaiting.clone();
            let cb = Closure::<dyn FnMut(CompositionEvent)>::new(move |e: CompositionEvent| {
                if !awaiting4.get() {
                    return;
                }
                if let Some(target) = e.target() {
                    if let Ok(el) = target.dyn_into::<HtmlInputElement>() {
                        vm4.borrow_mut().input_set_current(&el.value());
                    }
                }
            });
            el.add_event_listener_with_callback("compositionend", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }

        // Mouse → TOUCH. The Touch Screen is the bottom canvas, but we wire both so a tap reads
        // through in single-screen modes too (where only the top canvas is shown); each canvas
        // maps its own client rect into the shared 320×240 touch space.
        for canvas in [&top.0, &bottom.0] {
            let touch = touch.clone();
            let canvas2 = canvas.clone();
            let cb = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
                let (x, y) = touch_coords(&canvas2, &e);
                let mut t = touch.borrow_mut();
                t.x = x;
                t.y = y;
                t.down = (e.buttons() & 1) != 0;
            });
            for event in ["mousedown", "mousemove", "mouseup"] {
                canvas.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref())?;
            }
            cb.forget();
        }

        // The rAF loop: owns the VM + both surfaces, reads the shared input/touch each frame,
        // runs VM frames paced to wall-clock 60 fps, then paints both screens. `rAF` fires at
        // the *display* refresh rate (60/120/144 Hz), not 60 Hz fixed, so a naive one-frame-per-
        // tick loop runs the program too fast on high-refresh displays. The pacer accumulates
        // whole 1/60 s frames since the last deadline and runs that many VM frames per tick,
        // mirroring the native host's `WaitUntil(deadline)` loop (#92).
        let halted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = f.clone();
        let on_error_for_frame = on_error.clone();
        // The monotonic clock the pacer measures against. `performance.now()` is the
        // high-resolution counterpart of the native `Instant`. `window.performance()` is
        // effectively always present in browsers; if it's missing we fall back to running one
        // VM frame per rAF tick (the pre-fix behavior) rather than stalling.
        let perf = web_sys::window().and_then(|w| w.performance());
        // Origin of the wall-clock 60Hz frame counter. The VM's `MAINCNT` must track a free-
        // running 60Hz source, not the number of VM execution slices, so we sync the frame
        // clock to `floor((performance.now() - start_ms) / FRAME_MS)` each rAF tick.
        let start_ms = perf.as_ref().map_or(0.0, now_ms);
        let last_vblank: Rc<Cell<u64>> = Rc::new(Cell::new(0));
        // Wall-clock deadline (ms) of the next VM frame. Seeded to "now" so the first tick
        // runs exactly one frame (the deadline is at `now`, so `frames_due` counts it due
        // immediately and advances it by `FRAME_MS`), matching the native host's
        // `next_frame = Some(Instant::now() + FRAME_DURATION)` start.
        let next_frame: Rc<RefCell<f64>> = Rc::new(RefCell::new(perf.as_ref().map_or(0.0, now_ms)));
        let overlay_for_frame = input_overlay.clone();
        *g.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
            {
                let inp = input.borrow();
                let t = touch.borrow();
                let mut vm = vm.borrow_mut();

                // Sync the text input overlay with the VM's INPUT/LINPUT wait state.
                let now_waiting = vm.awaiting_input();
                let was_waiting = awaiting.get();
                if now_waiting != was_waiting {
                    if let Some(el) = overlay_for_frame.borrow().as_ref() {
                        if now_waiting {
                            el.set_hidden(false);
                            el.set_value(&vm.input_current_line());
                            let _ = el.focus();
                        } else {
                            el.set_hidden(true);
                            el.set_value("");
                            let _ = el.blur();
                        }
                    }
                    awaiting.set(now_waiting);
                } else if now_waiting {
                    // Keep focus while waiting so keystrokes go to the overlay.
                    if let Some(el) = overlay_for_frame.borrow().as_ref() {
                        let _ = el.focus();
                    }
                }
                // Pace VM frames to wall-clock 60 fps: run however many 1/60 s frames have
                // elapsed since the last deadline (capped, so a backgrounded tab doesn't fire
                // a catch-up burst). Input/touch advance once per VM frame so BUTTON/STICK/TOUCH
                // sample at 60 Hz regardless of the display rate. Without a clock, run one
                // frame per tick (degenerate, but never freezes).
                let frames = match &perf {
                    Some(p) => frames_due(now_ms(p), &mut next_frame.borrow_mut()),
                    None => 1,
                };
                // Free-running MAINCNT: sync the VM's frame clock to the wall-clock 60Hz
                // counter once per rAF tick, independent of how many VM execution slices we
                // are about to run. When the host has no monotonic clock we fall back to
                // ticking once per VM frame (the pre-fix coupling).
                if let Some(p) = &perf {
                    let target = ((now_ms(p) - start_ms) / FRAME_MS) as u64;
                    let last = last_vblank.get();
                    if target > last {
                        vm.tick_frames(target - last);
                        last_vblank.set(target);
                    }
                }
                for _ in 0..frames {
                    if perf.is_none() {
                        // VBlank heartbeat first: MAINCNT advances, animations step, and any
                        // pending VSYNC/WAIT target is resolved.  This mirrors the hardware
                        // model where `swi 0xa` fires before the program resumes — and ensures
                        // MAINCNT advances even in programs that never call VSYNC/WAIT (#94).
                        vm.tick_frame();
                    }

                    // `input_mut` borrows `vm` mutably; take it per-iteration so the borrow
                    // ends before `run_frame` borrows `vm` again below.
                    let device = vm.input_mut();
                    device.advance_frame(inp.held(), inp.stick(), inp.stickex());
                    device.advance_touch(t.down, t.x, t.y);

                    if !*halted.borrow() && !awaiting.get() {
                        match vm.run_frame(FRAME_BUDGET) {
                            Ok(Some(_)) => *halted.borrow_mut() = true,
                            Ok(None) => {}
                            Err(e) => {
                                *halted.borrow_mut() = true;
                                let _ = report_error(&on_error_for_frame, &format_vm_error(&e));
                                break;
                            }
                        }
                    }
                    // (post-halt: the wall-clock sync above keeps the heartbeat alive for
                    // sprite/BG animations and MAINCNT advancement even after END.)
                }

                let _ = paint_both(&top, &bottom, &vm);
            }
            if !STOP.load(Ordering::Relaxed) {
                request_frame(f.borrow().as_ref().unwrap());
            }
        }));
        request_frame(g.borrow().as_ref().unwrap());
        Ok(())
    }

    // ----- Audio backend (M5-T5): WebAudio output -------------------------------------------

    use sb_audio::mml;
    use sb_audio::synth::{Pcm, Synth};
    use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext};

    /// A WebAudio output sink: an [`AudioContext`] kept alive for the page, into which synth
    /// PCM is scheduled. The browser counterpart of the native cpal backend.
    ///
    /// The synth renders interleaved stereo PCM16 at
    /// [`SAMPLE_RATE`](sb_audio::synth::SAMPLE_RATE); we hand WebAudio an `AudioBuffer` *at that
    /// rate* and let the browser resample to the device — the WebAudio graph does the rate
    /// conversion the native backend does with `StereoResampler`.
    pub struct WebAudio {
        ctx: AudioContext,
    }

    impl WebAudio {
        /// Create the audio context. (It may start suspended until a user gesture; calling
        /// from a click handler — as `play_mml` is meant to be — resumes it.)
        pub fn new() -> Result<Self, JsValue> {
            Ok(WebAudio {
                ctx: AudioContext::new()?,
            })
        }

        /// Schedule a finite PCM clip to play immediately. De-interleaves to per-channel f32
        /// (WebAudio's planar `AudioBuffer` layout) and resamples via the WebAudio graph.
        pub fn play_pcm(&self, pcm: &Pcm) -> Result<(), JsValue> {
            let frames = pcm.frames();
            if frames == 0 {
                return Ok(());
            }
            let buffer: AudioBuffer =
                self.ctx
                    .create_buffer(2, frames as u32, pcm.sample_rate as f32)?;
            let mut left = vec![0.0f32; frames];
            let mut right = vec![0.0f32; frames];
            for (f, frame) in pcm.samples.chunks_exact(2).enumerate() {
                left[f] = frame[0] as f32 / 32768.0;
                right[f] = frame[1] as f32 / 32768.0;
            }
            buffer.copy_to_channel(&left, 0)?;
            buffer.copy_to_channel(&right, 1)?;

            let source: AudioBufferSourceNode = self.ctx.create_buffer_source()?;
            source.set_buffer(Some(&buffer));
            source.connect_with_audio_node(&self.ctx.destination())?;
            source.start()?;
            Ok(())
        }
    }

    /// Render an MML string with the synth and play it through WebAudio (M5-T5). Call from a
    /// user-gesture handler (e.g. a button click) so the browser lets audio start. `frames`,
    /// when > 0, renders a fixed number of 1/60 s frames (expanding endless loops); 0 renders
    /// the tune once.
    #[wasm_bindgen]
    pub fn play_mml(src: &str, frames: u32) -> Result<(), JsValue> {
        let song = mml::parse(src).map_err(|e| {
            JsValue::from_str(&format!("MML error (errnum {}): {}", e.errnum, e.message))
        })?;
        let synth = Synth::new();
        let pcm = if frames > 0 {
            synth.render_frames(&song, frames)
        } else {
            synth.render(&song)
        };
        WebAudio::new()?.play_pcm(&pcm)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_vm, compose_both, compose_for_screen, frames_due, keymap, render_program, FRAME_MS,
        MAX_CATCHUP,
    };
    use sb_core::host_input::{Bind, Stick};
    use sb_core::input::{BTN_A, BTN_RIGHT, BTN_ZR};

    /// The blue channel of a framebuffer pixel (ARGB8888 → `B`). Chosen as the per-screen
    /// discriminator in the dual-screen tests because RGB(0,0,255) survives the device's 5-bit
    /// blue truncation as a clearly-nonzero byte (0xF8), while a red/green fill leaves it 0.
    fn blue(fb: &super::Framebuffer, x: usize, y: usize) -> u8 {
        (fb.get_argb(x, y) & 0xFF) as u8
    }

    /// The green channel of a framebuffer pixel (ARGB8888 → `G`).
    fn green(fb: &super::Framebuffer, x: usize, y: usize) -> u8 {
        ((fb.get_argb(x, y) >> 8) & 0xFF) as u8
    }

    #[test]
    fn renders_printed_text_to_pixels() {
        let fb = render_program(r#"?"HI""#);
        assert_eq!(fb.width, sb_render::TOP_WIDTH);
        let lit = fb
            .pixels
            .chunks_exact(4)
            .any(|p| p[0] != 0 || p[1] != 0 || p[2] != 0);
        assert!(lit, "expected some lit text pixels on the black backdrop");
    }

    // ---- rAF frame pacing (#92) -------------------------------------------------------
    //
    // The pacer must turn display-refresh ticks (60/120/144 Hz) into a steady 60 VM-fps.
    // These cover the pure arithmetic in `frames_due` — the part of the fix that's testable
    // on the desktop without a browser — mirroring the native host's WaitUntil pacer.

    #[test]
    fn frames_due_runs_one_frame_when_less_than_a_frame_elapsed() {
        // Deadline at 0; 5 ms later (< 16.67 ms) → exactly one frame due (the deadline is
        // reached), and the deadline advances one frame. This is the steady-state 60 Hz case
        // and the first-tick case (deadline seeded to "now").
        let mut next = 0.0;
        assert_eq!(frames_due(5.0, &mut next), 1);
        assert_eq!(next, FRAME_MS);
    }

    #[test]
    fn frames_due_runs_multiple_frames_when_a_high_refresh_tick_overruns() {
        // On a 120 Hz display each rAF tick is ~8.33 ms, but the VM frame is 16.67 ms, so most
        // ticks owe zero frames and every other tick owes one. On a 144 Hz display (~6.94 ms
        // ticks) the same pattern holds. Model steady state (deadline one frame out, as it is
        // after the first frame fires) and land two 120 Hz ticks at 8 and 17 ms: the first
        // owes 0, the second owes 1 — the program never runs faster than 60 fps.
        let mut next = FRAME_MS; // deadline one frame in the future (steady state)
        assert_eq!(
            frames_due(8.0, &mut next),
            0,
            "first 120 Hz tick: no frame due yet"
        );
        assert_eq!(frames_due(17.0, &mut next), 1, "second tick: one frame due");
        assert_eq!(next, FRAME_MS * 2.0);
    }

    #[test]
    fn frames_due_catches_up_a_short_stall_within_the_cap() {
        // A 2-frame stall (deadline 0, now at ~2 frames): catch up both, capped at
        // MAX_CATCHUP. The deadline advances past `now` so the next tick owes nothing.
        let mut next = 0.0;
        let now = FRAME_MS * 2.0 - 1.0; // just under 2 frames
        let due = frames_due(now, &mut next);
        assert_eq!(due, 2);
        assert!(next > now, "deadline should advance past now");
    }

    #[test]
    fn frames_due_resyncs_after_a_long_stall_instead_of_bursting() {
        // A long stall (tab backgrounded) leaves the deadline far in the past. The pacer must
        // cap the catch-up at MAX_CATCHUP and resync the deadline to now + one frame, so it
        // doesn't fire a frame burst every tick afterwards. This is the web analog of the
        // native host's "if we fell far behind, resync to now" rule.
        let mut next = 0.0;
        let now = FRAME_MS * 100.0; // ~1.67 s stall
        let due = frames_due(now, &mut next);
        assert_eq!(due, MAX_CATCHUP, "long stall is capped, not burst");
        assert!(
            next > now && next <= now + FRAME_MS,
            "deadline resyncs to now + one frame, was {next}"
        );
    }

    #[test]
    fn keymap_matches_the_documented_layout() {
        // Same logical layout as the native host, keyed on DOM physical codes.
        assert_eq!(keymap::bind("ArrowRight"), Some(Bind::Button(BTN_RIGHT)));
        assert_eq!(keymap::bind("KeyK"), Some(Bind::Button(BTN_A)));
        assert_eq!(keymap::bind("Digit2"), Some(Bind::Button(BTN_ZR)));
        assert_eq!(keymap::bind("KeyW"), Some(Bind::AxisY(Stick::Left, 1.0)));
        assert_eq!(
            keymap::bind("Numpad6"),
            Some(Bind::AxisX(Stick::Right, 1.0))
        );
        assert_eq!(keymap::bind("Escape"), None);
    }

    #[test]
    fn xscreen_top_screen_keeps_default_size() {
        let fb = render_program("XSCREEN 2:DISPLAY 0");
        assert_eq!(fb.width, sb_render::TOP_WIDTH);
        assert_eq!(fb.height, sb_render::TOP_HEIGHT);
    }

    #[test]
    fn xscreen_bottom_screen_changes_framebuffer_size() {
        let fb = render_program("XSCREEN 2:DISPLAY 1");
        assert_eq!(fb.width, sb_render::BOTTOM_WIDTH);
        assert_eq!(fb.height, sb_render::BOTTOM_HEIGHT);
    }

    #[test]
    fn xscreen_combined_mode_changes_framebuffer_size() {
        let fb = render_program("XSCREEN 4");
        assert_eq!(fb.width, sb_render::BOTTOM_WIDTH);
        assert_eq!(fb.height, sb_render::TOP_HEIGHT + sb_render::BOTTOM_HEIGHT);
    }

    // ---- per-screen compositing (#85) -------------------------------------------------

    #[test]
    fn compose_for_screen_renders_each_screens_own_grp_page() {
        // Under XSCREEN 2 the two physical screens are independent graphics screens. Point each
        // at its OWN GRP page and clear them to distinct colors; compose_for_screen must render
        // the page belonging to the screen_id asked for, so the two framebuffers differ.
        //   screen 0 (Upper) → page 0, red   (blue channel 0)
        //   screen 1 (Touch) → page 2, blue  (blue channel 0xF8)
        let vm = build_vm(
            "XSCREEN 2\nGPAGE 0,0\nGCLS RGB(255,0,0)\nDISPLAY 1\nGPAGE 2,2\nGCLS RGB(0,0,255)",
        )
        .expect("program builds");
        let upper = compose_for_screen(&vm, 0);
        let touch = compose_for_screen(&vm, 1);
        // The Touch screen is 320×240; the Upper screen keeps its 3D-mode 400×240.
        assert_eq!((touch.width, touch.height), (320, 240));
        assert_eq!((upper.width, upper.height), (400, 240));
        // Each renders its own page: Upper has no blue, Touch is saturated blue.
        assert_eq!(blue(&upper, 0, 0), 0, "upper screen shows its red page");
        assert_eq!(blue(&touch, 0, 0), 0xF8, "touch screen shows its blue page");
        // Distinct content per screen → different pixels.
        assert_ne!(upper.get_argb(0, 0), touch.get_argb(0, 0));
    }

    #[test]
    fn compose_both_exposes_touch_only_when_the_mode_does() {
        // Modes 2/3 expose the Touch Screen as an independent second graphics screen → Some;
        // modes 0/1 (Upper only) and mode 4 (one combined surface) → None.
        // Mode 2 is a 3D upper (400×240); mode 3 is a 2D upper (320×240). Both expose Touch.
        for (src, upper_size) in [("XSCREEN 2", (400, 240)), ("XSCREEN 3", (320, 240))] {
            let vm = build_vm(src).expect("program builds");
            let (upper, touch) = compose_both(&vm);
            assert_eq!((upper.width, upper.height), upper_size, "{src} upper size");
            let touch = touch.unwrap_or_else(|| panic!("{src} must expose a touch fb"));
            assert_eq!((touch.width, touch.height), (320, 240), "{src} touch size");
        }
        for src in ["XSCREEN 0", "XSCREEN 1", "XSCREEN 4"] {
            let vm = build_vm(src).expect("program builds");
            let (_, touch) = compose_both(&vm);
            assert!(touch.is_none(), "{src} must not expose a second screen");
        }
    }

    #[test]
    fn sprite_under_display_1_appears_only_on_the_touch_screen() {
        // A sprite created while DISPLAY 1 is selected lives in the Touch screen's per-screen
        // sprite table. It must composite into compose_for_screen(vm,1) but NOT (vm,0), proving
        // end-to-end per-screen sprite compositing. The 8×8 green block is painted into the
        // shared sprite sheet (GRP4) first, while DISPLAY 0 is still selected.
        let vm = build_vm(
            "XSCREEN 2\n\
             GPAGE 4,4\nGFILL 0,0,7,7,RGB(0,255,0)\n\
             DISPLAY 1\nSPSET 0,0,0,8,8,1\nSPOFS 0,100,100",
        )
        .expect("program builds");
        let upper = compose_for_screen(&vm, 0);
        let touch = compose_for_screen(&vm, 1);
        // The sprite's home (0,0) lands its top-left texel at (SPOFS x, y) = (100,100).
        assert_eq!(
            green(&touch, 100, 100),
            0xF8,
            "sprite shows on the touch screen"
        );
        assert_eq!(
            green(&touch, 103, 103),
            0xF8,
            "sprite body on the touch screen"
        );
        // The Upper screen's sprite table is empty here → no green there.
        assert_eq!(
            green(&upper, 100, 100),
            0,
            "sprite absent from the upper screen"
        );
    }

    #[test]
    fn bg_under_display_1_appears_only_on_the_touch_screen() {
        // Same proof for BG: a BG cell placed under DISPLAY 1 lives in the Touch screen's
        // per-screen BG table and composites only into compose_for_screen(vm,1). Char 1 is
        // painted blue into the shared BG sheet (GRP5) under DISPLAY 0 first; the default 16×16
        // tile means cell (0,0) covers screen pixels (0..15, 0..15).
        let vm = build_vm(
            "XSCREEN 2\n\
             GPAGE 5,5\nGFILL 16,0,31,15,RGB(0,0,255)\n\
             DISPLAY 1\nBGPUT 0,0,0,1",
        )
        .expect("program builds");
        let upper = compose_for_screen(&vm, 0);
        let touch = compose_for_screen(&vm, 1);
        assert_eq!(
            blue(&touch, 0, 0),
            0xF8,
            "BG tile shows on the touch screen"
        );
        assert_eq!(
            blue(&touch, 15, 15),
            0xF8,
            "BG tile body on the touch screen"
        );
        assert_eq!(
            blue(&upper, 0, 0),
            0,
            "BG tile absent from the upper screen"
        );
    }

    #[test]
    fn xscreen_combined_mode_shows_bottom_half_of_page() {
        // XSCREEN 4 must expose the full 320×480 combined area, not just the upper 240 rows.
        let fb = render_program("XSCREEN 4:GPSET 0,240,RGB(255,0,0)");
        assert_eq!(fb.width, sb_render::BOTTOM_WIDTH);
        assert_eq!(fb.height, sb_render::TOP_HEIGHT + sb_render::BOTTOM_HEIGHT);
        // (0, 240) is the top-left pixel of the bottom half and should be red.
        assert_eq!(fb.get_argb(0, 240), 0xFFF8_0000);
    }

    #[test]
    fn bg_renders_across_full_combined_area_in_xscreen4() {
        // XSCREEN 4 is a single 320×480 surface. BG layers must tile across both the top
        // half (y=0..239) and the bottom half (y=240..479). The default 25×15 map with 16×16
        // tiles covers 400×240 px and wraps/tiles — so a tile at cell (0,0) should appear at
        // y=0..15 AND at y=240..255 (the second tiling cycle).
        //
        // Regression for #93: the compositor was routing BG to screen 1 (Touch) in mode 4
        // while compose_for_screen used screen 0, leaving the combined surface with no BG.
        let vm = build_vm(
            "XSCREEN 4\n\
             GPAGE 5,5\nGFILL 16,0,31,15,RGB(0,0,255)\n\
             BGPUT 0,0,0,1",
        )
        .expect("program builds");
        let fb = compose_for_screen(&vm, 0);
        assert_eq!(fb.width, 320);
        assert_eq!(fb.height, 480);
        // Top half: cell (0,0) covers screen y=0..15 (16-px tile, no scroll).
        assert_ne!(blue(&fb, 0, 0), 0, "BG tile must appear in top half");
        // Bottom half: map height=240px → y=240 tiles back to map-y=0 → same cell (0,0).
        assert_ne!(blue(&fb, 0, 240), 0, "BG tile must tile into bottom half");
    }
}
