//! Bytecode VM (milestone M1).
//!
//! A stack machine modeling SmileBASIC's 4 program slots + shared `COMMON DEF`
//! functions. Design reference: `osb/SMILEBASIC/VM.d` (`Code` opcodes + `run()`),
//! but cross-checked for 3.6.0 numeric behavior against the disassembly.

// TODO(M1): instruction set, frame layout, and the dispatch loop.
