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
//! Builtins ([`Op::CallBuiltin`]/[`Op::CallDynamic`], M1-T7), console + input
//! (`Print*`/`Input`/`Linput`, M1-T8) and `USE`/`EXEC` (M6), plus array-element /
//! runtime-name references ([`Op::PushArrayRef`]/[`Op::PushRefExpr`]), are not yet
//! wired and raise [`VmError::Unsupported`] rather than panicking — their handlers
//! land in the milestones above.
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
use crate::bytecode::{Const, Op, Program, VarRef};
use crate::token::Suffix;
use crate::value::{swap_cells, Cell, RuntimeError, SbStr, Value};
use std::cmp::Ordering;

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
}

impl VmError {
    /// The `ERRNUM` if this is a SmileBASIC runtime error.
    pub fn errnum(&self) -> Option<u32> {
        match self {
            VmError::Sb { errnum, .. } => Some(*errnum),
            VmError::Unsupported(_) => None,
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
        }
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
                Err(e) => return Err(self.attach_line(e, here)),
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

            // -- deferred to later milestones --------------------------------------
            Op::CallBuiltin { .. } => return Err(VmError::Unsupported("builtin call (M1-T7)")),
            Op::CallDynamic { .. } => return Err(VmError::Unsupported("CALL (M1-T7)")),
            Op::PrintItem | Op::PrintTab | Op::PrintNewline => {
                return Err(VmError::Unsupported("PRINT (M1-T8)"))
            }
            Op::Input { .. } => return Err(VmError::Unsupported("INPUT (M1-T8)")),
            Op::Linput { .. } => return Err(VmError::Unsupported("LINPUT (M1-T8)")),
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
            other => other,
        }
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
}
