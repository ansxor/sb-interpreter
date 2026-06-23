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
use sb_render::compositor::{compose_top_screen, DEFAULT_BACKDROP};
use sb_render::Framebuffer;

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

/// Parse → compile → run `src` to completion, returning the live VM (or `None` if it failed
/// to parse/compile). The halt result is ignored — a halted program's partial scene is still
/// worth showing / animating.
fn build_vm(src: &str) -> Option<Vm> {
    let ast = parse(src).ok()?;
    let program = compile_with(&ast, &StdBuiltins).ok()?;
    let mut vm = Vm::new(program);
    let _ = vm.run();
    Some(vm)
}

/// Composite the VM's current scene into a top-screen framebuffer (backdrop → GRP → BG →
/// sprites → console, the shared M2/M3 compositor).
fn compose(vm: &Vm) -> Framebuffer {
    compose_top_screen(
        vm.grp(),
        vm.bg(),
        vm.sprites(),
        vm.console(),
        DEFAULT_BACKDROP,
        vm.screen_visibility(),
    )
}

/// Run `src` to completion and return the final scene composited into a top-screen
/// framebuffer (RGBA8888). On a SmileBASIC error the partial scene is composited anyway — the
/// canvas shows whatever the program drew before it halted. Backdrop is opaque black so the
/// (transparent-by-default) GRP/console pixels blit to a visible surface.
pub fn render_program(src: &str) -> Framebuffer {
    match build_vm(src) {
        Some(vm) => compose(&vm),
        None => blank(),
    }
}

/// An opaque-black top-screen framebuffer (used when the program fails to parse/compile).
fn blank() -> Framebuffer {
    let mut fb = Framebuffer::top();
    fb.clear(DEFAULT_BACKDROP);
    fb
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::{blank, build_vm, compose, keymap};
    use sb_core::host_input::HostInput;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::{Clamped, JsCast};
    use web_sys::{
        CanvasRenderingContext2d, HtmlCanvasElement, ImageData, KeyboardEvent, MouseEvent,
    };

    /// The bottom (touch) screen is 320×240; `TOUCH` reports a coordinate in that space.
    const TOUCH_WIDTH: f64 = 320.0;
    const TOUCH_HEIGHT: f64 = 240.0;

    /// Look up the `<canvas>` with `canvas_id` and its 2D context.
    fn canvas_and_ctx(
        canvas_id: &str,
    ) -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), JsValue> {
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

    /// Run a SmileBASIC program and paint its console output onto the `<canvas>` with the
    /// given element id (one-shot, no input). The framebuffer is drawn 1:1 (use CSS
    /// `image-rendering: pixelated` + a transform to scale it up).
    #[wasm_bindgen]
    pub fn run_program(canvas_id: &str, src: &str) -> Result<(), JsValue> {
        let (canvas, ctx) = canvas_and_ctx(canvas_id)?;
        let fb = super::render_program(src);
        paint(&canvas, &ctx, &fb)
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

    /// Run a SmileBASIC program, then drive it with live host input in a
    /// `requestAnimationFrame` loop: each frame folds the accumulated keyboard / mouse input
    /// into the VM's `InputState` (so `BUTTON`/`STICK`/`STICKEX`/`TOUCH` read live input),
    /// ticks the frame clock, and re-paints the canvas (M4-T5). Mirrors the native host loop.
    #[wasm_bindgen]
    pub fn run_interactive(canvas_id: &str, src: &str) -> Result<(), JsValue> {
        let (canvas, ctx) = canvas_and_ctx(canvas_id)?;
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| JsValue::from_str("no document"))?;

        let vm = build_vm(src);
        // Paint the initial scene (or a blank backdrop on a compile failure) once up front.
        paint(&canvas, &ctx, &vm.as_ref().map_or_else(blank, compose))?;
        let Some(vm) = vm else {
            return Ok(()); // nothing to animate / drive
        };

        let input = Rc::new(RefCell::new(HostInput::new()));
        let touch = Rc::new(RefCell::new(Touch::default()));

        // Keyboard → button / stick masks. keydown sets, keyup clears.
        for (event, pressed) in [("keydown", true), ("keyup", false)] {
            let input = input.clone();
            let cb = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                input.borrow_mut().apply(keymap::bind(&e.code()), pressed);
            });
            document.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref())?;
            cb.forget(); // listener lives for the page's lifetime
        }

        // Mouse → TOUCH on the canvas.
        {
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

        // The rAF loop: owns the VM + context, reads the shared input/touch each frame.
        let vm = Rc::new(RefCell::new(vm));
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = f.clone();
        *g.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
            {
                let inp = input.borrow();
                let t = touch.borrow();
                let mut vm = vm.borrow_mut();
                let device = vm.input_mut();
                device.advance_frame(inp.held(), inp.stick(), inp.stickex());
                device.advance_touch(t.down, t.x, t.y);
                vm.tick_frame();
                let _ = paint(&canvas, &ctx, &compose(&vm));
            }
            request_frame(f.borrow().as_ref().unwrap());
        }));
        request_frame(g.borrow().as_ref().unwrap());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{keymap, render_program};
    use sb_core::host_input::{Bind, Stick};
    use sb_core::input::{BTN_A, BTN_RIGHT, BTN_ZR};

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
}
