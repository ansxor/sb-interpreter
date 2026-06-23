//! Bytecode (M1-T5) — the flat opcode list the compiler emits and the stack VM
//! (M1-T6) will run.
//!
//! Per `spec/concepts/execution-model.md` ("Compilation"), the compiler walks the
//! AST into a single flat `Vec<Op>` with a `match`-dispatch loop in mind — chosen
//! over osb's object-per-opcode `Code` subclasses (`VM.d`) because a flat enum is
//! better for Rust/wasm, serialization and determinism. The opcode *semantics* still
//! mirror `VM.d`'s `CodeType` families: push const / push var / operate / jumps +
//! conditional jumps + computed `ON` goto / gosub + return / array new+index+store /
//! call user fn / call builtin / read+restore / print / end.
//!
//! ## Stack contract (documented once here, relied on by the VM)
//!
//! The VM is a stack machine with an operand stack, a frame base pointer `bp`, a
//! program counter `pc`, and a slot index. Unless an opcode says otherwise:
//!
//! - Operands an opcode consumes are pushed **left-to-right in source order** before
//!   it, so it pops the *last* operand first. (Binary `Operate` pops `rhs` then
//!   `lhs`; `PushArray { dims }` pops the last subscript first.)
//! - A code address ([`usize`]) in a jump is an **index into the `code` vec** of the
//!   owning slot. Cross-slot transfers (`GotoExpr`/`GosubExpr`) carry a string target
//!   resolved at runtime.
//!
//! This module is pure data (no I/O), so it builds for `wasm32`.

use crate::ast::{BinOp, Name, UnOp};
use crate::sysvars::ErrSysvar;
use crate::token::Suffix;

/// A compile-time constant pushed by [`Op::Push`]. Mirrors the runtime scalar types
/// (Integer `i32` / Double `f64` / String UTF-16) but is decoupled from
/// [`crate::value::Value`] so bytecode is cheap to clone, compare and snapshot.
/// Strings are UTF-16 code units, matching the runtime string type.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    Int(i32),
    Real(f64),
    /// UTF-16 code units (the SmileBASIC string type).
    Str(Vec<u16>),
}

impl Const {
    /// Build a string constant from a Rust `&str` (UTF-16 encoded).
    pub fn str_from(s: &str) -> Const {
        Const::Str(s.encode_utf16().collect())
    }
}

/// The element type of an array, or the static type of a scalar — derived from a
/// name's [`Suffix`]. `$`→String, `#`→Double, `%`/none→Integer (the suffix-less
/// numeric default; `OPTION DEFINT` is a no-op against this default — see
/// `HARVEST_QUEUE.md` for the suffix-less Real-vs-Int question, cross-ref M1-T4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Int,
    Real,
    Str,
}

impl VarType {
    /// Map a declaration suffix to its concrete element/scalar type.
    pub fn from_suffix(suffix: Suffix) -> VarType {
        match suffix {
            Suffix::Str => VarType::Str,
            Suffix::Real => VarType::Real,
            Suffix::Int | Suffix::None => VarType::Int,
        }
    }
}

/// Where a variable lives. Globals get a slot-global index; a `DEF` body's params,
/// `OUT` params and `VAR`/`DIM`-declared locals get a **bp-relative** index
/// (`execution-model.md`, "Variable resolution").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarRef {
    Global(u32),
    Local(u32),
}

/// `OPTION` flags recorded by the compiler. They affect compilation rather than the
/// run: `STRICT` requires every variable be declared before use (else `Undefined
/// variable`, errnum 15); `DEFINT` makes the suffix-less numeric default Integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OptionFlags {
    pub strict: bool,
    pub defint: bool,
}

/// One opcode. The address operands are indices into the owning slot's `code` vec.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    // --- constants & raw stack ------------------------------------------------
    /// Push an immediate constant.
    Push(Const),
    /// Push the Void value (an omitted argument, e.g. the gap in `LOCATE ,5`).
    PushVoid,
    /// Discard the top of stack (osb `DecSP` — drop an unused builtin result).
    Pop,

    // --- scalar variables -----------------------------------------------------
    /// Push a variable's value.
    PushVar(VarRef),
    /// Push a *reference* to a scalar variable (for `OUT`/by-ref args and `SWAP`).
    PushRef(VarRef),
    /// Pop a value and store it into a variable. The VM coerces to the variable's
    /// static type (the `VarRef`'s declared [`Suffix`], looked up in the symbol
    /// table) per [`crate::value::Value::coerce_to_suffix`].
    PopVar(VarRef),
    /// Push a read-only error-state system variable (`ERRNUM`/`ERRLINE`/`ERRPRG`,
    /// M1-T13). The VM reads its current error state; assignment is rejected at
    /// compile time (Syntax error, errnum 3) so there is no matching pop.
    PushSysvar(ErrSysvar),

    // --- computed references (`VAR(expr)`) ------------------------------------
    /// Pop a value (a variable name), push a reference resolved at runtime.
    PushRefExpr,
    /// Pop a reference, then pop a value; store the value through the reference
    /// (`name(expr) = value` / `VAR(x) = value`, osb `PopRererence`).
    PopRef,

    // --- arrays ---------------------------------------------------------------
    /// Pop `dims` sizes, allocate a fresh array of `ty`, store it in `var` (`DIM`).
    NewArray { var: VarRef, ty: VarType, dims: u8 },
    /// Pop `dims` subscripts, push the addressed element (`A[i,j]` / `A(i,j)` read).
    PushArray { var: VarRef, dims: u8 },
    /// Pop `dims` subscripts, push a reference to the addressed element.
    PushArrayRef { var: VarRef, dims: u8 },
    /// Pop `dims` subscripts then a value, store it into the addressed element.
    PopArray { var: VarRef, dims: u8 },

    // --- operators ------------------------------------------------------------
    /// Binary operator: pop `rhs` then `lhs`, push the result.
    Operate(BinOp),
    /// Unary operator: pop the operand, push the result.
    Unary(UnOp),

    // --- control flow (addresses index this slot's `code` vec) ----------------
    /// Unconditional jump.
    Jump(usize),
    /// Pop a condition; jump if it is false (0).
    JumpFalse(usize),
    /// Pop a condition; jump if it is true (non-0).
    JumpTrue(usize),
    /// `&&` short-circuit: peek TOS; if false, jump (keeping the false), else pop.
    LogicalAnd(usize),
    /// `||` short-circuit: peek TOS; if true, jump (keeping the true), else pop.
    LogicalOr(usize),
    /// `GOTO @label` resolved to an address.
    Goto(usize),
    /// `GOTO <expr>`: pop a string target, resolve at runtime (allows cross-slot).
    GotoExpr,
    /// `GOSUB @label` resolved to an address: push a return address, jump.
    Gosub(usize),
    /// `GOSUB <expr>`: pop a string target, resolve at runtime.
    GosubExpr,
    /// `RETURN` from a `GOSUB`: pop the return address, jump back.
    Return,
    /// `ON expr GOTO l0, l1, ...`: pop the selector, jump to `targets[selector]`
    /// (out of range falls through).
    OnGoto(Vec<usize>),
    /// `ON expr GOSUB l0, l1, ...`.
    OnGosub(Vec<usize>),

    // --- user functions / builtins / commands ---------------------------------
    /// Call a user `DEF`/`COMMON DEF` resolved by name (cross-slot/common targets
    /// are resolved at runtime). `argc` value args are on the stack; `out_argc`
    /// `OUT` results are left on the stack for the caller to pop afterwards.
    CallUser {
        name: Name,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    },
    /// Call a builtin by canonical name (registered in M1-T7; unknown → `Undefined
    /// function`, errnum 16, at runtime). Same arg/out/value contract as `CallUser`.
    CallBuiltin {
        name: String,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    },
    /// `CALL "NAME", args...`: pop the name string, then dispatch like `CallUser`.
    CallDynamic {
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    },
    /// `RETURN [value]` from a `DEF` body.
    ReturnFunc { has_value: bool },

    // --- DATA / READ / RESTORE ------------------------------------------------
    /// Read one `DATA` item, advancing the DATA cursor, and push it. Out of DATA →
    /// errnum 13.
    ReadValue,
    /// `RESTORE @label`: set the DATA cursor to a resolved DATA index.
    Restore(usize),
    /// `RESTORE <expr>`: pop a string target, resolve at runtime.
    RestoreExpr,

    // --- console I/O ----------------------------------------------------------
    /// Pop a value and print it (separators handled by the surrounding op sequence).
    PrintItem,
    /// `,` separator — advance to the next `TABSTEP` column.
    PrintTab,
    /// End-of-statement newline for a non-`;`-terminated `PRINT`.
    PrintNewline,
    /// `INPUT`: print the (already-pushed) prompt if `has_prompt`, add `?` if
    /// `question`, read a line, split it on commas into `count` fields, parse each
    /// field per the matching receiver type in `types` (a `Str` receiver keeps the
    /// raw text, a numeric receiver parses a number), and push the fields for the
    /// following pops. `types.len() == count as usize`.
    Input {
        count: u8,
        question: bool,
        has_prompt: bool,
        types: Vec<VarType>,
    },
    /// `LINPUT`: read one whole line, push it for the following pop.
    Linput { has_prompt: bool },

    // --- misc statements ------------------------------------------------------
    /// `INC`/`DEC`: pop a reference then a delta, add the delta through the reference.
    IncRef,
    /// `SWAP`: pop two references, exchange the cells.
    Swap,
    /// `USE n`: pop a slot expression, make that slot executable.
    Use,
    /// `EXEC target`: pop a slot/file expression, load+run it.
    Exec,
    /// `END` — stop the program normally.
    End,
    /// `STOP` — halt (distinct from `END`; resumable via CONT in DIRECT mode).
    Stop,
}

/// A compiled user function (`DEF`/`COMMON DEF`), addressed into its slot's `code`.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Name,
    /// Entry point: an index into the slot's `code` vec.
    pub address: usize,
    /// By-value parameters, in order (each a frame-local).
    pub params: Vec<Name>,
    /// `OUT` (by-reference) parameters, in order (each a frame-local).
    pub out_params: Vec<Name>,
    /// Whether the function returns a value (vs. a command).
    pub returns_value: bool,
    /// `COMMON DEF` — published into the process-wide common function table.
    pub is_common: bool,
    /// Frame-local variables, indexed by their bp-relative [`VarRef::Local`] index:
    /// params, then `OUT` params, then body-declared/auto-declared locals.
    pub locals: Vec<VarInfo>,
}

/// Symbol-table entry for a variable: its [`Name`] (the `Suffix` drives coercion on
/// `PopVar`) and whether it was declared as an array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarInfo {
    pub name: Name,
    pub is_array: bool,
}

/// A compiled program slot: the flat code plus the link tables the VM needs
/// (`execution-model.md`, "Per-slot state").
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// The flat opcode list.
    pub code: Vec<Op>,
    /// Parallel to `code`: the source location of each opcode, for `ERRLINE`.
    pub locs: Vec<crate::token::SourceLoc>,
    /// Global variables, indexed by [`VarRef::Global`] index.
    pub globals: Vec<VarInfo>,
    /// User functions, keyed by canonical name.
    pub functions: Vec<Function>,
    /// The flat `DATA` constant pool, read by `ReadValue` and repositioned by
    /// `Restore`.
    pub data: Vec<Const>,
    /// `@label` → DATA-pool index, for `RESTORE @label`.
    pub data_labels: Vec<(String, usize)>,
    /// `@label` → code address, for tooling / cross-slot resolution.
    pub code_labels: Vec<(String, usize)>,
    /// The `OPTION` flags that governed compilation.
    pub options: OptionFlags,
}

impl Program {
    /// Look up a global variable's index by name.
    pub fn global_index(&self, name: &Name) -> Option<u32> {
        self.globals
            .iter()
            .position(|v| &v.name == name)
            .map(|i| i as u32)
    }

    /// Look up a function's index by name.
    pub fn function_index(&self, name: &Name) -> Option<usize> {
        self.functions.iter().position(|f| &f.name == name)
    }
}
