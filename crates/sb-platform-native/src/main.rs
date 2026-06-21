//! Native runner (`sb`). In M1 this grows a `winit` window that blits the
//! [`sb_render::Framebuffer`] and feeds input into the VM. For now it's a smoke-test
//! entry point that proves the workspace links end to end.

fn main() {
    let fb = sb_render::Framebuffer::top();
    println!(
        "sb-interpreter (SmileBASIC 3.6.0) v{} — workspace OK. top screen: {}x{}",
        env!("CARGO_PKG_VERSION"),
        fb.width,
        fb.height,
    );
}
