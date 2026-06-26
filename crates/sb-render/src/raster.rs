//! GRP drawing primitives (M2-T2) — deterministic integer rasterizers that write the
//! manipulation page of a [`GrpState`](crate::grp::GrpState).
//!
//! These back the SmileBASIC graphics statements `GPSET`, `GLINE`, `GBOX`, `GFILL`,
//! `GCIRCLE`, `GTRI` and `GPAINT`. Every primitive writes the **manipulation page**
//! (`GPAGE`'s draw page), in the device's 16-bit **RGBA5551** format (the same 8→5 channel
//! truncation [`GCLS`](crate::grp::GrpState::gcls) uses — see
//! [`argb8888_to_rgba5551`](crate::grp::argb8888_to_rgba5551)), and is bounded to the write
//! **clip** rectangle intersected with the 512×512 page. There is no anti-aliasing — SB
//! draws hard integer pixel coverage, which is what the eventual pixel-diff golden gate
//! (O-T6 / M2-T5) compares against.
//!
//! ## What is verified vs. queued
//!
//! The **call shape** of each statement (argument counts, the default-color path, the
//! errnum-4 guards) is `hw_verified` (sb-oracle s_t7b/s_t7c) and exercised by the spec's
//! inline tests + the graphics builtins. The **exact pixel coverage** of each shape
//! (line endpoint inclusivity, the circle/arc midpoint rule, the paint boundary test,
//! triangle edge fill) is a draw-helper detail the disassembly leaves to the framebuffer
//! oracle, so the algorithms here are faithful-but-unverified at the pixel level and are
//! queued for the O-T6 golden harvest (`bd:sb-interpreter-7td`). The unit tests assert only the
//! coverage these algorithms are *defined* to produce (a plotted pixel lands, a box is an
//! outline, a fill is solid, clipping holds), not parity with hardware sub-pixel rules.

use crate::grp::{argb8888_to_rgba5551, ClipRect, GrpState, GRP_DIM};

/// The effective inclusive draw bounds: the write-clip rectangle intersected with the
/// 512×512 page. Returns `None` when the intersection is empty (nothing is drawable).
fn draw_bounds(clip: &ClipRect) -> Option<(i32, i32, i32, i32)> {
    let max = GRP_DIM as i32 - 1;
    let x0 = clip.x0.max(0);
    let y0 = clip.y0.max(0);
    let x1 = clip.x1.min(max);
    let y1 = clip.y1.min(max);
    if x0 > x1 || y0 > y1 {
        None
    } else {
        Some((x0, y0, x1, y1))
    }
}

impl GrpState {
    /// Plot one device pixel (halfword `h`) at `(x, y)` on the manipulation page, if it lies
    /// within the write clip ∩ page. Out-of-bounds plots are silently dropped (the device
    /// clips rather than erroring).
    pub(crate) fn plot_dev(&mut self, x: i32, y: i32, h: u16) {
        let Some((x0, y0, x1, y1)) = draw_bounds(&self.cur().write_clip) else {
            return;
        };
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return;
        }
        let page = self.cur().manip_page as usize;
        self.pages[page].pixels[y as usize * GRP_DIM + x as usize] = h;
    }

    /// `GPSET x,y[,color]` — plot a single pixel in the (truncated) `color`.
    pub fn gpset(&mut self, x: i32, y: i32, color: u32) {
        let h = argb8888_to_rgba5551(color);
        self.plot_dev(x, y, h);
    }

    /// `GLINE x1,y1,x2,y2[,color]` — draw a straight line, endpoints inclusive, via integer
    /// Bresenham. A degenerate line (start == end) plots the single endpoint.
    pub fn gline(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
        let h = argb8888_to_rgba5551(color);
        self.line_dev(x1, y1, x2, y2, h);
    }

    /// Bresenham line in device space (used by `GLINE`, `GBOX`, and the sector radii).
    fn line_dev(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, h: u16) {
        // i64 deltas: extreme i32 endpoints (a degenerate GTRI line, or GLINE/GBOX with far
        // coords) make `x2 - x1` overflow i32 — only the err accumulation needs the width.
        let dx = (x2 as i64 - x1 as i64).abs();
        let dy = -(y2 as i64 - y1 as i64).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x1, y1);
        loop {
            self.plot_dev(x, y, h);
            if x == x2 && y == y2 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// `GBOX x1,y1,x2,y2[,color]` — draw the rectangle OUTLINE (four edges only) spanned by
    /// the two corners. Corners may be given in any order.
    pub fn gbox(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
        let h = argb8888_to_rgba5551(color);
        let (xa, xb) = (x1.min(x2), x1.max(x2));
        let (ya, yb) = (y1.min(y2), y1.max(y2));
        for x in xa..=xb {
            self.plot_dev(x, ya, h);
            self.plot_dev(x, yb, h);
        }
        for y in ya..=yb {
            self.plot_dev(xa, y, h);
            self.plot_dev(xb, y, h);
        }
    }

    /// `GFILL x1,y1,x2,y2[,color]` — fill the SOLID rectangle spanned by the two corners
    /// (inclusive). Corners may be given in any order.
    pub fn gfill(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
        let h = argb8888_to_rgba5551(color);
        let (xa, xb) = (x1.min(x2), x1.max(x2));
        let (ya, yb) = (y1.min(y2), y1.max(y2));
        for y in ya..=yb {
            for x in xa..=xb {
                self.plot_dev(x, y, h);
            }
        }
    }

    /// `GCIRCLE x,y,r[,color]` — draw a full circle outline (midpoint algorithm). `r <= 0`
    /// draws nothing.
    pub fn gcircle(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        if r <= 0 {
            return;
        }
        let h = argb8888_to_rgba5551(color);
        // Midpoint (Bresenham) circle: walk one octant, mirror to the other seven.
        let mut x = 0i32;
        let mut y = r;
        let mut d = 1 - r;
        while x <= y {
            self.plot_dev(cx + x, cy + y, h);
            self.plot_dev(cx - x, cy + y, h);
            self.plot_dev(cx + x, cy - y, h);
            self.plot_dev(cx - x, cy - y, h);
            self.plot_dev(cx + y, cy + x, h);
            self.plot_dev(cx - y, cy + x, h);
            self.plot_dev(cx + y, cy - x, h);
            self.plot_dev(cx - y, cy - x, h);
            if d < 0 {
                d += 2 * x + 3;
            } else {
                d += 2 * (x - y) + 5;
                y -= 1;
            }
            x += 1;
        }
    }

    /// `GCIRCLE x,y,r,start,end[,flag[,color]]` — draw an arc (`flag == 0`, open) or sector
    /// (`flag == 1`, pie slice with the two bounding radii) between `start` and `end`
    /// degrees. `r <= 0` draws nothing.
    ///
    /// The arc is plotted by stepping degrees and placing the circle point at each angle.
    /// The exact angle convention (where 0° points, the sweep direction, normalization of
    /// negative / >360 / end<start spans) is a draw-helper detail left to the framebuffer
    /// oracle (queued). This implementation takes 0° at +X, sweeps toward +Y, normalizes the
    /// span to `end >= start`, and steps one degree at a time — deterministic, but its
    /// sub-pixel parity with hardware is unverified.
    #[allow(clippy::too_many_arguments)]
    pub fn gcircle_arc(
        &mut self,
        cx: i32,
        cy: i32,
        r: i32,
        start: i32,
        end: i32,
        sector: bool,
        color: u32,
    ) {
        if r <= 0 {
            return;
        }
        let h = argb8888_to_rgba5551(color);
        let (a0, mut a1) = (start, end);
        if a1 < a0 {
            a1 += 360;
        }
        let point = |deg: i32| -> (i32, i32) {
            let rad = (deg as f64).to_radians();
            let px = cx + (r as f64 * rad.cos()).round() as i32;
            let py = cy + (r as f64 * rad.sin()).round() as i32;
            (px, py)
        };
        let mut deg = a0;
        while deg <= a1 {
            let (px, py) = point(deg);
            self.plot_dev(px, py, h);
            deg += 1;
        }
        if sector {
            // Pie slice: connect the centre to both span endpoints.
            let (sx, sy) = point(a0);
            let (ex, ey) = point(a1);
            self.line_dev(cx, cy, sx, sy, h);
            self.line_dev(cx, cy, ex, ey, h);
        }
    }

    /// `GTRI x1,y1,x2,y2,x3,y3[,color]` — draw a FILLED triangle, using integer edge
    /// functions over the triangle's bounding box (a pixel is filled when it is on the same
    /// side of all three edges; edges are inclusive — winding-independent). A degenerate
    /// (zero-area / collinear) triangle still draws the spanning line between its two most
    /// distant vertices (a single pixel when all three coincide) — hw_verified M7-T2: the
    /// handler's degenerate branch (@0x1942ec) routes to the line helpers (0x155f8c/0x15cab0),
    /// so e.g. `GTRI 10,200,20,200,30,200` lights the row 10..30 @ y=200, not nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn gtri(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, color: u32) {
        let h = argb8888_to_rgba5551(color);
        // Edge functions are products of coordinate differences. With i32 vertices at the far
        // extremes (a fuzzer-found case, M7-T1) a difference reaches ~2^32 and its product
        // ~2^64 — past i64 — so the geometry is computed in i128. Only the sign/zero matters,
        // and i128 holds it exactly for every i32 input; results are identical in range.
        let (ax, ay, bx, by, cx, cy) = (
            x1 as i128, y1 as i128, x2 as i128, y2 as i128, x3 as i128, y3 as i128,
        );
        // Signed area * 2; zero => degenerate (collinear) => draw the spanning line.
        let area = (bx - ax) * (cy - ay) - (cx - ax) * (by - ay);
        if area == 0 {
            // Collinear: draw the segment between the two farthest-apart vertices (it covers
            // the middle one too). For coincident vertices the distance is 0 -> one pixel.
            let v = [(x1, y1), (x2, y2), (x3, y3)];
            let dist2 = |p: (i32, i32), q: (i32, i32)| {
                let (dx, dy) = (p.0 as i128 - q.0 as i128, p.1 as i128 - q.1 as i128);
                dx * dx + dy * dy
            };
            let (i, j) = [(0usize, 1usize), (0, 2), (1, 2)]
                .into_iter()
                .max_by_key(|&(a, b)| dist2(v[a], v[b]))
                .unwrap();
            self.line_dev(v[i].0, v[i].1, v[j].0, v[j].1, h);
            return;
        }
        let (x1, y1, x2, y2, x3, y3) = (ax, ay, bx, by, cx, cy);
        let (minx, maxx) = (x1.min(x2).min(x3), x1.max(x2).max(x3));
        let (miny, maxy) = (y1.min(y2).min(y3), y1.max(y2).max(y3));
        // Clamp the scan box to the drawable region so a huge off-screen triangle is cheap.
        let Some((bx0, by0, bx1, by1)) = draw_bounds(&self.cur().write_clip) else {
            return;
        };
        let (minx, maxx) = (minx.max(bx0 as i128), maxx.min(bx1 as i128));
        let (miny, maxy) = (miny.max(by0 as i128), maxy.min(by1 as i128));
        let edge = |ax: i128, ay: i128, bx: i128, by: i128, px: i128, py: i128| {
            (bx - ax) * (py - ay) - (by - ay) * (px - ax)
        };
        for py in miny..=maxy {
            for px in minx..=maxx {
                let w0 = edge(x2, y2, x3, y3, px, py);
                let w1 = edge(x3, y3, x1, y1, px, py);
                let w2 = edge(x1, y1, x2, y2, px, py);
                let inside = (w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0);
                if inside {
                    // px/py are clamped to the (small) draw bounds, so they fit i32.
                    self.plot_dev(px as i32, py as i32, h);
                }
            }
        }
    }

    /// `GPAINT x,y,fill[,border]` — 4-connected flood fill from `(x, y)`.
    ///
    /// With `border` given, the fill spreads over connected pixels whose color is **not**
    /// the border color. With `border` omitted, the boundary is implicit: the contiguous run
    /// of the color sampled at the start point is replaced. Bounded to the write clip ∩ page.
    pub fn gpaint(&mut self, x: i32, y: i32, fill: u32, border: Option<u32>) {
        let Some((x0, y0, x1, y1)) = draw_bounds(&self.cur().write_clip) else {
            return;
        };
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return;
        }
        let fill_h = argb8888_to_rgba5551(fill);
        let page = self.cur().manip_page as usize;
        let idx = |px: i32, py: i32| py as usize * GRP_DIM + px as usize;
        let seed = self.pages[page].pixels[idx(x, y)];

        // Decide what a paintable pixel is. Border form: anything != border. Implicit form:
        // pixels matching the seed color. Either way, if the seed pixel is already not
        // paintable (or painting it would be a no-op), there is nothing to do.
        let border_h = border.map(argb8888_to_rgba5551);
        let paintable = |px: u16| match border_h {
            Some(b) => px != b,
            None => px == seed,
        };
        if !paintable(seed) || self.pages[page].pixels[idx(x, y)] == fill_h {
            return;
        }

        let pixels = &mut self.pages[page].pixels;
        let mut stack = vec![(x, y)];
        while let Some((px, py)) = stack.pop() {
            if px < x0 || px > x1 || py < y0 || py > y1 {
                continue;
            }
            let i = idx(px, py);
            if pixels[i] == fill_h || !paintable(pixels[i]) {
                continue;
            }
            pixels[i] = fill_h;
            stack.push((px - 1, py));
            stack.push((px + 1, py));
            stack.push((px, py - 1));
            stack.push((px, py + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grp::rgba5551_to_argb8888;

    const RED: u32 = 0xFFFF_0000;
    const WHITE: u32 = 0xFFFF_FFFF;

    fn read(g: &GrpState, x: i32, y: i32) -> u32 {
        g.gspoit(x, y)
    }

    #[test]
    fn gpset_plots_one_truncated_pixel() {
        let mut g = GrpState::new();
        g.gpset(0, 0, RED);
        // R 255 -> top 5 bits -> 248 on read-back (the documented RGBA5551 truncation).
        assert_eq!(read(&g, 0, 0), 0xFFF8_0000);
        // Neighbours untouched.
        assert_eq!(read(&g, 1, 0), 0);
    }

    #[test]
    fn gpset_off_page_and_clip_are_dropped() {
        let mut g = GrpState::new();
        g.gpset(-1, -1, WHITE);
        g.gpset(512, 0, WHITE);
        assert_eq!(read(&g, 0, 0), 0);
        // A write clip rejects outside plots without erroring.
        g.gclip_rect(true, 10, 10, 20, 20);
        g.gpset(0, 0, WHITE);
        assert_eq!(read(&g, 0, 0), 0);
        g.gpset(15, 15, WHITE);
        assert_eq!(
            read(&g, 15, 15),
            rgba5551_to_argb8888(argb8888_to_rgba5551(WHITE))
        );
    }

    #[test]
    fn gline_horizontal_vertical_and_diagonal() {
        let mut g = GrpState::new();
        g.gline(0, 0, 3, 0, WHITE); // horizontal, inclusive endpoints
        for x in 0..=3 {
            assert_ne!(read(&g, x, 0), 0, "h pixel {x}");
        }
        assert_eq!(read(&g, 4, 0), 0);

        let mut g = GrpState::new();
        g.gline(2, 2, 2, 2, WHITE); // degenerate -> single point
        assert_ne!(read(&g, 2, 2), 0);

        let mut g = GrpState::new();
        g.gline(0, 0, 3, 3, WHITE); // 45-degree diagonal hits the main diagonal
        for d in 0..=3 {
            assert_ne!(read(&g, d, d), 0, "diag {d}");
        }
    }

    #[test]
    fn gbox_is_outline_only() {
        let mut g = GrpState::new();
        g.gbox(0, 0, 4, 4, WHITE);
        // Corners + edges set.
        assert_ne!(read(&g, 0, 0), 0);
        assert_ne!(read(&g, 4, 4), 0);
        assert_ne!(read(&g, 2, 0), 0); // top edge
        assert_ne!(read(&g, 0, 2), 0); // left edge
                                       // Interior is empty.
        assert_eq!(read(&g, 2, 2), 0);
    }

    #[test]
    fn gfill_is_solid_inclusive() {
        let mut g = GrpState::new();
        g.gfill(1, 1, 3, 3, WHITE);
        for y in 1..=3 {
            for x in 1..=3 {
                assert_ne!(read(&g, x, y), 0, "fill {x},{y}");
            }
        }
        assert_eq!(read(&g, 0, 0), 0);
        assert_eq!(read(&g, 4, 4), 0);
        // Reversed corners fill the same span.
        let mut g2 = GrpState::new();
        g2.gfill(3, 3, 1, 1, WHITE);
        assert_eq!(g, g2);
    }

    #[test]
    fn gcircle_radius_zero_or_negative_is_noop() {
        let mut g = GrpState::new();
        g.gcircle(50, 50, 0, WHITE);
        g.gcircle(50, 50, -5, WHITE);
        assert_eq!(g.pages[0].pixels.iter().filter(|&&p| p != 0).count(), 0);
    }

    #[test]
    fn gcircle_plots_cardinal_points_and_empty_centre() {
        let mut g = GrpState::new();
        g.gcircle(50, 50, 10, WHITE);
        assert_ne!(read(&g, 60, 50), 0); // +x
        assert_ne!(read(&g, 40, 50), 0); // -x
        assert_ne!(read(&g, 50, 60), 0); // +y
        assert_ne!(read(&g, 50, 40), 0); // -y
        assert_eq!(read(&g, 50, 50), 0); // outline, not filled
    }

    #[test]
    fn gtri_fills_interior_and_draws_degenerate_line() {
        let mut g = GrpState::new();
        g.gtri(0, 0, 10, 0, 0, 10, WHITE);
        assert_ne!(read(&g, 1, 1), 0); // clearly inside
        assert_ne!(read(&g, 0, 0), 0); // vertex
        assert_eq!(read(&g, 9, 9), 0); // outside the hypotenuse
                                       // Degenerate (collinear) -> draws the spanning line, not
                                       // nothing (hw_verified M7-T2): the segment (0,0)-(10,10)
                                       // covers the middle vertex; off-line pixels stay clear.
        let mut d = GrpState::new();
        d.gtri(0, 0, 5, 5, 10, 10, WHITE);
        assert_ne!(read(&d, 0, 0), 0); // endpoint
        assert_ne!(read(&d, 5, 5), 0); // middle vertex on the line
        assert_ne!(read(&d, 10, 10), 0); // far endpoint
        assert_eq!(read(&d, 5, 6), 0); // off the line
                                       // All three vertices coincident -> a single pixel.
        let mut p = GrpState::new();
        p.gtri(7, 7, 7, 7, 7, 7, WHITE);
        assert_ne!(read(&p, 7, 7), 0);
        assert_eq!(p.pages[0].pixels.iter().filter(|&&px| px != 0).count(), 1);
    }

    #[test]
    fn gtri_extreme_coords_do_not_overflow() {
        // Far-apart i32 vertices overflowed the i32 edge-function products (M7-T1 fuzz find).
        // The geometry is now i64; this must just not panic (and still fills the on-page part).
        let mut g = GrpState::new();
        g.gtri(0, 0, 2_000_000_000, 0, 0, 2_000_000_000, WHITE);
        assert_ne!(read(&g, 1, 1), 0); // the on-screen corner is covered by the huge triangle
        let mut e = GrpState::new();
        e.gtri(i32::MIN, i32::MIN, i32::MAX, 0, 0, i32::MAX, WHITE); // no panic
    }

    #[test]
    fn gpaint_implicit_fills_contiguous_seed_region() {
        let mut g = GrpState::new();
        // A 5x5 white box outline; paint its black interior red.
        g.gbox(0, 0, 4, 4, WHITE);
        g.gpaint(2, 2, RED, None);
        assert_eq!(read(&g, 2, 2), 0xFFF8_0000); // interior painted
        assert_ne!(read(&g, 0, 0), 0xFFF8_0000); // outline kept (white, not red)
                                                 // Outside the box stays untouched (the seed run is bounded by the white outline).
        assert_eq!(read(&g, 4, 5), 0);
    }

    #[test]
    fn gpaint_border_form_spreads_until_border() {
        let mut g = GrpState::new();
        g.gbox(0, 0, 4, 4, WHITE);
        // Fill spreads over everything that is not the white border, from the interior.
        g.gpaint(2, 2, RED, Some(WHITE));
        assert_eq!(read(&g, 2, 2), 0xFFF8_0000);
        // The white outline is the border, so it is not overwritten.
        assert_ne!(read(&g, 0, 0), 0xFFF8_0000);
    }

    #[test]
    fn gpaint_seed_equals_fill_is_noop() {
        let mut g = GrpState::new();
        g.gfill(0, 0, 3, 3, RED);
        let before = g.clone();
        g.gpaint(1, 1, RED, None); // seed already (truncated) red -> nothing changes
        assert_eq!(g, before);
    }
}
