//! M7-T5 — dedicated VM throughput / fps benchmark harness.
//!
//! Acceptance criterion for M7-T5 is "benchmark a heavy program at acceptable fps
//! native + wasm". This is the *dedicated* harness for that (run 1 left only an
//! inline throughput probe). It compiles each workload once, then times repeated
//! `Vm::run()` passes (min-of-samples to reject scheduler noise) and reports:
//!
//!   * wall time per run,
//!   * logical iters/sec (each workload declares its iteration count),
//!   * effective VM-ops/sec (rough; iters × ops-per-iter), and
//!   * a frame budget verdict: a SmileBASIC game targets 60 fps (a 16.67 ms frame),
//!     so for the `frame-sim` workload we report how many such frames/sec the core
//!     sustains — it must clear 60 to be playable.
//!
//! Plain `harness = false` `main()` so it runs on **stable** Rust (no criterion,
//! no nightly `#[bench]`) — `cargo bench -p sb-core`. `std::time::Instant` keeps
//! this native-only; it is a dev/bench target, never built for the wasm32 lib, and
//! since `sb-core` is platform-free (no I/O / GUI / threads) the *same* dispatch
//! runs under wasm — see the module note in `lib.rs`. Determinism is guarded
//! separately by `vm::tests::heavy_program_is_deterministic`; this file only
//! measures speed, never the conformance gate.

use std::hint::black_box;
use std::time::{Duration, Instant};

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::parser::parse;
use sb_core::vm::Vm;

/// One benchmark workload: a SmileBASIC program plus the count of logical
/// iterations it performs (for the iters/sec figure) and an approximate
/// ops-per-iteration weight (for the rough VM-ops/sec figure).
struct Workload {
    name: &'static str,
    src: &'static str,
    iters: u64,
    ops_per_iter: u64,
    /// `Some(frames)` marks a per-frame game-logic workload: the figure of merit
    /// is frames/sec (= `frames / elapsed`), which must clear 60.
    frames: Option<u64>,
}

fn workloads() -> Vec<Workload> {
    vec![
        // Promoting (Number-typed) arithmetic hot path: + / * (with i64-then-narrow
        // promotion check) and MOD, the M7-T5 overflow paths, 1M times.
        Workload {
            name: "arith-loop (Number +,*,MOD)",
            src: "S=0\nFOR I=1 TO 1000000\nS=(S+I*3) MOD 1000000\nNEXT",
            iters: 1_000_000,
            ops_per_iter: 8,
            frames: None,
        },
        // Pure i32 path: `%`-typed wrapping + / * with a bitmask, no promotion.
        Workload {
            name: "int-loop (i32 +,*,AND)",
            src: "S%=0\nFOR I%=1 TO 1000000\nS%=(S%+I%*3) AND 1048575\nNEXT",
            iters: 1_000_000,
            ops_per_iter: 8,
            frames: None,
        },
        // Control-flow heavy: WHILE + IF branch + counter, exercising the dispatch
        // and jump machinery rather than arithmetic.
        Workload {
            name: "control-flow (WHILE/IF)",
            src: "N=0\nC=0\nWHILE N<1000000\nIF N MOD 2==0 THEN C=C+1\nN=N+1\nWEND",
            iters: 1_000_000,
            ops_per_iter: 7,
            frames: None,
        },
        // Array read-modify-write hot path: 1000 elements × 1000 passes.
        Workload {
            name: "array rmw (DIM[1000]x1000)",
            src: "DIM A[1000]\nFOR P=1 TO 1000\nFOR I=0 TO 999\nA[I]=A[I]+I\nNEXT\nNEXT",
            iters: 1_000_000,
            ops_per_iter: 6,
            frames: None,
        },
        // String churn: STR$ + concatenation, allocator-bound, 100k times.
        Workload {
            name: "string churn (STR$+&)",
            src: "FOR I=1 TO 100000\nA$=STR$(I)+\"x\"\nNEXT",
            iters: 100_000,
            ops_per_iter: 4,
            frames: None,
        },
        // Frame simulation: 256 moving objects with bounce, the kind of per-frame
        // game logic SmileBASIC programs run at 60 fps. 5000 frames → fps verdict.
        Workload {
            name: "frame-sim (256 objects)",
            src: concat!(
                "DIM X[256],Y[256],VX[256],VY[256]\n",
                "FOR I=0 TO 255\nVX[I]=(I MOD 5)+1\nVY[I]=(I MOD 3)+1\nNEXT\n",
                "FOR F=1 TO 5000\n",
                "FOR I=0 TO 255\n",
                "X[I]=X[I]+VX[I]\nY[I]=Y[I]+VY[I]\n",
                "IF X[I]>400 THEN VX[I]=-VX[I]\n",
                "IF Y[I]>240 THEN VY[I]=-VY[I]\n",
                "NEXT\nNEXT"
            ),
            iters: 5000 * 256,
            ops_per_iter: 12,
            frames: Some(5000),
        },
    ]
}

/// Time `f` `samples` times and return the *minimum* elapsed (the run least
/// disturbed by the OS scheduler — the standard way to read a CPU-bound figure).
fn best_of<F: FnMut()>(samples: u32, mut f: F) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..samples {
        let t = Instant::now();
        f();
        let dt = t.elapsed();
        if dt < best {
            best = dt;
        }
    }
    best
}

fn run_workload(w: &Workload) {
    let ast = parse(w.src).expect("parse benchmark workload");
    let program = compile_with(&ast, &StdBuiltins).expect("compile benchmark workload");

    // One warmup pass (compile caches, branch predictor, allocator) outside timing.
    {
        let mut vm = Vm::new(program.clone());
        vm.run().expect("warmup run");
    }

    let elapsed = best_of(5, || {
        let mut vm = Vm::new(program.clone());
        let halt = vm.run().expect("benchmark run");
        black_box(halt);
    });

    let secs = elapsed.as_secs_f64();
    let iters_per_sec = w.iters as f64 / secs;
    let ops_per_sec = (w.iters * w.ops_per_iter) as f64 / secs;

    print!(
        "  {:<30}  {:>8.2} ms   {:>10.2} M iter/s   {:>10.2} M op/s",
        w.name,
        secs * 1e3,
        iters_per_sec / 1e6,
        ops_per_sec / 1e6,
    );
    if let Some(frames) = w.frames {
        let fps = frames as f64 / secs;
        let verdict = if fps >= 60.0 {
            "OK >=60fps"
        } else {
            "SLOW <60fps"
        };
        print!("   {:>9.0} fps  [{}]", fps, verdict);
    }
    println!();
}

fn main() {
    println!("sb-core VM throughput benchmark (M7-T5)");
    println!("  release build recommended: cargo bench -p sb-core");
    println!();
    for w in workloads() {
        run_workload(&w);
    }
    println!();
    println!("fps verdict assumes a 60 fps (16.67 ms) frame budget; the frame-sim");
    println!("workload models 256 moving objects of per-frame game logic.");
}
