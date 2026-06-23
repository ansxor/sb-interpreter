//! GRP bitmap ops (M2-T3) — page↔page blits (`GCOPY`) and page↔array transfers
//! (`GSAVE`/`GLOAD`), as methods on [`GrpState`](crate::grp::GrpState).
//!
//! These move whole rectangles of device pixels rather than rasterizing shapes. A blit
//! reads a source region into a halfword snapshot (so a same-page overlapping copy is
//! safe) and writes it into the **manipulation page** through the same write-clip ∩ page
//! bound the drawing primitives use. The transfer respects SmileBASIC's **copy mode**: a
//! TRUE mode copies even transparent source pixels (overwriting the destination), a FALSE
//! mode skips transparent source pixels (the destination shows through) — a transparent
//! device pixel is one whose RGBA5551 alpha bit is clear. This is hw_verified (sb-oracle
//! s_t7d 2026-06-23): a transparent-source `GCOPY … ,0` keeps the destination, `… ,1`
//! overwrites it with transparent black.
//!
//! `GSAVE`/`GLOAD` carry pixels through a numeric array in one of two element formats,
//! selected by the convert flag: flag **1** stores the raw 16-bit RGBA5551 physical code,
//! flag **0** stores the expanded 32-bit logical ARGB color (the same value
//! [`GSPOIT`](crate::grp::GrpState::gspoit) returns). Marshalling between an array element
//! and a 32-bit word is the caller's job (it depends on the array's Int/Real type); this
//! module deals only in device halfwords and the logical-color conversion.
//!
//! GRPF (source page `-1`) is not backed in this model, so reading it yields transparent
//! pixels (the captured-screen content is a fidelity gap queued for the framebuffer oracle,
//! O-T6).

use crate::grp::{argb8888_to_rgba5551, rgba5551_to_argb8888, GrpState, GRP_DIM};

impl GrpState {
    /// The whole current drawing area as `(x, y, w, h)` — the write-clip rectangle
    /// intersected with the 512×512 page. Backs the `GSAVE`/`GLOAD` forms that omit an
    /// explicit rectangle (the default write clip is the full page → 512×512 = 262144
    /// elements, hw_verified sb-oracle s_t7d `LEN` of a whole-area `GSAVE`).
    pub fn whole_draw_area(&self) -> (i32, i32, i32, i32) {
        let max = GRP_DIM as i32 - 1;
        let x0 = self.write_clip.x0.clamp(0, max);
        let y0 = self.write_clip.y0.clamp(0, max);
        let x1 = self.write_clip.x1.clamp(0, max);
        let y1 = self.write_clip.y1.clamp(0, max);
        (x0, y0, (x1 - x0 + 1).max(0), (y1 - y0 + 1).max(0))
    }

    /// Read a `w`×`h` region of `page` (row-major, top-left at `(x, y)`) into RGBA5551
    /// halfwords. `page == -1` (GRPF) or any pixel outside the 512×512 page reads as 0
    /// (transparent black).
    pub fn read_region(&self, page: i32, x: i32, y: i32, w: i32, h: i32) -> Vec<u16> {
        let mut out = Vec::with_capacity((w.max(0) * h.max(0)) as usize);
        let src = if (0..self.pages.len() as i32).contains(&page) {
            Some(&self.pages[page as usize].pixels)
        } else {
            None // GRPF / out-of-set page: read as transparent
        };
        for j in 0..h {
            for i in 0..w {
                let (px, py) = (x + i, y + j);
                let h = match src {
                    Some(pix)
                        if (0..GRP_DIM as i32).contains(&px)
                            && (0..GRP_DIM as i32).contains(&py) =>
                    {
                        pix[py as usize * GRP_DIM + px as usize]
                    }
                    _ => 0,
                };
                out.push(h);
            }
        }
        out
    }

    /// Write a `w`×`h` halfword region (row-major) into the manipulation page with its
    /// top-left at `(x, y)`, honoring the write clip ∩ page. When `copy_transparent` is
    /// false, source pixels whose alpha bit is clear are skipped (the destination shows
    /// through); when true, every source pixel is written.
    pub fn write_region(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        data: &[u16],
        copy_transparent: bool,
    ) {
        for j in 0..h {
            for i in 0..w {
                let Some(&hw) = data.get((j * w + i) as usize) else {
                    return;
                };
                if !copy_transparent && hw & 1 == 0 {
                    continue;
                }
                self.plot_dev(x + i, y + j, hw);
            }
        }
    }

    /// `GCOPY [src_page,] x1,y1,x2,y2, dx,dy, copy_transparent` — blit the source rectangle
    /// (spanned by the two corners, given in any order) from `src_page` (or `-1` GRPF) onto
    /// the manipulation page with its top-left at `(dx, dy)`. The source is snapshotted
    /// first, so a same-page overlapping copy is well-defined.
    #[allow(clippy::too_many_arguments)]
    pub fn gcopy(
        &mut self,
        src_page: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        dx: i32,
        dy: i32,
        copy_transparent: bool,
    ) {
        let (sx, sy) = (x1.min(x2), y1.min(y2));
        let w = (x1 - x2).abs() + 1;
        let h = (y1 - y2).abs() + 1;
        let region = self.read_region(src_page, sx, sy, w, h);
        self.write_region(dx, dy, w, h, &region, copy_transparent);
    }

    /// Convert a read-back RGBA5551 halfword to the GSAVE element word for `convert_flag`:
    /// flag 1 keeps the raw 16-bit physical code, flag 0 expands to the 32-bit logical ARGB
    /// color (`GSPOIT`'s value).
    pub fn gsave_word(halfword: u16, raw: bool) -> u32 {
        if raw {
            halfword as u32
        } else {
            rgba5551_to_argb8888(halfword)
        }
    }

    /// Convert a GLOAD element word back to a device halfword for `convert_flag`: flag 1
    /// uses the low 16 bits as the raw physical code, flag 0 treats the word as a 32-bit
    /// logical ARGB color and truncates it to RGBA5551 (the inverse of [`gsave_word`]).
    pub fn gload_halfword(word: u32, raw: bool) -> u16 {
        if raw {
            word as u16
        } else {
            argb8888_to_rgba5551(word)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grp::rgba5551_to_argb8888;

    const RED: u32 = 0xFFFF_0000;
    const BLUE: u32 = 0xFF00_00FF;
    const GREEN: u32 = 0xFF00_FF00;

    #[test]
    fn gcopy_moves_a_pixel_within_a_page() {
        let mut g = GrpState::new();
        g.gpset(10, 10, RED);
        // Copy the (0,0)-(32,32) block to (100,100); (10,10) -> (110,110).
        g.gcopy(0, 0, 0, 32, 32, 100, 100, true);
        assert_eq!(g.gspoit(110, 110), 0xFFF8_0000); // red, truncated
        assert_eq!(g.gspoit(100, 100), 0); // was blank in the source
    }

    #[test]
    fn gcopy_copy_mode_governs_transparent_pixels() {
        // FALSE skips transparent source pixels -> destination kept (hw_verified s_t7d).
        let mut g = GrpState::new();
        g.gfill(100, 100, 131, 131, BLUE);
        g.gcopy(0, 0, 0, 31, 31, 100, 100, false); // source (0,0)-(31,31) is transparent
        assert_eq!(
            g.gspoit(110, 110),
            rgba5551_to_argb8888(argb8888_to_rgba5551(BLUE))
        );
        // TRUE copies transparent -> destination overwritten with transparent black.
        let mut g = GrpState::new();
        g.gfill(100, 100, 131, 131, BLUE);
        g.gcopy(0, 0, 0, 31, 31, 100, 100, true);
        assert_eq!(g.gspoit(110, 110), 0);
    }

    #[test]
    fn gsave_gload_round_trip_both_flags() {
        for raw in [true, false] {
            let mut g = GrpState::new();
            g.gpset(5, 5, GREEN);
            let region = g.read_region(0, 0, 0, 16, 16);
            let words: Vec<u32> = region
                .iter()
                .map(|&hw| GrpState::gsave_word(hw, raw))
                .collect();
            // Clear, then load the saved region somewhere else.
            g.gcls(0);
            let halfwords: Vec<u16> = words
                .iter()
                .map(|&w| GrpState::gload_halfword(w, raw))
                .collect();
            g.write_region(100, 100, 16, 16, &halfwords, true);
            assert_eq!(g.gspoit(105, 105), 0xFF00_F800, "raw={raw}"); // green, truncated
        }
    }

    #[test]
    fn gsave_word_formats_match_oracle() {
        // RGB(255,0,0) truncated to device -> 0xF801 raw; 0xFFF80000 logical (hw_verified).
        let hw = argb8888_to_rgba5551(RED);
        assert_eq!(GrpState::gsave_word(hw, true), 0xF801);
        assert_eq!(GrpState::gsave_word(hw, false), 0xFFF8_0000);
    }

    #[test]
    fn whole_draw_area_is_full_page_by_default() {
        let g = GrpState::new();
        assert_eq!(g.whole_draw_area(), (0, 0, 512, 512));
    }
}
