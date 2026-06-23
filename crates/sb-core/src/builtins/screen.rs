//! Screen configuration (M4-T4) — the `XSCREEN` / `DISPLAY` / `VISIBLE` / `HARDWARE`
//! commands over the VM-owned [`ScreenConfig`].
//!
//! These four route over screen *configuration* state (which screen mode is active, which
//! of the two 3DS screens output targets, which display layers are shown, and the hardware
//! model), so the VM handles them directly like the console/graphics commands rather than
//! through the stateless `dispatch`. The actual screen reconfiguration / dual-screen output
//! has no scalar golden; what is pinned (and tested) here is the argument-shape + range
//! validation the disassembled handlers enforce and the layer-visibility flags the
//! compositor consumes.
//!
//! Specs: `spec/instructions/{xscreen,display,visible}.yaml` (S-T11d, hw_verified arg/range
//! guards) and `spec/reference/sysvars.yaml` (`HARDWARE`, 1 = new3DS).
//!
//! The DIRECT-mode guards (`XSCREEN 4` / any `DISPLAY` → errnum 43) are not reachable here:
//! the VM only executes programs (program mode), exactly the context in which the oracle
//! captured these cases, so that guard never fires (queued for a DIRECT-mode harness).

use crate::builtins::{illegal, out_of_range};
use crate::value::{RuntimeError, Value};
use sb_render::compositor::LayerVisibility;

/// The configured hardware model `HARDWARE` reports: 1 = new3DS (what Azahar emulates, the
/// oracle's value), 0 = old 3DS (`sysvars.yaml`).
pub const DEFAULT_HARDWARE: i32 = 1;

/// Index of the four `VISIBLE` layer flags: Console, Graphic, BG, Sprite (the documented
/// argument order).
const CONSOLE: usize = 0;
const GRAPHIC: usize = 1;
const BG: usize = 2;
const SPRITE: usize = 3;

/// VM-owned screen configuration (M4-T4): the `XSCREEN` mode, the `DISPLAY` output target,
/// per-screen layer visibility (`VISIBLE`) and the `HARDWARE` model.
#[derive(Debug, Clone)]
pub struct ScreenConfig {
    /// `XSCREEN` mode 0..4 (boot default 0: Upper 3D, Touch unused). Governs which screen
    /// ids `DISPLAY` accepts.
    pub mode: i32,
    /// The currently selected output screen (`DISPLAY`): 0 = Upper, 1 = Touch.
    pub display: i32,
    /// Per-screen layer visibility, `[screen][Console, Graphic, BG, Sprite]`. Every layer is
    /// shown by default. `VISIBLE` writes the row of whichever screen `DISPLAY` selects.
    pub visible: [[bool; 4]; 2],
    /// The `HARDWARE` model (1 = new3DS).
    pub hardware: i32,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        ScreenConfig {
            mode: 0,
            display: 0,
            visible: [[true; 4]; 2],
            hardware: DEFAULT_HARDWARE,
        }
    }
}

impl ScreenConfig {
    /// A fresh boot configuration (mode 0, Upper screen selected, all layers shown, new3DS).
    pub fn new() -> Self {
        Self::default()
    }

    /// The Upper-screen layer visibility, as the compositor wants it. The reimplementation
    /// renders only the Upper screen (the Touch-screen framebuffer is queued), so the
    /// compositor always reads screen 0's flags.
    pub fn upper_visibility(&self) -> LayerVisibility {
        let v = self.visible[0];
        LayerVisibility {
            console: v[CONSOLE],
            graphic: v[GRAPHIC],
            bg: v[BG],
            sprite: v[SPRITE],
        }
    }

    /// `XSCREEN mode [, sprites, bg]` — set the screen mode and (optionally) the sprite / BG
    /// allocation split. Takes 1 or 3 arguments and never returns a value. Errors mirror the
    /// disassembled handler (`xscreen.yaml`, hw_verified): a 2-argument call or use as a
    /// function → errnum 4; a mode outside 0..4, a sprite count outside 0..512 or a BG count
    /// outside 0..4 → errnum 10.
    pub fn xscreen(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        // Used where a return value is expected (result count != 0) → Illegal function call.
        if wants_value {
            return Err(illegal());
        }
        // Exactly 1 or 3 arguments (a 2-argument call is rejected before the range checks).
        if args.len() != 1 && args.len() != 3 {
            return Err(illegal());
        }
        let mode = args[0].to_int()?;
        if !(0..=4).contains(&mode) {
            return Err(out_of_range());
        }
        if args.len() == 3 {
            let sprites = args[1].to_int()?;
            if !(0..=512).contains(&sprites) {
                return Err(out_of_range());
            }
            let bg = args[2].to_int()?;
            if !(0..=4).contains(&bg) {
                return Err(out_of_range());
            }
        }
        self.mode = mode;
        Ok(())
    }

    /// `DISPLAY screen_id` (SET, statement) / `DISPLAY()` (GET, function). The GET form
    /// returns the currently selected screen id (0 or 1). The SET form selects the target
    /// screen and is range-checked against the active `XSCREEN` mode: modes 0/1/4
    /// (single-screen / combined) require id == 0; modes 2/3 (Touch Screen used) accept id 0
    /// or 1. A bad call shape → errnum 4; an id the mode does not allow → errnum 10 (per
    /// `display.yaml`, hw_verified).
    pub fn display(
        &mut self,
        args: &[Value],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        if wants_value {
            // GET form: no arguments.
            if !args.is_empty() {
                return Err(illegal());
            }
            return Ok(Some(Value::Int(self.display)));
        }
        // SET form: exactly one argument.
        if args.len() != 1 {
            return Err(illegal());
        }
        let id = args[0].to_int()?;
        // Modes 2/3 expose the Touch Screen (id 0 or 1); every other mode is Upper only.
        let max_id = if self.mode == 2 || self.mode == 3 {
            1
        } else {
            0
        };
        if !(0..=max_id).contains(&id) {
            return Err(out_of_range());
        }
        self.display = id;
        Ok(None)
    }

    /// `VISIBLE console, graphic, bg, sprite` — toggle the four display layers of the
    /// currently selected screen on/off. Takes exactly four arguments and no return value;
    /// each flag is booleanized (nonzero = show, zero = hide). Any other call shape → errnum
    /// 4 (`visible.yaml`, hw_verified). There is no numeric range error.
    pub fn visible(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        if wants_value || args.len() != 4 {
            return Err(illegal());
        }
        let row = &mut self.visible[self.display as usize];
        for (slot, arg) in row.iter_mut().zip(args) {
            *slot = arg.to_int()? != 0;
        }
        Ok(())
    }

    /// `HARDWARE` — the read-only sysvar reporting the hardware model (1 = new3DS). Takes no
    /// arguments; any argument → errnum 4.
    pub fn hardware(&self, args: &[Value]) -> Result<Value, RuntimeError> {
        if !args.is_empty() {
            return Err(illegal());
        }
        Ok(Value::Int(self.hardware))
    }
}
