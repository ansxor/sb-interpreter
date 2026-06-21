//! Compiler (milestone M1): AST -> bytecode.
//!
//! Walks the AST once and emits the VM's instruction stream, resolving variable
//! slots (global vs `bp`-relative locals), labels, `DEF`/`COMMON DEF` functions, and
//! the `DATA` table. Design reference: `osb/SMILEBASIC/compiler.d`.

// TODO(M1): Scope/Function/DataTable + the emit walk.
