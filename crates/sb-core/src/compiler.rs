//! The SmileBASIC 3.6.0 compiler (M1-T5) — AST → flat bytecode.
//!
//! Walks the [`crate::ast`] tree (the parser's output) into a [`Program`]: a flat
//! `Vec<Op>` plus the link tables the stack VM (M1-T6) needs — globals, user
//! functions, the `DATA` pool, and label maps (`spec/concepts/execution-model.md`,
//! "Compilation"). It mirrors `osb/SMILEBASIC/compiler.d` **structurally only** (the
//! lowering shapes for `IF`/`FOR`/`WHILE`/`REPEAT`, the reference/array opcodes); the
//! behavior is SmileBASIC 3.6.0's, and the opcode set is our flat [`Op`] enum, not
//! osb's object-per-opcode `Code` subclasses.
//!
//! Responsibilities (per the execution model):
//!
//! - **Variable resolution** — globals get a slot-global index; a `DEF` body's
//!   params/`OUT` params/locals get a bp-relative index. `OPTION STRICT` makes an
//!   undeclared use a compile error (`Undefined variable`, errnum 15); otherwise the
//!   first use auto-declares in the current scope.
//! - **Labels** — resolved by backpatch so forward `@labels` work; an unresolved
//!   `GOTO`/`GOSUB` target is `Undefined label` (errnum 14).
//! - **Functions** — `DEF`/`COMMON DEF` bodies compile to addressed [`Function`]s
//!   appended after the main code; calls dispatch by name (so forward and
//!   cross-slot/common targets resolve at runtime).
//! - **DATA** — every `DATA` item flattens into the slot's pool in source order;
//!   `READ` walks a cursor, `RESTORE @label` repositions it.
//!
//! Calls are ambiguous in the AST: an `A(i)` paren form is an array read iff `A` is a
//! declared array, else a function call; an undeclared call name dispatches to a user
//! `DEF` if one exists, otherwise to a builtin (resolved in M1-T7 — the optional
//! [`Builtins`] predicate lets a caller pre-declare the builtin names so bare-name
//! zero-arg functions like `PI` resolve as calls rather than auto-declared variables).

use std::collections::HashSet;

use crate::ast::*;
use crate::bytecode::*;
use crate::sysvars::Sysvar;
use crate::token::{SourceLoc, Suffix};

/// A compile failure carrying the SmileBASIC error number to raise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub loc: SourceLoc,
    pub errnum: u32,
    pub msg: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Compile error (errnum {}) at line {} col {}: {}",
            self.errnum, self.loc.line, self.loc.col, self.msg
        )
    }
}

impl std::error::Error for CompileError {}

type CResult<T> = Result<T, CompileError>;

const ERR_SYNTAX: u32 = 3;
const ERR_UNDEFINED_LABEL: u32 = 14;
const ERR_UNDEFINED_VARIABLE: u32 = 15;
const ERR_DUPLICATE_VARIABLE: u32 = 18;

/// A predicate naming the builtin functions/commands known to the compiler, so a
/// bare-name use (`PI`) or paren call (`RND(8)`) of a builtin is compiled as a call
/// rather than treated as a variable. M1-T5 ships an empty default ([`NoBuiltins`]);
/// M1-T7 supplies the real registry.
pub trait Builtins {
    /// Is `name` (canonical, e.g. `MID$`) a registered builtin?
    fn is_builtin(&self, name: &str) -> bool;
}

/// The empty builtin set used by [`compile`]: no names are builtins, so every
/// undeclared identifier is a variable. Adequate for M1-T5 (the VM/builtins land in
/// M1-T6/T7).
pub struct NoBuiltins;
impl Builtins for NoBuiltins {
    fn is_builtin(&self, _name: &str) -> bool {
        false
    }
}

impl Builtins for HashSet<String> {
    fn is_builtin(&self, name: &str) -> bool {
        self.contains(name)
    }
}

/// Compile a parsed program (a top-level [`Block`]) into a [`Program`], with no
/// builtins known to the compiler. See [`compile_with`].
pub fn compile(block: &Block) -> CResult<Program> {
    compile_with(block, &NoBuiltins)
}

/// Compile a parsed program, consulting `builtins` to disambiguate builtin calls from
/// variables.
pub fn compile_with(block: &Block, builtins: &dyn Builtins) -> CResult<Program> {
    let mut c = Compiler::new(builtins);
    c.compile_program(block)?;
    Ok(c.finish())
}

/// Canonical name string (`ident` + suffix char), e.g. `MID$`, `A%`, `LOOP`.
fn canonical(name: &Name) -> String {
    let s = match name.suffix {
        Suffix::None => "",
        Suffix::Int => "%",
        Suffix::Real => "#",
        Suffix::Str => "$",
    };
    format!("{}{}", name.ident, s)
}

/// Whether a bare name is a **read-only** system variable that cannot be assigned to
/// (`spec/reference/sysvars.yaml`, M6-T3). The general system-variable surface ([`Sysvar`])
/// carries its own [`writable`](Sysvar::writable) flag; only `TABSTEP`/`SYSBEEP` are writable.
/// `HARDWARE` reads through the builtin registry (M4-T4), not [`Sysvar`], so it is gated here
/// for the assignment check. Assigning to any of these is a compile-time Syntax error (errnum 3).
fn is_readonly_sysvar(cname: &str) -> bool {
    Sysvar::from_name(cname).is_some_and(|sv| !sv.writable()) || cname == "HARDWARE"
}

/// The stack/queue ops (M1-T14) whose first operand is taken by reference so the op
/// can grow/shrink the caller's array or write a modified string scalar back.
fn is_stack_op(cname: &str) -> bool {
    matches!(cname, "PUSH" | "POP" | "SHIFT" | "UNSHIFT")
}

fn const_from_lit(lit: &Lit) -> Const {
    match lit {
        Lit::Int(i) => Const::Int(*i),
        Lit::Real(r) => Const::Real(*r),
        Lit::Str(s) => Const::str_from(s),
    }
}

/// One pending label reference to patch once the label's address is known.
struct LabelFixup {
    /// Index into `code` of the op to patch.
    op: usize,
    /// Which target inside the op (0 for single-target ops; the arm index for
    /// `OnGoto`/`OnGosub`).
    idx: usize,
    label: String,
    loc: SourceLoc,
}

/// The lexical scope of a `DEF` body being compiled.
struct FuncScope {
    name: Name,
    locals: Vec<VarInfo>,
    params: Vec<Name>,
    out_params: Vec<Name>,
    returns_value: bool,
    is_common: bool,
    /// Names explicitly declared via `DIM`/`VAR` in this function scope, to reject a
    /// second declaration of the same name → errnum 18 (Duplicate variable).
    declared: HashSet<Name>,
}

/// A loop's break/continue backpatch targets.
struct LoopCtx {
    break_fixups: Vec<usize>,
    continue_fixups: Vec<usize>,
}

struct Compiler<'a> {
    builtins: &'a dyn Builtins,
    code: Vec<Op>,
    locs: Vec<SourceLoc>,
    globals: Vec<VarInfo>,
    functions: Vec<Function>,
    data: Vec<Const>,
    data_labels: Vec<(String, usize)>,
    code_labels: Vec<(String, usize)>,
    options: OptionFlags,
    /// Names of user `DEF`s, so a call dispatches to a user function vs. a builtin.
    user_funcs: HashSet<String>,
    /// The function body currently being compiled, if any.
    func: Option<FuncScope>,
    /// Names explicitly declared via `DIM`/`VAR` at top level, to reject a second
    /// declaration of the same name → errnum 18 (Duplicate variable).
    declared_global: HashSet<Name>,
    /// Per-scope label table + pending references (reset for each function).
    labels: Vec<(String, usize)>,
    fixups: Vec<LabelFixup>,
    /// Active loop nest, for `BREAK`/`CONTINUE`.
    loops: Vec<LoopCtx>,
    /// Source location of the statement currently compiling (stamped onto ops).
    cur_loc: SourceLoc,
}

impl<'a> Compiler<'a> {
    fn new(builtins: &'a dyn Builtins) -> Self {
        Compiler {
            builtins,
            code: Vec::new(),
            locs: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            data: Vec::new(),
            data_labels: Vec::new(),
            code_labels: Vec::new(),
            options: OptionFlags::default(),
            user_funcs: HashSet::new(),
            func: None,
            declared_global: HashSet::new(),
            labels: Vec::new(),
            fixups: Vec::new(),
            loops: Vec::new(),
            cur_loc: SourceLoc::new(1, 1),
        }
    }

    fn finish(self) -> Program {
        Program {
            code: self.code,
            locs: self.locs,
            globals: self.globals,
            functions: self.functions,
            data: self.data,
            data_labels: self.data_labels,
            code_labels: self.code_labels,
            options: self.options,
        }
    }

    // -- top-level driver ------------------------------------------------------

    fn compile_program(&mut self, block: &Block) -> CResult<()> {
        // Pre-pass: OPTION flags, DATA pool + DATA labels, global arrays, user-DEF
        // names — all needed before the main pass resolves uses and dispatches calls.
        self.scan_options(block);
        self.collect_data_and_arrays(block);
        self.collect_user_funcs(block);

        // Main pass: top-level statements (DEF bodies deferred).
        let mut defs: Vec<&DefineFunction> = Vec::new();
        for stmt in block {
            if let StmtKind::Def(def) = &stmt.kind {
                defs.push(def);
                continue;
            }
            self.compile_stmt(stmt)?;
        }
        self.cur_loc = SourceLoc::new(0, 0);
        self.emit(Op::End);
        self.resolve_labels()?;

        // Compile each DEF body, appended after the main `End`.
        for def in defs {
            self.compile_function(def)?;
        }
        Ok(())
    }

    /// Scan top-level `OPTION` statements (they govern the whole compile).
    fn scan_options(&mut self, block: &Block) {
        for stmt in block {
            if let StmtKind::Option(arg) = &stmt.kind {
                match arg.to_ascii_uppercase().as_str() {
                    "STRICT" => self.options.strict = true,
                    "DEFINT" => self.options.defint = true,
                    _ => {}
                }
            }
        }
    }

    /// Walk top-level statements (recursing into block bodies, *not* `DEF` bodies) in
    /// source order to flatten the `DATA` pool, record `@label` → DATA-index for
    /// `RESTORE`, and pre-register global arrays (so a paren-form `A(i)` over a
    /// later-`DIM`'d array compiles as an array read).
    fn collect_data_and_arrays(&mut self, block: &Block) {
        for stmt in block {
            match &stmt.kind {
                StmtKind::Label(name) => {
                    self.data_labels.push((name.clone(), self.data.len()));
                }
                StmtKind::Data(items) => {
                    for lit in items {
                        self.data.push(const_from_lit(lit));
                    }
                }
                StmtKind::Dim(items) => {
                    for item in items {
                        if let DimItem::Array { name, .. } = item {
                            self.declare_global_array(name);
                        }
                    }
                }
                // Recurse into nested block bodies (DATA/labels/DIM may live there).
                StmtKind::If {
                    then_body,
                    elseifs,
                    else_body,
                    ..
                } => {
                    self.collect_data_and_arrays(then_body);
                    for ei in elseifs {
                        self.collect_data_and_arrays(&ei.body);
                    }
                    self.collect_data_and_arrays(else_body);
                }
                StmtKind::For { body, .. }
                | StmtKind::While { body, .. }
                | StmtKind::RepeatUntil { body, .. } => self.collect_data_and_arrays(body),
                _ => {}
            }
        }
    }

    /// Collect top-level `DEF` names so calls to them (even forward) dispatch as user
    /// functions rather than builtins.
    fn collect_user_funcs(&mut self, block: &Block) {
        for stmt in block {
            if let StmtKind::Def(def) = &stmt.kind {
                self.user_funcs.insert(canonical(&def.name));
            }
        }
    }

    fn declare_global_array(&mut self, name: &Name) {
        if self.globals.iter().any(|v| &v.name == name) {
            // Mark an existing entry as an array (e.g. DIM after a scalar use).
            if let Some(v) = self.globals.iter_mut().find(|v| &v.name == name) {
                v.is_array = true;
            }
        } else {
            self.globals.push(VarInfo {
                name: name.clone(),
                is_array: true,
            });
        }
    }

    // -- emission + patching ---------------------------------------------------

    /// Append an op (stamped with the current statement location) and return its index.
    fn emit(&mut self, op: Op) -> usize {
        self.code.push(op);
        self.locs.push(self.cur_loc);
        self.code.len() - 1
    }

    /// Patch a jump-like op's address (`idx` selects the arm of an `On*`).
    fn patch(&mut self, op: usize, idx: usize, addr: usize) {
        match &mut self.code[op] {
            Op::Jump(a)
            | Op::JumpFalse(a)
            | Op::JumpTrue(a)
            | Op::LogicalAnd(a)
            | Op::LogicalOr(a)
            | Op::Goto(a)
            | Op::Gosub(a)
            | Op::Restore(a) => *a = addr,
            Op::OnGoto(v) | Op::OnGosub(v) => v[idx] = addr,
            other => panic!("patch on non-jump op {other:?}"),
        }
    }

    fn here(&self) -> usize {
        self.code.len()
    }

    /// Resolve every pending label reference in the current scope, then clear the
    /// scope's label state.
    fn resolve_labels(&mut self) -> CResult<()> {
        let fixups = std::mem::take(&mut self.fixups);
        for f in fixups {
            let addr = self
                .labels
                .iter()
                .find(|(n, _)| *n == f.label)
                .map(|(_, a)| *a)
                .ok_or_else(|| CompileError {
                    loc: f.loc,
                    errnum: ERR_UNDEFINED_LABEL,
                    msg: format!("undefined label @{}", f.label),
                })?;
            self.patch(f.op, f.idx, addr);
        }
        self.labels.clear();
        Ok(())
    }

    fn add_label_fixup(&mut self, op: usize, idx: usize, label: String) {
        self.fixups.push(LabelFixup {
            op,
            idx,
            label,
            loc: self.cur_loc,
        });
    }

    // -- variable resolution ---------------------------------------------------

    /// Look up an existing variable by name (local first inside a `DEF`, then global).
    fn lookup(&self, name: &Name) -> Option<(VarRef, bool)> {
        if let Some(fs) = &self.func {
            if let Some(i) = fs.locals.iter().position(|v| &v.name == name) {
                return Some((VarRef::Local(i as u32), fs.locals[i].is_array));
            }
        }
        self.globals
            .iter()
            .position(|v| &v.name == name)
            .map(|i| (VarRef::Global(i as u32), self.globals[i].is_array))
    }

    /// Resolve a scalar variable for read or write, auto-declaring it in the current
    /// scope unless `OPTION STRICT` forbids undeclared use.
    fn resolve_scalar(&mut self, name: &Name) -> CResult<VarRef> {
        if let Some((vref, _)) = self.lookup(name) {
            return Ok(vref);
        }
        if self.options.strict {
            return Err(self.err(
                ERR_UNDEFINED_VARIABLE,
                format!("undefined variable {}", canonical(name)),
            ));
        }
        Ok(self.declare(name, false))
    }

    /// Resolve an array variable, auto-declaring it (as an array) unless strict.
    fn resolve_array(&mut self, name: &Name) -> CResult<VarRef> {
        if let Some((vref, _)) = self.lookup(name) {
            return Ok(vref);
        }
        if self.options.strict {
            return Err(self.err(
                ERR_UNDEFINED_VARIABLE,
                format!("undefined array {}", canonical(name)),
            ));
        }
        Ok(self.declare(name, true))
    }

    /// Declare a variable in the current scope (function-local inside a `DEF`, else
    /// global) and return its reference.
    fn declare(&mut self, name: &Name, is_array: bool) -> VarRef {
        let info = VarInfo {
            name: name.clone(),
            is_array,
        };
        if let Some(fs) = &mut self.func {
            fs.locals.push(info);
            VarRef::Local((fs.locals.len() - 1) as u32)
        } else {
            self.globals.push(info);
            VarRef::Global((self.globals.len() - 1) as u32)
        }
    }

    /// Declare a variable via a `DIM`/`VAR` declaration. Inside a `DEF` this ALWAYS
    /// binds a function-local, *shadowing* any same-named global — hw_verified
    /// (sb-oracle 2026-06-23): `DIM`/`VAR` of a name that also exists as a global
    /// creates a fresh local, leaving the global untouched (`A=5` then a DEF doing
    /// `DIM A[3]`/`VAR A=99` leaves the top-level `A` == 5). A repeated declaration of
    /// a name already local to this `DEF` (e.g. a param) reuses that slot. Contrast
    /// [`Self::resolve_scalar`]/[`resolve_array`], used for plain references, which fall
    /// through to a global if one exists (a plain `A=99` inside a DEF *does* write the
    /// global — hw_verified). At top level a declaration binds/updates the global.
    fn declare_decl(&mut self, name: &Name, is_array: bool) -> VarRef {
        if let Some(fs) = &mut self.func {
            if let Some(i) = fs.locals.iter().position(|v| &v.name == name) {
                if is_array {
                    fs.locals[i].is_array = true;
                }
                return VarRef::Local(i as u32);
            }
            fs.locals.push(VarInfo {
                name: name.clone(),
                is_array,
            });
            return VarRef::Local((fs.locals.len() - 1) as u32);
        }
        if let Some(i) = self.globals.iter().position(|v| &v.name == name) {
            if is_array {
                self.globals[i].is_array = true;
            }
            return VarRef::Global(i as u32);
        }
        self.globals.push(VarInfo {
            name: name.clone(),
            is_array,
        });
        VarRef::Global((self.globals.len() - 1) as u32)
    }

    /// Record an explicit `DIM`/`VAR` declaration of `name` in the current scope. A
    /// second explicit declaration of the same name (the suffix is part of identity,
    /// so `A` and `A%` are distinct) raises errnum 18 (Duplicate variable) at compile
    /// time — hw_verified (sb-oracle 2026-06-22 s_t4a: `VAR Q=1:VAR Q=2` → 18). Only
    /// `DIM`/`VAR` declarations are tracked here; params and auto-declared
    /// (plain-reference) names are not, so re-declaring over them does not trip 18.
    fn note_declaration(&mut self, name: &Name) -> CResult<()> {
        let fresh = match &mut self.func {
            Some(fs) => fs.declared.insert(name.clone()),
            None => self.declared_global.insert(name.clone()),
        };
        if !fresh {
            return Err(self.err(
                ERR_DUPLICATE_VARIABLE,
                format!("duplicate variable {}", canonical(name)),
            ));
        }
        Ok(())
    }

    fn err(&self, errnum: u32, msg: String) -> CompileError {
        CompileError {
            loc: self.cur_loc,
            errnum,
            msg,
        }
    }

    // -- statements ------------------------------------------------------------

    fn compile_block(&mut self, block: &Block) -> CResult<()> {
        for stmt in block {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> CResult<()> {
        self.cur_loc = stmt.loc;
        match &stmt.kind {
            StmtKind::Assign { name, expr } => {
                let cname = canonical(name);
                if let Some(sv) = Sysvar::from_name(&cname).filter(|sv| sv.writable()) {
                    // A writable system variable (TABSTEP/SYSBEEP, M6-T3): the assignment takes
                    // effect on the VM's state rather than declaring a user variable.
                    self.compile_expr(expr)?;
                    self.emit(Op::StoreSysvar(sv));
                } else if is_readonly_sysvar(&cname) {
                    // The read-only system variables (ERRNUM/ERRLINE/ERRPRG, M1-T13; MAINCNT,
                    // M4-T3; VERSION/FREEMEM/CSR*/…, M6-T3; HARDWARE, M4-T4) reject assignment
                    // (sysvars.yaml writable=false): a Syntax error (errnum 3), not a write.
                    return Err(self.err(
                        ERR_SYNTAX,
                        format!("cannot assign to read-only system variable {cname}"),
                    ));
                } else {
                    self.compile_expr(expr)?;
                    let v = self.resolve_scalar(name)?;
                    self.emit(Op::PopVar(v));
                }
            }
            StmtKind::ArrayAssign {
                name,
                indices,
                expr,
            } => {
                self.compile_expr(expr)?;
                for ix in indices {
                    self.compile_expr(ix)?;
                }
                let v = self.resolve_array(name)?;
                self.emit(Op::PopArray {
                    var: v,
                    dims: indices.len() as u8,
                });
            }
            StmtKind::AssignRef { target, expr } => {
                self.compile_expr(expr)?;
                self.compile_push_ref(target)?;
                self.emit(Op::PopRef);
            }
            StmtKind::Call {
                name,
                args,
                out_args,
            } => self.compile_call_stmt(name, args, out_args)?,
            StmtKind::Print(items) => {
                for item in items {
                    match item {
                        PrintItem::Expr(e) => {
                            self.compile_expr(e)?;
                            self.emit(Op::PrintItem);
                        }
                        PrintItem::Tab => {
                            self.emit(Op::PrintTab);
                        }
                        PrintItem::NewLine => {
                            self.emit(Op::PrintNewline);
                        }
                    }
                }
            }
            StmtKind::Label(name) => {
                let addr = self.here();
                self.labels.push((name.clone(), addr));
                self.code_labels.push((name.clone(), addr));
            }
            StmtKind::Goto(jump) => match jump {
                Jump::Label(l) => {
                    let op = self.emit(Op::Goto(0));
                    self.add_label_fixup(op, 0, l.clone());
                }
                Jump::Computed(e) => {
                    self.compile_expr(e)?;
                    self.emit(Op::GotoExpr);
                }
            },
            StmtKind::Gosub(jump) => match jump {
                Jump::Label(l) => {
                    let op = self.emit(Op::Gosub(0));
                    self.add_label_fixup(op, 0, l.clone());
                }
                Jump::Computed(e) => {
                    self.compile_expr(e)?;
                    self.emit(Op::GosubExpr);
                }
            },
            StmtKind::Return(value) => self.compile_return(value)?,
            StmtKind::On {
                value,
                kind,
                labels,
            } => self.compile_on(value, *kind, labels)?,
            StmtKind::If {
                cond,
                then_body,
                elseifs,
                else_body,
            } => self.compile_if(cond, then_body, elseifs, else_body)?,
            StmtKind::For {
                var,
                from,
                to,
                step,
                body,
            } => self.compile_for(var, from, to, step.as_ref(), body)?,
            StmtKind::While { cond, body } => self.compile_while(cond, body)?,
            StmtKind::RepeatUntil { body, cond } => self.compile_repeat(body, cond)?,
            StmtKind::End => {
                self.emit(Op::End);
            }
            StmtKind::Stop => {
                self.emit(Op::Stop);
            }
            StmtKind::Break => {
                let op = self.emit(Op::Jump(0));
                self.loops
                    .last_mut()
                    .ok_or_else(|| CompileError {
                        loc: stmt.loc,
                        errnum: ERR_SYNTAX,
                        msg: "BREAK outside a loop".into(),
                    })?
                    .break_fixups
                    .push(op);
            }
            StmtKind::Continue => {
                let op = self.emit(Op::Jump(0));
                self.loops
                    .last_mut()
                    .ok_or_else(|| CompileError {
                        loc: stmt.loc,
                        errnum: ERR_SYNTAX,
                        msg: "CONTINUE outside a loop".into(),
                    })?
                    .continue_fixups
                    .push(op);
            }
            StmtKind::Dim(items) => self.compile_dim(items)?,
            StmtKind::Def(def) => {
                // A nested DEF (the top-level driver handles top-level ones): record
                // and compile it after the current run of code, like the top level.
                self.user_funcs.insert(canonical(&def.name));
                let def = def.clone();
                // Defer is awkward mid-stream; compile inline behind a skip jump.
                let skip = self.emit(Op::Jump(0));
                self.compile_function(&def)?;
                let after = self.here();
                self.patch(skip, 0, after);
            }
            StmtKind::Data(_) => { /* collected in the pre-pass */ }
            StmtKind::Read(vars) => {
                for v in vars {
                    self.emit(Op::ReadValue);
                    self.compile_pop_target(v)?;
                }
            }
            StmtKind::Restore(None) => {
                // Argument-less `RESTORE`: real SB 3.6.0 evaluates a (missing) label
                // argument and fails its string-type check → Type mismatch (8) at
                // runtime. Push a non-string placeholder and let `RestoreExpr`'s
                // `as_str` raise 8 at the RESTORE line (hw_verified, restore.yaml).
                self.emit(Op::Push(Const::Int(0)));
                self.emit(Op::RestoreExpr);
            }
            StmtKind::Restore(Some(jump)) => match jump {
                Jump::Label(l) => {
                    let idx = self
                        .data_labels
                        .iter()
                        .find(|(n, _)| n == l)
                        .map(|(_, i)| *i)
                        .ok_or_else(|| {
                            self.err(ERR_UNDEFINED_LABEL, format!("undefined label @{l}"))
                        })?;
                    self.emit(Op::Restore(idx));
                }
                Jump::Computed(e) => {
                    self.compile_expr(e)?;
                    self.emit(Op::RestoreExpr);
                }
            },
            StmtKind::Input {
                prompt,
                question,
                vars,
            } => {
                if let Some(p) = prompt {
                    self.compile_expr(p)?;
                }
                let types: Vec<VarType> = vars.iter().map(input_target_type).collect();
                self.emit(Op::Input {
                    count: vars.len() as u8,
                    question: *question,
                    has_prompt: prompt.is_some(),
                    types,
                });
                for v in vars {
                    self.compile_pop_target(v)?;
                }
            }
            StmtKind::Linput { prompt, var } => {
                if let Some(p) = prompt {
                    self.compile_expr(p)?;
                }
                self.emit(Op::Linput {
                    has_prompt: prompt.is_some(),
                });
                self.compile_pop_target(var)?;
            }
            StmtKind::Inc { target, delta } => {
                self.compile_expr(delta)?;
                self.compile_push_ref(target)?;
                self.emit(Op::IncRef);
            }
            StmtKind::Swap { a, b } => {
                let a_suffix = self.lvalue_suffix(a);
                let b_suffix = self.lvalue_suffix(b);
                self.compile_push_ref(a)?;
                self.compile_push_ref(b)?;
                self.emit(Op::Swap {
                    a: a_suffix,
                    b: b_suffix,
                });
            }
            StmtKind::Option(_) => { /* handled in scan_options */ }
            StmtKind::Use(e) => {
                self.compile_expr(e)?;
                self.emit(Op::Use);
            }
            StmtKind::Exec(e) => {
                self.compile_expr(e)?;
                self.emit(Op::Exec);
            }
        }
        Ok(())
    }

    fn compile_dim(&mut self, items: &[DimItem]) -> CResult<()> {
        for item in items {
            match item {
                DimItem::Scalar { name, init } => {
                    // A second explicit declaration of the same name → errnum 18.
                    self.note_declaration(name)?;
                    // A VAR declaration binds the name in the current scope (local inside
                    // a DEF, shadowing any global; see `declare_decl`).
                    let v = self.declare_decl(name, false);
                    if let Some(e) = init {
                        self.compile_expr(e)?;
                        self.emit(Op::PopVar(v));
                    }
                }
                DimItem::Array { name, dims } => {
                    // A second explicit declaration of the same name → errnum 18.
                    self.note_declaration(name)?;
                    for d in dims {
                        self.compile_expr(d)?;
                    }
                    // A DIM declaration binds a fresh array in the current scope (local
                    // inside a DEF, shadowing any same-named global; see `declare_decl`).
                    let v = self.declare_decl(name, true);
                    self.emit(Op::NewArray {
                        var: v,
                        ty: VarType::from_suffix(name.suffix),
                        dims: dims.len() as u8,
                    });
                }
            }
        }
        Ok(())
    }

    fn compile_return(&mut self, value: &Option<Expr>) -> CResult<()> {
        match &self.func {
            None => {
                // Top-level RETURN ends a GOSUB; it carries no value.
                if value.is_some() {
                    return Err(self.err(ERR_SYNTAX, "RETURN value outside a function".into()));
                }
                self.emit(Op::Return);
            }
            Some(fs) => {
                let returns_value = fs.returns_value;
                if returns_value {
                    match value {
                        Some(e) => self.compile_expr(e)?,
                        None => {
                            self.emit(Op::PushVoid);
                        }
                    };
                    self.emit(Op::ReturnFunc { has_value: true });
                } else {
                    self.emit(Op::ReturnFunc { has_value: false });
                }
            }
        }
        Ok(())
    }

    fn compile_on(&mut self, value: &Expr, kind: OnKind, labels: &[Jump]) -> CResult<()> {
        self.compile_expr(value)?;
        let targets = vec![0usize; labels.len()];
        let op = self.emit(match kind {
            OnKind::Goto => Op::OnGoto(targets),
            OnKind::Gosub => Op::OnGosub(targets),
        });
        for (i, j) in labels.iter().enumerate() {
            match j {
                Jump::Label(l) => self.add_label_fixup(op, i, l.clone()),
                Jump::Computed(_) => {
                    return Err(self.err(
                        ERR_SYNTAX,
                        "ON .. GOTO/GOSUB requires literal labels".into(),
                    ))
                }
            }
        }
        Ok(())
    }

    fn compile_if(
        &mut self,
        cond: &Expr,
        then_body: &Block,
        elseifs: &[ElseIf],
        else_body: &Block,
    ) -> CResult<()> {
        self.compile_expr(cond)?;
        let jf = self.emit(Op::JumpFalse(0));
        self.compile_block(then_body)?;

        if elseifs.is_empty() && else_body.is_empty() {
            let end = self.here();
            self.patch(jf, 0, end);
            return Ok(());
        }

        // Jump from the end of each taken arm to the shared ENDIF.
        let mut to_end: Vec<usize> = Vec::new();
        to_end.push(self.emit(Op::Jump(0)));
        let mut prev_false = jf;

        for ei in elseifs {
            let here = self.here();
            self.patch(prev_false, 0, here);
            self.compile_expr(&ei.cond)?;
            prev_false = self.emit(Op::JumpFalse(0));
            self.compile_block(&ei.body)?;
            to_end.push(self.emit(Op::Jump(0)));
        }

        // The last false-branch lands on ELSE (or ENDIF if no ELSE).
        let else_start = self.here();
        self.patch(prev_false, 0, else_start);
        self.compile_block(else_body)?;

        let end = self.here();
        for j in to_end {
            self.patch(j, 0, end);
        }
        Ok(())
    }

    fn compile_for(
        &mut self,
        var: &Name,
        from: &Expr,
        to: &Expr,
        step: Option<&Expr>,
        body: &Block,
    ) -> CResult<()> {
        // counter = from
        self.compile_expr(from)?;
        let counter = self.resolve_scalar(var)?;
        self.emit(Op::PopVar(counter));

        let for_start = self.here();
        // Branch on the sign of STEP, comparing direction accordingly (mirrors
        // osb compileFor; STEP/TO are re-evaluated each iteration — queued for the
        // oracle to confirm vs once-at-entry).
        self.compile_step(step)?;
        self.emit(Op::Push(Const::Int(0)));
        self.emit(Op::Operate(BinOp::Ge)); // step >= 0 ?
        let positive = self.emit(Op::JumpTrue(0));

        // Negative step: break when to > counter.
        self.compile_expr(to)?;
        self.emit(Op::PushVar(counter));
        self.emit(Op::Operate(BinOp::Gt));
        let break_neg = self.emit(Op::JumpTrue(0));
        let to_body = self.emit(Op::Jump(0));

        // Positive step: break when to < counter.
        let pos_addr = self.here();
        self.patch(positive, 0, pos_addr);
        self.compile_expr(to)?;
        self.emit(Op::PushVar(counter));
        self.emit(Op::Operate(BinOp::Lt));
        let break_pos = self.emit(Op::JumpTrue(0));

        let body_addr = self.here();
        self.patch(to_body, 0, body_addr);

        self.loops.push(LoopCtx {
            break_fixups: Vec::new(),
            continue_fixups: Vec::new(),
        });
        self.compile_block(body)?;
        let ctx = self.loops.pop().unwrap();

        // CONTINUE lands here: counter = counter + step, then loop.
        let cont_addr = self.here();
        self.emit(Op::PushVar(counter));
        self.compile_step(step)?;
        self.emit(Op::Operate(BinOp::Add));
        self.emit(Op::PopVar(counter));
        self.emit(Op::Jump(for_start));

        let end = self.here();
        self.patch(break_neg, 0, end);
        self.patch(break_pos, 0, end);
        for j in ctx.break_fixups {
            self.patch(j, 0, end);
        }
        for j in ctx.continue_fixups {
            self.patch(j, 0, cont_addr);
        }
        Ok(())
    }

    /// Push the loop STEP expression, defaulting to `1` when omitted.
    fn compile_step(&mut self, step: Option<&Expr>) -> CResult<()> {
        match step {
            Some(e) => self.compile_expr(e),
            None => {
                self.emit(Op::Push(Const::Int(1)));
                Ok(())
            }
        }
    }

    fn compile_while(&mut self, cond: &Expr, body: &Block) -> CResult<()> {
        let start = self.here();
        self.compile_expr(cond)?;
        let break_jump = self.emit(Op::JumpFalse(0));

        self.loops.push(LoopCtx {
            break_fixups: Vec::new(),
            continue_fixups: Vec::new(),
        });
        self.compile_block(body)?;
        let ctx = self.loops.pop().unwrap();

        self.emit(Op::Jump(start));
        let end = self.here();
        self.patch(break_jump, 0, end);
        for j in ctx.break_fixups {
            self.patch(j, 0, end);
        }
        for j in ctx.continue_fixups {
            self.patch(j, 0, start);
        }
        Ok(())
    }

    fn compile_repeat(&mut self, body: &Block, cond: &Expr) -> CResult<()> {
        let start = self.here();
        self.loops.push(LoopCtx {
            break_fixups: Vec::new(),
            continue_fixups: Vec::new(),
        });
        self.compile_block(body)?;
        let ctx = self.loops.pop().unwrap();

        let cont_addr = self.here();
        self.compile_expr(cond)?;
        // Repeat until the condition is true: loop back while it is false.
        self.emit(Op::JumpFalse(start));
        let end = self.here();
        for j in ctx.break_fixups {
            self.patch(j, 0, end);
        }
        for j in ctx.continue_fixups {
            self.patch(j, 0, cont_addr);
        }
        Ok(())
    }

    fn compile_function(&mut self, def: &DefineFunction) -> CResult<()> {
        // A function body is its own scope: fresh locals (params, OUT params, then
        // body-declared/auto-declared), fresh labels, fresh loop nest.
        let mut locals: Vec<VarInfo> = Vec::new();
        for p in &def.params {
            locals.push(VarInfo {
                name: p.clone(),
                is_array: false,
            });
        }
        for p in &def.out_params {
            locals.push(VarInfo {
                name: p.clone(),
                is_array: false,
            });
        }
        let prev_loops = std::mem::take(&mut self.loops);
        // (labels/fixups are already drained per scope by resolve_labels)
        self.func = Some(FuncScope {
            name: def.name.clone(),
            locals,
            params: def.params.clone(),
            out_params: def.out_params.clone(),
            returns_value: def.returns_value,
            is_common: def.is_common,
            declared: HashSet::new(),
        });

        let address = self.here();
        self.compile_block(&def.body)?;
        // Fall-through return: a value function defaults to Void.
        if def.returns_value {
            self.emit(Op::PushVoid);
            self.emit(Op::ReturnFunc { has_value: true });
        } else {
            self.emit(Op::ReturnFunc { has_value: false });
        }
        self.resolve_labels()?;

        let fs = self.func.take().unwrap();
        self.loops = prev_loops;
        self.functions.push(Function {
            name: fs.name,
            address,
            params: fs.params,
            out_params: fs.out_params,
            returns_value: fs.returns_value,
            is_common: fs.is_common,
            locals: fs.locals,
        });
        Ok(())
    }

    // -- calls -----------------------------------------------------------------

    fn compile_call_stmt(&mut self, name: &Name, args: &[Expr], out_args: &[Expr]) -> CResult<()> {
        let cname = canonical(name);
        // The stack/queue ops (M1-T14) take their first operand by reference so they can
        // grow/shrink the caller's array or write a modified string scalar back.
        let leading_ref = is_stack_op(&cname);
        for (i, a) in args.iter().enumerate() {
            if leading_ref && i == 0 {
                self.compile_stack_operand(a)?;
            } else {
                self.compile_expr(a)?;
            }
        }
        if cname == "CALL" {
            self.emit(Op::CallDynamic {
                argc: args.len().saturating_sub(1) as u8,
                out_argc: out_args.len() as u8,
                wants_value: false,
            });
        } else if self.user_funcs.contains(&cname) {
            self.emit(Op::CallUser {
                name: name.clone(),
                argc: args.len() as u8,
                out_argc: out_args.len() as u8,
                wants_value: false,
            });
        } else {
            self.emit(Op::CallBuiltin {
                name: cname,
                argc: args.len() as u8,
                out_argc: out_args.len() as u8,
                wants_value: false,
            });
        }
        // The call leaves `out_argc` results on the stack (topmost = last OUT arg).
        for out in out_args.iter().rev() {
            self.compile_pop_target(out)?;
        }
        Ok(())
    }

    fn compile_call_expr(&mut self, name: &Name, args: &[Expr]) -> CResult<()> {
        // A paren call over a declared array is an element read.
        if let Some((vref, true)) = self.lookup(name) {
            for a in args {
                self.compile_expr(a)?;
            }
            self.emit(Op::PushArray {
                var: vref,
                dims: args.len() as u8,
            });
            return Ok(());
        }

        let cname = canonical(name);
        // POP/SHIFT (M1-T14) take their operand by reference, like the statement-form
        // stack ops, so a string-scalar operand is written back after the removal.
        let leading_ref = is_stack_op(&cname);
        for (i, a) in args.iter().enumerate() {
            if leading_ref && i == 0 {
                self.compile_stack_operand(a)?;
            } else {
                self.compile_expr(a)?;
            }
        }
        if cname == "CALL" {
            self.emit(Op::CallDynamic {
                argc: args.len().saturating_sub(1) as u8,
                out_argc: 0,
                wants_value: true,
            });
        } else if self.user_funcs.contains(&cname) {
            self.emit(Op::CallUser {
                name: name.clone(),
                argc: args.len() as u8,
                out_argc: 0,
                wants_value: true,
            });
        } else {
            self.emit(Op::CallBuiltin {
                name: cname,
                argc: args.len() as u8,
                out_argc: 0,
                wants_value: true,
            });
        }
        Ok(())
    }

    // -- expressions -----------------------------------------------------------

    fn compile_expr(&mut self, expr: &Expr) -> CResult<()> {
        match &expr.kind {
            ExprKind::Const(lit) => {
                self.emit(Op::Push(const_from_lit(lit)));
            }
            ExprKind::Var(name) => {
                if let Some(sv) = Sysvar::from_name(&canonical(name)) {
                    // A system variable (MAINCNT/VERSION/TIME$/ERRNUM/…, M6-T3): reserved, so
                    // it resolves before any user variable. The VM reads the live value.
                    self.emit(Op::PushSysvar(sv));
                } else if let Some((vref, _)) = self.lookup(name) {
                    self.emit(Op::PushVar(vref));
                } else if self.builtins.is_builtin(&canonical(name)) {
                    // A zero-arg builtin / constant used as a bare name (e.g. PI).
                    self.emit(Op::CallBuiltin {
                        name: canonical(name),
                        argc: 0,
                        out_argc: 0,
                        wants_value: true,
                    });
                } else {
                    let v = self.resolve_scalar(name)?;
                    self.emit(Op::PushVar(v));
                }
            }
            ExprKind::Index { array, indices } => {
                let name = self.expr_var_name(array)?;
                for ix in indices {
                    self.compile_expr(ix)?;
                }
                let v = self.resolve_array(&name)?;
                self.emit(Op::PushArray {
                    var: v,
                    dims: indices.len() as u8,
                });
            }
            ExprKind::Unary { op, operand } => {
                self.compile_expr(operand)?;
                self.emit(Op::Unary(*op));
            }
            ExprKind::Binary { op, lhs, rhs } => match op {
                BinOp::LAnd => {
                    self.compile_expr(lhs)?;
                    let j = self.emit(Op::LogicalAnd(0));
                    self.compile_expr(rhs)?;
                    let end = self.here();
                    self.patch(j, 0, end);
                }
                BinOp::LOr => {
                    self.compile_expr(lhs)?;
                    let j = self.emit(Op::LogicalOr(0));
                    self.compile_expr(rhs)?;
                    let end = self.here();
                    self.patch(j, 0, end);
                }
                _ => {
                    self.compile_expr(lhs)?;
                    self.compile_expr(rhs)?;
                    self.emit(Op::Operate(*op));
                }
            },
            ExprKind::Call { name, args } => self.compile_call_expr(name, args)?,
            ExprKind::Ref(inner) => {
                self.compile_expr(inner)?;
                self.emit(Op::PushRefExpr);
            }
            ExprKind::Void => {
                self.emit(Op::PushVoid);
            }
        }
        Ok(())
    }

    /// Compile the leading array/string operand of a stack/queue op (PUSH/POP/SHIFT/
    /// UNSHIFT, M1-T14). A declared array shares its `Rc` via [`Op::PushVar`] (in-place
    /// growth/shrink is visible to the caller); a scalar (string) variable is passed by
    /// reference ([`Op::PushRef`]) so the op can write the modified string back. Any
    /// other operand is compiled normally and raises Type mismatch (8) at runtime.
    fn compile_stack_operand(&mut self, expr: &Expr) -> CResult<()> {
        if let ExprKind::Var(name) = &expr.kind {
            match self.lookup(name) {
                Some((vref, true)) => {
                    self.emit(Op::PushVar(vref));
                    return Ok(());
                }
                Some((vref, false)) => {
                    self.emit(Op::PushRef(vref));
                    return Ok(());
                }
                None => {
                    // An as-yet-undeclared name: auto-declare a scalar (string) and pass
                    // it by reference, matching the character-array form.
                    let v = self.resolve_scalar(name)?;
                    self.emit(Op::PushRef(v));
                    return Ok(());
                }
            }
        }
        self.compile_expr(expr)
    }

    /// The static declared [`Suffix`] of an lvalue, used to coerce a value
    /// assigned into it (e.g. `SWAP`). A scalar/array name carries its suffix;
    /// a runtime `VAR()` ref has no static type, so it takes values verbatim
    /// (`Suffix::None`).
    fn lvalue_suffix(&self, expr: &Expr) -> Suffix {
        match &expr.kind {
            ExprKind::Var(name) => name.suffix,
            ExprKind::Index { array, .. } => match &array.kind {
                ExprKind::Var(name) => name.suffix,
                _ => Suffix::None,
            },
            _ => Suffix::None,
        }
    }

    /// Compile a reference to an lvalue (for `SWAP`/`INC`/`AssignRef` targets).
    fn compile_push_ref(&mut self, expr: &Expr) -> CResult<()> {
        match &expr.kind {
            ExprKind::Var(name) => {
                let v = self.resolve_scalar(name)?;
                self.emit(Op::PushRef(v));
            }
            ExprKind::Index { array, indices } => {
                let name = self.expr_var_name(array)?;
                for ix in indices {
                    self.compile_expr(ix)?;
                }
                let v = self.resolve_array(&name)?;
                self.emit(Op::PushArrayRef {
                    var: v,
                    dims: indices.len() as u8,
                });
            }
            ExprKind::Ref(inner) => {
                self.compile_expr(inner)?;
                self.emit(Op::PushRefExpr);
            }
            _ => return Err(self.err(ERR_SYNTAX, "expression is not a reference target".into())),
        }
        Ok(())
    }

    /// Compile storing a value (already on the stack, below) into an lvalue.
    fn compile_pop_target(&mut self, expr: &Expr) -> CResult<()> {
        match &expr.kind {
            // An omitted OUT slot (e.g. `TOUCH OUT TM,,`): the result is produced but has no
            // receiver, so discard it. The slot still counts toward the call's OUT count.
            ExprKind::Void => {
                self.emit(Op::Pop);
            }
            ExprKind::Var(name) => {
                let v = self.resolve_scalar(name)?;
                self.emit(Op::PopVar(v));
            }
            ExprKind::Index { array, indices } => {
                let name = self.expr_var_name(array)?;
                for ix in indices {
                    self.compile_expr(ix)?;
                }
                let v = self.resolve_array(&name)?;
                self.emit(Op::PopArray {
                    var: v,
                    dims: indices.len() as u8,
                });
            }
            ExprKind::Ref(inner) => {
                self.compile_expr(inner)?;
                self.emit(Op::PushRefExpr);
                self.emit(Op::PopRef);
            }
            _ => return Err(self.err(ERR_SYNTAX, "expression is not an assignable target".into())),
        }
        Ok(())
    }

    /// Extract the variable name from an array-base expression (it must be a plain
    /// variable, per the AST's `Index { array, .. }`).
    fn expr_var_name(&self, expr: &Expr) -> CResult<Name> {
        match &expr.kind {
            ExprKind::Var(name) => Ok(name.clone()),
            _ => Err(CompileError {
                loc: expr.loc,
                errnum: ERR_SYNTAX,
                msg: "array base must be a variable".into(),
            }),
        }
    }
}

/// The receiver type of an `INPUT` target, used to parse each typed field: a `$`-suffixed
/// variable (or string-array element) receives the raw text (`VarType::Str`); every other
/// receiver parses a number. A `VAR()`-style runtime reference defaults to numeric.
fn input_target_type(expr: &Expr) -> VarType {
    let suffix = match &expr.kind {
        ExprKind::Var(name) => name.suffix,
        ExprKind::Index { array, .. } => match &array.kind {
            ExprKind::Var(name) => name.suffix,
            _ => Suffix::None,
        },
        _ => Suffix::None,
    };
    match suffix {
        Suffix::Str => VarType::Str,
        Suffix::Int => VarType::Int,
        Suffix::Real | Suffix::None => VarType::Real,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn comp(src: &str) -> Program {
        let ast = parse(src).expect("parse");
        compile(&ast).expect("compile")
    }

    fn comp_err(src: &str) -> CompileError {
        let ast = parse(src).expect("parse");
        compile(&ast).expect_err("expected compile error")
    }

    #[test]
    fn arithmetic_assign_snapshot() {
        // X = 2 + 3 * 4   (parser folds the RHS to a single constant)
        let p = comp("X=2+3*4");
        assert_eq!(
            p.code,
            vec![
                Op::Push(Const::Int(14)),
                Op::PopVar(VarRef::Global(0)),
                Op::End,
            ]
        );
        assert_eq!(p.globals[0].name, Name::new("X", Suffix::None));
    }

    #[test]
    fn non_constant_binary_emits_operate() {
        // Y = X + 1  -> push X, push 1, add, store Y.
        let p = comp("X=5\nY=X+1");
        assert!(p.code.contains(&Op::Operate(BinOp::Add)));
        // X is global 0, Y global 1.
        assert_eq!(p.global_index(&Name::new("X", Suffix::None)), Some(0));
        assert_eq!(p.global_index(&Name::new("Y", Suffix::None)), Some(1));
    }

    #[test]
    fn locs_parallel_code() {
        let p = comp("A=1\nB=2");
        assert_eq!(p.code.len(), p.locs.len());
    }

    #[test]
    fn if_then_else_jumps_resolve() {
        let p = comp("IF A THEN\nB=1\nELSE\nB=2\nENDIF");
        // Every jump target is in range (resolved, none left at 0-placeholder unless 0
        // is genuinely the target).
        for op in &p.code {
            if let Op::JumpFalse(a) | Op::Jump(a) = op {
                assert!(*a <= p.code.len(), "jump target {a} out of range");
            }
        }
        // There must be a conditional jump (the IF) and an unconditional (then→endif).
        assert!(p.code.iter().any(|o| matches!(o, Op::JumpFalse(_))));
        assert!(p.code.iter().any(|o| matches!(o, Op::Jump(_))));
    }

    #[test]
    fn goto_label_resolves_to_address() {
        let p = comp("@LOOP\nGOTO @LOOP");
        // The label is at address 0; the GOTO must point there.
        let goto = p
            .code
            .iter()
            .find_map(|o| match o {
                Op::Goto(a) => Some(*a),
                _ => None,
            })
            .expect("a GOTO op");
        assert_eq!(goto, 0);
        assert_eq!(p.code_labels, vec![("LOOP".to_string(), 0)]);
    }

    #[test]
    fn undefined_label_is_errnum_14() {
        let e = comp_err("GOTO @NOPE");
        assert_eq!(e.errnum, 14);
    }

    #[test]
    fn for_loop_structure() {
        let p = comp("FOR I=1 TO 10\nNEXT");
        // FOR re-evaluates direction via a signed-step comparison and loops back.
        assert!(p.code.iter().any(|o| matches!(o, Op::Operate(BinOp::Ge))));
        assert!(p.code.iter().any(|o| matches!(o, Op::Operate(BinOp::Lt))));
        assert!(p.code.iter().any(|o| matches!(o, Op::Operate(BinOp::Gt))));
        // counter add-back uses Add.
        assert!(p.code.iter().any(|o| matches!(o, Op::Operate(BinOp::Add))));
    }

    #[test]
    fn break_and_continue_resolve() {
        let p = comp("WHILE 1\nBREAK\nCONTINUE\nWEND");
        // The two bare jumps (break, continue) must have valid targets.
        let jumps: Vec<usize> = p
            .code
            .iter()
            .filter_map(|o| match o {
                Op::Jump(a) => Some(*a),
                _ => None,
            })
            .collect();
        assert!(!jumps.is_empty());
        for a in jumps {
            assert!(a <= p.code.len());
        }
    }

    #[test]
    fn dim_array_then_index() {
        let p = comp("DIM A[3]\nA[1]=5\nB=A[1]");
        // A is a global array; NewArray + PopArray + PushArray all reference it.
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::NewArray { dims: 1, .. })));
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::PopArray { dims: 1, .. })));
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::PushArray { dims: 1, .. })));
        let a = p.global_index(&Name::new("A", Suffix::None)).unwrap();
        assert!(p.globals[a as usize].is_array);
    }

    #[test]
    fn paren_form_array_read_vs_call() {
        // A is DIM'd → A(1) is an array read; F(1) is a call.
        let p = comp("DIM A[3]\nX=A(1)\nY=F(1)");
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::PushArray { dims: 1, .. })));
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::CallBuiltin { name, argc: 1, .. } if name == "F")));
    }

    #[test]
    fn option_strict_undeclared_is_errnum_15() {
        let e = comp_err("OPTION STRICT\nX=1");
        assert_eq!(e.errnum, 15);
    }

    #[test]
    fn duplicate_var_declaration_is_errnum_18() {
        // hw_verified (sb-oracle 2026-06-22 s_t4a): `VAR Q=1:VAR Q=2` → 18.
        assert_eq!(comp_err("VAR Q=1\nVAR Q=2").errnum, 18);
        // DIM is interchangeable with VAR; a re-DIM is also a duplicate.
        assert_eq!(comp_err("DIM A[3]\nDIM A[5]").errnum, 18);
        // Two items on one line collide too.
        assert_eq!(comp_err("VAR A=1,A=2").errnum, 18);
    }

    #[test]
    fn distinct_suffix_is_not_a_duplicate() {
        // The suffix is part of identity, so `A` and `A%` are different variables.
        let p = comp("VAR A=1\nVAR A%=2");
        assert!(p.global_index(&Name::new("A", Suffix::None)).is_some());
        assert!(p.global_index(&Name::new("A", Suffix::Int)).is_some());
    }

    #[test]
    fn redeclaration_is_scoped_per_function() {
        // A global `VAR A` and a DEF-local `VAR A` do not collide (separate scopes);
        // the DEF-local shadows the global. A duplicate *within* the DEF still → 18.
        let p = comp("VAR A=1\nDEF F\nVAR A=2\nEND");
        assert!(p.global_index(&Name::new("A", Suffix::None)).is_some());
        assert_eq!(comp_err("DEF F\nVAR A=1\nVAR A=2\nEND").errnum, 18);
        // A param is not a tracked declaration, so a body `VAR` over it does not trip 18.
        let _ = comp("DEF F A\nVAR B=A\nEND");
    }

    #[test]
    fn option_strict_declared_is_ok() {
        let p = comp("OPTION STRICT\nVAR X\nX=1");
        assert!(p.options.strict);
        assert!(p.global_index(&Name::new("X", Suffix::None)).is_some());
    }

    #[test]
    fn auto_declare_without_strict() {
        let p = comp("X=1");
        assert!(!p.options.strict);
        assert_eq!(p.globals.len(), 1);
    }

    #[test]
    fn def_function_is_addressed_and_dispatched() {
        let p = comp("X=ADD(1,2)\nDEF ADD(A,B)\nRETURN A+B\nEND");
        // The call dispatches to the user function by name.
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::CallUser { name, argc: 2, .. } if name.ident == "ADD")));
        // The function is recorded with two params and an address past the main End.
        let f = &p.functions[0];
        assert_eq!(f.name.ident, "ADD");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.locals.len(), 2);
        assert!(p.code.get(f.address).is_some());
        // Params resolve as locals inside the body (PushVar Local), not globals.
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::PushVar(VarRef::Local(_)))));
    }

    #[test]
    fn def_dim_binds_local_shadowing_global() {
        // `A` is a top-level global scalar; `DIM A[3]` inside the DEF must bind a fresh
        // function-LOCAL array (shadowing the global), NOT flip the global to an array.
        // hw_verified scoping (sb-oracle 2026-06-23, def_scope.yaml).
        let p = comp("A=5\nMK\nDEF MK\nDIM A[3]\nEND");
        let g = p.globals.iter().find(|v| v.name.ident == "A").unwrap();
        assert!(!g.is_array, "global A must stay a scalar");
        let f = p.functions.iter().find(|f| f.name.ident == "MK").unwrap();
        assert!(
            f.locals.iter().any(|v| v.name.ident == "A" && v.is_array),
            "DIM A[3] must bind a local array inside the DEF"
        );
    }

    #[test]
    fn def_plain_assign_hits_global() {
        // A plain assignment inside a DEF to a name that already exists as a global
        // writes the GLOBAL (no new local). hw_verified scope_write -> 99.
        let p = comp("A=5\nSETA\nDEF SETA\nA=99\nEND");
        let f = p.functions.iter().find(|f| f.name.ident == "SETA").unwrap();
        assert!(
            !f.locals.iter().any(|v| v.name.ident == "A"),
            "plain A=99 must not create a local when a global A exists"
        );
    }

    #[test]
    fn data_pool_and_read() {
        let p = comp("DATA 1,2,3\nREAD A,B");
        assert_eq!(p.data, vec![Const::Int(1), Const::Int(2), Const::Int(3)]);
        // READ A,B → two ReadValue/PopVar pairs.
        assert_eq!(
            p.code.iter().filter(|o| matches!(o, Op::ReadValue)).count(),
            2
        );
    }

    #[test]
    fn restore_label_targets_data_index() {
        let p = comp("DATA 1\n@MID\nDATA 2,3\nRESTORE @MID");
        // @MID precedes the second DATA, i.e. DATA index 1.
        assert!(p.code.contains(&Op::Restore(1)));
    }

    #[test]
    fn print_items_lower_to_ops() {
        let p = comp("PRINT 1,2");
        assert_eq!(
            p.code.iter().filter(|o| matches!(o, Op::PrintItem)).count(),
            2
        );
        assert!(p.code.contains(&Op::PrintTab));
        // No trailing separator → a closing newline.
        assert!(p.code.contains(&Op::PrintNewline));
    }

    #[test]
    fn swap_pushes_two_refs() {
        let p = comp("A=1\nB=2\nSWAP A,B");
        assert_eq!(
            p.code
                .iter()
                .filter(|o| matches!(o, Op::PushRef(_)))
                .count(),
            2
        );
        assert!(p.code.iter().any(|o| matches!(o, Op::Swap { .. })));
    }

    #[test]
    fn builtins_predicate_resolves_bare_name() {
        // With PI registered, a bare `PI` compiles to a zero-arg call, not a variable.
        let mut set = HashSet::new();
        set.insert("PI".to_string());
        let ast = parse("X=PI").unwrap();
        let p = compile_with(&ast, &set).unwrap();
        assert!(p
            .code
            .iter()
            .any(|o| matches!(o, Op::CallBuiltin { name, argc: 0, .. } if name == "PI")));
        // PI was not auto-declared as a variable.
        assert!(p.global_index(&Name::new("PI", Suffix::None)).is_none());
    }
}
