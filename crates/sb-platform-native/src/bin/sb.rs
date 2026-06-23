//! `sb` — the windowed SmileBASIC host (M1-T12).
//!
//! Loads a `.sb3` file (plain UTF-8 SmileBASIC source), runs it through the `sb-core`
//! pipeline exactly like the headless `sb-run`, then renders the resulting text console
//! into an [`sb_render::Framebuffer`] and blits that buffer to a real OS window
//! (winit + softbuffer). It is the visual counterpart of `sb-run`: same VM, same
//! framebuffer, one drawn to a window instead of dumped as text.
//!
//! M1 has no frame-yielding execution model yet (VSYNC/WAIT scheduling is M4), so the VM
//! runs to completion up front and the window then displays the final console state,
//! redrawing it on resize. The per-frame blit loop lands once the VM yields per frame.
//!
//! winit + softbuffer are desktop-only; this whole file compiles to an empty `main` on
//! `wasm32` (the canvas host is the separate `sb-platform-wasm` crate) so the workspace's
//! `--target wasm32-unknown-unknown` build still walks this crate cleanly.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    native::main()
}

// On wasm32 there is no window; the canvas host lives in `sb-platform-wasm`.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::num::NonZeroU32;
    use std::process::ExitCode;
    use std::rc::Rc;

    use sb_core::builtins::StdBuiltins;
    use sb_core::compiler::compile_with;
    use sb_core::{parse, Vm, VmError};
    use sb_render::compositor::{compose_top_screen, DEFAULT_BACKDROP};
    use sb_render::Framebuffer;

    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::{Window, WindowId};

    /// Default integer zoom — the 400×240 top screen is tiny on a desktop, so show it at 2×.
    const SCALE: f64 = 2.0;

    /// A SmileBASIC error reduced to its `ERRNUM` + 1-based `ERRLINE` (as in `sb-run`).
    struct SbError {
        errnum: u32,
        line: u32,
    }

    /// Run `src` to completion and render its final console state into a top-screen
    /// framebuffer. The buffer is returned even on error so the window still shows whatever
    /// the program printed before it halted; the error (if any) is reported alongside.
    fn run_to_framebuffer(src: &str) -> (Framebuffer, Option<SbError>) {
        let mut fb = Framebuffer::top();
        // Opaque-black backdrop: GRP/console pixels are transparent by default, so a cleared
        // buffer would leave un-blittable garbage — paint the backdrop first. On a parse/
        // compile failure (before any scene exists) this backdrop is what the window shows.
        fb.clear(DEFAULT_BACKDROP);

        let err = run_console(src, &mut fb);
        (fb, err)
    }

    /// Parse → compile → run, then composite the scene (backdrop → GRP display page →
    /// console, M2-T4) into `fb`. Returns the SmileBASIC error if any stage raised one (the
    /// partial scene is still composited).
    fn run_console(src: &str, fb: &mut Framebuffer) -> Option<SbError> {
        let ast = match parse(src) {
            Ok(ast) => ast,
            Err(e) => {
                return Some(SbError {
                    errnum: e.errnum,
                    line: e.loc.line,
                })
            }
        };
        let program = match compile_with(&ast, &StdBuiltins) {
            Ok(p) => p,
            Err(e) => {
                return Some(SbError {
                    errnum: e.errnum,
                    line: e.loc.line,
                })
            }
        };
        let mut vm = Vm::new(program);
        let result = vm.run();
        // Composite whatever the program drew, halted or not.
        *fb = compose_top_screen(vm.grp(), vm.console(), DEFAULT_BACKDROP);
        match result {
            Ok(_) => None,
            Err(VmError::Sb { errnum, line }) => Some(SbError { errnum, line }),
            Err(VmError::Unsupported(what)) => {
                eprintln!("sb: unsupported: {what}");
                Some(SbError { errnum: 0, line: 0 })
            }
            Err(VmError::Assert { message, line }) => {
                eprintln!("sb: ASSERT__ failed at line {line}: {message}");
                Some(SbError { errnum: 0, line })
            }
        }
    }

    /// winit application state: the program's rendered framebuffer plus the live window +
    /// softbuffer surface (created in `resumed`, kept for the window's lifetime).
    struct App {
        fb: Framebuffer,
        window: Option<Rc<Window>>,
        surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    }

    impl App {
        fn new(fb: Framebuffer) -> Self {
            Self {
                fb,
                window: None,
                surface: None,
            }
        }

        /// Blit the framebuffer to the window via softbuffer, nearest-neighbour scaling it
        /// to fill the current window size. softbuffer pixels are `0x00RRGGBB` (no alpha).
        fn redraw(&mut self) {
            let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut())
            else {
                return;
            };
            let size = window.inner_size();
            let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
            else {
                return; // minimised / zero-sized: nothing to draw
            };
            if surface.resize(w, h).is_err() {
                return;
            }
            let Ok(mut buffer) = surface.buffer_mut() else {
                return;
            };
            let (w, h) = (w.get() as usize, h.get() as usize);
            let (fw, fh) = (self.fb.width, self.fb.height);
            for y in 0..h {
                let sy = (y * fh / h).min(fh - 1);
                for x in 0..w {
                    let sx = (x * fw / w).min(fw - 1);
                    buffer[y * w + x] = self.fb.get_argb(sx, sy) & 0x00FF_FFFF;
                }
            }
            let _ = buffer.present();
        }
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = Window::default_attributes()
                .with_title("SmileBASIC")
                .with_inner_size(LogicalSize::new(
                    self.fb.width as f64 * SCALE,
                    self.fb.height as f64 * SCALE,
                ));
            let window = match event_loop.create_window(attrs) {
                Ok(w) => Rc::new(w),
                Err(e) => {
                    eprintln!("sb: cannot create window: {e}");
                    event_loop.exit();
                    return;
                }
            };
            let context = match softbuffer::Context::new(window.clone()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("sb: softbuffer context: {e}");
                    event_loop.exit();
                    return;
                }
            };
            match softbuffer::Surface::new(&context, window.clone()) {
                Ok(s) => self.surface = Some(s),
                Err(e) => {
                    eprintln!("sb: softbuffer surface: {e}");
                    event_loop.exit();
                    return;
                }
            }
            self.window = Some(window);
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => self.redraw(),
                WindowEvent::Resized(_) => {
                    if let Some(w) = self.window.as_ref() {
                        w.request_redraw();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn main() -> ExitCode {
        let mut args = std::env::args().skip(1);
        let Some(path) = args.next() else {
            eprintln!("usage: sb <program.sb3>");
            return ExitCode::from(2);
        };

        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("sb: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        };

        let (fb, err) = run_to_framebuffer(&src);
        // Report a SmileBASIC error like `sb-run` does (the window still shows the output).
        if let Some(e) = &err {
            eprintln!("ERRNUM={} ERRLINE={}", e.errnum, e.line);
        }

        let event_loop = match EventLoop::new() {
            Ok(el) => el,
            Err(e) => {
                // No display server (e.g. headless CI): we can't open a window, but the
                // program already ran — surface that rather than panicking.
                eprintln!("sb: cannot open a window ({e}); ran headless.");
                return if err.is_some() {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                };
            }
        };
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = App::new(fb);
        if let Err(e) = event_loop.run_app(&mut app) {
            eprintln!("sb: event loop error: {e}");
            return ExitCode::from(1);
        }
        if err.is_some() {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn renders_printed_text_to_pixels() {
            // `?"HI"` should light up some non-black foreground pixels on the black backdrop.
            let (fb, err) = run_to_framebuffer(r#"?"HI""#);
            assert!(
                err.is_none(),
                "clean program, got {:?}",
                err.map(|e| e.errnum)
            );
            assert_eq!(fb.width, sb_render::TOP_WIDTH);
            let lit = (0..fb.width)
                .flat_map(|x| (0..fb.height).map(move |y| (x, y)))
                .any(|(x, y)| fb.get_argb(x, y) & 0x00FF_FFFF != 0);
            assert!(lit, "expected some lit text pixels");
        }

        #[test]
        fn error_is_reported_but_console_still_renders() {
            // SQR(-1) → Out of range (10) on line 1; the framebuffer is still produced.
            let (fb, err) = run_to_framebuffer("A=SQR(-1)");
            let err = err.expect("should error");
            assert_eq!(err.errnum, 10);
            assert_eq!(err.line, 1);
            assert_eq!(fb.width, sb_render::TOP_WIDTH);
        }
    }
}
