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
//! Host input mapping (M4-T5): keyboard + mouse events are folded into the VM's
//! [`InputState`](sb_core::input::InputState) each frame via the shared
//! [`HostInput`](sb_core::host_input::HostInput) accumulator, so `BUTTON`/`STICK`/`STICKEX`
//! and `TOUCH` read live host input. The default desktop keymap lives in [`keymap`]:
//!
//! ```text
//!   Arrow keys ........ D-pad   (#UP #DOWN #LEFT #RIGHT)
//!   U I J K ........... face    (#Y #X #B #A)
//!   Q E ............... shoulders (#L #R)        1 2 ... #ZL #ZR
//!   W A S D ........... left Circle Pad  (STICK)
//!   Numpad 8 4 2 6 .... right Circle Pad Pro (STICKEX)
//!   Mouse on window ... touch screen (TOUCH); left button = touching
//!   typed text ........ INKEY$ queue
//! ```
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
    use sb_core::host_input::HostInput;
    use sb_core::{parse, Vm, VmError};
    use sb_render::compositor::{apply_fader, compose_top_screen, DEFAULT_BACKDROP};
    use sb_render::Framebuffer;

    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::{MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::PhysicalKey;
    use winit::window::{Window, WindowId};

    /// Default integer zoom — the 400×240 top screen is tiny on a desktop, so show it at 2×.
    const SCALE: f64 = 2.0;

    /// The bottom (touch) screen is 320×240; `TOUCH` reports a coordinate in that space.
    const TOUCH_WIDTH: f64 = 320.0;
    const TOUCH_HEIGHT: f64 = 240.0;

    /// The default desktop keymap: physical keys → logical [`Bind`](sb_core::host_input::Bind)
    /// (see the module docs for the layout table). A physical-key map is per platform because
    /// the key enum differs (winit `KeyCode` here, DOM `event.code` on the web); the
    /// accumulation it feeds is the shared `sb_core::host_input::HostInput`.
    mod keymap {
        use sb_core::host_input::{Bind, Stick};
        use sb_core::input::{
            BTN_A, BTN_B, BTN_DOWN, BTN_L, BTN_LEFT, BTN_R, BTN_RIGHT, BTN_UP, BTN_X, BTN_Y,
            BTN_ZL, BTN_ZR,
        };
        use winit::keyboard::KeyCode;

        /// Map one physical key to its default binding, or `None` if unbound.
        pub fn bind(key: KeyCode) -> Option<Bind> {
            use KeyCode::*;
            Some(match key {
                // D-pad — arrow keys.
                ArrowUp => Bind::Button(BTN_UP),
                ArrowDown => Bind::Button(BTN_DOWN),
                ArrowLeft => Bind::Button(BTN_LEFT),
                ArrowRight => Bind::Button(BTN_RIGHT),
                // Face buttons — U/I/J/K diamond (#Y top, #A bottom, #B left, #X right).
                KeyU => Bind::Button(BTN_Y),
                KeyI => Bind::Button(BTN_X),
                KeyJ => Bind::Button(BTN_B),
                KeyK => Bind::Button(BTN_A),
                // Shoulders.
                KeyQ => Bind::Button(BTN_L),
                KeyE => Bind::Button(BTN_R),
                Digit1 => Bind::Button(BTN_ZL),
                Digit2 => Bind::Button(BTN_ZR),
                // Left Circle Pad (STICK) — WASD.
                KeyW => Bind::AxisY(Stick::Left, 1.0),
                KeyS => Bind::AxisY(Stick::Left, -1.0),
                KeyA => Bind::AxisX(Stick::Left, -1.0),
                KeyD => Bind::AxisX(Stick::Left, 1.0),
                // Right Circle Pad Pro (STICKEX) — numpad arrows.
                Numpad8 => Bind::AxisY(Stick::Right, 1.0),
                Numpad2 => Bind::AxisY(Stick::Right, -1.0),
                Numpad4 => Bind::AxisX(Stick::Right, -1.0),
                Numpad6 => Bind::AxisX(Stick::Right, 1.0),
                _ => return None,
            })
        }
    }

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
            Scene::Live(vm) => {
                let mut fb = compose_top_screen(
                    vm.grp(),
                    // The native runner renders the Upper screen (screen 0) — draw its BG + sprites.
                    vm.bg_for(0),
                    vm.sprites_for(0),
                    vm.console_for(0),
                    DEFAULT_BACKDROP,
                    vm.screen_visibility(),
                );
                // The screen fader is a global overlay drawn in front of the composed frame.
                if let Some(color) = vm.fader_overlay() {
                    apply_fader(&mut fb, color);
                }
                fb
            }
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
        /// Host input accumulator (M4-T5): key events fold into this, and each frame its
        /// button mask + stick axes feed the VM's `InputState`.
        input: HostInput,
        /// Last cursor position in physical window pixels (mapped to the touch screen).
        cursor: (f64, f64),
        /// Whether the left mouse button is down (drives `TOUCH`'s touched state).
        touching: bool,
        /// Typed text (UTF-16) buffered since the last frame, drained into the `INKEY$` queue.
        pending_keys: Vec<u16>,
        /// Wall-clock origin for the free-running 60Hz `MAINCNT` counter (M4-T3). Set when
        /// the window appears so the frame clock ticks by real elapsed time, not by how the
        /// host slices VM execution.
        clock_origin: Option<Instant>,
        /// The wall-clock frame count already synced to the VM's clock (`clock_origin`).
        clock_sync_frame: u64,
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
                input: HostInput::new(),
                cursor: (0.0, 0.0),
                touching: false,
                pending_keys: Vec::new(),
                clock_origin: None,
                clock_sync_frame: 0,
            }
        }

        /// Map the last cursor position to a `TOUCH` coordinate on the 320×240 bottom screen
        /// (the window shows the top screen, so the cursor is scaled proportionally into the
        /// touch space — a best-effort until the bottom screen is rendered).
        fn touch_coords(&self) -> (i32, i32) {
            let Some(window) = self.window.as_ref() else {
                return (0, 0);
            };
            let size = window.inner_size();
            if size.width == 0 || size.height == 0 {
                return (0, 0);
            }
            let x = (self.cursor.0 / size.width as f64 * TOUCH_WIDTH).clamp(0.0, TOUCH_WIDTH - 1.0);
            let y =
                (self.cursor.1 / size.height as f64 * TOUCH_HEIGHT).clamp(0.0, TOUCH_HEIGHT - 1.0);
            (x as i32, y as i32)
        }

        /// Advance one displayed frame: feed the accumulated host input into the VM's
        /// `InputState`, tick the frame clock + animations, and recomposite. A no-op for a
        /// failed scene (nothing to animate / drive).
        fn step_frame(&mut self) {
            // Snapshot host input first (these borrows end before the VM is touched).
            let held = self.input.held();
            let stick = self.input.stick();
            let stickex = self.input.stickex();
            let (tx, ty) = self.touch_coords();
            let touching = self.touching;
            let keys = std::mem::take(&mut self.pending_keys);
            if let Scene::Live(vm) = &mut self.scene {
                // Free-running MAINCNT: sync the VM's frame clock to the wall-clock 60Hz source
                // once per displayed frame, independent of how much VM work this tick runs.
                match self.clock_origin {
                    Some(origin) => {
                        let frame_ns = FRAME_DURATION.as_nanos();
                        let target = (origin.elapsed().as_nanos() / frame_ns) as u64;
                        if target > self.clock_sync_frame {
                            vm.tick_frames(target - self.clock_sync_frame);
                            self.clock_sync_frame = target;
                        }
                    }
                    None => vm.tick_frame(),
                }
                let inp = vm.input_mut();
                inp.advance_frame(held, stick, stickex);
                inp.advance_touch(touching, tx, ty);
                for unit in keys {
                    inp.push_key(unit);
                }
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
            // Start the 60 fps pacer and the free-running MAINCNT origin from the moment the
            // window is up.
            let now = Instant::now();
            self.next_frame = Some(now + FRAME_DURATION);
            self.clock_origin = Some(now);
            self.clock_sync_frame = 0;
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
                // Keyboard → button / stick masks + the INKEY$ queue (M4-T5).
                WindowEvent::KeyboardInput { event, .. } => {
                    let pressed = event.state.is_pressed();
                    if let PhysicalKey::Code(code) = event.physical_key {
                        self.input.apply(keymap::bind(code), pressed);
                    }
                    // Typed characters feed INKEY$; only on the press edge (not on release),
                    // and winit already filters this to actual text input.
                    if pressed {
                        if let Some(text) = &event.text {
                            self.pending_keys.extend(text.encode_utf16());
                        }
                    }
                }
                // Mouse → TOUCH: track the cursor and whether the left button is down.
                WindowEvent::CursorMoved { position, .. } => {
                    self.cursor = (position.x, position.y);
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    self.touching = state.is_pressed();
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
        fn keymap_covers_dpad_face_and_sticks() {
            use sb_core::host_input::{Bind, Stick};
            use sb_core::input::{BTN_A, BTN_UP, BTN_ZL};
            use winit::keyboard::KeyCode;
            assert_eq!(keymap::bind(KeyCode::ArrowUp), Some(Bind::Button(BTN_UP)));
            assert_eq!(keymap::bind(KeyCode::KeyK), Some(Bind::Button(BTN_A)));
            assert_eq!(keymap::bind(KeyCode::Digit1), Some(Bind::Button(BTN_ZL)));
            assert_eq!(
                keymap::bind(KeyCode::KeyD),
                Some(Bind::AxisX(Stick::Left, 1.0))
            );
            assert_eq!(
                keymap::bind(KeyCode::Numpad8),
                Some(Bind::AxisY(Stick::Right, 1.0))
            );
            // An unbound key maps to nothing.
            assert_eq!(keymap::bind(KeyCode::F12), None);
        }

        #[test]
        fn step_frame_feeds_host_input_to_the_vm() {
            // A held button, a touch, and a typed key fed through the frame loop must reach
            // the VM's InputState so BUTTON/TOUCH/INKEY$ would observe them.
            use sb_core::host_input::Bind;
            use sb_core::input::BTN_A;
            let (scene, err) = prepare("BGSCREEN 0,32,32");
            assert!(err.is_none());
            let mut app = App::new(scene);
            app.input.apply(Some(Bind::Button(BTN_A)), true);
            app.touching = true;
            app.pending_keys.extend("X".encode_utf16());
            app.step_frame();
            let Scene::Live(vm) = &mut app.scene else {
                panic!("expected a live scene");
            };
            assert_eq!(vm.input().button(0), Some(BTN_A as i32)); // held
            assert_eq!(vm.input().touch().0, 1); // touched for one frame
            assert_eq!(vm.input_mut().pop_key(), Some(b'X' as u16)); // INKEY$ queue
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
