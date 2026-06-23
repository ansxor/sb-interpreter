//! Stack VM (M1-T6) — runs the [`Program`] the compiler (M1-T5) emits.
//!
//! Per `spec/concepts/execution-model.md` and `prd/M1.md`, this is a stack machine:
//! an operand stack of [`Value`]s, a program counter `pc`, a `GOSUB` return-address
//! stack, and a stack of `DEF`-call frames (each with its own bp-relative locals).
//! Opcodes are dispatched by a `match` over the flat [`Op`] enum (chosen over osb's
//! object-per-opcode `Code` for Rust/wasm + determinism; the *semantics* mirror
//! `VM.d`'s `run()`). The crate stays I/O-free, so the VM builds for `wasm32`.
//!
//! ## What this slice runs
//!
//! The language core: constants, scalar variables, 1–4D arrays (new/read/write),
//! the full operator set, `IF`/`FOR`/`WHILE`/`REPEAT` (lowered to jumps by the
//! compiler), `GOTO`/`GOSUB`/`RETURN`, `ON … GOTO/GOSUB`, `DEF`/`COMMON DEF` calls
//! with by-value params, `OUT` results and a return value, `DATA`/`READ`/`RESTORE`,
//! and scalar `INC`/`DEC`/`SWAP`. Recursion past [`CALL_STACK_LIMIT`] raises **Stack
//! overflow** (errnum 5).
//!
//! Builtins ([`Op::CallBuiltin`]/[`Op::CallDynamic`], M1-T7) and console + input
//! (`Print*`/`Input`/`Linput` plus the `LOCATE`/`COLOR`/`BACKCOLOR`/`CLS`/`ACLS`/`INKEY$`
//! builtins, M1-T8) are wired: the VM owns an [`sb_render::console::Console`] that `PRINT`
//! and the console commands drive, and a headless input queue ([`Vm::push_input`]) feeds
//! `INPUT`/`LINPUT`. `USE`/`EXEC` (M6) and array-element / runtime-name references
//! ([`Op::PushArrayRef`]/[`Op::PushRefExpr`]) are not yet wired and raise
//! [`VmError::Unsupported`] rather than panicking — their handlers land in the milestones
//! above.
//!
//! ## Operator semantics (from the spec/disassembly)
//!
//! Integer arithmetic wraps mod 2³² (`+`/`-`/`*`/unary `-` use `wrapping_*`); a
//! Double operand promotes the result to Double. `/` is **always** real division
//! (`7/2 == 3.5`), divide-by-zero → errnum 7. `DIV`/`MOD`/`AND`/`OR`/`XOR` and the
//! shifts **truncate each operand toward zero to `i32` first** (`7 AND 2.9 == 2`),
//! then do the integer op; `DIV`/`MOD` by a (truncated) zero → errnum 7
//! (`spec/instructions/{div,mod,and,or,xor}.yaml`, hw_verified). Comparisons yield
//! Integer `1`/`0`; strings compare by UTF-16 code units; a string-vs-number compare
//! or any string in an arithmetic op → Type mismatch (errnum 8).

use crate::array::SbArray;
use crate::ast::{BinOp, Name, UnOp};
use crate::bytecode::{Const, Op, Program, VarRef, VarType};
use crate::sysvars::ErrSysvar;
use crate::token::Suffix;
use crate::value::{swap_cells, Cell, RuntimeError, SbStr, Value};
use sb_render::console::Console;
use std::cmp::Ordering;
use std::collections::VecDeque;

/// Max combined depth of the `GOSUB` return stack + `DEF` call frames before raising
/// **Stack overflow** (errnum 5). The exact value real SB 3.6.0 trips at is queued
/// (`HARVEST_QUEUE.md`, execution-model "recursion depth that trips Stack overflow");
/// this is a generous hypothesis bound that lets ordinary recursion run while still
/// catching unbounded recursion.
pub const CALL_STACK_LIMIT: usize = 8192;

// errnums used directly by the VM (names per `spec/reference/errors.yaml`).
const ERR_STACK_OVERFLOW: u32 = 5;
const ERR_STACK_UNDERFLOW: u32 = 6;
const ERR_DIVIDE_BY_ZERO: u32 = 7;
const ERR_TYPE_MISMATCH: u32 = 8;
const ERR_OUT_OF_DATA: u32 = 13;
const ERR_UNDEFINED_LABEL: u32 = 14;
const ERR_UNDEFINED_FUNCTION: u32 = 16;
const ERR_RETURN_WITHOUT_GOSUB: u32 = 30;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    /// `END`, or fell off the end of the code.
    End,
    /// `STOP` (distinct from `END`; resumable via `CONT` in DIRECT mode — M1-T13).
    Stop,
}

/// A run failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// A SmileBASIC runtime error: an `ERRNUM` and the 1-based source `line`
    /// (`ERRLINE`) it occurred on.
    Sb { errnum: u32, line: u32 },
    /// An opcode whose handler is implemented in a later milestone (builtins M1-T7,
    /// console/input M1-T8, `USE`/`EXEC` M6, array-element/runtime-name refs).
    Unsupported(&'static str),
    /// A failed `ASSERT__` (the test-mode builtin, M1-T14): the condition was false.
    /// Carries the assertion's message and the 1-based source `line` it fired on. This
    /// is a harness construct, NOT a SmileBASIC runtime error — it has no `ERRNUM`.
    Assert { message: String, line: u32 },
}

impl VmError {
    /// The `ERRNUM` if this is a SmileBASIC runtime error.
    pub fn errnum(&self) -> Option<u32> {
        match self {
            VmError::Sb { errnum, .. } => Some(*errnum),
            VmError::Unsupported(_) | VmError::Assert { .. } => None,
        }
    }

    /// The `ERRLINE` (source line) if this is a SmileBASIC runtime error.
    pub fn errline(&self) -> Option<u32> {
        match self {
            VmError::Sb { line, .. } => Some(*line),
            VmError::Unsupported(_) | VmError::Assert { .. } => None,
        }
    }
}

/// One `DEF`/`COMMON DEF` activation record.
struct Frame {
    /// Frame-local storage cells, indexed by [`VarRef::Local`]: params, then `OUT`
    /// params, then body-declared/auto-declared locals.
    locals: Vec<Cell>,
    /// `pc` to resume at in the caller after the call returns.
    return_pc: usize,
    /// Index into [`Program::functions`] of the function running in this frame.
    func: usize,
    /// Whether the caller wants the function's return value left on the stack.
    wants_value: bool,
}

/// The stack VM.
pub struct Vm {
    program: Program,
    /// One storage [`Cell`] per [`Program::globals`] entry.
    globals: Vec<Cell>,
    /// The operand stack.
    stack: Vec<Value>,
    /// `DEF`-call activation records.
    frames: Vec<Frame>,
    /// `GOSUB` return addresses (indices into `program.code`).
    gosub: Vec<usize>,
    /// Program counter (index into `program.code`).
    pc: usize,
    /// `DATA` read cursor (index into `program.data`).
    data_cursor: usize,
    /// The 8 TinyMT32 random series behind `RND`/`RNDF`/`RANDOMIZE` (M1-T9).
    rng: crate::rng::Rng,
    /// The text console model (M1-T10): grid + cursor + COLOR/ATTR state, driven by
    /// `PRINT`/`LOCATE`/`COLOR`/`CLS`/`ACLS` (M1-T8).
    console: Console,
    /// The screen background color code (`BACKCOLOR`). The handler round-trips the user's
    /// RGB code, so we store it verbatim; the rendered border color is screen state (M2).
    back_color: i32,
    /// `TABSTEP` — the `PRINT ,` tab-stop width. Boot default 4 (`sysvars.yaml`); the
    /// writable system-variable wiring lands with M6-T3 (queued).
    tabstep: usize,
    /// Queued input lines for `INPUT`/`LINPUT` (one entry = one ENTER-terminated line).
    /// Headless there is no live keyboard; a runner/test preloads this via
    /// [`Vm::push_input`]. An empty queue yields an empty line.
    input_lines: VecDeque<SbStr>,
    /// Error state (M1-T13): the `ERRNUM`/`ERRLINE`/`ERRPRG` read-only sysvars. Boot/`RUN`
    /// reset to 0 (= "No Error"); set at the moment of a halting error and left readable
    /// afterwards (the DIRECT-mode residue — see `spec/concepts/error-model.md`). `ERRPRG`
    /// is the executing program SLOT, always 0 in single-slot M1 (multi-slot → M6).
    errnum: i32,
    errline: i32,
    errprg: i32,
}

impl Vm {
    /// Build a VM for a compiled program, with every global initialised to its
    /// declared type's zero value (numeric → 0, string → "").
    pub fn new(program: Program) -> Self {
        let globals = program
            .globals
            .iter()
            .map(|v| Value::cell(Value::default_for_suffix(v.name.suffix)))
            .collect();
        Vm {
            program,
            globals,
            stack: Vec::new(),
            frames: Vec::new(),
            gosub: Vec::new(),
            pc: 0,
            data_cursor: 0,
            rng: crate::rng::Rng::new(),
            console: Console::top(),
            back_color: 0,
            tabstep: 4,
            input_lines: VecDeque::new(),
            errnum: 0,
            errline: 0,
            errprg: 0,
        }
    }

    /// The `ERRNUM` of the last halting error (0 = none) — the DIRECT-mode residue.
    pub fn errnum(&self) -> i32 {
        self.errnum
    }

    /// The `ERRLINE` of the last halting error (the 1-based source line).
    pub fn errline(&self) -> i32 {
        self.errline
    }

    /// The `ERRPRG` of the last halting error (the program SLOT; always 0 in M1).
    pub fn errprg(&self) -> i32 {
        self.errprg
    }

    /// Borrow the text console (grid + cursor + colors) for rendering / inspection.
    pub fn console(&self) -> &Console {
        &self.console
    }

    /// The console contents as text: each grid row trimmed of trailing blanks, rows joined
    /// by `\n`, trailing blank rows dropped. This is the deterministic `stdout` of a run
    /// (it mirrors what the oracle scrapes from console memory — e.g. `CLS` empties it).
    pub fn console_text(&self) -> String {
        let c = &self.console;
        let mut lines: Vec<String> = Vec::with_capacity(c.rows);
        for y in 0..c.rows {
            let mut line = String::new();
            for x in 0..c.cols {
                let ch = c.cell(x, y).ch;
                // An empty cell (never written) reads as a space; trailing ones are
                // trimmed off below.
                line.push(if ch == 0 {
                    ' '
                } else {
                    char::from_u32(ch as u32).unwrap_or('\u{FFFD}')
                });
            }
            lines.push(line.trim_end().to_string());
        }
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Queue one line of input for the next `INPUT`/`LINPUT` (headless input source).
    pub fn push_input(&mut self, line: &str) {
        self.input_lines.push_back(line.encode_utf16().collect());
    }

    /// Run to completion (or error). The operand stack is empty between statements
    /// in well-formed bytecode; a non-empty stack at `End` is tolerated.
    pub fn run(&mut self) -> Result<Halt, VmError> {
        loop {
            if self.pc >= self.program.code.len() {
                return Ok(Halt::End);
            }
            let here = self.pc;
            let op = self.program.code[here].clone();
            self.pc += 1;
            match self.step(op) {
                Ok(None) => {}
                Ok(Some(halt)) => return Ok(halt),
                Err(e) => {
                    let e = self.attach_line(e, here);
                    // Capture the error-state residue so ERRNUM/ERRLINE/ERRPRG are
                    // readable after the halt (the DIRECT-mode window, M1-T13). Only a
                    // SmileBASIC runtime error sets it; an `Unsupported` op does not.
                    if let VmError::Sb { errnum, line } = e {
                        self.errnum = errnum as i32;
                        self.errline = line as i32;
                        self.errprg = 0; // single-slot M1; multi-slot ERRPRG → M6.
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Read a global's current value by name + suffix (for tests / a future REPL).
    pub fn global_value(&self, ident: &str, suffix: Suffix) -> Option<Value> {
        let name = Name::new(ident.to_ascii_uppercase(), suffix);
        let idx = self.program.global_index(&name)? as usize;
        Some(self.globals[idx].borrow().clone())
    }

    // -- dispatch --------------------------------------------------------------

    /// Execute one opcode. `Ok(None)` continues; `Ok(Some(halt))` stops the run.
    fn step(&mut self, op: Op) -> Result<Option<Halt>, VmError> {
        match op {
            Op::Push(c) => self.stack.push(const_to_value(&c)),
            Op::PushVoid => self.stack.push(Value::Void),
            Op::Pop => {
                self.pop()?;
            }

            Op::PushVar(vref) => {
                let v = self.cell(vref)?.borrow().clone();
                self.stack.push(v);
            }
            Op::PushRef(vref) => {
                let cell = self.cell(vref)?.clone();
                self.stack.push(Value::Ref(cell));
            }
            Op::PopVar(vref) => {
                let suffix = self.var_suffix(vref)?;
                let v = self.pop()?.coerce_to_suffix(suffix).map_err(sb)?;
                *self.cell(vref)?.borrow_mut() = v;
            }
            Op::PushSysvar(sv) => {
                let v = match sv {
                    ErrSysvar::Errnum => self.errnum,
                    ErrSysvar::Errline => self.errline,
                    ErrSysvar::Errprg => self.errprg,
                };
                self.stack.push(Value::Int(v));
            }

            Op::NewArray { var, ty, dims } => self.new_array(var, ty, dims)?,
            Op::PushArray { var, dims } => self.push_array(var, dims)?,
            Op::PopArray { var, dims } => self.pop_array(var, dims)?,

            Op::Operate(binop) => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                self.stack.push(operate(binop, lhs, rhs).map_err(sb)?);
            }
            Op::Unary(unop) => {
                let v = self.pop()?;
                self.stack.push(unary(unop, v).map_err(sb)?);
            }

            Op::Jump(addr) => self.pc = addr,
            Op::JumpFalse(addr) => {
                if !self.truthy()? {
                    self.pc = addr;
                }
            }
            Op::JumpTrue(addr) => {
                if self.truthy()? {
                    self.pc = addr;
                }
            }
            Op::LogicalAnd(addr) => {
                // Peek: if false, keep it and jump past the rhs; else drop and fall in.
                if !self.peek_truthy()? {
                    self.pc = addr;
                } else {
                    self.pop()?;
                }
            }
            Op::LogicalOr(addr) => {
                if self.peek_truthy()? {
                    self.pc = addr;
                } else {
                    self.pop()?;
                }
            }

            Op::Goto(addr) => self.pc = addr,
            Op::GotoExpr => {
                let addr = self.resolve_code_label()?;
                self.pc = addr;
            }
            Op::Gosub(addr) => {
                self.push_gosub(self.pc)?;
                self.pc = addr;
            }
            Op::GosubExpr => {
                let addr = self.resolve_code_label()?;
                self.push_gosub(self.pc)?;
                self.pc = addr;
            }
            Op::Return => {
                self.pc = self.gosub.pop().ok_or(VmError::Sb {
                    errnum: ERR_RETURN_WITHOUT_GOSUB,
                    line: 0,
                })?;
            }
            Op::OnGoto(targets) => {
                let sel = self.pop()?.to_int().map_err(sb)?;
                if let Some(&addr) = usize::try_from(sel).ok().and_then(|i| targets.get(i)) {
                    self.pc = addr;
                }
            }
            Op::OnGosub(targets) => {
                let sel = self.pop()?.to_int().map_err(sb)?;
                if let Some(&addr) = usize::try_from(sel).ok().and_then(|i| targets.get(i)) {
                    self.push_gosub(self.pc)?;
                    self.pc = addr;
                }
            }

            Op::CallUser {
                name,
                argc,
                out_argc,
                wants_value,
            } => self.call_user(&name, argc, out_argc, wants_value)?,
            Op::ReturnFunc { has_value } => self.return_func(has_value)?,

            Op::ReadValue => {
                let c = self
                    .program
                    .data
                    .get(self.data_cursor)
                    .cloned()
                    .ok_or(VmError::Sb {
                        errnum: ERR_OUT_OF_DATA,
                        line: 0,
                    })?;
                self.data_cursor += 1;
                self.stack.push(const_to_value(&c));
            }
            Op::Restore(idx) => self.data_cursor = idx,
            Op::RestoreExpr => {
                let label = self.pop_label_name()?;
                self.data_cursor = self
                    .program
                    .data_labels
                    .iter()
                    .find(|(n, _)| *n == label)
                    .map(|(_, i)| *i)
                    .ok_or(VmError::Sb {
                        errnum: ERR_UNDEFINED_LABEL,
                        line: 0,
                    })?;
            }

            Op::IncRef => {
                let target = as_ref(self.pop()?)?;
                let delta = self.pop()?;
                let cur = target.deref();
                let new = operate(BinOp::Add, cur, delta).map_err(sb)?;
                target.assign_through(new).map_err(sb)?;
            }
            Op::Swap => {
                let b = as_ref(self.pop()?)?;
                let a = as_ref(self.pop()?)?;
                if let (Value::Ref(ca), Value::Ref(cb)) = (&a, &b) {
                    swap_cells(ca, cb);
                }
            }

            Op::End => return Ok(Some(Halt::End)),
            Op::Stop => return Ok(Some(Halt::Stop)),

            Op::CallBuiltin {
                name,
                argc,
                out_argc,
                wants_value,
            } => self.call_builtin(&name, argc, out_argc, wants_value)?,

            // -- console output (M1-T8) --------------------------------------------
            Op::PrintItem => {
                let v = self.pop()?.deref();
                let units = crate::builtins::console::format_print_item(&v).map_err(sb)?;
                for u in units {
                    self.console.put_char(u);
                }
            }
            Op::PrintTab => self.console.tab(self.tabstep),
            Op::PrintNewline => self.console.newline(),
            Op::Input {
                count,
                question,
                has_prompt,
                types,
            } => self.do_input(count, question, has_prompt, &types)?,
            Op::Linput { has_prompt } => self.do_linput(has_prompt)?,

            // -- deferred to later milestones --------------------------------------
            Op::CallDynamic { .. } => return Err(VmError::Unsupported("CALL (M6)")),
            Op::Use => return Err(VmError::Unsupported("USE (M6)")),
            Op::Exec => return Err(VmError::Unsupported("EXEC (M6)")),
            Op::PushRefExpr | Op::PopRef => {
                return Err(VmError::Unsupported("runtime-name reference (VAR())"))
            }
            Op::PushArrayRef { .. } => return Err(VmError::Unsupported("array-element reference")),
        }
        Ok(None)
    }

    // -- arrays ----------------------------------------------------------------

    fn new_array(
        &mut self,
        var: VarRef,
        ty: crate::bytecode::VarType,
        dims: u8,
    ) -> Result<(), VmError> {
        let sizes = self.pop_indices(dims)?;
        let arr = match ty {
            crate::bytecode::VarType::Int => {
                Value::IntArray(SbArray::<i32>::new(&sizes).map_err(sb)?.into_ref())
            }
            crate::bytecode::VarType::Real => {
                Value::RealArray(SbArray::<f64>::new(&sizes).map_err(sb)?.into_ref())
            }
            crate::bytecode::VarType::Str => {
                Value::StrArray(SbArray::<SbStr>::new(&sizes).map_err(sb)?.into_ref())
            }
        };
        *self.cell(var)?.borrow_mut() = arr;
        Ok(())
    }

    fn push_array(&mut self, var: VarRef, dims: u8) -> Result<(), VmError> {
        let idx = self.pop_indices(dims)?;
        let v = match &*self.cell(var)?.borrow() {
            Value::IntArray(a) => Value::Int(a.borrow().get(&idx).map_err(sb)?),
            Value::RealArray(a) => Value::Real(a.borrow().get(&idx).map_err(sb)?),
            Value::StrArray(a) => Value::Str(a.borrow().get(&idx).map_err(sb)?),
            _ => {
                return Err(VmError::Sb {
                    errnum: ERR_TYPE_MISMATCH,
                    line: 0,
                })
            }
        };
        self.stack.push(v);
        Ok(())
    }

    fn pop_array(&mut self, var: VarRef, dims: u8) -> Result<(), VmError> {
        let idx = self.pop_indices(dims)?;
        let val = self.pop()?;
        match &*self.cell(var)?.borrow() {
            Value::IntArray(a) => a
                .borrow_mut()
                .set(&idx, val.to_int().map_err(sb)?)
                .map_err(sb)?,
            Value::RealArray(a) => a
                .borrow_mut()
                .set(&idx, val.to_real().map_err(sb)?)
                .map_err(sb)?,
            Value::StrArray(a) => a
                .borrow_mut()
                .set(&idx, val.as_str().map_err(sb)?.clone())
                .map_err(sb)?,
            _ => {
                return Err(VmError::Sb {
                    errnum: ERR_TYPE_MISMATCH,
                    line: 0,
                })
            }
        }
        Ok(())
    }

    /// Pop `n` subscripts/sizes, returning them in source order (`[i0, i1, …]`).
    fn pop_indices(&mut self, n: u8) -> Result<Vec<i32>, VmError> {
        let mut out = vec![0i32; n as usize];
        for slot in out.iter_mut().rev() {
            *slot = self.pop()?.to_int().map_err(sb)?;
        }
        Ok(out)
    }

    // -- user functions --------------------------------------------------------

    fn call_user(
        &mut self,
        name: &Name,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), VmError> {
        let func = self.program.function_index(name).ok_or(VmError::Sb {
            errnum: ERR_UNDEFINED_FUNCTION,
            line: 0,
        })?;
        if self.depth() >= CALL_STACK_LIMIT {
            return Err(VmError::Sb {
                errnum: ERR_STACK_OVERFLOW,
                line: 0,
            });
        }

        let _ = out_argc; // OUT results are produced by ReturnFunc reading out-param locals.
                          // Snapshot the function's local + param types (drops the `program` borrow before
                          // we touch the operand stack below).
        let f = &self.program.functions[func];
        let local_suffixes: Vec<Suffix> = f.locals.iter().map(|v| v.name.suffix).collect();
        let param_suffixes: Vec<Suffix> = f.params.iter().map(|p| p.suffix).collect();

        // Build the frame's locals, each defaulted to its declared type's zero.
        let locals: Vec<Cell> = local_suffixes
            .iter()
            .map(|&s| Value::cell(Value::default_for_suffix(s)))
            .collect();
        // Bind the `argc` by-value args (topmost = last param) into the leading locals,
        // coercing each to its parameter's static type. Surplus args are dropped.
        for i in (0..argc as usize).rev() {
            let v = self.pop()?;
            if let Some(&suffix) = param_suffixes.get(i) {
                *locals[i].borrow_mut() = v.coerce_to_suffix(suffix).map_err(sb)?;
            }
        }

        self.frames.push(Frame {
            locals,
            return_pc: self.pc,
            func,
            wants_value,
        });
        self.pc = self.program.functions[func].address;
        Ok(())
    }

    /// Call a registered builtin (M1-T7 math/string set). Pops `argc` value args
    /// (topmost = last arg), dispatches by canonical name, and pushes the single return
    /// value when the caller wants it. These builtins take no `OUT` params, so
    /// `out_argc` is expected to be 0. An unknown name → Undefined function (errnum 16).
    fn call_builtin(
        &mut self,
        name: &str,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), VmError> {
        let _ = out_argc; // math/string builtins produce no OUT results.
        let mut args = Vec::with_capacity(argc as usize);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        args.reverse();
        // RNG builtins (RND/RNDF/RANDOMIZE, M1-T9) read/mutate the VM's TinyMT series, so
        // they can't go through the stateless `builtins::dispatch`.
        if let Some(ret) = self.call_rng(name, &args).map_err(sb)? {
            if wants_value {
                self.stack.push(ret);
            }
            return Ok(());
        }
        // `ASSERT__` (M1-T14) is the test-mode builtin: a false condition halts the run
        // with [`VmError::Assert`] (not a SmileBASIC error). It is a statement command, so
        // it produces no value.
        if name == "ASSERT__" {
            self.call_assert(&args)?;
            return Ok(());
        }
        // Console builtins (LOCATE/COLOR/CLS/ACLS/BACKCOLOR/INKEY$, M1-T8) mutate the
        // VM-owned console / screen state, so they too sidestep the stateless dispatch.
        if let Some(ret) = self.call_console(name, &args, wants_value).map_err(sb)? {
            if wants_value && !matches!(ret, Value::Void) {
                self.stack.push(ret);
            }
            return Ok(());
        }
        let ret = crate::builtins::dispatch(name, args).map_err(sb)?;
        if wants_value {
            self.stack.push(ret);
        }
        Ok(())
    }

    /// Handle the RNG builtins against the VM-owned [`Rng`](crate::rng::Rng). Returns
    /// `Ok(Some(value))` for `RND`/`RNDF` (the drawn value), `Ok(Some(Void))` for the
    /// `RANDOMIZE` statement, or `Ok(None)` when `name` is not an RNG builtin (the caller
    /// then falls through to the stateless dispatch). Argument validation follows the
    /// `spec/instructions/{rnd,rndf,randomize}.yaml` contract: bad arg count → Illegal
    /// function call (4), string arg → Type mismatch (8), out-of-range seed/max → Out of
    /// range (10).
    fn call_rng(&mut self, name: &str, args: &[Value]) -> Result<Option<Value>, RuntimeError> {
        match name {
            "RND" => {
                // RND(max) draws from series 0; RND(seed_id, max) selects the series.
                let (seed_id, max) = match args {
                    [m] => (0, m.to_int()?),
                    [s, m] => (rng_seed_id(s)?, m.to_int()?),
                    _ => {
                        return Err(RuntimeError::new(
                            crate::builtins::ERR_ILLEGAL_FUNCTION_CALL,
                        ))
                    }
                };
                if max < 0 {
                    return Err(crate::builtins::out_of_range());
                }
                Ok(Some(Value::Int(self.rng.rnd(seed_id, max))))
            }
            "RNDF" => {
                // RNDF() draws from series 0; RNDF(seed_id) selects the series.
                let seed_id = match args {
                    [] => 0,
                    [s] => rng_seed_id(s)?,
                    _ => {
                        return Err(RuntimeError::new(
                            crate::builtins::ERR_ILLEGAL_FUNCTION_CALL,
                        ))
                    }
                };
                Ok(Some(Value::Real(self.rng.rndf(seed_id))))
            }
            "RANDOMIZE" => {
                // RANDOMIZE seed_id [, seed_value]; seed_value 0/omitted → entropy.
                let (seed_id, seed_value) = match args {
                    [s] => (rng_seed_id(s)?, 0),
                    [s, v] => (rng_seed_id(s)?, v.to_int()?),
                    _ => {
                        return Err(RuntimeError::new(
                            crate::builtins::ERR_ILLEGAL_FUNCTION_CALL,
                        ))
                    }
                };
                self.rng.randomize(seed_id, seed_value);
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }

    /// Route a console builtin (M1-T8) over the VM-owned [`Console`] / screen state.
    /// Returns `Ok(Some(value))` when handled (statement commands return [`Value::Void`]),
    /// or `Ok(None)` when `name` is not a console builtin (the caller falls through to the
    /// stateless dispatch). Argument validation follows the console specs.
    fn call_console(
        &mut self,
        name: &str,
        args: &[Value],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        use crate::builtins::console as cons;
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            "LOCATE" => {
                cons::locate(&mut self.console, &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "COLOR" => {
                cons::color(&mut self.console, &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "CLS" => {
                cons::cls(&mut self.console, &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "ACLS" => {
                cons::acls(&mut self.console, &args, wants_value)?;
                // ACLS resets the wider screen draw state too (not just the console grid).
                self.back_color = 0;
                self.tabstep = 4;
                Ok(Some(Value::Void))
            }
            "INKEY$" => Ok(Some(cons::inkey(&args)?)),
            "BACKCOLOR" => Ok(Some(self.backcolor(&args, wants_value)?)),
            _ => Ok(None),
        }
    }

    /// `ASSERT__ condition[, message$]` — the test-mode assertion (M1-T14). A truthy
    /// condition (non-zero number) is a no-op; a false (zero) condition halts the run with
    /// [`VmError::Assert`] carrying `message$` (empty if omitted). A bad argument count is
    /// an Illegal function call (4); a non-numeric condition is a Type mismatch (8). The
    /// `line` is filled by [`Vm::attach_line`] from the call site.
    fn call_assert(&mut self, args: &[Value]) -> Result<(), VmError> {
        let (cond, message) = match args {
            [c] => (c.deref(), String::new()),
            [c, m] => {
                let msg = match m.deref() {
                    Value::Str(s) => String::from_utf16_lossy(&s),
                    other => crate::builtins::format_number(&other).map_err(sb)?,
                };
                (c.deref(), msg)
            }
            _ => return Err(sb(crate::builtins::illegal())),
        };
        let truthy = match cond {
            Value::Int(i) => i != 0,
            Value::Real(r) => r != 0.0,
            _ => return Err(sb(crate::builtins::type_mismatch())),
        };
        if truthy {
            Ok(())
        } else {
            Err(VmError::Assert { message, line: 0 })
        }
    }

    /// `BACKCOLOR` — the SET form (statement, exactly 1 argument) stores the background
    /// color code; the GET form (function, 0 arguments) returns it. The handler round-trips
    /// the user's RGB code, so we store and return it verbatim. Any other call shape →
    /// errnum 4 (`backcolor.yaml`, hw_verified).
    fn backcolor(&mut self, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
        if wants_value {
            if !args.is_empty() {
                return Err(crate::builtins::illegal());
            }
            Ok(Value::Int(self.back_color))
        } else {
            if args.len() != 1 {
                return Err(crate::builtins::illegal());
            }
            self.back_color = args[0].to_int()?;
            Ok(Value::Void)
        }
    }

    /// `INPUT` (M1-T8): optionally print the prompt and `?`, read one line, split it on
    /// commas into `count` fields, parse each per its receiver type, and push the fields so
    /// the following `PopVar`s assign them in receiver order.
    fn do_input(
        &mut self,
        count: u8,
        question: bool,
        has_prompt: bool,
        types: &[VarType],
    ) -> Result<(), VmError> {
        if has_prompt {
            let p = self.pop()?.deref();
            self.print_units(&p)?;
        }
        if question {
            self.console.put_char(u16::from(b'?'));
        }
        let line = self.input_lines.pop_front().unwrap_or_default();
        self.console.newline(); // ENTER moves to the next line
        let fields: Vec<&[u16]> = line.split(|&u| u == COMMA).collect();
        let mut values: Vec<Value> = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            let field = fields.get(i).copied().unwrap_or(&[]);
            let ty = types.get(i).copied().unwrap_or(VarType::Real);
            values.push(parse_input_field(field, ty));
        }
        // The receivers' `PopVar`s pop top-first in declaration order, so push reversed
        // (the first receiver's value ends on top).
        for v in values.into_iter().rev() {
            self.stack.push(v);
        }
        Ok(())
    }

    /// `LINPUT` (M1-T8): optionally print the prompt, then read one whole line (commas
    /// kept) into the single following string receiver.
    fn do_linput(&mut self, has_prompt: bool) -> Result<(), VmError> {
        if has_prompt {
            let p = self.pop()?.deref();
            self.print_units(&p)?;
        }
        let line = self.input_lines.pop_front().unwrap_or_default();
        self.console.newline();
        self.stack.push(Value::Str(line));
        Ok(())
    }

    /// Print a value's formatted code units to the console (shared by INPUT/LINPUT prompts).
    fn print_units(&mut self, v: &Value) -> Result<(), VmError> {
        let units = crate::builtins::console::format_print_item(v).map_err(sb)?;
        for u in units {
            self.console.put_char(u);
        }
        Ok(())
    }

    fn return_func(&mut self, has_value: bool) -> Result<(), VmError> {
        // The return value (if any) is on top of the operand stack.
        let ret = if has_value { Some(self.pop()?) } else { None };
        let frame = self.frames.pop().ok_or(VmError::Sb {
            errnum: ERR_RETURN_WITHOUT_GOSUB,
            line: 0,
        })?;
        let f = &self.program.functions[frame.func];
        // Read OUT-param results from their frame locals, in declaration order.
        let nparams = f.params.len();
        let out_vals: Vec<Value> = (0..f.out_params.len())
            .map(|i| frame.locals[nparams + i].borrow().clone())
            .collect();
        self.pc = frame.return_pc;
        // Leave OUT results for the caller's pop targets (last OUT arg ends on top).
        for v in out_vals {
            self.stack.push(v);
        }
        // Leave the return value only if the caller wanted it.
        if let Some(v) = ret {
            if frame.wants_value {
                self.stack.push(v);
            }
        }
        Ok(())
    }

    // -- helpers ---------------------------------------------------------------

    /// Combined `GOSUB` + `DEF`-frame depth (for the Stack-overflow guard).
    fn depth(&self) -> usize {
        self.frames.len() + self.gosub.len()
    }

    fn push_gosub(&mut self, addr: usize) -> Result<(), VmError> {
        if self.depth() >= CALL_STACK_LIMIT {
            return Err(VmError::Sb {
                errnum: ERR_STACK_OVERFLOW,
                line: 0,
            });
        }
        self.gosub.push(addr);
        Ok(())
    }

    /// The storage cell a [`VarRef`] addresses (a global, or a current-frame local).
    fn cell(&self, vref: VarRef) -> Result<&Cell, VmError> {
        match vref {
            VarRef::Global(i) => self.globals.get(i as usize).ok_or(VmError::Sb {
                errnum: ERR_UNDEFINED_FUNCTION,
                line: 0,
            }),
            VarRef::Local(i) => self
                .frames
                .last()
                .and_then(|fr| fr.locals.get(i as usize))
                .ok_or(VmError::Sb {
                    errnum: ERR_UNDEFINED_FUNCTION,
                    line: 0,
                }),
        }
    }

    /// The declared suffix of a variable (drives `PopVar` coercion).
    fn var_suffix(&self, vref: VarRef) -> Result<Suffix, VmError> {
        match vref {
            VarRef::Global(i) => self
                .program
                .globals
                .get(i as usize)
                .map(|v| v.name.suffix)
                .ok_or(VmError::Sb {
                    errnum: ERR_UNDEFINED_FUNCTION,
                    line: 0,
                }),
            VarRef::Local(i) => {
                let func = self.frames.last().map(|f| f.func).ok_or(VmError::Sb {
                    errnum: ERR_UNDEFINED_FUNCTION,
                    line: 0,
                })?;
                self.program.functions[func]
                    .locals
                    .get(i as usize)
                    .map(|v| v.name.suffix)
                    .ok_or(VmError::Sb {
                        errnum: ERR_UNDEFINED_FUNCTION,
                        line: 0,
                    })
            }
        }
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::Sb {
            errnum: ERR_STACK_UNDERFLOW,
            line: 0,
        })
    }

    /// Pop a condition and test its truthiness (nonzero).
    fn truthy(&mut self) -> Result<bool, VmError> {
        Ok(self.pop()?.to_real().map_err(sb)? != 0.0)
    }

    /// Peek the top condition's truthiness without consuming it.
    fn peek_truthy(&self) -> Result<bool, VmError> {
        let v = self.stack.last().ok_or(VmError::Sb {
            errnum: ERR_STACK_UNDERFLOW,
            line: 0,
        })?;
        Ok(v.to_real().map_err(sb)? != 0.0)
    }

    /// Pop a string and read it as a `@label` name (folded, leading `@` stripped).
    fn pop_label_name(&mut self) -> Result<String, VmError> {
        let v = self.pop()?;
        let s = v.as_str().map_err(sb)?;
        let text = String::from_utf16_lossy(s);
        Ok(text.trim_start_matches('@').to_ascii_uppercase())
    }

    /// Resolve a popped `@label` string to a code address.
    fn resolve_code_label(&mut self) -> Result<usize, VmError> {
        let label = self.pop_label_name()?;
        self.program
            .code_labels
            .iter()
            .find(|(n, _)| *n == label)
            .map(|(_, a)| *a)
            .ok_or(VmError::Sb {
                errnum: ERR_UNDEFINED_LABEL,
                line: 0,
            })
    }

    /// Stamp the source line of the op at `pc` onto a runtime error (for `ERRLINE`).
    fn attach_line(&self, e: VmError, pc: usize) -> VmError {
        match e {
            VmError::Sb { errnum, line: 0 } => {
                let line = self.program.locs.get(pc).map(|l| l.line).unwrap_or(0);
                VmError::Sb { errnum, line }
            }
            VmError::Assert { message, line: 0 } => {
                let line = self.program.locs.get(pc).map(|l| l.line).unwrap_or(0);
                VmError::Assert { message, line }
            }
            other => other,
        }
    }
}

/// The UTF-16 code unit for `,`, the `INPUT` field delimiter.
const COMMA: u16 = b',' as u16;

/// Parse one `INPUT` field into the value its receiver expects: a string receiver keeps the
/// raw text; a numeric receiver parses a number (integer if it has no fractional/exponent
/// part, else a Double), defaulting to `0` when the field is not a valid number (a degraded
/// stand-in for SmileBASIC's interactive "?Redo from start" re-prompt — queued).
fn parse_input_field(field: &[u16], ty: VarType) -> Value {
    match ty {
        VarType::Str => Value::Str(field.to_vec()),
        VarType::Int => Value::Int(parse_input_number(field).to_int().unwrap_or(0)),
        VarType::Real => parse_input_number(field),
    }
}

/// Parse a numeric `INPUT` field, returning an Integer when it parses as `i32` and a Double
/// otherwise (or Integer `0` when it is not a number).
fn parse_input_number(field: &[u16]) -> Value {
    let s = String::from_utf16_lossy(field);
    let s = s.trim();
    if let Ok(i) = s.parse::<i32>() {
        Value::Int(i)
    } else if let Ok(r) = s.parse::<f64>() {
        Value::Real(r)
    } else {
        Value::Int(0)
    }
}

/// Materialise a bytecode [`Const`] into a runtime [`Value`].
fn const_to_value(c: &Const) -> Value {
    match c {
        Const::Int(i) => Value::Int(*i),
        Const::Real(r) => Value::Real(*r),
        Const::Str(s) => Value::Str(s.clone()),
    }
}

/// Wrap a [`RuntimeError`] (no line yet) as a [`VmError::Sb`]; the run loop fills the
/// line via [`Vm::attach_line`].
fn sb(e: RuntimeError) -> VmError {
    VmError::Sb {
        errnum: e.errnum,
        line: 0,
    }
}

/// Coerce a seed-ID argument to a series index, validating the `0..=7` range. A string →
/// Type mismatch (8); out of range → Out of range (10) (disasm `seed_id >= 8 → errnum 10`).
fn rng_seed_id(v: &Value) -> Result<usize, RuntimeError> {
    let id = v.to_int()?;
    if (0..=7).contains(&id) {
        Ok(id as usize)
    } else {
        Err(crate::builtins::out_of_range())
    }
}

/// Extract a [`Value::Ref`], erroring (errnum 8) if the operand is not a reference.
fn as_ref(v: Value) -> Result<Value, VmError> {
    match v {
        Value::Ref(_) => Ok(v),
        _ => Err(VmError::Sb {
            errnum: ERR_TYPE_MISMATCH,
            line: 0,
        }),
    }
}

// -- operator evaluation -------------------------------------------------------

/// Evaluate a binary operator on two runtime values (see the module-level operator
/// semantics). Errors carry only an errnum; the VM attaches the line.
fn operate(op: BinOp, lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    use BinOp::*;
    match op {
        Add => add(lhs, rhs),
        Sub => num_arith(lhs, rhs, i32::wrapping_sub, |a, b| a - b),
        Mul => num_arith(lhs, rhs, i32::wrapping_mul, |a, b| a * b),
        Div => {
            let (x, y) = (lhs.to_real()?, rhs.to_real()?);
            if y == 0.0 {
                Err(RuntimeError::new(ERR_DIVIDE_BY_ZERO))
            } else {
                Ok(Value::Real(x / y))
            }
        }
        IntDiv => int_div_mod(lhs, rhs, true),
        Mod => int_div_mod(lhs, rhs, false),
        And => bitwise(lhs, rhs, |a, b| a & b),
        Or => bitwise(lhs, rhs, |a, b| a | b),
        Xor => bitwise(lhs, rhs, |a, b| a ^ b),
        Shl => bitwise(lhs, rhs, |a, b| a.wrapping_shl(b as u32)),
        Shr => bitwise(lhs, rhs, |a, b| a.wrapping_shr(b as u32)),
        Eq | Ne | Lt | Le | Gt | Ge => compare(op, lhs, rhs),
        LAnd => Ok(Value::Int((truthy(&lhs)? && truthy(&rhs)?) as i32)),
        LOr => Ok(Value::Int((truthy(&lhs)? || truthy(&rhs)?) as i32)),
    }
}

/// `+`: numeric add (Integer wraps; a Double promotes) or string concatenation.
fn add(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (&lhs, &rhs) {
        (Value::Str(a), Value::Str(b)) => {
            let mut s = a.clone();
            s.extend_from_slice(b);
            Ok(Value::Str(s))
        }
        _ if lhs.is_numeric() && rhs.is_numeric() => {
            num_arith(lhs, rhs, i32::wrapping_add, |a, b| a + b)
        }
        _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    }
}

/// Integer-wrapping `+`/`-`/`*`; promote to Double if either operand is a Double.
fn num_arith(
    lhs: Value,
    rhs: Value,
    fi: fn(i32, i32) -> i32,
    fr: fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (&lhs, &rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(fi(*a, *b))),
        _ => {
            let (x, y) = (lhs.to_real()?, rhs.to_real()?);
            Ok(Value::Real(fr(x, y)))
        }
    }
}

/// `DIV`/`MOD`: truncate both operands toward zero to `i32`, then integer
/// quotient/remainder. A (truncated) zero divisor → errnum 7.
fn int_div_mod(lhs: Value, rhs: Value, is_div: bool) -> Result<Value, RuntimeError> {
    let (x, y) = (lhs.to_int()?, rhs.to_int()?);
    if y == 0 {
        return Err(RuntimeError::new(ERR_DIVIDE_BY_ZERO));
    }
    Ok(Value::Int(if is_div {
        x.wrapping_div(y)
    } else {
        x.wrapping_rem(y)
    }))
}

/// `AND`/`OR`/`XOR`/`<<`/`>>`: truncate both operands toward zero to `i32`, then the
/// bitwise op. The result is always Integer.
fn bitwise(lhs: Value, rhs: Value, f: fn(i32, i32) -> i32) -> Result<Value, RuntimeError> {
    let (x, y) = (lhs.to_int()?, rhs.to_int()?);
    Ok(Value::Int(f(x, y)))
}

/// Comparisons yield Integer `1`/`0`. Strings compare by UTF-16 code units; numbers
/// by `f64` value; a string-vs-number comparison is Type mismatch (errnum 8).
fn compare(op: BinOp, lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let ord = match (&lhs, &rhs) {
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        _ if lhs.is_numeric() && rhs.is_numeric() => {
            let (x, y) = (lhs.to_real()?, rhs.to_real()?);
            x.partial_cmp(&y).unwrap_or(Ordering::Greater)
        }
        _ => return Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    };
    let res = match op {
        BinOp::Eq => ord == Ordering::Equal,
        BinOp::Ne => ord != Ordering::Equal,
        BinOp::Lt => ord == Ordering::Less,
        BinOp::Le => ord != Ordering::Greater,
        BinOp::Gt => ord == Ordering::Greater,
        BinOp::Ge => ord != Ordering::Less,
        _ => unreachable!("compare only handles comparison ops"),
    };
    Ok(Value::Int(res as i32))
}

/// Evaluate a unary operator.
fn unary(op: UnOp, v: Value) -> Result<Value, RuntimeError> {
    match op {
        UnOp::Neg => match v {
            Value::Int(i) => Ok(Value::Int(i.wrapping_neg())),
            Value::Real(r) => Ok(Value::Real(-r)),
            _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
        },
        // NOT: bitwise complement of the integer-truncated operand.
        UnOp::Not => Ok(Value::Int(!v.to_int()?)),
        // `!`: logical not — 1 if the operand is zero, else 0.
        UnOp::LNot => Ok(Value::Int((v.to_real()? == 0.0) as i32)),
    }
}

/// Truthiness of a value: a nonzero number is true (string → Type mismatch).
fn truthy(v: &Value) -> Result<bool, RuntimeError> {
    Ok(v.to_real()? != 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::parser::parse;

    /// Compile + run a program to completion, returning the VM for inspection.
    fn run(src: &str) -> Vm {
        let ast = parse(src).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect("run");
        vm
    }

    fn run_err(src: &str) -> VmError {
        let ast = parse(src).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect_err("expected a runtime error")
    }

    /// Compile + run with the M1-T7 builtin registry (so bare names like `PI` resolve
    /// as calls), returning the VM for inspection.
    fn run_b(src: &str) -> Vm {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect("run");
        vm
    }

    /// Compile (with the builtin registry) + run, returning the error it halts with.
    fn run_b_err(src: &str) -> VmError {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect_err("expected a runtime error")
    }

    // ---- ASSERT__ (test-mode builtin, M1-T14) ----

    #[test]
    fn assert_true_is_a_noop() {
        // A truthy condition lets the program continue to the PRINT.
        let vm = run_b(r#"ASSERT__ 1,"msg":PRINT "OK""#);
        assert_eq!(vm.console_text(), "OK");
    }

    #[test]
    fn assert_false_halts_with_message_and_line() {
        let err = run_b_err("PRINT 1\nASSERT__ 2==3,\"twothree\"\nPRINT 9");
        match err {
            VmError::Assert { message, line } => {
                assert_eq!(message, "twothree");
                assert_eq!(line, 2); // the ASSERT__ is on source line 2
            }
            other => panic!("expected VmError::Assert, got {other:?}"),
        }
        // A failed assertion is NOT a SmileBASIC error, so it carries no ERRNUM.
        assert_eq!(run_b_err(r#"ASSERT__ 0,"x""#).errnum(), None);
    }

    #[test]
    fn assert_message_is_optional() {
        // One-arg form: false condition still halts, with an empty message.
        match run_b_err("ASSERT__ 0") {
            VmError::Assert { message, .. } => assert_eq!(message, ""),
            other => panic!("expected VmError::Assert, got {other:?}"),
        }
    }

    #[test]
    fn assert_bad_arg_count_is_illegal_function_call() {
        // Zero / three args → Illegal function call (errnum 4).
        assert_eq!(run_b_err("ASSERT__").errnum(), Some(4));
        assert_eq!(run_b_err(r#"ASSERT__ 1,"a",2"#).errnum(), Some(4));
    }

    #[test]
    fn assert_string_condition_is_type_mismatch() {
        assert_eq!(run_b_err(r#"ASSERT__ "x","msg""#).errnum(), Some(8));
    }

    /// Read a string global (`A$`) as a Rust `String`.
    fn string(vm: &Vm, ident: &str) -> String {
        match vm.global_value(ident, Suffix::Str).expect("var exists") {
            Value::Str(u) => String::from_utf16_lossy(&u),
            other => panic!("{ident}$ is not Str: {other:?}"),
        }
    }

    fn int(vm: &Vm, name: &str) -> i32 {
        match vm.global_value(name, Suffix::None).expect("var exists") {
            Value::Int(i) => i,
            other => panic!("{name} is not Int: {other:?}"),
        }
    }

    fn real(vm: &Vm, name: &str) -> f64 {
        match vm.global_value(name, Suffix::None).expect("var exists") {
            Value::Real(r) => r,
            other => panic!("{name} is not Real: {other:?}"),
        }
    }

    // ---- arithmetic & precedence ----

    #[test]
    fn arithmetic_with_precedence() {
        // Folded at parse time, but exercises Push/PopVar.
        let vm = run("A=2+3*4");
        assert_eq!(int(&vm, "A"), 14);
    }

    #[test]
    fn runtime_operators_via_variables() {
        // Not folded (variable operands) -> exercises Operate at runtime.
        let vm = run("A=3\nB=4\nC=A+B*2\nD=A*A-B");
        assert_eq!(int(&vm, "C"), 11);
        assert_eq!(int(&vm, "D"), 5);
    }

    #[test]
    fn integer_arithmetic_wraps() {
        // i32 wrap on multiply (2_000_000_000 * 2 overflows).
        let vm = run("A=2000000000\nB=A*2");
        assert_eq!(int(&vm, "B"), 2_000_000_000i32.wrapping_mul(2));
    }

    #[test]
    fn slash_is_real_division() {
        let vm = run("A=7\nB=2\nC=A/B");
        assert_eq!(real(&vm, "C"), 3.5);
    }

    #[test]
    fn div_mod_truncate_operands_first() {
        // 7 DIV 2 == 3, -7 DIV 2 == -3 (toward zero); 7 MOD 3 == 1, -7 MOD 3 == -1.
        let vm = run("A=7 DIV 2\nB=-7 DIV 2\nC=7 MOD 3\nD=-7 MOD 3");
        assert_eq!(int(&vm, "A"), 3);
        assert_eq!(int(&vm, "B"), -3);
        assert_eq!(int(&vm, "C"), 1);
        assert_eq!(int(&vm, "D"), -1);
    }

    #[test]
    fn bitwise_truncates_doubles() {
        // 7 AND 2.9 == 2 (operand truncation toward zero, not rounding).
        let vm = run("X=2.9\nA=7 AND X\nB=128 OR &HA3\nC=100 XOR &H4C");
        assert_eq!(int(&vm, "A"), 2);
        assert_eq!(int(&vm, "B"), 163);
        assert_eq!(int(&vm, "C"), 40);
    }

    #[test]
    fn divide_by_zero_is_errnum_7() {
        assert_eq!(run_err("A=1\nB=A/0").errnum(), Some(7));
        assert_eq!(run_err("A=1\nB=A DIV 0").errnum(), Some(7));
        assert_eq!(run_err("A=1\nB=A MOD 0").errnum(), Some(7));
    }

    #[test]
    fn string_in_arithmetic_is_type_mismatch() {
        assert_eq!(
            run_err(
                r#"A$="x"
B=A$+1"#
            )
            .errnum(),
            Some(8)
        );
    }

    #[test]
    fn string_concatenation() {
        let vm = run(r#"A$="foo"
B$="bar"
C$=A$+B$"#);
        match vm.global_value("C", Suffix::Str).unwrap() {
            Value::Str(s) => assert_eq!(String::from_utf16_lossy(&s), "foobar"),
            other => panic!("not a string: {other:?}"),
        }
    }

    #[test]
    fn comparisons_yield_one_or_zero() {
        let vm = run("A=3\nB=4\nLT=A<B\nGE=A>=B\nEQ=A==3");
        assert_eq!(int(&vm, "LT"), 1);
        assert_eq!(int(&vm, "GE"), 0);
        assert_eq!(int(&vm, "EQ"), 1);
    }

    #[test]
    fn unary_neg_and_not() {
        let vm = run("A=5\nB=-A\nC=NOT 0\nD=!A");
        assert_eq!(int(&vm, "B"), -5);
        assert_eq!(int(&vm, "C"), !0); // bitwise complement of 0 = -1
        assert_eq!(int(&vm, "D"), 0); // logical not of nonzero
    }

    // ---- coercion on assignment ----

    #[test]
    fn integer_suffix_truncates_toward_zero() {
        // A%=2.7 -> 2 (hw_verified coercion, exercised through PopVar).
        let vm = run("A%=2.7\nB%=-2.7");
        assert_eq!(vm.global_value("A", Suffix::Int).unwrap(), Value::Int(2));
        assert_eq!(vm.global_value("B", Suffix::Int).unwrap(), Value::Int(-2));
    }

    // ---- control flow ----

    #[test]
    fn if_then_else() {
        let vm = run("A=5\nIF A>3 THEN B=1 ELSE B=2");
        assert_eq!(int(&vm, "B"), 1);
        let vm = run("A=1\nIF A>3 THEN B=1 ELSE B=2");
        assert_eq!(int(&vm, "B"), 2);
    }

    #[test]
    fn for_loop_sums() {
        let vm = run("S=0\nFOR I=1 TO 10\nS=S+I\nNEXT");
        assert_eq!(int(&vm, "S"), 55);
    }

    #[test]
    fn for_loop_negative_step() {
        let vm = run("S=0\nFOR I=5 TO 1 STEP -1\nS=S+I\nNEXT");
        assert_eq!(int(&vm, "S"), 15);
        assert_eq!(int(&vm, "I"), 0); // last decrement steps past the bound
    }

    #[test]
    fn while_loop() {
        let vm = run("I=0\nS=0\nWHILE I<5\nS=S+I\nI=I+1\nWEND");
        assert_eq!(int(&vm, "S"), 10);
    }

    #[test]
    fn repeat_until() {
        let vm = run("I=0\nS=0\nREPEAT\nS=S+I\nI=I+1\nUNTIL I>=5");
        assert_eq!(int(&vm, "S"), 10);
    }

    #[test]
    fn break_and_continue() {
        // sum 1..10 but skip 5 and stop at 8.
        let vm = run("S=0\nFOR I=1 TO 10\nIF I==5 THEN CONTINUE\nIF I>8 THEN BREAK\nS=S+I\nNEXT");
        // 1+2+3+4+6+7+8 = 31
        assert_eq!(int(&vm, "S"), 31);
    }

    #[test]
    fn short_circuit_and_or() {
        let vm = run("A=1\nB=0\nX=A&&B\nY=A||B");
        assert_eq!(int(&vm, "X"), 0);
        assert_eq!(int(&vm, "Y"), 1);
    }

    // ---- GOSUB / ON ----

    #[test]
    fn gosub_return() {
        let vm = run("A=0\nGOSUB @SUB\nA=A+10\nEND\n@SUB\nA=5\nRETURN");
        assert_eq!(int(&vm, "A"), 15);
    }

    #[test]
    fn return_without_gosub_is_errnum_30() {
        assert_eq!(run_err("RETURN").errnum(), Some(30));
    }

    #[test]
    fn on_goto_selects_branch() {
        // index 1 selects the second target (0-based).
        let vm = run("N=1\nR=0\nON N GOTO @A,@B\n@A\nR=10\nEND\n@B\nR=20\nEND");
        assert_eq!(int(&vm, "R"), 20);
    }

    #[test]
    fn on_goto_out_of_range_falls_through() {
        let vm = run("N=5\nR=0\nON N GOTO @A,@B\nR=99\nEND\n@A\nR=10\nEND\n@B\nR=20\nEND");
        assert_eq!(int(&vm, "R"), 99);
    }

    #[test]
    fn on_gosub_returns() {
        let vm = run("N=0\nR=0\nON N GOSUB @A\nR=R+1\nEND\n@A\nR=100\nRETURN");
        assert_eq!(int(&vm, "R"), 101);
    }

    // ---- arrays ----

    #[test]
    fn array_declare_store_read() {
        let vm = run("DIM A[5]\nA[2]=7\nB=A[2]");
        assert_eq!(int(&vm, "B"), 7);
    }

    #[test]
    fn array_2d_row_major() {
        let vm = run("DIM P[3,2]\nP[2,1]=9\nB=P[2,1]");
        assert_eq!(int(&vm, "B"), 9);
    }

    #[test]
    fn array_subscript_out_of_range_is_errnum_31() {
        assert_eq!(run_err("DIM A[3]\nB=A[3]").errnum(), Some(31));
    }

    // ---- DATA / READ / RESTORE ----

    #[test]
    fn data_read_restore() {
        let vm = run("READ A,B\nRESTORE @D\nREAD C\nDATA 10,20,30\n@D\nDATA 99");
        assert_eq!(int(&vm, "A"), 10);
        assert_eq!(int(&vm, "B"), 20);
        assert_eq!(int(&vm, "C"), 99);
    }

    #[test]
    fn out_of_data_is_errnum_13() {
        assert_eq!(run_err("READ A,B\nDATA 1").errnum(), Some(13));
    }

    #[test]
    fn bare_restore_is_type_mismatch_8() {
        // Argument-less RESTORE has no reset-to-first form on real SB 3.6.0; it
        // raises Type mismatch (8) at runtime — hw_verified (restore.yaml, oracle
        // 2026-06-23: bare `RESTORE` halts with errnum 8 where `RESTORE @D1` works).
        let e = run_err("READ A:RESTORE:READ B:@D1:DATA 7");
        assert_eq!(e.errnum(), Some(8));
        assert_eq!(e.errline(), Some(1));
    }

    // ---- INC / SWAP ----

    #[test]
    fn inc_and_dec() {
        let vm = run("A=5\nINC A\nINC A,3\nDEC A,2");
        assert_eq!(int(&vm, "A"), 7);
    }

    #[test]
    fn swap_scalars() {
        let vm = run("A=1\nB=2\nSWAP A,B");
        assert_eq!(int(&vm, "A"), 2);
        assert_eq!(int(&vm, "B"), 1);
    }

    // ---- DEF functions ----

    #[test]
    fn def_value_function() {
        let vm = run("DEF SQ(X)\nRETURN X*X\nEND\nA=SQ(5)");
        assert_eq!(int(&vm, "A"), 25);
    }

    #[test]
    fn def_with_out_param() {
        let vm = run("DEF HALF X OUT Y\nY=X/2\nEND\nHALF 10 OUT R");
        assert_eq!(real(&vm, "R"), 5.0);
    }

    #[test]
    fn def_recursion_factorial() {
        let vm = run("DEF FACT(N)\nIF N<=1 THEN RETURN 1\nRETURN N*FACT(N-1)\nEND\nA=FACT(5)");
        assert_eq!(int(&vm, "A"), 120);
    }

    #[test]
    fn unbounded_recursion_is_stack_overflow() {
        // A DEF that always recurses must trip Stack overflow (errnum 5).
        let err = run_err("DEF LOOP\nLOOP\nEND\nLOOP");
        assert_eq!(err.errnum(), Some(5));
    }

    #[test]
    fn errline_points_at_failing_line() {
        // The divide-by-zero is on line 2.
        match run_err("A=1\nB=A/0") {
            VmError::Sb { errnum, line } => {
                assert_eq!(errnum, 7);
                assert_eq!(line, 2);
            }
            other => panic!("expected Sb error, got {other:?}"),
        }
    }

    // ---- error model: ERRNUM / ERRLINE / ERRPRG sysvars (M1-T13) ----

    /// Compile + run, returning the VM even when the run halts with an error (so the
    /// post-halt error-state residue can be inspected). Uses the builtin registry.
    fn run_to_halt(src: &str) -> Vm {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        let _ = vm.run();
        vm
    }

    #[test]
    fn errnum_for_documented_error_cases() {
        // hw_verified codes (errors.yaml / error-model.md): type mismatch, divide by
        // zero, subscript out of range, illegal function call, out of range.
        assert_eq!(run_err("A=FLOOR(\"x\")").errnum(), Some(8));
        assert_eq!(run_err("A=1/0").errnum(), Some(7));
        assert_eq!(run_err("DIM A[3]\nA[5]=1").errnum(), Some(31));
        assert_eq!(run_err("A=ABS()").errnum(), Some(4));
        assert_eq!(run_err("A=SQR(-1)").errnum(), Some(10));
    }

    #[test]
    fn error_state_persists_after_halt() {
        // After a halting error, ERRNUM/ERRLINE/ERRPRG are readable (the DIRECT-mode
        // residue). The SQR(-1) is on line 2; single-slot → ERRPRG = 0.
        let vm = run_to_halt("A=0\nB=SQR(-1)");
        assert_eq!(vm.errnum(), 10);
        assert_eq!(vm.errline(), 2);
        assert_eq!(vm.errprg(), 0);
    }

    #[test]
    fn errnum_reads_zero_on_a_clean_run() {
        // No error yet ⇒ ERRNUM/ERRLINE/ERRPRG read 0 mid-program (errnum 0 = No Error).
        let vm = run("N=ERRNUM\nL=ERRLINE\nP=ERRPRG");
        assert_eq!(int(&vm, "N"), 0);
        assert_eq!(int(&vm, "L"), 0);
        assert_eq!(int(&vm, "P"), 0);
        // A clean END leaves ERRNUM = 0.
        assert_eq!(vm.errnum(), 0);
    }

    #[test]
    fn error_sysvars_are_read_only() {
        // Assigning to a read-only error sysvar is a Syntax error (errnum 3) at compile.
        for src in ["ERRNUM=5", "ERRLINE=1", "ERRPRG=2"] {
            let ast = parse(src).expect("parse");
            let err = crate::compiler::compile(&ast).expect_err("expected a compile error");
            assert_eq!(err.errnum, 3, "{src} should be Syntax error");
        }
    }

    // ---- builtin calls (M1-T7) ----

    #[test]
    fn math_builtins_via_assignment() {
        // Paren calls compile to CallBuiltin even without the registry.
        let vm = run("A=FLOOR(3.7)\nB=ABS(-5)\nC=MAX(3,2.5)\nD=MIN(1,2,3,4)");
        assert_eq!(real(&vm, "A"), 3.0);
        assert_eq!(int(&vm, "B"), 5);
        assert_eq!(int(&vm, "C"), 3); // MAX keeps the Integer winner's type
        assert_eq!(int(&vm, "D"), 1);
    }

    #[test]
    fn niladic_pi_call() {
        // PI() is a 0-arg paren call -> CallBuiltin, Double pi.
        let vm = run("A=PI()");
        assert!((real(&vm, "A") - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn bare_pi_resolves_with_registry() {
        // With the builtin registry, the bare name `PI` (no parens) is a call, not a var.
        let vm = run_b("A=PI*2");
        assert!((real(&vm, "A") - std::f64::consts::TAU).abs() < 1e-12);
    }

    #[test]
    fn nested_builtin_calls() {
        // SQR(POW(3,2)) = SQR(9) = 3.
        let vm = run("A=SQR(POW(3,2))");
        assert_eq!(real(&vm, "A"), 3.0);
    }

    #[test]
    fn string_builtins_via_assignment() {
        let vm = run(r#"A$=LEFT$("ABCDEF",3)
B$=MID$("ABCDEF",2,2)
C=LEN("HELLO")
D=INSTR("ABCDEF","CD")
E$=CHR$(65)
F=ASC("Z")
G$=STR$(123)
H$=HEX$(255)"#);
        assert_eq!(string(&vm, "A"), "ABC");
        assert_eq!(string(&vm, "B"), "CD");
        assert_eq!(int(&vm, "C"), 5);
        assert_eq!(int(&vm, "D"), 2);
        assert_eq!(string(&vm, "E"), "A");
        assert_eq!(int(&vm, "F"), 90);
        assert_eq!(string(&vm, "G"), "123");
        assert_eq!(string(&vm, "H"), "FF");
    }

    #[test]
    fn builtin_value_arg_errors_propagate() {
        // SQR(-1) -> Out of range (10); FLOOR("x") -> Type mismatch (8).
        assert_eq!(run_err("A=SQR(-1)").errnum(), Some(10));
        assert_eq!(run_err(r#"A=FLOOR("x")"#).errnum(), Some(8));
        assert_eq!(run_err(r#"A=LEFT$("ABC",-1)"#).errnum(), Some(10));
    }

    #[test]
    fn builtin_result_discarded_in_statement_position() {
        // A bare function call as a statement runs (and discards the result) cleanly.
        run("FLOOR(3.7)");
    }

    // ---- RNG (M1-T9): RND / RNDF / RANDOMIZE through the full program path ----

    #[test]
    fn seeded_rnd_sequence_matches_otya_golden() {
        // The `otya_test.sb3` fixture (real-SB golden): after `RANDOMIZE 0,1`, the first
        // four `RND(100)` draws are 89,33,33,52; `RNDF` ≈ 0.836095; next `RND(100)` == 66.
        let vm = run(
            "RANDOMIZE 0,1\nA=RND(100)\nB=RND(100)\nC=RND(100)\nD=RND(100)\nF=RNDF(0)\nG=RND(100)",
        );
        assert_eq!(int(&vm, "A"), 89);
        assert_eq!(int(&vm, "B"), 33);
        assert_eq!(int(&vm, "C"), 33);
        assert_eq!(int(&vm, "D"), 52);
        assert_eq!(format!("{:.6}", real(&vm, "F")), "0.836095");
        assert_eq!(int(&vm, "G"), 66);
    }

    #[test]
    fn rnd_two_arg_selects_series() {
        // The two-arg form picks the series; same seed → same golden as series 0.
        let vm = run("RANDOMIZE 5,1\nA=RND(5,100)");
        assert_eq!(int(&vm, "A"), 89);
    }

    #[test]
    fn rnd_one_is_zero() {
        let vm = run("RANDOMIZE 0,1\nA=RND(1)");
        assert_eq!(int(&vm, "A"), 0);
    }

    #[test]
    fn rng_error_conditions() {
        // RND(-1): max < 0 → Out of range (10).
        assert_eq!(run_err("A=RND(-1)").errnum(), Some(10));
        // RND(8,5): seed_id 8 out of 0-7 → Out of range (10).
        assert_eq!(run_err("A=RND(8,5)").errnum(), Some(10));
        // RND("x"): string arg → Type mismatch (8).
        assert_eq!(run_err(r#"A=RND("x")"#).errnum(), Some(8));
        // RNDF(8): seed_id out of range → 10; RNDF("x") → 8.
        assert_eq!(run_err("A=RNDF(8)").errnum(), Some(10));
        assert_eq!(run_err(r#"A=RNDF("x")"#).errnum(), Some(8));
        // RANDOMIZE 8 → 10; RANDOMIZE "x" → 8.
        assert_eq!(run_err("RANDOMIZE 8").errnum(), Some(10));
        assert_eq!(run_err(r#"RANDOMIZE "x""#).errnum(), Some(8));
    }

    // ---- console output: PRINT (M1-T8) ----

    /// Run a program and return its console text (the deterministic `stdout`).
    fn out(src: &str) -> String {
        run(src).console_text()
    }

    #[test]
    fn print_integer_and_string() {
        assert_eq!(out("PRINT 42"), "42");
        assert_eq!(out(r#"PRINT "HI""#), "HI");
        assert_eq!(out("PRINT 2*3+1"), "7");
    }

    #[test]
    fn print_negative_number_has_no_leading_space() {
        // SmileBASIC does NOT pad positive numbers with a leading space (unlike MS BASIC).
        assert_eq!(out("PRINT -5"), "-5");
        assert_eq!(out("PRINT 7"), "7");
    }

    #[test]
    fn print_real_uses_g_format() {
        assert_eq!(out("PRINT 7/2"), "3.5");
        assert_eq!(out("PRINT 1.0"), "1");
    }

    #[test]
    fn print_semicolon_concatenates_with_no_gap() {
        assert_eq!(out(r#"PRINT "A";"B""#), "AB");
        assert_eq!(out("PRINT 1;2;3"), "123");
    }

    #[test]
    fn print_comma_tabs_to_next_tabstep() {
        // TABSTEP default 4: "1" at col 0, tab to col 4, "2".
        assert_eq!(out("PRINT 1,2"), "1   2");
    }

    #[test]
    fn print_question_alias() {
        assert_eq!(out(r#"?"Q""#), "Q");
    }

    #[test]
    fn print_multiple_lines() {
        assert_eq!(out(r#"PRINT "A":PRINT "B""#), "A\nB");
    }

    #[test]
    fn print_trailing_semicolon_suppresses_newline() {
        // Two PRINTs, the first `;`-terminated, share a line.
        assert_eq!(out(r#"PRINT "A";:PRINT "B""#), "AB");
    }

    #[test]
    fn print_type_mismatch_is_errnum_8() {
        // A bare string/number mix that produces a non-printable value can't arise from a
        // literal here; PRINT of an array name is a Type mismatch.
        assert_eq!(run_err("DIM A[3]\nPRINT A").errnum(), Some(8));
    }

    // ---- console state: LOCATE / COLOR / CLS (M1-T8) ----

    #[test]
    fn locate_then_print_positions_text() {
        // `;` suppresses the trailing newline so the cursor stays where the text left it.
        let vm = run(r#"LOCATE 5,2:PRINT "X";"#);
        assert_eq!(vm.console().cell(5, 2).ch, u16::from(b'X'));
        // After printing one char the cursor advanced to col 6.
        assert_eq!((vm.console().cur_x, vm.console().cur_y), (6, 2));
    }

    #[test]
    fn color_sets_cell_palette() {
        let vm = run(r#"COLOR 3,4:PRINT "X""#);
        assert_eq!(vm.console().cell(0, 0).fg, 3);
        assert_eq!(vm.console().cell(0, 0).bg, 4);
    }

    #[test]
    fn cls_clears_console() {
        // hw_verified: PRINT then CLS leaves the console empty.
        assert_eq!(out(r#"PRINT "X":CLS"#), "");
    }

    #[test]
    fn console_command_error_conditions() {
        // hw_verified expects from the console specs.
        assert_eq!(run_err("LOCATE 51,0").errnum(), Some(10)); // X out of range
        assert_eq!(run_err("LOCATE 0,30").errnum(), Some(10)); // Y out of range
        assert_eq!(run_err("LOCATE 0,0,2000").errnum(), Some(10)); // Z out of range
        assert_eq!(run_err("LOCATE 0").errnum(), Some(4)); // single slot
        assert_eq!(run_err("COLOR 16").errnum(), Some(10)); // fg out of range
        assert_eq!(run_err("COLOR 0,16").errnum(), Some(10)); // bg out of range
        assert_eq!(run_err("CLS 0").errnum(), Some(4)); // CLS takes no args
        assert_eq!(run_err("BACKCOLOR").errnum(), Some(4)); // SET needs 1 arg
        assert_eq!(run_err("BACKCOLOR 0,1").errnum(), Some(4)); // too many
        assert_eq!(run_err("ACLS 1").errnum(), Some(4)); // 1 arg illegal
        assert_eq!(run_err("ACLS 1,1").errnum(), Some(4)); // 2 args illegal
    }

    #[test]
    fn console_commands_as_functions_error() {
        assert_eq!(run_err("A=LOCATE(0,0)").errnum(), Some(4));
        assert_eq!(run_err("A=COLOR(7)").errnum(), Some(4));
        assert_eq!(run_err("A=CLS()").errnum(), Some(4));
    }

    #[test]
    fn x_edge_50_wraps_not_panics() {
        // LOCATE 50 (off-screen edge) is legal and must not panic; printing wraps to the
        // next row, so "X" still lands on the console.
        assert!(out(r#"LOCATE 50,0:PRINT "X""#).contains('X'));
    }

    #[test]
    fn acls_runs_and_resets() {
        // The no-arg form and the corpus-verified 3-arg form both run; no error.
        run(r#"COLOR 3,4:PRINT "X":ACLS"#);
        run("ACLS 1,1,0");
    }

    #[test]
    fn backcolor_round_trips() {
        // SET then GET returns the stored color code.
        let vm = run("BACKCOLOR 12345\nA=BACKCOLOR()");
        assert_eq!(int(&vm, "A"), 12345);
    }

    // ---- INKEY$ (M1-T8) ----

    #[test]
    fn inkey_is_empty_headless() {
        // No live keyboard buffer headless → "".
        assert_eq!(out("C$=INKEY$():PRINT LEN(C$)"), "0");
        assert_eq!(run_err("C$=INKEY$(1)").errnum(), Some(4));
    }

    // ---- INPUT / LINPUT (M1-T8) ----

    /// Run with a preloaded input queue, returning the VM for inspection.
    fn run_with_input(src: &str, lines: &[&str]) -> Vm {
        let ast = parse(src).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        for l in lines {
            vm.push_input(l);
        }
        vm.run().expect("run");
        vm
    }

    #[test]
    fn input_numeric_and_string() {
        let vm = run_with_input("INPUT A\nINPUT B$", &["42", "hello"]);
        assert_eq!(int(&vm, "A"), 42);
        assert_eq!(string(&vm, "B"), "hello");
    }

    #[test]
    fn input_multiple_comma_fields() {
        let vm = run_with_input("INPUT A,B,C", &["1,2,3"]);
        assert_eq!(int(&vm, "A"), 1);
        assert_eq!(int(&vm, "B"), 2);
        assert_eq!(int(&vm, "C"), 3);
    }

    #[test]
    fn input_real_field() {
        let vm = run_with_input("INPUT R", &["3.5"]);
        assert_eq!(real(&vm, "R"), 3.5);
    }

    #[test]
    fn input_literal_receiver_is_syntax_error() {
        // `INPUT "X";1` — a literal receiver is rejected (errnum 3); the parser catches it
        // before compilation (hw_verified: real SB raises errnum 3 for a non-variable
        // receiver).
        let err = parse(r#"INPUT "X";1"#).expect_err("syntax error");
        assert_eq!(err.errnum, 3);
    }

    #[test]
    fn linput_keeps_commas() {
        let vm = run_with_input("LINPUT S$", &["a,b,c"]);
        assert_eq!(string(&vm, "S"), "a,b,c");
    }

    #[test]
    fn input_prompt_is_printed() {
        // The guide text + `?` show before the (queued) input is read.
        let vm = run_with_input(r#"INPUT "NAME";A$"#, &["bob"]);
        assert_eq!(string(&vm, "A"), "bob");
        assert!(vm.console_text().starts_with("NAME?"));
    }
}
