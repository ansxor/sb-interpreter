//! M1-T10 acceptance: rendering a known console grid produces a **deterministic**
//! framebuffer, and a committed golden PNG of a simple `PRINT` screen matches it exactly.
//!
//! This is the first golden fixture. The golden is the renderer's *own* output (the SB font
//! ROM is not harvested yet — see `crate::font`), so it proves determinism/self-consistency,
//! not a pixel match against the emulator (that is gated on the font harvest, O-T6).
//!
//! Regenerate the golden after an intentional render change with:
//!   `UPDATE_GOLDEN=1 cargo test -p sb-render --test console_golden`

use sb_render::console::Console;
use sb_render::{png, Framebuffer};
use std::path::PathBuf;

/// Build the canonical "PRINT screen": a few lines of text exercising colors + an attribute.
fn render_print_screen() -> Framebuffer {
    let mut con = Console::top();

    // Title line in default white.
    con.print_str("SMILEBASIC 3.6.0");
    con.newline();
    con.newline();

    // A greeting in cyan (#TCYAN = 13).
    con.color(13, 0);
    con.print_str("HELLO, WORLD!");
    con.newline();

    // A score line: label in yellow (7), number in white (15).
    con.color(7, 0);
    con.print_str("SCORE:");
    con.color(15, 0);
    con.print_str("12345");
    con.newline();

    // Exercise the ATTR rotation/inversion path: 180-degree rotated red text.
    con.color(3, 0);
    con.set_attr(2); // #TROT180
    con.print_str("ROTATED");
    con.set_attr(0);

    let mut fb = Framebuffer::top();
    fb.clear(0xFF00_0000); // opaque black backdrop (SB default background)
    con.render(&mut fb);
    fb
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/console_print.png")
}

#[test]
fn console_render_is_deterministic() {
    // The render must be a pure function of the program: two runs are bit-identical.
    assert_eq!(render_print_screen(), render_print_screen());
}

#[test]
fn console_print_golden_matches() {
    let fb = render_print_screen();
    let encoded = png::encode(&fb);
    let path = golden_path();

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &encoded).unwrap();
        return;
    }

    let golden = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "golden {} missing ({e}); regenerate with UPDATE_GOLDEN=1 cargo test -p sb-render",
            path.display()
        )
    });
    assert_eq!(
        encoded.len(),
        golden.len(),
        "rendered PNG size {} != golden size {} — render changed; regenerate with UPDATE_GOLDEN=1",
        encoded.len(),
        golden.len()
    );
    assert!(
        encoded == golden,
        "rendered console PNG differs from the committed golden; if intentional, regenerate \
         with UPDATE_GOLDEN=1 cargo test -p sb-render"
    );
}
