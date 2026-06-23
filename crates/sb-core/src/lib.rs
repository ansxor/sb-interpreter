//! `sb-core` — the SmileBASIC 3.6.0 interpreter core.
//!
//! This crate is the spec-first reimplementation of the language pipeline
//! (`source → lexer → parser → compiler → stack VM`, see
//! `spec/concepts/execution-model.md`). It is deliberately free of I/O, GUI and
//! threads so it builds for `wasm32-unknown-unknown`; platform concerns live in
//! the `sb-platform-*` crates.
//!
//! The lexer (M1-T1) and AST node types (M1-T2) are implemented so far.

pub mod ast;
pub mod lexer;
pub mod token;

pub use lexer::Lexer;
pub use token::{SourceLoc, Suffix, Token, TokenKind};
