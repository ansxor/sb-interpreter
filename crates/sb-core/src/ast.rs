//! Abstract syntax tree (M1-T2) — the parser's output and the compiler's input.
//!
//! The node shape follows the SmileBASIC execution model
//! (`spec/concepts/execution-model.md`) and is cross-checked structurally against
//! `osb/SMILEBASIC/node.d` (D, 3.5.0 — a STRUCTURAL reference only; where it
//! disagrees with the docs/disassembly the docs/disassembly win, per prd/M1.md).
//! It is **not** a line-by-line port: this AST is decoupled from the runtime value
//! type (M1-T4 `value.rs`) so M1-T3's parser can be written and tested first.
//!
//! Design choices and where they differ from osb:
//!
//! - Every node carries a [`SourceLoc`] (osb's `Node.location`) so the VM can set
//!   `ERRLINE`. Each node is a small `{ kind, loc }` struct ([`Expr`], [`Stmt`])
//!   rather than osb's abstract base class.
//! - Operators are typed [`BinOp`]/[`UnOp`] enums, not raw token kinds as in osb's
//!   `BinaryOperator.operator`. The closed operator set gives the compiler
//!   exhaustive `match`es. [`BinOp::from_token`]/[`BinOp::from_word`] bridge from the
//!   lexer (symbolic ops are [`TokenKind`]s; word ops `AND`/`OR`/`XOR`/`MOD`/`DIV`/
//!   `NOT` arrive as [`TokenKind::Ident`] and resolve by name), and [`BinOp::rank`]
//!   mirrors `parser.d`'s `getOPRank` precedence table.
//! - Literal constants are an AST-local [`Lit`] (Integer `i32` / Real `f64` / String),
//!   mirroring the lexer's literal tokens — the AST never names the runtime `Value`.
//! - A [`Name`] pairs the lexer's upper-cased identifier with its type [`Suffix`]
//!   (`$`/`%`/`#`), which together form a variable's identity per the execution model.
//! - Array *element reads* `A[i,j]` are a dedicated [`ExprKind::Index`] node; the
//!   `A(i)` paren form is ambiguous between an array read and a function call at parse
//!   time, so — like osb — it stays an [`ExprKind::Call`] resolved by the compiler.

use crate::token::{SourceLoc, Suffix, TokenKind};

/// A sequence of statements: a program, or the body of a block. (osb `Statements`.)
pub type Block = Vec<Stmt>;

// =============================================================================
// Names & literals
// =============================================================================

/// A variable / call name: the lexer's upper-cased identifier plus its type
/// [`Suffix`]. The suffix is part of the identity (`A`, `A%`, `A#`, `A$` are four
/// distinct names), per `execution-model.md` ("Type by suffix").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name {
    /// Upper-cased identifier text, excluding any suffix.
    pub ident: String,
    /// The `$`/`%`/`#` suffix, or [`Suffix::None`].
    pub suffix: Suffix,
}

impl Name {
    pub fn new(ident: impl Into<String>, suffix: Suffix) -> Self {
        Name {
            ident: ident.into(),
            suffix,
        }
    }
}

/// A literal constant value, as produced by the lexer (and by the parser's
/// constant folding). Decoupled from the runtime `Value` (M1-T4): Integer is `i32`
/// and Real is `f64`, matching SmileBASIC.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// Integer literal (`i32`). `TRUE`/`FALSE` lex to `Int(1)`/`Int(0)`.
    Int(i32),
    /// Real / Double literal (`f64`).
    Real(f64),
    /// String literal (UTF-8 here; the runtime string type is UTF-16, M1-T4).
    Str(String),
}

// =============================================================================
// Operators
// =============================================================================

/// A binary operator. Covers the operators ranked by `parser.d:getOPRank` and folded
/// by `parser.d:constcalc`. `/` is real division; `DIV` is integer division.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // arithmetic
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /   (real division)
    IntDiv, // DIV (integer division)
    Mod,    // MOD
    // comparison (each yields integer 1/0)
    Eq, // == (and a bare `=` used inside an expression)
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
    // bit shifts
    Shl, // <<
    Shr, // >>
    // bitwise / logical-on-integers
    And, // AND
    Or,  // OR
    Xor, // XOR
    // short-circuit logical
    LAnd, // &&
    LOr,  // ||
}

impl BinOp {
    /// Map a symbolic lexer [`TokenKind`] to its binary operator, if it is one.
    ///
    /// A bare `=` ([`TokenKind::Assign`]) and `==` ([`TokenKind::EqEq`]) both map to
    /// [`BinOp::Eq`]: inside an expression SmileBASIC treats `=` as equality (the
    /// parser uses statement context, not the lexer, to tell an assignment apart).
    /// Word operators (`AND`/`OR`/`XOR`/`MOD`/`DIV`) arrive as identifiers — see
    /// [`BinOp::from_word`].
    pub fn from_token(t: &TokenKind) -> Option<BinOp> {
        use TokenKind as T;
        Some(match t {
            T::Plus => BinOp::Add,
            T::Minus => BinOp::Sub,
            T::Star => BinOp::Mul,
            T::Slash => BinOp::Div,
            T::Assign | T::EqEq => BinOp::Eq,
            T::NotEq => BinOp::Ne,
            T::Less => BinOp::Lt,
            T::LessEq => BinOp::Le,
            T::Greater => BinOp::Gt,
            T::GreaterEq => BinOp::Ge,
            T::Shl => BinOp::Shl,
            T::Shr => BinOp::Shr,
            T::AndAnd => BinOp::LAnd,
            T::OrOr => BinOp::LOr,
            _ => return None,
        })
    }

    /// Map an upper-cased identifier to a word binary operator, if it is one.
    /// The lexer emits `AND`/`OR`/`XOR`/`MOD`/`DIV`/`NOT` as [`TokenKind::Ident`]; the
    /// parser resolves them here. (`NOT` is a unary operator — see [`UnOp::from_word`].)
    pub fn from_word(ident: &str) -> Option<BinOp> {
        Some(match ident {
            "DIV" => BinOp::IntDiv,
            "MOD" => BinOp::Mod,
            "AND" => BinOp::And,
            "OR" => BinOp::Or,
            "XOR" => BinOp::Xor,
            _ => return None,
        })
    }

    /// Precedence rank, mirroring `parser.d:getOPRank`: a *lower* number binds
    /// *tighter* (the parser descends to `rank - 1` for an operator's operands).
    /// `||`=11, `&&`=10, OR/XOR=9, AND=7, comparisons=6, shifts=5, `+`/`-`=4,
    /// `*`/`/`/DIV/MOD=3. (Array indexing, rank 2, is a dedicated node here.)
    pub fn rank(self) -> u8 {
        use BinOp::*;
        match self {
            LOr => 11,
            LAnd => 10,
            Or | Xor => 9,
            And => 7,
            Eq | Ne | Lt | Le | Gt | Ge => 6,
            Shl | Shr => 5,
            Add | Sub => 4,
            Mul | Div | IntDiv | Mod => 3,
        }
    }
}

/// A unary (prefix) operator.
///
/// osb desugars unary minus to `0 - x`; we keep a [`UnOp::Neg`] so the parser may
/// emit a single negation node for a non-constant operand (and fold it for a
/// constant one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,  // - (arithmetic negation)
    Not,  // NOT (bitwise complement)
    LNot, // ! (logical not)
}

impl UnOp {
    /// Map a symbolic lexer [`TokenKind`] to its prefix unary operator, if it is one.
    pub fn from_token(t: &TokenKind) -> Option<UnOp> {
        use TokenKind as T;
        Some(match t {
            T::Minus => UnOp::Neg,
            T::Bang => UnOp::LNot,
            _ => return None,
        })
    }

    /// Map an upper-cased identifier to a word unary operator (`NOT`), if it is one.
    pub fn from_word(ident: &str) -> Option<UnOp> {
        match ident {
            "NOT" => Some(UnOp::Not),
            _ => None,
        }
    }
}

// =============================================================================
// Expressions
// =============================================================================

/// An expression node: its [`ExprKind`] plus the [`SourceLoc`] it started at.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub loc: SourceLoc,
}

impl Expr {
    pub fn new(kind: ExprKind, loc: SourceLoc) -> Self {
        Expr { kind, loc }
    }

    /// A literal-constant expression.
    pub fn constant(value: Lit, loc: SourceLoc) -> Self {
        Expr::new(ExprKind::Const(value), loc)
    }

    /// A variable-reference expression.
    pub fn var(name: Name, loc: SourceLoc) -> Self {
        Expr::new(ExprKind::Var(name), loc)
    }

    /// Whether this expression can appear on the left of an assignment or as a
    /// `READ`/`INC`/`SWAP`/`OUT` target. Mirrors `parser.d:isLValue`: a plain
    /// variable, a `VAR(...)` reference, or an array index whose base is an lvalue.
    pub fn is_lvalue(&self) -> bool {
        match &self.kind {
            ExprKind::Var(_) | ExprKind::Ref(_) => true,
            ExprKind::Index { array, .. } => array.is_lvalue(),
            _ => false,
        }
    }
}

/// The kinds of expression. Mirrors the `Expression` subclasses in `node.d`.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Literal constant (`node.d:Constant`). Also produced by constant folding.
    Const(Lit),
    /// Variable reference by name+suffix (`node.d:Variable`).
    Var(Name),
    /// Array element read `A[i, j]` (`node.d`'s `LBracket` `BinaryOperator`). The
    /// base is usually a [`ExprKind::Var`]; `indices` holds 1–4 subscripts.
    Index {
        array: Box<Expr>,
        indices: Vec<Expr>,
    },
    /// Unary operator applied to one operand (`node.d:UnaryOperator`).
    Unary { op: UnOp, operand: Box<Expr> },
    /// Binary operator (`node.d:BinaryOperator`).
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `name(args)` — a function call *or* a paren-form array read, disambiguated at
    /// compile time (`node.d:CallFunction`).
    Call { name: Name, args: Vec<Expr> },
    /// `VAR(expr)` — pass an lvalue by reference (`node.d:VarRef`).
    Ref(Box<Expr>),
    /// An omitted argument, e.g. the gap in `LOCATE ,5` (`node.d:VoidExpression`).
    Void,
    /// A Class-1 statement keyword used as a sole expression operand, e.g. the RHS of
    /// `A=STOP`. SmileBASIC 3.6.0 (hw_verified, sb-oracle 2026-06-26,
    /// harness/harvest/out/ctm_bareword_kw.tsv) treats the seven "structural-flow"
    /// statement keywords — `STOP END GOTO GOSUB RETURN PRINT RESTORE` — as reserved
    /// in statement-leading / assignment-target position (so `STOP=5` is Syntax 3 and
    /// they can never be assigned), but a sole bareword of one as an *expression*
    /// falls through to a variable-read of an uninitialized name: under
    /// `OPTION STRICT` that trips the undeclared-variable gate (errnum 15, compile
    /// time); without STRICT it raises errnum 48 (Uninitialized variable used) at
    /// runtime when the operand is evaluated. A *compound* expression (`STOP+1`,
    /// `(STOP)`, `1+STOP`) is Syntax 3 — only the sole bareword form routes here.
    BarewordKeyword(Name),
}

// =============================================================================
// Statement support types
// =============================================================================

/// Target of a `GOTO`/`GOSUB`/`RESTORE`: a literal `@label` or a computed
/// expression (e.g. `GOTO "@" + A$`). Mirrors osb's `label` vs `labelexpr` forms.
#[derive(Debug, Clone, PartialEq)]
pub enum Jump {
    /// A literal label name, *without* the leading `@` (the lexer strips it).
    Label(String),
    /// A computed target (e.g. a string expression, incl. a cross-slot `"n:@L"`).
    Computed(Expr),
}

/// Whether an `ON ... GOTO`/`GOSUB` dispatches by `GOTO` or `GOSUB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnKind {
    Goto,
    Gosub,
}

/// One `ELSEIF cond THEN body` arm of an [`StmtKind::If`].
#[derive(Debug, Clone, PartialEq)]
pub struct ElseIf {
    pub cond: Expr,
    pub body: Block,
}

/// One declaration inside a `VAR`/`DIM` statement (`node.d:DefineVariable` /
/// `DefineArray`, both held in `node.d:Var`).
#[derive(Debug, Clone, PartialEq)]
pub enum DimItem {
    /// `VAR X` or `VAR X = expr`.
    Scalar { name: Name, init: Option<Expr> },
    /// `DIM A[d0, d1, ...]` (1–4 dimensions).
    Array { name: Name, dims: Vec<Expr> },
}

/// One item of a `PRINT` statement (`node.d:PrintArgument`). A trailing item that is
/// not [`PrintItem::NewLine`] (i.e. a `;`-terminated `PRINT`) suppresses the newline.
#[derive(Debug, Clone, PartialEq)]
pub enum PrintItem {
    /// An expression to print.
    Expr(Expr),
    /// `,` — advance to the next `TABSTEP` column.
    Tab,
    /// An end-of-statement newline.
    NewLine,
}

/// A `DEF`/`DEF ... END` user function or command (`node.d:DefineFunction`).
#[derive(Debug, Clone, PartialEq)]
pub struct DefineFunction {
    pub name: Name,
    /// By-value parameters, in order.
    pub params: Vec<Name>,
    /// `OUT` (by-reference) parameters, in order.
    pub out_params: Vec<Name>,
    /// Whether the definition returns a value (a function) vs. a command.
    pub returns_value: bool,
    /// `COMMON DEF` — published into the process-wide common function table.
    pub is_common: bool,
    pub body: Block,
}

// =============================================================================
// Statements
// =============================================================================

/// A statement node: its [`StmtKind`] plus the [`SourceLoc`] it started at.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub loc: SourceLoc,
}

impl Stmt {
    pub fn new(kind: StmtKind, loc: SourceLoc) -> Self {
        Stmt { kind, loc }
    }
}

/// The kinds of statement. Mirrors the `Statement` subclasses in `node.d`.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `name = expr` (`node.d:Assign`).
    Assign { name: Name, expr: Expr },
    /// `A[i, j] = expr` (`node.d:ArrayAssign`).
    ArrayAssign {
        name: Name,
        indices: Vec<Expr>,
        expr: Expr,
    },
    /// Assignment through a computed lvalue, e.g. `VAR(X) = expr` (`node.d:AssignRef`).
    AssignRef { target: Expr, expr: Expr },
    /// A bare command / function call as a statement, with optional `OUT` targets
    /// (`node.d:CallFunctionStatement`).
    Call {
        name: Name,
        args: Vec<Expr>,
        out_args: Vec<Expr>,
    },
    /// A value-returning builtin written in the whole-parenthesised *statement* form
    /// `NAME(args)` (no `OUT`, no assignment) that real SB 3.6.0 rejects at **runtime**
    /// with Illegal function call (4) — the builtin's handler reaches the dispatcher and
    /// refuses the discarded-return shape. Deferred to runtime (not a parse error) so a
    /// preceding statement on the same line still runs, e.g. `PRINT"HI":ABS(5)` prints
    /// "HI" first. The held expressions are the call arguments, evaluated before the
    /// error is raised (the caller pushes args before the handler runs). Which builtins
    /// land here vs. the parse-time Syntax-error (3) bucket is a per-builtin keyword-table
    /// flag — see [`crate::parser::expr_stmt_class`]. hw_verified (sb-oracle 2026-06-26,
    /// `harness/harvest/out/exprstmt2.tsv`).
    IllegalFnStmt(Vec<Expr>),
    /// `PRINT`/`?` (`node.d:Print`).
    Print(Vec<PrintItem>),
    /// `@label` definition site (`node.d:Label`), *without* the leading `@`.
    Label(String),
    /// `GOTO` (`node.d:Goto`).
    Goto(Jump),
    /// `GOSUB` (`node.d:Gosub`).
    Gosub(Jump),
    /// `RETURN`, optionally with a value from a `DEF` function (`node.d:Return`).
    Return(Option<Expr>),
    /// `ON expr GOTO/GOSUB l0, l1, ...` (`node.d:On`). Targets are literal labels.
    On {
        value: Expr,
        kind: OnKind,
        labels: Vec<Jump>,
    },
    /// `IF cond THEN ... [ELSEIF ...] [ELSE ...] ENDIF` (`node.d:If`).
    If {
        cond: Expr,
        then_body: Block,
        elseifs: Vec<ElseIf>,
        else_body: Block,
    },
    /// `FOR var = from TO to [STEP step] ... NEXT` (`node.d:For`).
    For {
        var: Name,
        from: Expr,
        to: Expr,
        step: Option<Expr>,
        body: Block,
    },
    /// `WHILE cond ... WEND` (`node.d:While`).
    While { cond: Expr, body: Block },
    /// `REPEAT ... UNTIL cond` (`node.d:RepeatUntil`).
    RepeatUntil { body: Block, cond: Expr },
    /// `END` (`node.d:End`).
    End,
    /// `STOP` (`node.d:StopStatement`).
    Stop,
    /// `BREAK` (`node.d:Break`).
    Break,
    /// `CONTINUE` (`node.d:Continue`).
    Continue,
    /// `VAR`/`DIM` declarations (`node.d:Var`).
    Dim(Vec<DimItem>),
    /// `DEF ... END` (`node.d:DefineFunction`).
    Def(DefineFunction),
    /// `DATA` literals (`node.d:Data`). Bareword items lex to string constants.
    Data(Vec<Lit>),
    /// `READ var0, var1, ...` (`node.d:Read`).
    Read(Vec<Expr>),
    /// `RESTORE @label` — the label resolves at runtime (`node.d:Restore`).
    /// `None` is the argument-less `RESTORE`: real SB 3.6.0 parses it but raises
    /// Type mismatch (8) at runtime (it has no reset-to-first form) — hw_verified.
    Restore(Option<Jump>),
    /// `INPUT [prompt;] var0, var1, ...` (`node.d:Input`). `question` adds the `?`.
    Input {
        prompt: Option<Expr>,
        question: bool,
        vars: Vec<Expr>,
    },
    /// `LINPUT [prompt;] var` (`node.d:Linput`).
    Linput { prompt: Option<Expr>, var: Expr },
    /// `INC`/`DEC` — `DEC` is lowered to a negated `delta` (`node.d:Inc`).
    Inc { target: Expr, delta: Expr },
    /// `SWAP a, b` (`node.d:Swap`).
    Swap { a: Expr, b: Expr },
    /// `OPTION <arg>` e.g. `OPTION STRICT` / `OPTION DEFINT` (`node.d:Option`).
    Option(String),
    /// `USE <slot expr>` (`node.d:Use`).
    Use(Expr),
    /// `EXEC <slot/file expr>` (`node.d:Exec`).
    Exec(Expr),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> SourceLoc {
        SourceLoc::new(1, 1)
    }

    fn name(s: &str) -> Name {
        Name::new(s, Suffix::None)
    }

    fn int(n: i32) -> Expr {
        Expr::constant(Lit::Int(n), loc())
    }

    /// `2 + 3 * 4` as a tree — exercises nesting, boxing, and the operator enums.
    fn sample() -> Expr {
        Expr::new(
            ExprKind::Binary {
                op: BinOp::Add,
                lhs: Box::new(int(2)),
                rhs: Box::new(Expr::new(
                    ExprKind::Binary {
                        op: BinOp::Mul,
                        lhs: Box::new(int(3)),
                        rhs: Box::new(int(4)),
                    },
                    loc(),
                )),
            },
            loc(),
        )
    }

    #[test]
    fn clone_and_eq_round_trip() {
        let e = sample();
        assert_eq!(e, e.clone());
    }

    #[test]
    fn debug_is_non_empty() {
        // Acceptance: nodes implement `Debug` (used by parser snapshot tests).
        assert!(!format!("{:?}", sample()).is_empty());
    }

    #[test]
    fn binop_from_token_and_rank() {
        use TokenKind as T;
        // `=` and `==` both mean equality inside an expression.
        assert_eq!(BinOp::from_token(&T::Assign), Some(BinOp::Eq));
        assert_eq!(BinOp::from_token(&T::EqEq), Some(BinOp::Eq));
        assert_eq!(BinOp::from_token(&T::Slash), Some(BinOp::Div));
        assert_eq!(BinOp::from_token(&T::OrOr), Some(BinOp::LOr));
        assert_eq!(BinOp::from_token(&T::AndAnd), Some(BinOp::LAnd));
        // Non-operators map to None.
        assert_eq!(BinOp::from_token(&T::LParen), None);
        assert_eq!(BinOp::from_token(&T::Comma), None);

        // Word operators arrive as identifiers, resolved by name.
        assert_eq!(BinOp::from_word("DIV"), Some(BinOp::IntDiv));
        assert_eq!(BinOp::from_word("MOD"), Some(BinOp::Mod));
        assert_eq!(BinOp::from_word("AND"), Some(BinOp::And));
        assert_eq!(BinOp::from_word("OR"), Some(BinOp::Or));
        assert_eq!(BinOp::from_word("XOR"), Some(BinOp::Xor));
        assert_eq!(BinOp::from_word("PRINT"), None);

        // Ranks reproduce getOPRank: lower number binds tighter, so
        // `*` (3) < `+` (4) < `==` (6) < `||` (11).
        assert!(BinOp::Mul.rank() < BinOp::Add.rank());
        assert!(BinOp::Add.rank() < BinOp::Eq.rank());
        assert!(BinOp::Eq.rank() < BinOp::LOr.rank());
        assert_eq!(BinOp::LOr.rank(), 11);
        assert_eq!(BinOp::LAnd.rank(), 10);
        assert_eq!(BinOp::Or.rank(), 9);
        assert_eq!(BinOp::Xor.rank(), 9);
        assert_eq!(BinOp::And.rank(), 7);
        assert_eq!(BinOp::Eq.rank(), 6);
        assert_eq!(BinOp::Shl.rank(), 5);
        assert_eq!(BinOp::IntDiv.rank(), 3);
        assert_eq!(BinOp::Mod.rank(), 3);
    }

    #[test]
    fn unop_from_token_and_word() {
        use TokenKind as T;
        assert_eq!(UnOp::from_token(&T::Minus), Some(UnOp::Neg));
        assert_eq!(UnOp::from_token(&T::Bang), Some(UnOp::LNot));
        assert_eq!(UnOp::from_token(&T::Plus), None);
        assert_eq!(UnOp::from_word("NOT"), Some(UnOp::Not));
        assert_eq!(UnOp::from_word("ABS"), None);
    }

    #[test]
    fn name_carries_suffix_identity() {
        // `A`, `A%`, `A#`, `A$` are four distinct names.
        assert_ne!(Name::new("A", Suffix::None), Name::new("A", Suffix::Str));
        assert_ne!(Name::new("A", Suffix::Int), Name::new("A", Suffix::Real));
        assert_eq!(Name::new("A", Suffix::Str), Name::new("A", Suffix::Str));
    }

    #[test]
    fn lvalue_classification() {
        // A plain variable is an lvalue.
        assert!(Expr::var(name("A"), loc()).is_lvalue());
        // An array index over a variable is an lvalue.
        let indexed = Expr::new(
            ExprKind::Index {
                array: Box::new(Expr::var(name("A"), loc())),
                indices: vec![int(1)],
            },
            loc(),
        );
        assert!(indexed.is_lvalue());
        // A `VAR(...)` reference is an lvalue.
        assert!(Expr::new(ExprKind::Ref(Box::new(Expr::var(name("A"), loc()))), loc()).is_lvalue());
        // A literal and a function call are not.
        assert!(!int(5).is_lvalue());
        assert!(!Expr::new(
            ExprKind::Call {
                name: name("ABS"),
                args: vec![int(1)],
            },
            loc(),
        )
        .is_lvalue());
        // An index over a non-lvalue base is not an lvalue.
        let bad = Expr::new(
            ExprKind::Index {
                array: Box::new(int(1)),
                indices: vec![int(0)],
            },
            loc(),
        );
        assert!(!bad.is_lvalue());
    }

    /// A spread of statement nodes compile, clone, and compare — covers the block
    /// constructs the parser (M1-T3) builds.
    #[test]
    fn statements_build_clone_eq() {
        let assign = Stmt::new(
            StmtKind::Assign {
                name: name("X"),
                expr: sample(),
            },
            loc(),
        );
        let if_stmt = Stmt::new(
            StmtKind::If {
                cond: int(1),
                then_body: vec![assign.clone()],
                elseifs: vec![ElseIf {
                    cond: int(0),
                    body: vec![],
                }],
                else_body: vec![Stmt::new(StmtKind::End, loc())],
            },
            loc(),
        );
        let for_stmt = Stmt::new(
            StmtKind::For {
                var: name("I"),
                from: int(1),
                to: int(10),
                step: Some(int(2)),
                body: vec![Stmt::new(StmtKind::Break, loc())],
            },
            loc(),
        );
        let print = Stmt::new(
            StmtKind::Print(vec![
                PrintItem::Expr(int(1)),
                PrintItem::Tab,
                PrintItem::Expr(Expr::constant(Lit::Str("HI".into()), loc())),
                PrintItem::NewLine,
            ]),
            loc(),
        );
        let def = Stmt::new(
            StmtKind::Def(DefineFunction {
                name: name("ADD"),
                params: vec![name("A"), name("B")],
                out_params: vec![],
                returns_value: true,
                is_common: false,
                body: vec![Stmt::new(StmtKind::Return(Some(sample())), loc())],
            }),
            loc(),
        );
        let goto = Stmt::new(StmtKind::Goto(Jump::Label("LOOP".into())), loc());
        let on = Stmt::new(
            StmtKind::On {
                value: int(2),
                kind: OnKind::Gosub,
                labels: vec![Jump::Label("A".into()), Jump::Label("B".into())],
            },
            loc(),
        );
        let data = Stmt::new(
            StmtKind::Data(vec![Lit::Int(1), Lit::Real(2.5), Lit::Str("x".into())]),
            loc(),
        );

        for s in [
            &assign, &if_stmt, &for_stmt, &print, &def, &goto, &on, &data,
        ] {
            assert_eq!(*s, s.clone());
            assert!(!format!("{s:?}").is_empty());
        }
    }
}
