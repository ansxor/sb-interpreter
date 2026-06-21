//! `sb-platform-wasm` — the browser runner (milestone M1+).
//!
//! Exposes the interpreter to JS via `wasm-bindgen`, renders the
//! [`sb_render::Framebuffer`] to a `<canvas>`, and maps keyboard/gamepad to SB input.
//! WASM is a first-class target (the SmileBASIC community is web-based).

/// Smoke-test export so the crate has linkable content for `wasm32` builds.
pub fn screen_dimensions() -> (usize, usize) {
    (sb_render::TOP_WIDTH, sb_render::TOP_HEIGHT)
}

// TODO(M1): #[wasm_bindgen] interpreter handle, canvas blit, input, IndexedDB storage.
