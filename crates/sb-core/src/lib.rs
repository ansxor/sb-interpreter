//! `sb-core` — the faithful SmileBASIC 3.6.0 language core.
//!
//! Pipeline (mirrors the original 3DS interpreter and otya128's `osb`):
//!
//! ```text
//! source (UTF-16) -> lexer -> parser (AST) -> compiler (bytecode) -> stack VM
//! ```
//!
//! This crate is deliberately free of I/O and platform/GUI concerns so it compiles
//! cleanly to `wasm32-unknown-unknown`. Rendering lives in `sb-render`, audio in
//! `sb-audio`, and windowing/storage in the `sb-platform-*` crates.
//!
//! Faithfulness is verified against three reference sources (see the project plan):
//! the 3.6.0 Ghidra disassembly (`sb-disassembly/`), the official docs (`sb-docs/`),
//! and otya128's `osb` D interpreter (3.5.0, behavioral cross-check only).

pub mod error;
pub mod value;

// --- pipeline stages (implemented in milestone M1) ---
pub mod ast;
pub mod builtins;
pub mod compiler;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod vm;

pub use ast::{BinOp, Block, Expr, ExprKind, Stmt, StmtKind, UnOp};
pub use error::{ErrNum, SbError};
pub use lexer::{lex, Lexer};
pub use token::{SourceLocation, Token, TokenType};
pub use value::{Value, ValueType};
