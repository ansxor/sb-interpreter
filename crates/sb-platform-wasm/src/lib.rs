//! `sb-platform-wasm` — the browser host for the SmileBASIC interpreter (M1-T12).
//!
//! A `wasm-bindgen` module that runs a program through the `sb-core` pipeline (the same
//! `parse → compile → VM` as the headless `sb-run` and the desktop `sb` window), renders the
//! resulting text console into an [`sb_render::Framebuffer`], and blits that RGBA8888 buffer
//! straight onto an HTML `<canvas>` via `CanvasRenderingContext2d::put_image_data`.
//!
//! The framebuffer model is shared with the native host; only the final blit differs
//! (canvas `ImageData` here, a softbuffer window there). The browser bindings are gated to
//! `wasm32`, so this crate compiles to a thin rlib (just [`render_program`]) on the desktop
//! and the workspace's native `cargo build`/`cargo test` never pull in `web-sys`.

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::{parse, Vm};
use sb_render::Framebuffer;

/// Run `src` to completion and return the final console rendered into a top-screen
/// framebuffer (RGBA8888). On a SmileBASIC error the partial console is rendered anyway —
/// the canvas shows whatever the program drew before it halted. Backdrop is opaque black so
/// the (transparent-by-default) console background blits to a visible surface.
pub fn render_program(src: &str) -> Framebuffer {
    let mut fb = Framebuffer::top();
    fb.clear(0xFF00_0000);

    let Ok(ast) = parse(src) else {
        return fb;
    };
    let Ok(program) = compile_with(&ast, &StdBuiltins) else {
        return fb;
    };
    let mut vm = Vm::new(program);
    // Ignore the halt result: a halted program's partial console is still worth showing.
    let _ = vm.run();
    vm.console().render(&mut fb);
    fb
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::render_program;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::Clamped;
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

    /// Run a SmileBASIC program and paint its console output onto the `<canvas>` with the
    /// given element id. The canvas is sized to the 400×240 top screen and the framebuffer
    /// is drawn 1:1 (use CSS `image-rendering: pixelated` + a transform to scale it up).
    #[wasm_bindgen]
    pub fn run_program(canvas_id: &str, src: &str) -> Result<(), JsValue> {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| JsValue::from_str("no document"))?;
        let canvas: HtmlCanvasElement = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas element not found"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("element is not a <canvas>"))?;

        let fb = render_program(src);
        canvas.set_width(fb.width as u32);
        canvas.set_height(fb.height as u32);

        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("not a 2d context"))?;

        // Framebuffer pixels are already RGBA8888 row-major — exactly ImageData's layout.
        let image = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&fb.pixels),
            fb.width as u32,
            fb.height as u32,
        )?;
        ctx.put_image_data(&image, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::render_program;

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
}
