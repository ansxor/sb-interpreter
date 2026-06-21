//! Abstract syntax tree (milestone M1).
//!
//! A faithful port of `otya.smilebasic.node` (`osb/SMILEBASIC/node.d`), restructured
//! into idiomatic Rust. The [parser](crate::parser) builds these nodes; the
//! [compiler](crate::compiler) lowers them to bytecode.
//!
//! Shape, and where it differs from osb:
//!
//! - Every node carries a [`SourceLocation`] (osb's `Node.location`) for `ERRLINE`.
//!   We wrap each node in a small `{ kind, loc }` struct ([`Expr`], [`Stmt`]) instead
//!   of an abstract base class.
//! - Operators are typed [`BinOp`]/[`UnOp`] enums, not raw [`TokenType`]s as in osb's
//!   `BinaryOperator.operator`. The small, closed operator set makes a typed enum
//!   safer and gives the compiler exhaustive `match`es; [`BinOp::from_token`] /
//!   [`UnOp::from_token`] bridge back to the lexer's tokens, and [`BinOp::rank`]
//!   mirrors `parser.d`'s `getOPRank` precedence table.
//! - Array *element reads* `A[i,j]` are a dedicated [`ExprKind::Index`] node, where osb
//!   reuses a `BinaryOperator` with an `LBracket` "operator". The `A(i)` paren form is
//!   ambiguous between an array read and a function call at parse time, so — like osb —
//!   it stays an [`ExprKind::Call`] and is resolved in the compiler.
//! - osb's nested `Statements` block node is just a [`Block`] (`Vec<Stmt>`) here.
//!
//! Names mirror SB: identifiers/labels are upper-cased UTF-16 ([`SbString`]), labels
//! keep their leading `@`.

use crate::token::{SourceLocation, TokenType};
use crate::value::{SbString, Value};

/// A sequence of statements (a program, or the body of a block). osb's `Statements`.
pub type Block = Vec<Stmt>;

// =============================================================================
// Operators
// =============================================================================

/// A binary operator. Mirrors the operators ranked by `parser.d:getOPRank` and the
/// constant-folding cases in `parser.d:constcalc`.
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
    Eq, // == (and bare `=` inside an expression)
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
    /// Map a lexer [`TokenType`] to its binary operator, if it is one.
    ///
    /// Both `==` ([`TokenType::Equal`]) and a bare `=` ([`TokenType::Assign`]) map to
    /// [`BinOp::Eq`]: inside an expression SmileBASIC treats `=` as equality (the
    /// parser uses statement context, not the lexer, to tell assignment apart).
    pub fn from_token(t: &TokenType) -> Option<BinOp> {
        use TokenType as T;
        Some(match t {
            T::Plus => BinOp::Add,
            T::Minus => BinOp::Sub,
            T::Mul => BinOp::Mul,
            T::Div => BinOp::Div,
            T::IntDiv => BinOp::IntDiv,
            T::Mod => BinOp::Mod,
            T::Equal | T::Assign => BinOp::Eq,
            T::NotEqual => BinOp::Ne,
            T::Less => BinOp::Lt,
            T::LessEqual => BinOp::Le,
            T::Greater => BinOp::Gt,
            T::GreaterEqual => BinOp::Ge,
            T::LeftShift => BinOp::Shl,
            T::RightShift => BinOp::Shr,
            T::And => BinOp::And,
            T::Or => BinOp::Or,
            T::Xor => BinOp::Xor,
            T::LogicalAnd => BinOp::LAnd,
            T::LogicalOr => BinOp::LOr,
            _ => return None,
        })
    }

    /// Precedence rank. Mirrors `parser.d:getOPRank` exactly, where a *lower* number
    /// binds *tighter* (the parser descends to `order - 1` for an operator's operands):
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
/// osb desugars unary minus to `0 - x` in the parser; we keep a [`UnOp::Neg`] so the
/// Rust parser may instead emit a single negation node for non-constant operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,  // - (arithmetic negation)
    Not,  // NOT (bitwise complement)
    LNot, // ! (logical not)
}

impl UnOp {
    /// Map a lexer [`TokenType`] to its prefix unary operator, if it is one.
    pub fn from_token(t: &TokenType) -> Option<UnOp> {
        use TokenType as T;
        Some(match t {
            T::Minus => UnOp::Neg,
            T::Not => UnOp::Not,
            T::LogicalNot => UnOp::LNot,
            _ => return None,
        })
    }
}

// =============================================================================
// Expressions
// =============================================================================

/// An expression node: its [`ExprKind`] plus the [`SourceLocation`] it started at.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub loc: SourceLocation,
}

impl Expr {
    pub fn new(kind: ExprKind, loc: SourceLocation) -> Self {
        Self { kind, loc }
    }

    /// A literal-constant expression.
    pub fn constant(value: Value, loc: SourceLocation) -> Self {
        Self::new(ExprKind::Const(value), loc)
    }

    /// A variable-reference expression.
    pub fn var(name: SbString, loc: SourceLocation) -> Self {
        Self::new(ExprKind::Var(name), loc)
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
    Const(Value),
    /// Variable reference by upper-cased, suffix-included name (`node.d:Variable`).
    Var(SbString),
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
    Call { name: SbString, args: Vec<Expr> },
    /// `VAR(expr)` — pass an lvalue by reference (`node.d:VarRef`).
    Ref(Box<Expr>),
    /// An omitted argument, e.g. the gap in `LOCATE ,5` (`node.d:VoidExpression`).
    Void,
}

// =============================================================================
// Statements
// =============================================================================

/// A statement node: its [`StmtKind`] plus the [`SourceLocation`] it started at.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub loc: SourceLocation,
}

impl Stmt {
    pub fn new(kind: StmtKind, loc: SourceLocation) -> Self {
        Self { kind, loc }
    }
}

/// Target of a `GOTO`/`GOSUB`: a literal `@label` or a computed expression.
/// Mirrors osb's two `Goto`/`Gosub` constructors (`label` vs `labelexpr`).
#[derive(Debug, Clone, PartialEq)]
pub enum Jump {
    /// A literal label name, including its leading `@` (e.g. `@LOOP`).
    Label(SbString),
    /// A computed target (e.g. `GOTO "@" + A$`).
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
    Scalar { name: SbString, init: Option<Expr> },
    /// `DIM A[d0, d1, ...]` (1–4 dimensions).
    Array { name: SbString, dims: Vec<Expr> },
}

/// One item of a `PRINT` statement (`node.d:PrintArgument`). A `;` separator emits no
/// item — it just suppresses the trailing newline.
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
    pub name: SbString,
    /// By-value parameters, in order.
    pub args: Vec<SbString>,
    /// `OUT` (by-reference) parameters, in order.
    pub out_args: Vec<SbString>,
    /// Whether the definition returns a value (a function) vs. a command.
    pub returns_value: bool,
    /// `COMMON DEF` — shared across program slots.
    pub is_common: bool,
    pub body: Block,
}

/// The kinds of statement. Mirrors the `Statement` subclasses in `node.d`.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `name = expr` (`node.d:Assign`).
    Assign { name: SbString, expr: Expr },
    /// `A[i, j] = expr` (`node.d:ArrayAssign`).
    ArrayAssign {
        name: SbString,
        indices: Vec<Expr>,
        expr: Expr,
    },
    /// Assignment through a computed lvalue (`node.d:AssignRef`).
    AssignRef { target: Expr, expr: Expr },
    /// A bare command / function call as a statement, with optional `OUT` targets
    /// (`node.d:CallFunctionStatement`).
    Call {
        name: SbString,
        args: Vec<Expr>,
        out_args: Vec<Expr>,
    },
    /// `PRINT`/`?` (`node.d:Print`).
    Print(Vec<PrintItem>),
    /// `@label:` definition site (`node.d:Label`). Includes the leading `@`.
    Label(SbString),
    /// `GOTO` (`node.d:Goto`).
    Goto(Jump),
    /// `GOSUB` (`node.d:Gosub`).
    Gosub(Jump),
    /// `RETURN`, optionally with a value from a `DEF` function (`node.d:Return`).
    Return(Option<Expr>),
    /// `ON expr GOTO/GOSUB l0, l1, ...` (`node.d:On`).
    On {
        value: Expr,
        kind: OnKind,
        labels: Vec<SbString>,
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
        var: SbString,
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
    /// `BREAK` (`node.d:Break`).
    Break,
    /// `CONTINUE` (`node.d:Continue`).
    Continue,
    /// `VAR`/`DIM` declarations (`node.d:Var`).
    Dim(Vec<DimItem>),
    /// `DEF ... END` (`node.d:DefineFunction`).
    Def(DefineFunction),
    /// `DATA` literals (`node.d:Data`). Bareword items lex to string constants.
    Data(Vec<Value>),
    /// `READ var0, var1, ...` (`node.d:Read`).
    Read(Vec<Expr>),
    /// `RESTORE label` — the label resolves at runtime (`node.d:Restore`).
    Restore(Expr),
    /// `INPUT [prompt;] var0, var1, ...` (`node.d:Input`). `question` adds the `?`.
    Input {
        prompt: Option<Expr>,
        question: bool,
        vars: Vec<Expr>,
    },
    /// `LINPUT [prompt;] var` (`node.d:Linput`).
    Linput { prompt: Option<Expr>, var: Expr },
    /// `INC`/`DEC` — `DEC` is lowered to a negative `delta` (`node.d:Inc`).
    Inc { target: Expr, delta: Expr },
    /// `SWAP a, b` (`node.d:Swap`).
    Swap { a: Expr, b: Expr },
    /// `OPTION <arg>` e.g. `OPTION STRICT` (`node.d:Option`).
    Option(SbString),
    /// `USE <slot expr>` (`node.d:Use`).
    Use(Expr),
    /// `EXEC <slot expr>` (`node.d:Exec`).
    Exec(Expr),
    /// `STOP` (`node.d:StopStatement`).
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> SourceLocation {
        SourceLocation::new(1, 0)
    }

    fn name(s: &str) -> SbString {
        s.encode_utf16().collect()
    }

    fn int(n: i32) -> Expr {
        Expr::constant(Value::Int(n), loc())
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
        use TokenType as T;
        // `=` and `==` both mean equality inside an expression.
        assert_eq!(BinOp::from_token(&T::Assign), Some(BinOp::Eq));
        assert_eq!(BinOp::from_token(&T::Equal), Some(BinOp::Eq));
        assert_eq!(BinOp::from_token(&T::IntDiv), Some(BinOp::IntDiv));
        assert_eq!(BinOp::from_token(&T::LogicalOr), Some(BinOp::LOr));
        // Non-operators map to None.
        assert_eq!(BinOp::from_token(&T::LParen), None);
        assert_eq!(BinOp::from_token(&T::Print), None);

        // Ranks reproduce getOPRank: lower number binds tighter, so
        // `*` (3) < `+` (4) < `==` (6) < `||` (11).
        assert!(BinOp::Mul.rank() < BinOp::Add.rank());
        assert!(BinOp::Add.rank() < BinOp::Eq.rank());
        assert!(BinOp::Eq.rank() < BinOp::LOr.rank());
        assert_eq!(BinOp::LOr.rank(), 11);
        assert_eq!(BinOp::LAnd.rank(), 10);
        assert_eq!(BinOp::Or.rank(), 9);
        assert_eq!(BinOp::And.rank(), 7);
        assert_eq!(BinOp::Eq.rank(), 6);
        assert_eq!(BinOp::Shl.rank(), 5);
    }

    #[test]
    fn unop_from_token() {
        use TokenType as T;
        assert_eq!(UnOp::from_token(&T::Minus), Some(UnOp::Neg));
        assert_eq!(UnOp::from_token(&T::Not), Some(UnOp::Not));
        assert_eq!(UnOp::from_token(&T::LogicalNot), Some(UnOp::LNot));
        assert_eq!(UnOp::from_token(&T::Plus), None);
    }

    #[test]
    fn lvalue_classification_matches_osb() {
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

    /// A handful of statement nodes compile, clone, and compare.
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
                body: vec![Stmt::new(
                    StmtKind::Print(vec![PrintItem::Expr(int(42))]),
                    loc(),
                )],
            },
            loc(),
        );
        let def = Stmt::new(
            StmtKind::Def(DefineFunction {
                name: name("F"),
                args: vec![name("A")],
                out_args: vec![name("R")],
                returns_value: true,
                is_common: false,
                body: vec![Stmt::new(StmtKind::Return(Some(int(1))), loc())],
            }),
            loc(),
        );
        let goto = Stmt::new(StmtKind::Goto(Jump::Label(name("@LOOP"))), loc());

        for s in [&assign, &if_stmt, &for_stmt, &def, &goto] {
            assert_eq!(*s, s.clone());
            assert!(!format!("{s:?}").is_empty());
        }
    }
}
