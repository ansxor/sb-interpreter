//! `sb-core` — the SmileBASIC 3.6.0 interpreter core.
//!
//! This crate is the spec-first reimplementation of the language pipeline
//! (`source → lexer → parser → compiler → stack VM`, see
//! `spec/concepts/execution-model.md`). It is deliberately free of I/O, GUI and
//! threads so it builds for `wasm32-unknown-unknown`; platform concerns live in
//! the `sb-platform-*` crates.
//!
//! The lexer (M1-T1), AST node types (M1-T2), parser (M1-T3), runtime
//! value/array types (M1-T4), bytecode + compiler (M1-T5), stack VM (M1-T6), the
//! math/string builtins (M1-T7), the TinyMT RNG (M1-T9) and the console builtins
//! (M1-T8: `PRINT`/`LOCATE`/`COLOR`/`BACKCOLOR`/`CLS`/`ACLS`/`INPUT`/`LINPUT`/`INKEY$`,
//! driving the [`sb_render`] console model) are implemented so far.

pub mod array;
pub mod ast;
pub mod builtins;
pub mod bytecode;
pub mod compiler;
pub mod lexer;
pub mod parser;
pub mod rng;
pub mod token;
pub mod value;
pub mod vm;

pub use array::{ArrayRef, SbArray};
pub use builtins::{dispatch as call_builtin, StdBuiltins, BUILTIN_NAMES};
pub use bytecode::{Const, Function, Op, OptionFlags, Program, VarInfo, VarRef, VarType};
pub use compiler::{compile, compile_with, Builtins, CompileError, NoBuiltins};
pub use lexer::Lexer;
pub use parser::{parse, parse_expression, ParseError, Parser};
pub use token::{SourceLoc, Suffix, Token, TokenKind};
pub use value::{swap_cells, Cell, RuntimeError, SbStr, Value, ValueType};
pub use vm::{Halt, Vm, VmError, CALL_STACK_LIMIT};
