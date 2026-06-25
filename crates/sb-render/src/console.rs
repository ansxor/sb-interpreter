//! The SmileBASIC text **console** layer: a character grid + cursor + colors/attributes,
//! and a rasterizer that paints the grid into a [`Framebuffer`].
//!
//! Model (cross-checked against `osb/SMILEBASIC/console.d` `ConsoleCharacter` and the
//! disassembled `LOCATE`/`COLOR`/`ATTR`/`CLS` handlers — see the matching
//! `spec/instructions/*.yaml`):
//!
//! - The top screen's console is a **50×30** grid of 8×8 cells (400×240). The touch screen
//!   is 40×30 (320×240). Each [`Cell`] holds a character code plus per-cell foreground and
//!   background palette indices and a display attribute (rotation/inversion).
//! - **Colors** are a 16-entry palette index (`COLOR fg[,bg]`, 0..15): index **0 is
//!   transparent**, 1..15 are `#TBLACK..#TWHITE`. The default drawing color is white (15)
//!   on transparent (0).
//! - **Attribute** (`ATTR`, 0..15): bits 0–1 rotation 0/90/180/270° (`#TROT0..#TROT270`),
//!   bit 2 horizontal invert (`#TREVH`), bit 3 vertical invert (`#TREVV`). It is persistent
//!   console state applied to every character printed after it is set.
//! - Printing past the right edge wraps to the next row; printing past the bottom row
//!   scrolls the whole grid up by one line.
//!
//! Glyph source: [`render_with_font`](Console::render_with_font) samples the firmware **font
//! page** (the hidden GRPF page, [`crate::assets::default_font_page`]) via the codepoint→
//! position [`atlas`](crate::assets::atlas) — so on-screen text matches real SmileBASIC. A
//! `FONTDEF` override takes precedence per cell, and the plain [`render`](Console::render)
//! (no font page) falls back to the self-contained placeholder [`crate::font`] glyphs.

use std::collections::HashMap;

use crate::assets::atlas;
use crate::font;
use crate::grp::{GrpPage, GRP_DIM};
use crate::Framebuffer;

/// Cell size in dots (SB console font is 8×8 — osb `console.d` `fontDefWidth/Height`).
pub const CELL: usize = 8;

/// Top-screen console: 50 columns × 30 rows.
pub const TOP_COLS: usize = 50;
pub const TOP_ROWS: usize = 30;

/// Touch-screen console: 40 columns × 30 rows (320×240 dots).
pub const BOTTOM_COLS: usize = 40;
pub const BOTTOM_ROWS: usize = 30;

/// Default drawing (foreground) color: white `#TWHITE` = 15.
pub const DEFAULT_FG: u8 = 15;
/// Default background color: transparent = 0.
pub const DEFAULT_BG: u8 = 0;

/// The 16-color text palette as ARGB8888, index 0 = transparent.
///
/// Derived from the documented 16-color set (`COLOR` doc: 0 Transparent, 1 Black … 15
/// White) cross-checked against `osb/SMILEBASIC/console.d` `consoleColor`, then **quantized
/// to SB 3.6.0's hw_verified 5-bit device precision** (`quantize8`, low 3 bits forced 0):
/// e.g. white = `0xF8F8F8` (== the hw_verified `#WHITE`), red = `0xF80000`. osb (3.5.0)
/// stored the un-quantized `0xFF`/`0x7F` values; the exact text-layer ARGB on 3.6.0 is
/// oracle-pending (composite screenshot capture, O-T6 → `HARVEST_QUEUE.md`).
pub const TEXT_PALETTE: [u32; 16] = [
    0x0000_0000, // 0 transparent
    0xFF00_0000, // 1 black   #TBLACK
    0xFF78_0000, // 2 maroon  #TMAROON
    0xFFF8_0000, // 3 red     #TRED
    0xFF00_7800, // 4 green   #TGREEN
    0xFF00_F800, // 5 lime    #TLIME
    0xFF78_7800, // 6 olive   #TOLIVE
    0xFFF8_F800, // 7 yellow  #TYELLOW
    0xFF00_0078, // 8 navy    #TNAVY
    0xFF00_00F8, // 9 blue    #TBLUE
    0xFF78_0078, // 10 purple #TPURPLE
    0xFFF8_00F8, // 11 magenta #TMAGENTA
    0xFF00_7878, // 12 teal   #TTEAL
    0xFF00_F8F8, // 13 cyan   #TCYAN
    0xFF78_7878, // 14 gray   #TGRAY
    0xFFF8_F8F8, // 15 white  #TWHITE
];

/// A single console cell. Mirrors osb's `ConsoleCharacter { character, foreColor, backColor,
/// attr, z }` (z/depth is tracked on the console as cursor state, not per-cell here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Character code (UTF-16 code unit, as SB stores wide chars). 0 = empty.
    pub ch: u16,
    /// Foreground palette index (0..15).
    pub fg: u8,
    /// Background palette index (0..15).
    pub bg: u8,
    /// Display attribute (rotation/inversion bits, 0..15).
    pub attr: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: 0,
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            attr: 0,
        }
    }
}

/// The text console: a `cols × rows` grid plus cursor + current color/attribute state.
#[derive(Debug, Clone)]
pub struct Console {
    pub cols: usize,
    pub rows: usize,
    cells: Vec<Cell>,
    pub cur_x: usize,
    pub cur_y: usize,
    pub fg: u8,
    pub bg: u8,
    pub attr: u8,
    /// Console font size in pixels: 8 (standard) or 16 (double). Affects glyph scale only;
    /// the grid dimensions stay fixed in this model (the reflow is screen-state, M2-T4).
    pub font_size: u8,
    /// Console scroll offset in character cells (positive = viewpoint moves right/down).
    pub scroll_x: i32,
    pub scroll_y: i32,
    /// Custom 8×8 glyphs defined by FONTDEF; code → row bitmaps.
    custom_glyphs: HashMap<u16, [u8; 8]>,
}

impl Console {
    /// A console of the given grid dimensions, cleared, cursor home, default colors.
    pub fn new(cols: usize, rows: usize) -> Self {
        Console {
            cols,
            rows,
            cells: vec![Cell::default(); cols * rows],
            cur_x: 0,
            cur_y: 0,
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            attr: 0,
            font_size: 8,
            scroll_x: 0,
            scroll_y: 0,
            custom_glyphs: HashMap::new(),
        }
    }

    /// The standard top-screen console (50×30).
    pub fn top() -> Self {
        Self::new(TOP_COLS, TOP_ROWS)
    }

    /// The standard Touch-screen console (40×30).
    pub fn bottom() -> Self {
        Self::new(BOTTOM_COLS, BOTTOM_ROWS)
    }

    /// Resize the grid to the given dimensions, clearing the buffer and homing the cursor.
    /// Color/attribute/font state is preserved; this is used when `XSCREEN` changes the
    /// active screen's resolution.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.cells = vec![Cell::default(); cols * rows];
        self.cur_x = 0;
        self.cur_y = 0;
        self.scroll_x = 0;
        self.scroll_y = 0;
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.cols + x
    }

    /// Read a cell (returns `Cell::default()` for out-of-range coordinates).
    pub fn cell(&self, x: usize, y: usize) -> Cell {
        if x >= self.cols || y >= self.rows {
            return Cell::default();
        }
        self.cells[self.idx(x, y)]
    }

    /// `CLS`: clear the grid to empty cells and home the cursor. Does NOT change the current
    /// `COLOR`/`ATTR` (matches the disassembled CLS handler — only clears + homes).
    pub fn cls(&mut self) {
        for c in self.cells.iter_mut() {
            *c = Cell {
                ch: 0,
                fg: self.fg,
                bg: DEFAULT_BG,
                attr: 0,
            };
        }
        self.cur_x = 0;
        self.cur_y = 0;
    }

    /// `LOCATE x,y`: set the cursor. Coordinates are clamped to the grid here; range
    /// validation (errnum 4/10) is the VM's job per `locate.yaml`.
    pub fn locate(&mut self, x: usize, y: usize) {
        self.cur_x = x.min(self.cols.saturating_sub(1));
        self.cur_y = y.min(self.rows.saturating_sub(1));
    }

    /// `COLOR fg[,bg]`: set the current drawing/background palette indices.
    pub fn color(&mut self, fg: u8, bg: u8) {
        self.fg = fg;
        self.bg = bg;
    }

    /// `ATTR a`: set the persistent display attribute applied to subsequent characters.
    pub fn set_attr(&mut self, attr: u8) {
        self.attr = attr;
    }

    /// `WIDTH size`: set the console font size (8 or 16).
    pub fn set_font_size(&mut self, size: u8) {
        self.font_size = size;
    }

    /// `WIDTH()` query.
    pub fn font_size(&self) -> u8 {
        self.font_size
    }

    /// `SCROLL x,y`: set the viewpoint offset in character cells.
    pub fn scroll(&mut self, x: i32, y: i32) {
        self.scroll_x = x;
        self.scroll_y = y;
    }

    /// `FONTDEF code,"string"` / `FONTDEF code,array`: install a custom 8×8 glyph.
    pub fn set_custom_glyph(&mut self, code: u16, glyph: [u8; 8]) {
        self.custom_glyphs.insert(code, glyph);
    }

    /// `FONTDEF` with no arguments: reset all custom font definitions.
    pub fn reset_font(&mut self) {
        self.custom_glyphs.clear();
    }

    /// Copy every custom glyph definition from another console. FONTDEF edits are global to
    /// the console font, so the VM mirrors them across both physical screens.
    pub fn copy_custom_glyphs_from(&mut self, other: &Console) {
        self.custom_glyphs = other.custom_glyphs.clone();
    }

    /// Borrow the custom glyph table (for cross-screen mirroring).
    pub fn custom_glyphs(&self) -> &HashMap<u16, [u8; 8]> {
        &self.custom_glyphs
    }

    /// Replace the custom glyph table (for cross-screen mirroring).
    pub fn set_custom_glyphs(&mut self, glyphs: HashMap<u16, [u8; 8]>) {
        self.custom_glyphs = glyphs;
    }

    /// Glyph for a character code as an 8×8 1-bpp bitmap (bit set = foreground dot).
    ///
    /// Precedence: a `FONTDEF` override wins; otherwise, if a font page is supplied (the
    /// firmware GRPF page, [`crate::assets::default_font_page`]) and it has a glyph for `ch`,
    /// the 8×8 cell at the atlas position is sampled — an opaque (alpha-bit-set) device pixel
    /// becomes a foreground dot (the firmware font is monochrome white-on-transparent, tinted
    /// by the cell's `COLOR`). With no font page, or for a codepoint the font lacks, the
    /// self-contained placeholder [`font::glyph`] is used.
    fn glyph_for(&self, ch: u16, font_page: Option<&GrpPage>) -> [u8; 8] {
        if let Some(&g) = self.custom_glyphs.get(&ch) {
            return g;
        }
        if let Some(page) = font_page {
            if let Some((gx, gy)) = atlas::pos(ch) {
                return sample_glyph(page, gx as usize, gy as usize);
            }
        }
        font::glyph(char::from_u32(ch as u32).unwrap_or('\u{FFFD}'))
    }

    /// Write one character at the cursor using the current color/attribute, then advance.
    /// Wraps at the right edge; a wrap that moves past the bottom row leaves the cursor on
    /// a virtual off-screen row until the next character is written, at which point the grid
    /// scrolls up. This matches real SB behavior where a trailing PRINT newline on the last
    /// row does not scroll the just-printed line off the screen.
    pub fn put_char(&mut self, ch: u16) {
        // Normalize a cursor that has moved onto the virtual off-screen row or past the
        // right edge before writing.
        while self.cur_y >= self.rows || self.cur_x >= self.cols {
            if self.cur_x >= self.cols {
                self.cur_x = 0;
                self.line_feed();
            } else {
                self.scroll_up();
                self.cur_y = self.rows - 1;
                self.cur_x = 0;
            }
        }
        let i = self.idx(self.cur_x, self.cur_y);
        self.cells[i] = Cell {
            ch,
            fg: self.fg,
            bg: self.bg,
            attr: self.attr,
        };
        self.advance();
    }

    /// Advance the cursor one cell, wrapping at the right edge. The wrap moves the cursor
    /// to the next row (which may become the virtual off-screen row); actual scrolling is
    /// deferred until a character is written while past the bottom.
    fn advance(&mut self) {
        self.cur_x += 1;
        if self.cur_x >= self.cols {
            self.cur_x = 0;
            self.line_feed();
        }
    }

    /// `PRINT ,` tab: advance the cursor to the next column that is a multiple of
    /// `step` (the TABSTEP system variable, default 4); cells skipped over are left
    /// untouched (they render as their background). If the next stop is at or past the
    /// right edge, wrap to column 0 of the next row.
    pub fn tab(&mut self, step: usize) {
        let step = step.max(1);
        let target = (self.cur_x / step + 1) * step;
        if target >= self.cols {
            self.newline();
        } else {
            self.cur_x = target;
        }
    }

    /// Move to column 0 of the next row. The cursor may move onto the virtual off-screen
    /// row; the grid only scrolls when a subsequent character is written past the bottom.
    pub fn newline(&mut self) {
        self.cur_x = 0;
        self.line_feed();
    }

    fn line_feed(&mut self) {
        self.cur_y = (self.cur_y + 1).min(self.rows);
    }

    /// Scroll the whole grid up by one row; the new bottom row is cleared.
    pub fn scroll_up(&mut self) {
        let row_len = self.cols;
        self.cells.copy_within(row_len.., 0);
        let start = (self.rows - 1) * self.cols;
        let blank = Cell {
            ch: 0,
            fg: self.fg,
            bg: DEFAULT_BG,
            attr: 0,
        };
        for c in self.cells[start..].iter_mut() {
            *c = blank;
        }
    }

    /// Convenience: print an ASCII/Unicode string at the cursor (no trailing newline).
    pub fn print_str(&mut self, s: &str) {
        for ch in s.encode_utf16() {
            self.put_char(ch);
        }
    }

    /// Rasterize the whole grid into `fb`. Cell (col,row) maps to the 8×8 block at
    /// `(col*8, row*8)`. Foreground dots use the cell's `fg` palette color, the rest its
    /// `bg`; a **transparent** (index 0) color leaves the framebuffer untouched (so a
    /// backdrop/other layers show through). Per-cell rotation/inversion is applied via
    /// [`attr_map`].
    pub fn render(&self, fb: &mut Framebuffer) {
        self.render_with_font(fb, None);
    }

    /// Rasterize the grid using a supplied **font page** (the firmware GRPF page) as the glyph
    /// source. This is what the compositor uses so on-screen text matches real SmileBASIC;
    /// [`render`](Self::render) is the `None` shorthand (placeholder font) the standalone
    /// console tests/goldens use. `FONTDEF` overrides still take precedence per cell.
    pub fn render_with_font(&self, fb: &mut Framebuffer, font_page: Option<&GrpPage>) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                self.render_cell(fb, col, row, font_page);
            }
        }
    }

    fn render_cell(
        &self,
        fb: &mut Framebuffer,
        col: usize,
        row: usize,
        font_page: Option<&GrpPage>,
    ) {
        let cell = self.cells[self.idx(col, row)];
        // ch == 0 is an empty cell (no character), not NUL — draw only the background.
        let glyph = if cell.ch == 0 {
            [0u8; 8]
        } else {
            self.glyph_for(cell.ch, font_page)
        };
        let fg = TEXT_PALETTE[(cell.fg & 0x0F) as usize];
        let bg = TEXT_PALETTE[(cell.bg & 0x0F) as usize];
        let px = col * CELL;
        let py = row * CELL;
        for (sy, &bits) in glyph.iter().enumerate() {
            for sx in 0..CELL {
                let on = (bits >> (7 - sx)) & 1 != 0;
                let color = if on { fg } else { bg };
                // Transparent (alpha 0 / palette index 0) draws nothing.
                if color >> 24 == 0 {
                    continue;
                }
                let (dx, dy) = attr_map(sx, sy, cell.attr);
                fb.set_argb(px + dx, py + dy, color);
            }
        }
    }
}

/// Sample the 8×8 cell at `(gx, gy)` on a font page into a 1-bpp [`font`] bitmap: an opaque
/// (alpha-bit-set) device pixel becomes a set foreground bit (bit `0x80` = leftmost column),
/// transparent pixels clear. Off-page samples read transparent.
fn sample_glyph(page: &GrpPage, gx: usize, gy: usize) -> [u8; 8] {
    let mut rows = [0u8; CELL];
    for (dy, out) in rows.iter_mut().enumerate() {
        let y = gy + dy;
        if y >= GRP_DIM {
            break;
        }
        let base = y * GRP_DIM + gx;
        let mut bits = 0u8;
        for dx in 0..CELL {
            if gx + dx < GRP_DIM && page.pixels[base + dx] & 1 != 0 {
                bits |= 0x80 >> dx;
            }
        }
        *out = bits;
    }
    rows
}

/// Map a source dot `(sx,sy)` in an 8×8 cell to its destination after applying an `ATTR`
/// byte: rotation (bits 0–1, clockwise) then horizontal (bit 2) / vertical (bit 3) invert.
///
/// The bit meanings are documented + disassembled (`attr.yaml`); the exact rotation
/// *direction* and whether rotate-then-flip vs flip-then-rotate is oracle-pending (queued,
/// O-T6 composite capture) — we use clockwise rotation then flips, which round-trips the
/// `#TROT*`/`#TREVH`/`#TREVV` combinations consistently.
#[inline]
pub fn attr_map(sx: usize, sy: usize, attr: u8) -> (usize, usize) {
    let n = CELL - 1; // 7
    let (mut x, mut y) = match attr & 0b11 {
        0 => (sx, sy),         // 0°
        1 => (n - sy, sx),     // 90° CW
        2 => (n - sx, n - sy), // 180°
        _ => (sy, n - sx),     // 270° CW
    };
    if attr & 0b0100 != 0 {
        x = n - x; // #TREVH horizontal invert
    }
    if attr & 0b1000 != 0 {
        y = n - y; // #TREVV vertical invert
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_white_on_transparent() {
        let c = Console::top();
        assert_eq!((c.cols, c.rows), (50, 30));
        assert_eq!((c.fg, c.bg, c.attr), (15, 0, 0));
        assert_eq!(c.font_size, 8);
        assert_eq!((c.scroll_x, c.scroll_y), (0, 0));
        assert_eq!(c.cur_x, 0);
        assert_eq!(c.cur_y, 0);
    }

    #[test]
    fn bottom_console_matches_touch_screen_dimensions() {
        let c = Console::bottom();
        assert_eq!((c.cols, c.rows), (40, 30));
    }

    #[test]
    fn resize_changes_dimensions_and_clears_cells() {
        let mut c = Console::top();
        c.print_str("X");
        c.resize(40, 30);
        assert_eq!((c.cols, c.rows), (40, 30));
        assert_eq!(c.cell(0, 0).ch, 0);
        assert_eq!((c.cur_x, c.cur_y), (0, 0));
        // Color/attribute state survives the resize.
        assert_eq!((c.fg, c.bg, c.attr), (15, 0, 0));
    }

    #[test]
    fn print_advances_and_stores_cells() {
        let mut c = Console::top();
        c.print_str("HI");
        assert_eq!(c.cell(0, 0).ch, u16::from(b'H'));
        assert_eq!(c.cell(1, 0).ch, u16::from(b'I'));
        assert_eq!(c.cur_x, 2);
        assert_eq!(c.cur_y, 0);
    }

    #[test]
    fn wraps_at_right_edge() {
        let mut c = Console::new(4, 3);
        c.print_str("ABCDE"); // 5 chars into a 4-wide grid
        assert_eq!(c.cell(0, 0).ch, u16::from(b'A'));
        assert_eq!(c.cell(3, 0).ch, u16::from(b'D'));
        assert_eq!(c.cell(0, 1).ch, u16::from(b'E'));
        assert_eq!((c.cur_x, c.cur_y), (1, 1));
    }

    #[test]
    fn newline_then_scroll_on_last_row() {
        let mut c = Console::new(3, 2);
        c.print_str("AB");
        c.newline(); // -> row 1
        c.print_str("CD"); // C,D ; D wraps but grid only 3 wide -> stays row1 until full
        assert_eq!(c.cell(0, 1).ch, u16::from(b'C'));
        // A newline from the last row moves the cursor to the virtual off-screen row;
        // the grid does not scroll until a subsequent character is written past the bottom.
        c.newline();
        assert_eq!(c.cur_y, 2);
        assert_eq!(c.cell(0, 0).ch, u16::from(b'A')); // still intact
        assert_eq!(c.cell(0, 1).ch, u16::from(b'C'));
        // Printing one more character scrolls the grid up first.
        c.put_char(u16::from(b'E'));
        assert_eq!(c.cur_y, 1);
        assert_eq!(c.cell(0, 0).ch, u16::from(b'C'));
        assert_eq!(c.cell(1, 0).ch, u16::from(b'D'));
        assert_eq!(c.cell(0, 1).ch, u16::from(b'E'));
    }

    #[test]
    fn cls_clears_and_homes_but_keeps_color() {
        let mut c = Console::top();
        c.color(3, 4);
        c.locate(10, 5);
        c.print_str("X");
        c.cls();
        assert_eq!(c.cell(10, 5).ch, 0);
        assert_eq!((c.cur_x, c.cur_y), (0, 0));
        assert_eq!((c.fg, c.bg), (3, 4)); // CLS does not reset COLOR
    }

    #[test]
    fn attr_map_identity_and_rotations() {
        assert_eq!(attr_map(0, 0, 0), (0, 0));
        assert_eq!(attr_map(7, 0, 0), (7, 0));
        // 180° sends a corner to the opposite corner.
        assert_eq!(attr_map(0, 0, 2), (7, 7));
        // Horizontal invert mirrors X.
        assert_eq!(attr_map(0, 3, 4), (7, 3));
        // Vertical invert mirrors Y.
        assert_eq!(attr_map(2, 0, 8), (2, 7));
        // 90° CW maps top-left (0,0) -> top-right (7,0).
        assert_eq!(attr_map(0, 0, 1), (7, 0));
    }

    #[test]
    fn render_paints_white_glyph_on_black_backdrop() {
        let mut c = Console::top();
        c.print_str("E");
        let mut fb = Framebuffer::top();
        fb.clear(0xFF00_0000); // opaque black backdrop
        c.render(&mut fb);
        // 'E' top row [0x7E] => columns 1..7 set (bit pattern 0111_1110).
        assert_eq!(fb.get_argb(0, 0), 0xFF00_0000); // bg dot (transparent fg) -> backdrop
        assert_eq!(fb.get_argb(1, 0), 0xFFF8_F8F8); // first fg dot -> white
        assert_eq!(fb.get_argb(7, 0), 0xFF00_0000); // last column unset
    }
}
