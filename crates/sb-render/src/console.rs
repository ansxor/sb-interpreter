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
//! The real SB font glyphs are a firmware ROM asset (see [`crate::font`]); this renderer
//! takes whatever glyph table that module exposes, so swapping in a harvested font later
//! does not change the model.

use crate::font;
use crate::Framebuffer;

/// Cell size in dots (SB console font is 8×8 — osb `console.d` `fontDefWidth/Height`).
pub const CELL: usize = 8;

/// Top-screen console: 50 columns × 30 rows.
pub const TOP_COLS: usize = 50;
pub const TOP_ROWS: usize = 30;

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
        }
    }

    /// The standard top-screen console (50×30).
    pub fn top() -> Self {
        Self::new(TOP_COLS, TOP_ROWS)
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

    /// Write one character at the cursor using the current color/attribute, then advance.
    /// Wraps at the right edge; scrolls the grid up when advancing past the bottom row.
    pub fn put_char(&mut self, ch: u16) {
        // The cursor may sit on the off-screen right edge (`LOCATE 50` is a legal column
        // per locate.yaml: 0..49 displayable, 50 = off-screen edge). Wrap to the next row
        // before writing so the write stays in bounds.
        if self.cur_x >= self.cols {
            self.cur_x = 0;
            self.line_feed();
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

    /// Advance the cursor one cell, wrapping and scrolling as needed.
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

    /// Move to column 0 of the next row, scrolling the grid up if already on the last row.
    pub fn newline(&mut self) {
        self.cur_x = 0;
        self.line_feed();
    }

    fn line_feed(&mut self) {
        if self.cur_y + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cur_y += 1;
        }
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
        for row in 0..self.rows {
            for col in 0..self.cols {
                self.render_cell(fb, col, row);
            }
        }
    }

    fn render_cell(&self, fb: &mut Framebuffer, col: usize, row: usize) {
        let cell = self.cells[self.idx(col, row)];
        // ch == 0 is an empty cell (no character), not NUL — draw only the background.
        let glyph = if cell.ch == 0 {
            [0u8; 8]
        } else {
            font::glyph(char::from_u32(cell.ch as u32).unwrap_or('\u{FFFD}'))
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
        assert_eq!(c.cur_x, 0);
        assert_eq!(c.cur_y, 0);
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
        // Force a scroll: newline from the last row shifts everything up.
        c.newline();
        // Row 0 now holds what was row 1 ("CD"), row 1 cleared.
        assert_eq!(c.cell(0, 0).ch, u16::from(b'C'));
        assert_eq!(c.cell(1, 0).ch, u16::from(b'D'));
        assert_eq!(c.cell(0, 1).ch, 0);
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
