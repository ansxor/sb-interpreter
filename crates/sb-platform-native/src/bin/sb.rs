//! `sb` — the windowed SmileBASIC host (M1-T12).
//!
//! Loads a `.sb3` file (plain UTF-8 SmileBASIC source), runs it through the `sb-core`
//! pipeline exactly like the headless `sb-run`, then renders the resulting text console
//! into an [`sb_render::Framebuffer`] and blits that buffer to a real OS window
//! (winit + softbuffer). It is the visual counterpart of `sb-run`: same VM, same
//! framebuffer, one drawn to a window instead of dumped as text.
//!
//! The VM still runs to completion up front (a frame-*yielding* execution model — where a
//! program's own `VSYNC` loop drives the window live — is a later milestone). Once it
//! returns, the host enters a real **60 fps loop** (M4-T3): every `clock::FRAME_DURATION` it
//! ticks the VM's frame clock (`MAINCNT` advances; sprite/BG animations step) and re-blits,
//! so animations set up before the program ended keep playing at a steady ~60 fps.
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
    use std::time::Instant;

    use sb_core::builtins::StdBuiltins;
    use sb_core::clock::FRAME_DURATION;
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

    /// The result of preparing a program: either a live VM (so the host loop can keep
    /// ticking its frame clock + animations) or a failure before any scene existed.
    enum Scene {
        /// Parse → compile → run succeeded enough to have a VM; the host loop animates it.
        /// Boxed — a `Vm` is far larger than the `Failed` variant.
        Live(Box<Vm>),
        /// Parse/compile failed before a VM existed — only the backdrop is shown.
        Failed,
    }

    /// Composite the current scene into a fresh top-screen framebuffer (backdrop → GRP
    /// display page → BG → sprites → console, M2-T4 / M3-T6). For a [`Scene::Failed`] there
    /// is nothing to draw, so it is just the opaque-black backdrop (GRP/console pixels are
    /// transparent by default, so the backdrop must be painted first to be blittable).
    fn compose(scene: &Scene) -> Framebuffer {
        match scene {
            Scene::Live(vm) => compose_top_screen(
                vm.grp(),
                vm.bg(),
                vm.sprites(),
                vm.console(),
                DEFAULT_BACKDROP,
                vm.screen_visibility(),
            ),
            Scene::Failed => {
                let mut fb = Framebuffer::top();
                fb.clear(DEFAULT_BACKDROP);
                fb
            }
        }
    }

    /// Parse → compile → run `src` to completion. Returns the scene to display plus the
    /// SmileBASIC error if any stage raised one (the partial scene is still kept + animated).
    fn prepare(src: &str) -> (Scene, Option<SbError>) {
        let ast = match parse(src) {
            Ok(ast) => ast,
            Err(e) => {
                return (
                    Scene::Failed,
                    Some(SbError {
                        errnum: e.errnum,
                        line: e.loc.line,
                    }),
                )
            }
        };
        let program = match compile_with(&ast, &StdBuiltins) {
            Ok(p) => p,
            Err(e) => {
                return (
                    Scene::Failed,
                    Some(SbError {
                        errnum: e.errnum,
                        line: e.loc.line,
                    }),
                )
            }
        };
        let mut vm = Vm::new(program);
        let err = match vm.run() {
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
        };
        (Scene::Live(Box::new(vm)), err)
    }

    /// winit application state: the live scene (VM) + its last composited framebuffer, the
    /// window + softbuffer surface (created in `resumed`), and the next frame's deadline for
    /// the 60 fps pacer.
    struct App {
        scene: Scene,
        fb: Framebuffer,
        /// The instant the next frame is due. `None` until the window is up; once running it
        /// advances by `FRAME_DURATION` each tick so the loop holds a steady ~60 fps.
        next_frame: Option<Instant>,
        window: Option<Rc<Window>>,
        surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    }

    impl App {
        fn new(scene: Scene) -> Self {
            let fb = compose(&scene);
            Self {
                scene,
                fb,
                next_frame: None,
                window: None,
                surface: None,
            }
        }

        /// Advance one displayed frame: tick the VM's frame clock + animations and
        /// recomposite. A no-op for a failed scene (nothing to animate).
        fn step_frame(&mut self) {
            if let Scene::Live(vm) = &mut self.scene {
                vm.tick_frame();
                self.fb = compose(&self.scene);
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
            // Start the 60 fps pacer from the moment the window is up.
            self.next_frame = Some(Instant::now() + FRAME_DURATION);
        }

        /// The 60 fps heartbeat (M4-T3). Each `FRAME_DURATION` boundary we step the scene one
        /// frame and request a redraw; `ControlFlow::WaitUntil` sleeps until the next boundary
        /// so the loop holds ~60 fps without busy-spinning. A failed scene has nothing to
        /// animate, so it falls back to event-driven `Wait`.
        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if matches!(self.scene, Scene::Failed) {
                event_loop.set_control_flow(ControlFlow::Wait);
                return;
            }
            let Some(deadline) = self.next_frame else {
                return;
            };
            let now = Instant::now();
            if now >= deadline {
                self.step_frame();
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
                // Advance to the next frame boundary; if we fell far behind (e.g. the window
                // was hidden), resync to "now" rather than firing a burst of catch-up frames.
                let next = deadline + FRAME_DURATION;
                self.next_frame = Some(if next <= now {
                    now + FRAME_DURATION
                } else {
                    next
                });
            }
            if let Some(next) = self.next_frame {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next));
            }
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

        let (scene, err) = prepare(&src);
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

        let mut app = App::new(scene);
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
            let (scene, err) = prepare(r#"?"HI""#);
            assert!(
                err.is_none(),
                "clean program, got {:?}",
                err.map(|e| e.errnum)
            );
            let fb = compose(&scene);
            assert_eq!(fb.width, sb_render::TOP_WIDTH);
            let lit = (0..fb.width)
                .flat_map(|x| (0..fb.height).map(move |y| (x, y)))
                .any(|(x, y)| fb.get_argb(x, y) & 0x00FF_FFFF != 0);
            assert!(lit, "expected some lit text pixels");
        }

        #[test]
        fn error_is_reported_but_console_still_renders() {
            // SQR(-1) → Out of range (10) on line 1; the framebuffer is still produced.
            let (scene, err) = prepare("A=SQR(-1)");
            let err = err.expect("should error");
            assert_eq!(err.errnum, 10);
            assert_eq!(err.line, 1);
            assert_eq!(compose(&scene).width, sb_render::TOP_WIDTH);
        }

        #[test]
        fn frame_loop_ticks_maincnt_and_animation() {
            // The 60 fps host loop drives the VM's frame clock: stepping a frame advances
            // MAINCNT and steps a BG scroll animation set up before the program ended.
            let (mut scene, err) = prepare("BGSCREEN 0,32,32\nBGANIM 0,\"XY\",2,16,8");
            assert!(err.is_none());
            let Scene::Live(vm) = &mut scene else {
                panic!("expected a live scene");
            };
            assert_eq!(vm.maincnt(), 0);
            vm.tick_frame();
            assert_eq!(vm.maincnt(), 1);
            assert_eq!((vm.bg().layers[0].ofs_x, vm.bg().layers[0].ofs_y), (16, 8));
        }
    }
}
