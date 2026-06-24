//! The SmileBASIC 3.6.0 parser (M1-T3) — recursive descent + precedence climbing
//! + parse-time constant folding.
//!
//! Turns the [`crate::lexer`] token stream into the [`crate::ast`] tree
//! (`spec/concepts/execution-model.md`, "Parsing"). The grammar and the
//! operator-precedence ladder follow `parser.d` (osb, D 3.5.0) **structurally
//! only** — the behavior is SmileBASIC 3.6.0's, taken from the docs + the
//! disassembly + the operator specs (`spec/instructions/{and,or,xor,div,mod}.yaml`),
//! and the operator ranks come from `execution-model.md` (mirrored in
//! [`crate::ast::BinOp::rank`]). It is **not** a line-by-line port.
//!
//! Two SmileBASIC facts shape the design and differ from osb's parser:
//!
//! - **The lexer does not resolve keywords.** Command names *and* control keywords
//!   (`IF`, `FOR`, `PRINT`, `THEN`, `GOTO`, …) all arrive as
//!   [`TokenKind::Ident`]; the parser recognises them by their upper-cased name.
//!   osb's lexer pre-classifies them into distinct token types — we do not, so the
//!   dispatch lives here. (A name with a type suffix, e.g. `IF$`, is never a
//!   keyword — it is a variable.)
//! - **`=` is context-sensitive.** The lexer emits one [`TokenKind::Assign`] for
//!   `=`. At the *start of a statement* the first top-level `=` is an assignment;
//!   *inside an expression* `=` means equality ([`BinOp::Eq`], like `==`), so
//!   `IF A=1 THEN …` works. The statement dispatcher parses the assignment target
//!   with [`Parser::parse_operand`] (rank ≤ 2, which never consumes a binary `=`)
//!   and only then looks for the `=`; every nested expression goes through
//!   [`Parser::parse_expr`] where `=` is equality.
//!
//! ### Constant folding
//!
//! A constant-`op`-constant subexpression, and unary minus on a constant, are folded
//! during the parse using **runtime** numeric semantics so a folded constant equals
//! the value the VM (M1-T6) would compute (`execution-model.md`): Integer ops wrap
//! mod 2³² (i32), a Real operand makes the result Real (f64), `/` is always Real,
//! and `DIV`/`MOD`/`AND`/`OR`/`XOR` are Integer. Folding is **skipped** when it would
//! divide by zero (so the VM raises `Divide by zero` (errnum 7) only if the code is
//! actually reached) and for shifts / comparisons / `&&` / `||` (their exact 3.6.0
//! semantics are left to the VM, where they are verified). See `HARVEST_QUEUE.md`.
//!
//! Malformed input raises [`ParseError`] with `errnum: 3` (Syntax error).

use crate::ast::*;
use crate::consts;
use crate::lexer::Lexer;
use crate::token::{SourceLoc, Suffix, Token, TokenKind};

/// A parse failure. `errnum` is the SmileBASIC error number — usually `3` (Syntax error),
/// but block-structure mismatches carry their own errnum (e.g. "NEXT without FOR" = 21,
/// "DEF without END" = 29; see [`Parser::structural_error`] and `spec/reference/errors.yaml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub loc: SourceLoc,
    pub errnum: u32,
    pub msg: String,
}

impl ParseError {
    /// The byte-for-byte message SmileBASIC displays for this error's `errnum`
    /// (e.g. errnum 3 → `"Syntax error"`, errnum 21 → `"NEXT without FOR"`; see
    /// [`crate::error`]), distinct from the diagnostic [`Self::msg`] detail.
    pub fn message(&self) -> &'static str {
        crate::error::error_message(self.errnum)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Syntax error (errnum {}) at line {} col {}: {}",
            self.errnum, self.loc.line, self.loc.col, self.msg
        )
    }
}

impl std::error::Error for ParseError {}

type PResult<T> = Result<T, ParseError>;

/// The maximum operator rank (`||`); see [`BinOp::rank`].
const MAX_RANK: u8 = 11;

/// Parse a whole program into a top-level [`Block`].
pub fn parse(src: &str) -> PResult<Block> {
    let toks = Lexer::tokenize(src);
    let mut p = Parser::new(toks);
    let block = p.parse_block(BlockKind::Program)?;
    p.expect_kind(&TokenKind::Eof)?;
    Ok(block)
}

/// Parse a single expression (used by tests and by callers that want just an
/// expression, e.g. constant evaluation). Trailing tokens after the expression are
/// rejected as a Syntax error.
pub fn parse_expression(src: &str) -> PResult<Expr> {
    let toks = Lexer::tokenize(src);
    let mut p = Parser::new(toks);
    let e = p.parse_expr()?;
    if !matches!(p.cur_kind(), TokenKind::Eof | TokenKind::Newline) {
        return Err(p.syntax_error("unexpected trailing tokens after expression"));
    }
    Ok(e)
}

/// What kind of statement block is being collected — determines the terminator
/// keywords and whether a newline ends the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    /// The whole program: runs to `Eof`.
    Program,
    /// A `DEF … END` body: ends at the `END` keyword (left for the caller).
    Function,
    /// A single-line `IF … THEN <here>` / `ELSE <here>` body: ends at the newline
    /// or an `ELSE`/`ELSEIF`/`ENDIF`/loop-terminator keyword. A leading `@label`
    /// in this context is a `GOTO`, not a label definition (SmileBASIC `IF c THEN
    /// @L`).
    SingleLineIf,
    /// A multi-line `IF` body: ends at `ELSE`/`ELSEIF`/`ENDIF` (newlines separate).
    MultiLineIf,
    /// A `FOR … NEXT` body: ends at `NEXT`.
    For,
    /// A `WHILE … WEND` body: ends at `WEND`.
    While,
    /// A `REPEAT … UNTIL` body: ends at `UNTIL`.
    Repeat,
}

/// Recursive-descent parser over a flat token vector.
pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    /// Open `FOR` blocks currently being parsed. A bare `NEXT` reached as a statement with
    /// no open `FOR` is "NEXT without FOR" (errnum 21), not the loop-continue idiom.
    for_depth: u32,
    /// How many statements have been committed so far. A stray `ENDIF` raises the dedicated
    /// "ENDIF without IF" (28) ONLY when it is the program's first statement (`stmt_count
    /// == 0`); anywhere later real SB collapses it to generic Syntax error 3 (hw_verified
    /// sb-oracle 2026-06-23 — see the `ENDIF` arm of [`Parser::parse_statement`]).
    stmt_count: u32,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Parser {
            toks,
            pos: 0,
            for_depth: 0,
            stmt_count: 0,
        }
    }

    // ----- cursor helpers -----

    fn cur(&self) -> &Token {
        // `tokenize` always terminates with an `Eof`, so the last token is a valid
        // sentinel and indexing past it clamps to `Eof`.
        self.toks.get(self.pos).unwrap_or_else(|| {
            self.toks
                .last()
                .expect("token stream always contains at least Eof")
        })
    }

    fn cur_kind(&self) -> &TokenKind {
        &self.cur().kind
    }

    fn cur_loc(&self) -> SourceLoc {
        self.cur().loc
    }

    fn advance(&mut self) -> Token {
        let t = self.cur().clone();
        if self.pos < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    /// True if the current token has the same variant as `k` (payload ignored).
    /// Only meaningful for payload-free variants (punctuation, operators, `Eof`).
    fn is(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(self.cur_kind()) == std::mem::discriminant(k)
    }

    fn eat(&mut self, k: &TokenKind) -> bool {
        if self.is(k) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_kind(&mut self, k: &TokenKind) -> PResult<()> {
        if self.eat(k) {
            Ok(())
        } else {
            Err(self.syntax_error(&format!("expected {k:?}")))
        }
    }

    /// The current token's upper-cased keyword name, if it is a suffix-less
    /// identifier. Word operators / command names / control keywords all surface
    /// this way.
    fn cur_keyword(&self) -> Option<&str> {
        match self.cur_kind() {
            TokenKind::Ident {
                name,
                suffix: Suffix::None,
            } => Some(name.as_str()),
            _ => None,
        }
    }

    fn at_kw(&self, kw: &str) -> bool {
        self.cur_keyword() == Some(kw)
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn syntax_error(&self, msg: &str) -> ParseError {
        ParseError {
            loc: self.cur_loc(),
            errnum: 3,
            msg: msg.to_string(),
        }
    }

    /// A block-structure mismatch with its own SmileBASIC errnum (20..29 — e.g. "NEXT
    /// without FOR" = 21, "DEF without END" = 29), distinct from the generic Syntax error
    /// (3). errnums per `spec/reference/errors.yaml` (disassembled error table @0x3054f8).
    fn structural_error(&self, errnum: u32, msg: &str) -> ParseError {
        ParseError {
            loc: self.cur_loc(),
            errnum,
            msg: msg.to_string(),
        }
    }

    // =========================================================================
    // Blocks & statements
    // =========================================================================

    /// True if the current token terminates `kind`'s block (the terminator is left
    /// unconsumed for the caller, except program/`Eof`).
    fn at_block_end(&self, kind: BlockKind) -> bool {
        if matches!(self.cur_kind(), TokenKind::Eof) {
            return true;
        }
        match kind {
            BlockKind::Program => false,
            BlockKind::Function => self.at_kw("END"),
            BlockKind::SingleLineIf => {
                matches!(self.cur_kind(), TokenKind::Newline)
                    || self.at_kw("ELSE")
                    || self.at_kw("ELSEIF")
                    || self.at_kw("ENDIF")
            }
            BlockKind::MultiLineIf => {
                self.at_kw("ELSE") || self.at_kw("ELSEIF") || self.at_kw("ENDIF")
            }
            BlockKind::For => self.at_kw("NEXT"),
            BlockKind::While => self.at_kw("WEND"),
            BlockKind::Repeat => self.at_kw("UNTIL"),
        }
    }

    /// Collect statements until the block's terminator. Statement separators
    /// (`:` always; newlines except in a single-line `IF`) are consumed between
    /// statements.
    fn parse_block(&mut self, kind: BlockKind) -> PResult<Block> {
        let multiline = !matches!(kind, BlockKind::SingleLineIf);
        let mut out = Block::new();
        loop {
            // Eat separators.
            loop {
                match self.cur_kind() {
                    TokenKind::Colon => {
                        self.advance();
                    }
                    TokenKind::Newline if multiline => {
                        self.advance();
                    }
                    _ => break,
                }
            }
            if self.at_block_end(kind) {
                break;
            }
            // In a single-line IF body a leading label is a GOTO target.
            if matches!(kind, BlockKind::SingleLineIf) {
                if let TokenKind::Label(name) = self.cur_kind() {
                    let name = name.clone();
                    let loc = self.cur_loc();
                    self.advance();
                    out.push(Stmt::new(StmtKind::Goto(Jump::Label(name)), loc));
                    continue;
                }
            }
            out.push(self.parse_statement()?);
            self.stmt_count = self.stmt_count.saturating_add(1);
        }
        Ok(out)
    }

    /// Parse one statement (dispatch on the leading token / keyword).
    fn parse_statement(&mut self) -> PResult<Stmt> {
        let loc = self.cur_loc();

        // `?` is the PRINT alias.
        if self.is(&TokenKind::Question) {
            self.advance();
            return self.parse_print(loc);
        }

        // A bare `@label` is a label definition.
        if let TokenKind::Label(name) = self.cur_kind() {
            let name = name.clone();
            self.advance();
            return Ok(Stmt::new(StmtKind::Label(name), loc));
        }

        let Some(kw) = self.cur_keyword() else {
            // A type-suffixed name (`A$`, `R%`, `X#`) is never a keyword — it is a
            // variable, so this is an assignment or a call (`A$="hi"`, `R%=…`).
            if matches!(self.cur_kind(), TokenKind::Ident { .. }) {
                return self.parse_assign_or_call(loc);
            }
            return Err(self.syntax_error("statement cannot start here"));
        };
        let kw = kw.to_string();

        match kw.as_str() {
            "PRINT" => {
                self.advance();
                self.parse_print(loc)
            }
            "IF" => self.parse_if(loc),
            "FOR" => self.parse_for(loc),
            "WHILE" => self.parse_while(loc),
            "REPEAT" => self.parse_repeat(loc),
            "GOTO" => {
                self.advance();
                let j = self.parse_jump()?;
                Ok(Stmt::new(StmtKind::Goto(j), loc))
            }
            "GOSUB" => {
                self.advance();
                let j = self.parse_jump()?;
                Ok(Stmt::new(StmtKind::Gosub(j), loc))
            }
            "RETURN" => {
                self.advance();
                let val = if self.can_start_expr() {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                Ok(Stmt::new(StmtKind::Return(val), loc))
            }
            "ON" => self.parse_on(loc),
            "END" => {
                self.advance();
                Ok(Stmt::new(StmtKind::End, loc))
            }
            "STOP" => {
                self.advance();
                Ok(Stmt::new(StmtKind::Stop, loc))
            }
            "BREAK" => {
                self.advance();
                Ok(Stmt::new(StmtKind::Break, loc))
            }
            "CONTINUE" => {
                self.advance();
                Ok(Stmt::new(StmtKind::Continue, loc))
            }
            // A bare `NEXT` used as a statement (not closing a visible `FOR` body)
            // is a loop-continue, e.g. the `IF cond THEN NEXT` idiom — it jumps to
            // the enclosing FOR's increment (osb `statement()` Next → Continue). A
            // `NEXT` that closes a FOR is consumed by `parse_for` before reaching
            // here. The optional variable list after it is ignored. With no open FOR
            // it is "NEXT without FOR" (errnum 21), not a Syntax error.
            "NEXT" => {
                if self.for_depth == 0 {
                    return Err(self.structural_error(21, "NEXT without FOR"));
                }
                self.advance();
                self.skip_next_var_list();
                Ok(Stmt::new(StmtKind::Continue, loc))
            }
            "DIM" => {
                self.advance();
                self.parse_dim(loc)
            }
            // `VAR(x)` is a by-reference expression (an lvalue), not a declaration:
            // fall through to the assignment/call path when a `(` follows.
            "VAR" if !self.peek_is_lparen_after_ident() => {
                self.advance();
                self.parse_dim(loc)
            }
            "DEF" => self.parse_def(loc, false),
            "COMMON" => {
                self.advance();
                if !self.at_kw("DEF") {
                    return Err(self.syntax_error("expected DEF after COMMON"));
                }
                // `parse_def` consumes the `DEF` keyword itself.
                self.parse_def(loc, true)
            }
            "DATA" => self.parse_data(loc),
            "READ" => self.parse_read(loc),
            "RESTORE" => {
                self.advance();
                // Bare `RESTORE` (no label) is accepted by the grammar but has no
                // reset-to-first semantics on real SB 3.6.0 — it raises Type mismatch
                // (8) at runtime (hw_verified). The compiler lowers `None` to that.
                let j = if self.at_arg_end() {
                    None
                } else {
                    Some(self.parse_jump()?)
                };
                Ok(Stmt::new(StmtKind::Restore(j), loc))
            }
            "INPUT" => self.parse_input(loc),
            "LINPUT" => self.parse_linput(loc),
            "INC" | "DEC" => self.parse_inc_dec(loc, kw == "DEC"),
            "SWAP" => self.parse_swap(loc),
            "OPTION" => self.parse_option(loc),
            "XON" | "XOFF" => self.parse_xon(loc, kw == "XOFF"),
            "USE" => {
                self.advance();
                let e = self.parse_expr()?;
                Ok(Stmt::new(StmtKind::Use(e), loc))
            }
            "EXEC" => {
                self.advance();
                let e = self.parse_expr()?;
                Ok(Stmt::new(StmtKind::Exec(e), loc))
            }
            // Stray loop-closing keywords get their own structural errnum (the matching
            // opener was never seen), per the error table.
            "WEND" => Err(self.structural_error(25, "WEND without WHILE")),
            "UNTIL" => Err(self.structural_error(23, "UNTIL without REPEAT")),
            // A stray `ENDIF` (no open IF block consumed it). Real SB raises the dedicated
            // "ENDIF without IF" (28) ONLY when the ENDIF is the program's first statement;
            // a stray ENDIF after any other statement collapses to generic Syntax error 3
            // (hw_verified sb-oracle 2026-06-23: `ENDIF`/`ENDIF\nPRINT 1`/`ENDIF:PRINT 1` → 28;
            // `PRINT 1\nENDIF`/`A=1\nENDIF`/`IF 1 THEN\nENDIF\nENDIF` → 3).
            "ENDIF" if self.stmt_count == 0 => Err(self.structural_error(28, "ENDIF without IF")),
            // Other stray block / clause keywords must not start a statement, and a
            // non-leading stray `ENDIF` falls here too. (`NEXT` is handled above as a
            // loop-continue / "NEXT without FOR"; `THEN`/`ELSE`/`ELSEIF` are consumed inside
            // `parse_if`, so reaching here means a malformed construct.)
            "THEN" | "ELSE" | "ELSEIF" | "ENDIF" | "TO" | "STEP" => {
                Err(self.syntax_error(&format!("unexpected `{kw}`")))
            }
            // Anything else is an assignment or a command call.
            _ => self.parse_assign_or_call(loc),
        }
    }

    /// Does an `(` immediately follow the current (identifier) token?
    fn peek_is_lparen_after_ident(&self) -> bool {
        matches!(
            self.toks.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::LParen)
        )
    }

    /// `NAME = expr` / `A[i]=expr` / `A(i)=expr` / `VAR(x)=expr`, or a command call.
    fn parse_assign_or_call(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        let save = self.pos;
        // The assignment target is parsed at rank ≤ 2 so a top-level `=` is NOT
        // swallowed as equality.
        let target = self.parse_operand()?;
        if self.is(&TokenKind::Assign) {
            self.advance();
            let rhs = self.parse_expr()?;
            return self.make_assignment(target, rhs, loc);
        }
        // A paren-form call `NAME(args)` whose parentheses span the *whole*
        // statement (e.g. `SIMPLE_INIT("a","b",1)`) was already parsed into a
        // `Call` — reuse it rather than re-parsing the parenthesised comma list as
        // one expression. But a bareword command whose *first argument* is
        // parenthesised, e.g. `LOCATE (20-LEN(S$)/2),Y` or `COLOR (X MOD 2)*3,0`,
        // also looks like `NAME(...)` here; for those, more tokens follow the `)`
        // (a `,`, `-`, `*`, …), so fall back to the comma-separated arg parser
        // where the leading `(` is just a grouped sub-expression. (The whitespace
        // that distinguishes the two forms in SmileBASIC is lost by the lexer.)
        if matches!(target.kind, ExprKind::Call { .. }) && (self.at_arg_end() || self.at_kw("OUT"))
        {
            let ExprKind::Call { name, args } = target.kind else {
                unreachable!()
            };
            let out_args = if self.eat_kw("OUT") {
                self.parse_out_args()?
            } else {
                Vec::new()
            };
            return Ok(Stmt::new(
                StmtKind::Call {
                    name,
                    args,
                    out_args,
                },
                loc,
            ));
        }
        // Otherwise (a bareword command like `LOCATE 5,5` or `COLOR (X)*3,0`):
        // rewind and parse the space/comma-separated argument list.
        self.pos = save;
        self.parse_command_call(loc)
    }

    /// Build the right assignment node for a target expression.
    fn make_assignment(&self, target: Expr, rhs: Expr, loc: SourceLoc) -> PResult<Stmt> {
        match target.kind {
            ExprKind::Var(name) => Ok(Stmt::new(StmtKind::Assign { name, expr: rhs }, loc)),
            // `A[i,j] = …` over a plain variable.
            ExprKind::Index { array, indices } if matches!(array.kind, ExprKind::Var(_)) => {
                let ExprKind::Var(name) = array.kind else {
                    unreachable!()
                };
                Ok(Stmt::new(
                    StmtKind::ArrayAssign {
                        name,
                        indices,
                        expr: rhs,
                    },
                    loc,
                ))
            }
            // `A(i) = …` paren-form array element assignment.
            ExprKind::Call { name, args } => Ok(Stmt::new(
                StmtKind::ArrayAssign {
                    name,
                    indices: args,
                    expr: rhs,
                },
                loc,
            )),
            // `VAR(x) = …` or an index over a non-variable lvalue.
            ExprKind::Ref(_) | ExprKind::Index { .. } => {
                if target.is_lvalue() {
                    Ok(Stmt::new(StmtKind::AssignRef { target, expr: rhs }, loc))
                } else {
                    Err(ParseError {
                        loc,
                        errnum: 3,
                        msg: "invalid assignment target".into(),
                    })
                }
            }
            _ => Err(ParseError {
                loc,
                errnum: 3,
                msg: "invalid assignment target".into(),
            }),
        }
    }

    /// `NAME arg, arg, … [OUT lvalue, …]` — a command / function call statement.
    fn parse_command_call(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        let name = self.expect_name()?;
        let mut args: Vec<Expr> = Vec::new();
        let mut out_args: Vec<Expr> = Vec::new();
        let mut prev_comma = false;
        loop {
            if self.at_arg_end() || self.at_kw("OUT") {
                if prev_comma {
                    // A trailing comma leaves an omitted final argument.
                    args.push(Expr::new(ExprKind::Void, self.cur_loc()));
                }
                break;
            }
            if self.is(&TokenKind::Comma) {
                // An omitted argument, e.g. `LOCATE ,5`.
                args.push(Expr::new(ExprKind::Void, self.cur_loc()));
            } else {
                args.push(self.parse_expr()?);
            }
            if self.at_kw("OUT") {
                break;
            }
            if !self.is(&TokenKind::Comma) {
                break;
            }
            self.advance();
            prev_comma = true;
        }
        if self.eat_kw("OUT") {
            out_args = self.parse_out_args()?;
        }
        Ok(Stmt::new(
            StmtKind::Call {
                name,
                args,
                out_args,
            },
            loc,
        ))
    }

    /// The `OUT lvalue, lvalue, …` tail of a call (the `OUT` keyword already eaten).
    fn parse_out_args(&mut self) -> PResult<Vec<Expr>> {
        let mut out_args: Vec<Expr> = Vec::new();
        // `OUT` with no targets at all yields an empty list (no slot).
        if self.at_arg_end() {
            return Ok(out_args);
        }
        // Comma-separated slots, any of which may be omitted (e.g. `TOUCH OUT TM,,` keeps only
        // the touch time): N commas denote N+1 slots, so a leading/trailing/interior gap is a
        // `Void` placeholder that still counts toward the call's OUT count.
        loop {
            if self.is(&TokenKind::Comma) || self.at_arg_end() {
                out_args.push(Expr::new(ExprKind::Void, self.cur_loc()));
            } else {
                let e = self.parse_expr()?;
                if !e.is_lvalue() {
                    return Err(self.syntax_error("OUT argument must be an lvalue"));
                }
                out_args.push(e);
            }
            if self.is(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(out_args)
    }

    /// True at the end of a statement's argument list.
    fn at_arg_end(&self) -> bool {
        matches!(
            self.cur_kind(),
            TokenKind::Newline | TokenKind::Colon | TokenKind::Eof
        ) || self.cur_starts_statement()
    }

    /// True if the current token can only begin a *new statement* — the `?` PRINT
    /// alias or a control/statement keyword. Such a token never appears inside an
    /// expression or argument, so it ends the current statement even without a `:`
    /// (e.g. `COLOR 5,PRINT "x"`). `VAR` and the word operators are deliberately
    /// excluded: `VAR(x)` is an expression and `AND`/`OR`/… are operators.
    fn cur_starts_statement(&self) -> bool {
        if self.is(&TokenKind::Question) {
            return true;
        }
        matches!(
            self.cur_keyword(),
            Some(
                "PRINT"
                    | "IF"
                    | "THEN"
                    | "ELSE"
                    | "ELSEIF"
                    | "ENDIF"
                    | "FOR"
                    | "TO"
                    | "STEP"
                    | "NEXT"
                    | "WHILE"
                    | "WEND"
                    | "REPEAT"
                    | "UNTIL"
                    | "GOTO"
                    | "GOSUB"
                    | "RETURN"
                    | "ON"
                    | "END"
                    | "STOP"
                    | "BREAK"
                    | "CONTINUE"
                    | "DIM"
                    | "DEF"
                    | "COMMON"
                    | "DATA"
                    | "READ"
                    | "RESTORE"
                    | "INPUT"
                    | "LINPUT"
                    | "INC"
                    | "DEC"
                    | "SWAP"
                    | "OPTION"
                    | "USE"
                    | "EXEC"
            )
        )
    }

    // ----- PRINT -----

    fn parse_print(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        let mut items: Vec<PrintItem> = Vec::new();
        // `addline` tracks whether a trailing newline is emitted: a `PRINT` that
        // ends right after `;` or `,` suppresses it.
        let mut addline = true;
        loop {
            if self.at_print_end() {
                if addline {
                    items.push(PrintItem::NewLine);
                }
                break;
            }
            items.push(PrintItem::Expr(self.parse_expr()?));
            match self.cur_kind() {
                TokenKind::Semicolon => {
                    self.advance();
                    addline = false;
                }
                TokenKind::Comma => {
                    self.advance();
                    items.push(PrintItem::Tab);
                    addline = false;
                }
                // Any other token ends the PRINT and begins a new statement —
                // SmileBASIC lets a statement abut PRINT without a `:`
                // (e.g. `?"":INC M` written as `?""INC M`, `PRINT "x" BEEP 2`).
                _ => {
                    items.push(PrintItem::NewLine);
                    break;
                }
            }
        }
        Ok(Stmt::new(StmtKind::Print(items), loc))
    }

    fn at_print_end(&self) -> bool {
        matches!(
            self.cur_kind(),
            TokenKind::Colon | TokenKind::Newline | TokenKind::Eof | TokenKind::Question
        ) || self.cur_starts_statement()
    }

    // ----- IF -----

    fn parse_if(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // IF
        let cond = self.parse_expr()?;
        let multiline = self.expect_then()?;
        let then_body = self.parse_if_body(multiline)?;

        let mut elseifs: Vec<ElseIf> = Vec::new();
        let mut else_body: Block = Block::new();
        loop {
            if self.eat_kw("ELSEIF") {
                let c = self.parse_expr()?;
                let ml = self.expect_then()?;
                let body = self.parse_if_body(ml || multiline)?;
                elseifs.push(ElseIf { cond: c, body });
                continue;
            }
            break;
        }
        if self.eat_kw("ELSE") {
            else_body = self.parse_if_body(multiline)?;
        }
        // ENDIF is REQUIRED for a multi-line IF and optional for a single-line one. Real SB
        // does NOT surface the table's "THEN/ELSE without ENDIF" (26/27) for an unterminated
        // multi-line block — it collapses to generic Syntax error 3 (hw_verified sb-oracle
        // 2026-06-23: `IF 1 THEN\nPRINT 1` and `IF 1 THEN\nA=1\nELSE\nB=2` both → errnum 3).
        // (The earlier silent-accept of a missing ENDIF was a latent bug.)
        if multiline {
            if !self.eat_kw("ENDIF") {
                return Err(self.syntax_error("multi-line IF block missing ENDIF"));
            }
        } else {
            self.eat_kw("ENDIF");
        }

        Ok(Stmt::new(
            StmtKind::If {
                cond,
                then_body,
                elseifs,
                else_body,
            },
            loc,
        ))
    }

    /// Consume the `THEN` (or recognise the `IF c GOTO @L` form, where the `GOTO`
    /// is left for the body parser). Returns whether the body is multi-line (a
    /// newline immediately follows `THEN`).
    fn expect_then(&mut self) -> PResult<bool> {
        if self.at_kw("GOTO") {
            // `IF c GOTO @L`: the GOTO statement is the single-line body.
            return Ok(false);
        }
        if !self.eat_kw("THEN") {
            return Err(self.syntax_error("expected THEN or GOTO after IF condition"));
        }
        Ok(matches!(self.cur_kind(), TokenKind::Newline))
    }

    fn parse_if_body(&mut self, multiline: bool) -> PResult<Block> {
        if multiline {
            // Skip the newline after THEN, if present.
            self.eat(&TokenKind::Newline);
            self.parse_block(BlockKind::MultiLineIf)
        } else {
            self.parse_block(BlockKind::SingleLineIf)
        }
    }

    // ----- FOR / WHILE / REPEAT -----

    fn parse_for(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // FOR
        let var = self.expect_name()?;
        self.expect_kind(&TokenKind::Assign)?;
        let from = self.parse_expr()?;
        if !self.eat_kw("TO") {
            return Err(self.syntax_error("expected TO in FOR"));
        }
        let to = self.parse_expr()?;
        let step = if self.eat_kw("STEP") {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.for_depth += 1;
        let body = self.parse_block(BlockKind::For)?;
        self.for_depth -= 1;
        // Consume `NEXT [var[,var…]]` (the variable list is ignored, per osb /
        // SmileBASIC 3 — NEXT does not have to name the loop variable). An unterminated
        // FOR (block ended at EOF, not `NEXT`) is "FOR without NEXT" (errnum 20).
        if !self.eat_kw("NEXT") {
            return Err(self.structural_error(20, "FOR without NEXT"));
        }
        self.skip_next_var_list();
        Ok(Stmt::new(
            StmtKind::For {
                var,
                from,
                to,
                step,
                body,
            },
            loc,
        ))
    }

    fn skip_next_var_list(&mut self) {
        if self.cur_keyword().is_some() && !self.is_reserved_after_next() {
            self.advance();
            while self.is(&TokenKind::Comma) {
                self.advance();
                if self.cur_keyword().is_some() && !self.is_reserved_after_next() {
                    self.advance();
                } else {
                    break;
                }
            }
        }
    }

    /// Avoid swallowing the next statement's keyword as a `NEXT` variable.
    fn is_reserved_after_next(&self) -> bool {
        matches!(
            self.cur_keyword(),
            Some(
                "NEXT"
                    | "WEND"
                    | "UNTIL"
                    | "ENDIF"
                    | "ELSE"
                    | "ELSEIF"
                    | "END"
                    | "PRINT"
                    | "IF"
                    | "FOR"
                    | "WHILE"
                    | "REPEAT"
            )
        )
    }

    fn parse_while(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // WHILE
        let cond = self.parse_expr()?;
        let body = self.parse_block(BlockKind::While)?;
        if !self.eat_kw("WEND") {
            return Err(self.structural_error(24, "WHILE without WEND"));
        }
        Ok(Stmt::new(StmtKind::While { cond, body }, loc))
    }

    fn parse_repeat(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // REPEAT
        let body = self.parse_block(BlockKind::Repeat)?;
        if !self.eat_kw("UNTIL") {
            return Err(self.structural_error(22, "REPEAT without UNTIL"));
        }
        let cond = self.parse_expr()?;
        Ok(Stmt::new(StmtKind::RepeatUntil { body, cond }, loc))
    }

    // ----- ON … GOTO/GOSUB -----

    fn parse_on(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // ON
        let value = self.parse_expr()?;
        let kind = if self.eat_kw("GOTO") {
            OnKind::Goto
        } else if self.eat_kw("GOSUB") {
            OnKind::Gosub
        } else {
            return Err(self.syntax_error("expected GOTO or GOSUB after ON"));
        };
        let mut labels = vec![self.parse_jump()?];
        while self.is(&TokenKind::Comma) {
            self.advance();
            labels.push(self.parse_jump()?);
        }
        Ok(Stmt::new(
            StmtKind::On {
                value,
                kind,
                labels,
            },
            loc,
        ))
    }

    // ----- DIM / VAR -----

    fn parse_dim(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        let mut items: Vec<DimItem> = Vec::new();
        loop {
            let name = self.expect_name()?;
            if self.eat(&TokenKind::LBracket) {
                let dims = self.parse_index_list()?;
                self.expect_kind(&TokenKind::RBracket)?;
                items.push(DimItem::Array { name, dims });
            } else if self.eat(&TokenKind::Assign) {
                let init = self.parse_expr()?;
                items.push(DimItem::Scalar {
                    name,
                    init: Some(init),
                });
            } else {
                items.push(DimItem::Scalar { name, init: None });
            }
            if self.is(&TokenKind::Comma) {
                self.advance();
                // A trailing comma with no following name ends the list.
                if !matches!(self.cur_kind(), TokenKind::Ident { .. }) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(Stmt::new(StmtKind::Dim(items), loc))
    }

    // ----- DEF / COMMON DEF -----

    fn parse_def(&mut self, loc: SourceLoc, is_common: bool) -> PResult<Stmt> {
        self.advance(); // DEF
        let name = self.expect_name()?;
        let mut params: Vec<Name> = Vec::new();
        let mut out_params: Vec<Name> = Vec::new();
        let returns_value;
        if self.eat(&TokenKind::LParen) {
            // Function form: `DEF F(a, b)` — returns a value.
            returns_value = true;
            while !self.is(&TokenKind::RParen) {
                params.push(self.parse_def_param()?);
                if self.is(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
            self.expect_kind(&TokenKind::RParen)?;
            // The parenthesised `(args)` form is the single-return *function* syntax and
            // cannot be combined with OUT params: real SB 3.6.0 rejects
            // `DEF F(N) OUT D` with a Syntax error (errnum 3) at the DEF line. The
            // multi-output form omits the parentheses (`DEF F N OUT D`). hw_verified via
            // the sb-oracle skill (M7-T2 run 6): def_paren_out -> errnum=3, errline=1.
            if self.at_kw("OUT") {
                return Err(self.syntax_error("OUT params cannot follow parenthesised DEF args"));
            }
        } else {
            // Command form: `DEF C a, b [OUT x, y]` — no return value.
            returns_value = false;
            if matches!(self.cur_kind(), TokenKind::Ident { .. }) && !self.at_kw("OUT") {
                loop {
                    params.push(self.parse_def_param()?);
                    if self.is(&TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
            if self.eat_kw("OUT") {
                loop {
                    out_params.push(self.parse_def_param()?);
                    if self.is(&TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
        }
        let body = self.parse_block(BlockKind::Function)?;
        if !self.eat_kw("END") {
            return Err(self.structural_error(29, "DEF without END"));
        }
        Ok(Stmt::new(
            StmtKind::Def(DefineFunction {
                name,
                params,
                out_params,
                returns_value,
                is_common,
                body,
            }),
            loc,
        ))
    }

    /// A `DEF` parameter name; a trailing `[]` (array parameter) is accepted and
    /// ignored, per osb ("引数に[]を付けようが扱いは同一").
    fn parse_def_param(&mut self) -> PResult<Name> {
        let name = self.expect_name()?;
        if self.eat(&TokenKind::LBracket) {
            self.expect_kind(&TokenKind::RBracket)?;
        }
        Ok(name)
    }

    // ----- DATA / READ / RESTORE -----

    fn parse_data(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // DATA
        let mut items: Vec<Lit> = Vec::new();
        loop {
            items.push(self.parse_data_item()?);
            if self.is(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(Stmt::new(StmtKind::Data(items), loc))
    }

    /// One `DATA` literal. A quoted string is a string; a standalone bareword
    /// identifier is an unquoted string (`DATA APPLE` → `"APPLE"`); anything else is
    /// a constant numeric expression, folded to its value (`DATA 6*4` → `24`,
    /// `DATA 25*4+1` → `101`, `DATA -2` → `-2`). The corpus shows arithmetic in
    /// `DATA` is common, so items go through the expression parser + folder.
    ///
    /// (Two cases the token stream can't fully recover are left for the oracle —
    /// A `#NAME` item (`DATA #L` → 256) folds via the constant table in `parse_primary`
    /// before reaching here, so it arrives as a plain `Const` (hw_verified, S-T4d/S-T14c).
    /// (Still queued: spaces inside an unquoted `DATA` string up to the comma.)
    fn parse_data_item(&mut self) -> PResult<Lit> {
        // Quoted string.
        if let TokenKind::Str(s) = self.cur_kind() {
            let s = s.clone();
            self.advance();
            return Ok(Lit::Str(s));
        }
        // Standalone bareword → unquoted string. (If a bareword is followed by an
        // operator it would not be a constant anyway, so only treat it as a string
        // when an item separator follows.)
        if let TokenKind::Ident { name, .. } = self.cur_kind() {
            let next_is_sep = matches!(
                self.toks.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Comma | TokenKind::Newline | TokenKind::Colon | TokenKind::Eof)
            );
            if next_is_sep {
                let name = name.clone();
                self.advance();
                return Ok(Lit::Str(name));
            }
        }
        // Otherwise a constant numeric expression.
        let e = self.parse_expr()?;
        match e.kind {
            ExprKind::Const(lit) => Ok(lit),
            _ => Err(self.syntax_error("DATA item must be a constant")),
        }
    }

    fn parse_read(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // READ
        let mut vars: Vec<Expr> = Vec::new();
        loop {
            let v = self.parse_expr()?;
            if !v.is_lvalue() {
                return Err(self.syntax_error("READ target must be a variable"));
            }
            vars.push(v);
            if self.is(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(Stmt::new(StmtKind::Read(vars), loc))
    }

    // ----- INPUT / LINPUT -----

    fn parse_input(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // INPUT
        let first = self.parse_expr()?;
        // `INPUT "prompt"; v` / `INPUT "prompt", v` use the first expr as a prompt;
        // a `,` only counts as a prompt separator when the first expr is not itself
        // a variable.
        let prompt_sep =
            self.is(&TokenKind::Semicolon) || (self.is(&TokenKind::Comma) && !first.is_lvalue());
        if prompt_sep {
            let question = self.is(&TokenKind::Semicolon);
            let mut vars: Vec<Expr> = Vec::new();
            loop {
                self.advance(); // the `;` / `,` separator (or a later `,`)
                let v = self.parse_expr()?;
                if !v.is_lvalue() {
                    return Err(self.syntax_error("INPUT target must be a variable"));
                }
                vars.push(v);
                if !self.is(&TokenKind::Comma) {
                    break;
                }
            }
            Ok(Stmt::new(
                StmtKind::Input {
                    prompt: Some(first),
                    question,
                    vars,
                },
                loc,
            ))
        } else {
            if !first.is_lvalue() {
                return Err(self.syntax_error("INPUT target must be a variable"));
            }
            let mut vars = vec![first];
            while self.is(&TokenKind::Comma) {
                self.advance();
                let v = self.parse_expr()?;
                if !v.is_lvalue() {
                    return Err(self.syntax_error("INPUT target must be a variable"));
                }
                vars.push(v);
            }
            Ok(Stmt::new(
                StmtKind::Input {
                    prompt: None,
                    question: true,
                    vars,
                },
                loc,
            ))
        }
    }

    fn parse_linput(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // LINPUT
        let first = self.parse_expr()?;
        if self.eat(&TokenKind::Semicolon) {
            let var = self.parse_expr()?;
            if !var.is_lvalue() {
                return Err(self.syntax_error("LINPUT target must be a variable"));
            }
            Ok(Stmt::new(
                StmtKind::Linput {
                    prompt: Some(first),
                    var,
                },
                loc,
            ))
        } else {
            if !first.is_lvalue() {
                return Err(self.syntax_error("LINPUT target must be a variable"));
            }
            Ok(Stmt::new(
                StmtKind::Linput {
                    prompt: None,
                    var: first,
                },
                loc,
            ))
        }
    }

    // ----- INC / DEC / SWAP / OPTION -----

    fn parse_inc_dec(&mut self, loc: SourceLoc, is_dec: bool) -> PResult<Stmt> {
        self.advance(); // INC / DEC
        let target = self.parse_expr()?;
        if !target.is_lvalue() {
            return Err(self.syntax_error("INC/DEC target must be a variable"));
        }
        let raw_delta = if self.eat(&TokenKind::Comma) {
            self.parse_expr()?
        } else {
            Expr::constant(Lit::Int(1), loc)
        };
        // `DEC v, d` decrements by `d`: lower it to a negated delta (folded if `d`
        // is a constant), so the compiler only needs the `Inc` form.
        let delta = if is_dec {
            self.negate(raw_delta)
        } else {
            raw_delta
        };
        Ok(Stmt::new(StmtKind::Inc { target, delta }, loc))
    }

    fn parse_swap(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // SWAP
        let a = self.parse_expr()?;
        self.expect_kind(&TokenKind::Comma)?;
        let b = self.parse_expr()?;
        if !a.is_lvalue() || !b.is_lvalue() {
            return Err(self.syntax_error("SWAP arguments must be variables"));
        }
        Ok(Stmt::new(StmtKind::Swap { a, b }, loc))
    }

    fn parse_option(&mut self, loc: SourceLoc) -> PResult<Stmt> {
        self.advance(); // OPTION
        let arg = match self.cur_keyword() {
            Some(a @ ("STRICT" | "DEFINT" | "TOOL")) => a.to_string(),
            _ => return Err(self.syntax_error("expected STRICT, DEFINT or TOOL after OPTION")),
        };
        self.advance();
        Ok(Stmt::new(StmtKind::Option(arg), loc))
    }

    /// `XON feature` / `XOFF feature` (M6-T5) — enable/disable a special hardware feature.
    /// The feature is a bareword keyword (MOTION / EXPAD / MIC), like `OPTION STRICT`, so we
    /// lower it to a `XON`/`XOFF` command call carrying a synthetic integer feature code
    /// (0=MOTION, 1=EXPAD, 2=MIC) that the VM's `call_device` interprets.
    fn parse_xon(&mut self, loc: SourceLoc, is_off: bool) -> PResult<Stmt> {
        self.advance(); // XON / XOFF
        let code = match self.cur_keyword() {
            Some("MOTION") => 0,
            Some("EXPAD") => 1,
            Some("MIC") => 2,
            _ => return Err(self.syntax_error("expected MOTION, EXPAD or MIC after XON/XOFF")),
        };
        self.advance();
        let name = Name::new(if is_off { "XOFF" } else { "XON" }, Suffix::None);
        Ok(Stmt::new(
            StmtKind::Call {
                name,
                args: vec![Expr::constant(Lit::Int(code), loc)],
                out_args: Vec::new(),
            },
            loc,
        ))
    }

    // ----- shared helpers -----

    /// Parse a `GOTO`/`GOSUB`/`RESTORE`/`ON` target: a literal `@label` or a
    /// computed expression.
    fn parse_jump(&mut self) -> PResult<Jump> {
        if let TokenKind::Label(name) = self.cur_kind() {
            let name = name.clone();
            self.advance();
            Ok(Jump::Label(name))
        } else {
            Ok(Jump::Computed(self.parse_expr()?))
        }
    }

    /// Consume the current token as a `Name` (identifier + suffix).
    fn expect_name(&mut self) -> PResult<Name> {
        match self.cur_kind().clone() {
            TokenKind::Ident { name, suffix } => {
                self.advance();
                Ok(Name::new(name, suffix))
            }
            _ => Err(self.syntax_error("expected a name")),
        }
    }

    // =========================================================================
    // Expressions (precedence climbing + constant folding)
    // =========================================================================

    /// Parse a full expression. `=` here means equality.
    pub fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_binary(MAX_RANK)
    }

    /// Precedence climbing. A *lower* rank binds *tighter*; an operator of rank `r`
    /// parses its right operand at `r - 1`, so equal-rank operators chain left.
    fn parse_binary(&mut self, max_rank: u8) -> PResult<Expr> {
        let mut lhs = self.parse_operand()?;
        while let Some((op, oploc)) = self.peek_binop() {
            let r = op.rank();
            if r > max_rank {
                break;
            }
            self.advance(); // the (single-token) operator
            let rhs = self.parse_binary(r - 1)?;
            lhs = self.make_binary(op, lhs, rhs, oploc);
        }
        Ok(lhs)
    }

    /// The current token as a binary operator, if any (symbolic via
    /// [`BinOp::from_token`] — note `=`/`==` both map to [`BinOp::Eq`] here — or a
    /// word operator `AND`/`OR`/`XOR`/`MOD`/`DIV`).
    fn peek_binop(&self) -> Option<(BinOp, SourceLoc)> {
        let loc = self.cur_loc();
        if let Some(op) = BinOp::from_token(self.cur_kind()) {
            return Some((op, loc));
        }
        if let Some(kw) = self.cur_keyword() {
            if let Some(op) = BinOp::from_word(kw) {
                return Some((op, loc));
            }
        }
        None
    }

    /// Build a binary node, folding constant-`op`-constant per the runtime numeric
    /// rules (see the module docs).
    fn make_binary(&self, op: BinOp, lhs: Expr, rhs: Expr, oploc: SourceLoc) -> Expr {
        if let (ExprKind::Const(a), ExprKind::Const(b)) = (&lhs.kind, &rhs.kind) {
            if let Some(folded) = fold_binary(op, a, b) {
                return Expr::constant(folded, lhs.loc);
            }
        }
        Expr::new(
            ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            oploc,
        )
    }

    /// A rank-≤2 operand: array indexing (`[ ]`) over a unary/primary factor. This
    /// is what the assignment dispatcher uses so a top-level `=` is left alone.
    fn parse_operand(&mut self) -> PResult<Expr> {
        let mut base = self.parse_factor()?;
        while self.is(&TokenKind::LBracket) {
            let bl = base.loc;
            self.advance();
            let indices = self.parse_index_list()?;
            self.expect_kind(&TokenKind::RBracket)?;
            base = Expr::new(
                ExprKind::Index {
                    array: Box::new(base),
                    indices,
                },
                bl,
            );
        }
        Ok(base)
    }

    /// A unary-prefixed (`-`, `!`, `NOT`) factor, or a primary. Unary minus on a
    /// constant is folded.
    fn parse_factor(&mut self) -> PResult<Expr> {
        let loc = self.cur_loc();
        if self.is(&TokenKind::Minus) {
            self.advance();
            let operand = self.parse_operand()?;
            return Ok(self.negate(operand));
        }
        if self.is(&TokenKind::Bang) {
            self.advance();
            let operand = self.parse_operand()?;
            return Ok(Expr::new(
                ExprKind::Unary {
                    op: UnOp::LNot,
                    operand: Box::new(operand),
                },
                loc,
            ));
        }
        if self.at_kw("NOT") {
            self.advance();
            let operand = self.parse_operand()?;
            return Ok(Expr::new(
                ExprKind::Unary {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                },
                loc,
            ));
        }
        self.parse_primary()
    }

    /// Negate an expression, folding a constant operand.
    fn negate(&self, e: Expr) -> Expr {
        match &e.kind {
            ExprKind::Const(Lit::Int(n)) => Expr::constant(Lit::Int(n.wrapping_neg()), e.loc),
            ExprKind::Const(Lit::Real(x)) => Expr::constant(Lit::Real(-x), e.loc),
            _ => {
                let loc = e.loc;
                Expr::new(
                    ExprKind::Unary {
                        op: UnOp::Neg,
                        operand: Box::new(e),
                    },
                    loc,
                )
            }
        }
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let loc = self.cur_loc();
        // A statement-only command keyword (PRINT/INPUT/LINPUT/FOR/…) can never be
        // a value: SmileBASIC rejects it in expression position with a Syntax error
        // (errnum 3) *before* the handler runs — e.g. `A=LINPUT("X")` is errnum 3,
        // not the undefined-call errnum 16 it would get if parsed as `Call{LINPUT}`
        // (hw_verified: sb-oracle s_t5b 2026-06-22, linput.yaml). The same predicate
        // that ends an abutting statement marks exactly these keywords; `VAR(x)` and
        // the word operators are deliberately excluded (see `cur_starts_statement`).
        if self.cur_starts_statement() {
            return Err(self.syntax_error("expected an expression, found a command keyword"));
        }
        match self.cur_kind().clone() {
            TokenKind::Int(n) => {
                self.advance();
                Ok(Expr::constant(Lit::Int(n), loc))
            }
            TokenKind::Real(x) => {
                self.advance();
                Ok(Expr::constant(Lit::Real(x), loc))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::constant(Lit::Str(s), loc))
            }
            // A bare `@label` used as a value is the string `"@LABEL"` (osb 3.1).
            TokenKind::Label(name) => {
                self.advance();
                Ok(Expr::constant(Lit::Str(format!("@{name}")), loc))
            }
            // A built-in `#NAME` constant (`#WHITE`, `#UP`, `#L`, …). SmileBASIC folds these
            // to an inline Integer literal wherever they appear — they are not runtime
            // variables (see `crate::consts` / `spec/reference/constants.yaml`). Resolving
            // here means the value participates in constant-folding and is a legal `DATA`
            // item. An UNKNOWN `#NAME` keeps the `#`-prefixed `Var` marker (variable names
            // can never contain `#`); the compiler resolves/rejects it later. (Exact errnum
            // for an undefined `#const` is oracle-pending — see HARVEST_QUEUE.md.)
            TokenKind::Const(name) => {
                self.advance();
                match consts::lookup(&name) {
                    Some(v) => Ok(Expr::constant(Lit::Int(v), loc)),
                    None => Ok(Expr::var(Name::new(format!("#{name}"), Suffix::None), loc)),
                }
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect_kind(&TokenKind::RParen)?;
                Ok(inner)
            }
            TokenKind::Ident { name, suffix } => {
                self.advance();
                // `VAR(x)` — pass an lvalue by reference.
                if name == "VAR" && suffix == Suffix::None && self.is(&TokenKind::LParen) {
                    self.advance();
                    let inner = self.parse_expr()?;
                    self.expect_kind(&TokenKind::RParen)?;
                    return Ok(Expr::new(ExprKind::Ref(Box::new(inner)), loc));
                }
                // `name(args)` — a call or paren-form array read (resolved later).
                if self.is(&TokenKind::LParen) {
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect_kind(&TokenKind::RParen)?;
                    return Ok(Expr::new(
                        ExprKind::Call {
                            name: Name::new(name, suffix),
                            args,
                        },
                        loc,
                    ));
                }
                Ok(Expr::var(Name::new(name, suffix), loc))
            }
            _ => Err(self.syntax_error("expected an expression")),
        }
    }

    /// Comma-separated call arguments up to (not including) the closing `)`.
    /// An omitted argument (e.g. `F(,5)`) is an [`ExprKind::Void`].
    fn parse_call_args(&mut self) -> PResult<Vec<Expr>> {
        let mut args: Vec<Expr> = Vec::new();
        if self.is(&TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            if self.is(&TokenKind::Comma) || self.is(&TokenKind::RParen) {
                args.push(Expr::new(ExprKind::Void, self.cur_loc()));
            } else {
                args.push(self.parse_expr()?);
            }
            if self.is(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(args)
    }

    /// 1–4 comma-separated subscript expressions up to (not including) the `]`.
    fn parse_index_list(&mut self) -> PResult<Vec<Expr>> {
        let mut v: Vec<Expr> = Vec::new();
        loop {
            v.push(self.parse_expr()?);
            if v.len() > 4 {
                return Err(self.syntax_error("at most 4 array dimensions"));
            }
            if self.is(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(v)
    }

    /// True if the current token can begin an expression.
    fn can_start_expr(&self) -> bool {
        match self.cur_kind() {
            TokenKind::Int(_)
            | TokenKind::Real(_)
            | TokenKind::Str(_)
            | TokenKind::Ident { .. }
            | TokenKind::Label(_)
            | TokenKind::Const(_)
            | TokenKind::LParen
            | TokenKind::Minus
            | TokenKind::Bang => true,
            // `NOT` arrives as an Ident, already covered above.
            _ => false,
        }
    }
}

// =============================================================================
// Constant folding (free functions — pure, runtime numeric semantics)
// =============================================================================

/// Fold a constant-`op`-constant numeric subexpression, or return `None` to leave
/// it for the VM. `None` is returned for non-numeric operands, for a divide-by-zero
/// (so the VM raises errnum 7 only if reached), and for operators whose 3.6.0
/// semantics are not folded here (shifts, comparisons, `&&`, `||`).
fn fold_binary(op: BinOp, a: &Lit, b: &Lit) -> Option<Lit> {
    use BinOp::*;
    match op {
        Add | Sub | Mul => match (a, b) {
            (Lit::Int(x), Lit::Int(y)) => Some(Lit::Int(int_arith(op, *x, *y))),
            _ => {
                let (x, y) = (to_f64(a)?, to_f64(b)?);
                Some(Lit::Real(real_arith(op, x, y)))
            }
        },
        // `/` is always real division; never fold a divide-by-zero.
        Div => {
            let (x, y) = (to_f64(a)?, to_f64(b)?);
            if y == 0.0 {
                None
            } else {
                Some(Lit::Real(x / y))
            }
        }
        // Integer division / remainder: fold only two integer constants (truncating
        // a Real operand out of i32 range is unverified — left to the VM).
        IntDiv => match (a, b) {
            (Lit::Int(x), Lit::Int(y)) if *y != 0 => Some(Lit::Int(x.wrapping_div(*y))),
            _ => None,
        },
        Mod => match (a, b) {
            (Lit::Int(x), Lit::Int(y)) if *y != 0 => Some(Lit::Int(x.wrapping_rem(*y))),
            _ => None,
        },
        And => int_pair(a, b, |x, y| x & y),
        Or => int_pair(a, b, |x, y| x | y),
        Xor => int_pair(a, b, |x, y| x ^ y),
        // Shifts, comparisons, and short-circuit logicals are left to the VM.
        Shl | Shr | Eq | Ne | Lt | Le | Gt | Ge | LAnd | LOr => None,
    }
}

fn int_arith(op: BinOp, x: i32, y: i32) -> i32 {
    match op {
        BinOp::Add => x.wrapping_add(y),
        BinOp::Sub => x.wrapping_sub(y),
        BinOp::Mul => x.wrapping_mul(y),
        _ => unreachable!("int_arith only handles + - *"),
    }
}

fn real_arith(op: BinOp, x: f64, y: f64) -> f64 {
    match op {
        BinOp::Add => x + y,
        BinOp::Sub => x - y,
        BinOp::Mul => x * y,
        _ => unreachable!("real_arith only handles + - *"),
    }
}

/// Fold a bitwise op over two integer constants only.
fn int_pair(a: &Lit, b: &Lit, f: impl Fn(i32, i32) -> i32) -> Option<Lit> {
    match (a, b) {
        (Lit::Int(x), Lit::Int(y)) => Some(Lit::Int(f(*x, *y))),
        _ => None,
    }
}

fn to_f64(l: &Lit) -> Option<f64> {
    match l {
        Lit::Int(n) => Some(*n as f64),
        Lit::Real(x) => Some(*x),
        Lit::Str(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(src: &str) -> Expr {
        parse_expression(src).expect("expression should parse")
    }

    fn prog(src: &str) -> Block {
        parse(src).expect("program should parse")
    }

    // ----- expressions: precedence -----

    #[test]
    fn precedence_mul_over_add() {
        // 2+3*4 → 2 + (3*4); both constant → folds to 14.
        assert_eq!(expr("2+3*4").kind, ExprKind::Const(Lit::Int(14)));
        // (2+3)*4 → 20.
        assert_eq!(expr("(2+3)*4").kind, ExprKind::Const(Lit::Int(20)));
    }

    #[test]
    fn precedence_with_variables_is_left_assoc() {
        // A-B-C → (A-B)-C.
        let e = expr("A-B-C");
        let ExprKind::Binary { op, lhs, rhs } = &e.kind else {
            panic!("expected binary, got {:?}", e.kind);
        };
        assert_eq!(*op, BinOp::Sub);
        assert!(matches!(rhs.kind, ExprKind::Var(_)));
        assert!(matches!(lhs.kind, ExprKind::Binary { op: BinOp::Sub, .. }));
    }

    #[test]
    fn comparison_binds_looser_than_arithmetic() {
        // A+1 == B → (A+1) == B  (comparison is rank 6, + is rank 4).
        let e = expr("A+1 == B");
        let ExprKind::Binary { op, lhs, .. } = &e.kind else {
            panic!("expected binary");
        };
        assert_eq!(*op, BinOp::Eq);
        assert!(matches!(lhs.kind, ExprKind::Binary { op: BinOp::Add, .. }));
    }

    #[test]
    fn bare_equals_is_equality_inside_expression() {
        // `=` inside an expression means equality (==).
        assert!(matches!(
            expr("A = B").kind,
            ExprKind::Binary { op: BinOp::Eq, .. }
        ));
    }

    #[test]
    fn word_operators_parse() {
        assert!(matches!(
            expr("A MOD B").kind,
            ExprKind::Binary { op: BinOp::Mod, .. }
        ));
        assert!(matches!(
            expr("X AND Y").kind,
            ExprKind::Binary { op: BinOp::And, .. }
        ));
        // 1+2 AND 4 → (1+2) AND 4 (AND looser than +) → folds 3 AND 4 → 0.
        assert_eq!(expr("1+2 AND 4").kind, ExprKind::Const(Lit::Int(0)));
    }

    // ----- expressions: unary -----

    #[test]
    fn unary_minus_folds_constants() {
        assert_eq!(expr("-5").kind, ExprKind::Const(Lit::Int(-5)));
        assert_eq!(expr("- -5").kind, ExprKind::Const(Lit::Int(5)));
        assert_eq!(expr("-2.5").kind, ExprKind::Const(Lit::Real(-2.5)));
        // -2*3 → (-2)*3 → -6.
        assert_eq!(expr("-2*3").kind, ExprKind::Const(Lit::Int(-6)));
    }

    #[test]
    fn unary_minus_on_variable_is_a_node() {
        let e = expr("-A");
        assert!(matches!(e.kind, ExprKind::Unary { op: UnOp::Neg, .. }));
    }

    #[test]
    fn not_and_lnot_are_nodes() {
        assert!(matches!(
            expr("NOT A").kind,
            ExprKind::Unary { op: UnOp::Not, .. }
        ));
        assert!(matches!(
            expr("!A").kind,
            ExprKind::Unary { op: UnOp::LNot, .. }
        ));
    }

    // ----- expressions: constant folding semantics -----

    #[test]
    fn folding_matches_runtime_numeric_rules() {
        // Integer + integer wraps mod 2^32.
        assert_eq!(
            expr("2147483647+1").kind,
            ExprKind::Const(Lit::Int(i32::MIN))
        );
        // `/` is always real division.
        assert_eq!(expr("7/2").kind, ExprKind::Const(Lit::Real(3.5)));
        assert_eq!(expr("6/2").kind, ExprKind::Const(Lit::Real(3.0)));
        // DIV / MOD truncate to integers (matches div.yaml / mod.yaml).
        assert_eq!(expr("200 DIV 5").kind, ExprKind::Const(Lit::Int(40)));
        assert_eq!(expr("-7 DIV 2").kind, ExprKind::Const(Lit::Int(-3)));
        assert_eq!(expr("-7 MOD 3").kind, ExprKind::Const(Lit::Int(-1)));
        // Bitwise ops fold over integers.
        assert_eq!(expr("200 AND &HE7").kind, ExprKind::Const(Lit::Int(192)));
        assert_eq!(expr("128 OR &HA3").kind, ExprKind::Const(Lit::Int(163)));
        assert_eq!(expr("100 XOR &H4C").kind, ExprKind::Const(Lit::Int(40)));
        // Mixed int/real arithmetic promotes to real.
        assert_eq!(expr("2+0.5").kind, ExprKind::Const(Lit::Real(2.5)));
    }

    #[test]
    fn divide_by_zero_is_not_folded() {
        // Left as a node so the VM raises errnum 7 only if the code is reached.
        assert!(matches!(
            expr("1/0").kind,
            ExprKind::Binary { op: BinOp::Div, .. }
        ));
        assert!(matches!(
            expr("1 DIV 0").kind,
            ExprKind::Binary {
                op: BinOp::IntDiv,
                ..
            }
        ));
        assert!(matches!(
            expr("1 MOD 0").kind,
            ExprKind::Binary { op: BinOp::Mod, .. }
        ));
    }

    #[test]
    fn comparisons_and_shifts_are_not_folded() {
        assert!(matches!(
            expr("1 == 1").kind,
            ExprKind::Binary { op: BinOp::Eq, .. }
        ));
        assert!(matches!(
            expr("1 << 4").kind,
            ExprKind::Binary { op: BinOp::Shl, .. }
        ));
    }

    // ----- expressions: postfix / calls / refs -----

    #[test]
    fn array_index_and_calls() {
        assert!(matches!(expr("A[1,2]").kind, ExprKind::Index { .. }));
        assert!(matches!(expr("ABS(X)").kind, ExprKind::Call { .. }));
        assert!(matches!(expr("VAR(X)").kind, ExprKind::Ref(_)));
    }

    #[test]
    fn known_const_folds_to_its_value() {
        // A built-in `#NAME` constant folds to an inline Integer (hw_verified S-T14c).
        assert_eq!(expr("#WHITE").kind, ExprKind::Const(Lit::Int(-460552)));
        assert_eq!(expr("#L").kind, ExprKind::Const(Lit::Int(256)));
        // Participates in constant folding.
        assert_eq!(expr("#L+1").kind, ExprKind::Const(Lit::Int(257)));
    }

    #[test]
    fn unknown_const_keeps_hash_marker() {
        // An undefined `#NAME` keeps the `#`-prefixed marker for the compiler to resolve/reject.
        let ExprKind::Var(name) = &expr("#NOTACONST").kind else {
            panic!("expected var marker for unknown const");
        };
        assert_eq!(name.ident, "#NOTACONST");
    }

    // ----- statements: assignment forms -----

    #[test]
    fn assignment_forms() {
        assert!(matches!(prog("A=1")[0].kind, StmtKind::Assign { .. }));
        assert!(matches!(
            prog("A[3]=1")[0].kind,
            StmtKind::ArrayAssign { .. }
        ));
        // Paren-form array element assignment.
        assert!(matches!(
            prog("A(3)=1")[0].kind,
            StmtKind::ArrayAssign { .. }
        ));
        assert!(matches!(
            prog("VAR(X)=1")[0].kind,
            StmtKind::AssignRef { .. }
        ));
    }

    #[test]
    fn assignment_rhs_can_use_equality() {
        // X = Y = Z → X := (Y == Z).
        let StmtKind::Assign { expr, .. } = &prog("X=Y=Z")[0].kind else {
            panic!("expected assign");
        };
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinOp::Eq, .. }));
    }

    // ----- statements: command calls -----

    #[test]
    fn command_calls() {
        let StmtKind::Call { name, args, .. } = &prog("LOCATE 5,10")[0].kind else {
            panic!("expected call");
        };
        assert_eq!(name.ident, "LOCATE");
        assert_eq!(args.len(), 2);

        // No-arg command.
        let StmtKind::Call { name, args, .. } = &prog("CLS")[0].kind else {
            panic!("expected call");
        };
        assert_eq!(name.ident, "CLS");
        assert!(args.is_empty());
    }

    #[test]
    fn paren_form_command_call() {
        // `NAME(a,b,c)` spanning the whole statement is a call.
        let StmtKind::Call { name, args, .. } = &prog("SIMPLE_INIT(\"a\",\"b\",1)")[0].kind else {
            panic!("expected call");
        };
        assert_eq!(name.ident, "SIMPLE_INIT");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn command_with_parenthesized_first_arg() {
        // `LOCATE (e1)-…, Y` — the leading `(` groups a sub-expression; LOCATE gets
        // two args, not one paren-call arg.
        let StmtKind::Call { name, args, .. } = &prog("LOCATE (20-LEN(S$)/2)-1,Y")[0].kind else {
            panic!("expected call");
        };
        assert_eq!(name.ident, "LOCATE");
        assert_eq!(args.len(), 2);
        // `COLOR (X MOD 2)*3,0`.
        let StmtKind::Call { args, .. } = &prog("COLOR (X MOD 2)*3,0")[0].kind else {
            panic!("expected call");
        };
        assert_eq!(args.len(), 2);
        assert!(matches!(
            args[0].kind,
            ExprKind::Binary { op: BinOp::Mul, .. }
        ));
    }

    #[test]
    fn command_call_with_omitted_arg() {
        // `LOCATE ,5` → first arg is Void.
        let StmtKind::Call { args, .. } = &prog("LOCATE ,5")[0].kind else {
            panic!("expected call");
        };
        assert!(matches!(args[0].kind, ExprKind::Void));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn command_call_with_out_args() {
        let StmtKind::Call {
            name,
            args,
            out_args,
        } = &prog("SPHITINFO ID OUT X,Y")[0].kind
        else {
            panic!("expected call");
        };
        assert_eq!(name.ident, "SPHITINFO");
        assert_eq!(args.len(), 1);
        assert_eq!(out_args.len(), 2);
    }

    // ----- statements: multi-statement lines -----

    #[test]
    fn multi_statement_line() {
        let p = prog("A=1:B=2:C=3");
        assert_eq!(p.len(), 3);
        assert!(p.iter().all(|s| matches!(s.kind, StmtKind::Assign { .. })));
    }

    // ----- statements: block constructs -----

    #[test]
    fn single_line_if() {
        let StmtKind::If {
            then_body,
            else_body,
            ..
        } = &prog("IF X THEN A=1 ELSE A=2")[0].kind
        else {
            panic!("expected if");
        };
        assert_eq!(then_body.len(), 1);
        assert_eq!(else_body.len(), 1);
    }

    #[test]
    fn if_goto_form() {
        // `IF c GOTO @L` → then-body is a single GOTO.
        let StmtKind::If { then_body, .. } = &prog("IF X GOTO @DONE")[0].kind else {
            panic!("expected if");
        };
        assert!(matches!(then_body[0].kind, StmtKind::Goto(_)));
    }

    #[test]
    fn single_line_if_label_is_goto() {
        // `IF c THEN @L` jumps.
        let StmtKind::If { then_body, .. } = &prog("IF X THEN @LOOP")[0].kind else {
            panic!("expected if");
        };
        assert!(matches!(then_body[0].kind, StmtKind::Goto(_)));
    }

    #[test]
    fn multiline_if_with_elseif() {
        let src = "IF A THEN\nX=1\nELSEIF B THEN\nX=2\nELSE\nX=3\nENDIF";
        let StmtKind::If {
            then_body,
            elseifs,
            else_body,
            ..
        } = &prog(src)[0].kind
        else {
            panic!("expected if");
        };
        assert_eq!(then_body.len(), 1);
        assert_eq!(elseifs.len(), 1);
        assert_eq!(else_body.len(), 1);
    }

    #[test]
    fn for_loop_with_step() {
        let StmtKind::For {
            var, step, body, ..
        } = &prog("FOR I=0 TO 10 STEP 2\nPRINT I\nNEXT")[0].kind
        else {
            panic!("expected for");
        };
        assert_eq!(var.ident, "I");
        assert!(step.is_some());
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn for_loop_next_var_ignored() {
        // `NEXT I` — the variable after NEXT is consumed and ignored.
        let p = prog("FOR I=0 TO 2\nA=I\nNEXT I\nB=9");
        assert_eq!(p.len(), 2);
        assert!(matches!(p[0].kind, StmtKind::For { .. }));
        assert!(matches!(p[1].kind, StmtKind::Assign { .. }));
    }

    #[test]
    fn next_as_statement_is_loop_continue() {
        // `IF cond THEN NEXT` — the NEXT is a loop-continue statement (osb
        // `statement()` Next → Continue), not a block terminator.
        let StmtKind::For { body, .. } = &prog("FOR I=0 TO 9\nIF I THEN NEXT\nA=I\nNEXT")[0].kind
        else {
            panic!("expected for");
        };
        let StmtKind::If { then_body, .. } = &body[0].kind else {
            panic!("expected the IF inside the FOR body");
        };
        assert!(matches!(then_body[0].kind, StmtKind::Continue));
    }

    #[test]
    fn for_body_with_inner_single_line_if() {
        // The FOR body holds the IF, then its own NEXT (on a later line) closes it.
        let StmtKind::For { body, .. } = &prog("FOR I=1 TO 3\nIF I THEN A=I\nNEXT")[0].kind else {
            panic!("expected for");
        };
        assert_eq!(body.len(), 1);
        assert!(matches!(body[0].kind, StmtKind::If { .. }));
    }

    #[test]
    fn while_and_repeat() {
        assert!(matches!(
            prog("WHILE X\nX=X-1\nWEND")[0].kind,
            StmtKind::While { .. }
        ));
        let StmtKind::RepeatUntil { body, .. } = &prog("REPEAT\nA=A+1\nUNTIL A>9")[0].kind else {
            panic!("expected repeat");
        };
        assert_eq!(body.len(), 1);
    }

    // ----- statements: misc -----

    #[test]
    fn print_items() {
        let StmtKind::Print(items) = &prog("PRINT 1;2,3")[0].kind else {
            panic!("expected print");
        };
        // 1 ; 2 , 3  →  Expr, Expr, Tab, Expr  (no trailing newline: ends after expr,
        // so a newline IS added).
        assert!(matches!(items.last(), Some(PrintItem::NewLine)));
        assert!(items.iter().any(|i| matches!(i, PrintItem::Tab)));
    }

    #[test]
    fn print_trailing_semicolon_suppresses_newline() {
        let StmtKind::Print(items) = &prog("PRINT 1;")[0].kind else {
            panic!("expected print");
        };
        assert!(!items.iter().any(|i| matches!(i, PrintItem::NewLine)));
    }

    #[test]
    fn question_is_print() {
        assert!(matches!(prog("?42")[0].kind, StmtKind::Print(_)));
    }

    #[test]
    fn dim_and_var() {
        let StmtKind::Dim(items) = &prog("DIM A[10],B[2,3]")[0].kind else {
            panic!("expected dim");
        };
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| matches!(i, DimItem::Array { .. })));

        let StmtKind::Dim(items) = &prog("VAR X=5,Y")[0].kind else {
            panic!("expected dim");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], DimItem::Scalar { init: Some(_), .. }));
        assert!(matches!(items[1], DimItem::Scalar { init: None, .. }));
    }

    #[test]
    fn def_function_and_command() {
        let StmtKind::Def(f) = &prog("DEF ADD(A,B)\nRETURN A+B\nEND")[0].kind else {
            panic!("expected def");
        };
        assert_eq!(f.name.ident, "ADD");
        assert_eq!(f.params.len(), 2);
        assert!(f.returns_value);
        assert!(!f.is_common);

        let StmtKind::Def(f) = &prog("COMMON DEF GREET NAME$ OUT R\nR=NAME$\nEND")[0].kind else {
            panic!("expected def");
        };
        assert!(f.is_common);
        assert!(!f.returns_value);
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.out_params.len(), 1);
    }

    #[test]
    fn data_read_restore() {
        let StmtKind::Data(items) = &prog("DATA 1,-2,3.5,HELLO,\"world\"")[0].kind else {
            panic!("expected data");
        };
        assert_eq!(
            items,
            &vec![
                Lit::Int(1),
                Lit::Int(-2),
                Lit::Real(3.5),
                Lit::Str("HELLO".into()),
                Lit::Str("world".into()),
            ]
        );
        assert!(matches!(prog("READ A,B,C")[0].kind, StmtKind::Read(_)));
        assert!(matches!(
            prog("RESTORE @TBL")[0].kind,
            StmtKind::Restore(Some(_))
        ));
        // Bare RESTORE parses (no label); the compiler makes it a runtime Type mismatch.
        assert!(matches!(prog("RESTORE")[0].kind, StmtKind::Restore(None)));
        assert!(matches!(
            prog("RESTORE:READ A")[0].kind,
            StmtKind::Restore(None)
        ));
    }

    #[test]
    fn inc_dec_swap() {
        assert!(matches!(prog("INC X")[0].kind, StmtKind::Inc { .. }));
        // DEC lowers to a negated delta.
        let StmtKind::Inc { delta, .. } = &prog("DEC X,3")[0].kind else {
            panic!("expected inc");
        };
        assert_eq!(delta.kind, ExprKind::Const(Lit::Int(-3)));
        assert!(matches!(prog("SWAP A,B")[0].kind, StmtKind::Swap { .. }));
    }

    #[test]
    fn input_forms() {
        // Prompt + var.
        let StmtKind::Input {
            prompt, question, ..
        } = &prog("INPUT \"name\";N$")[0].kind
        else {
            panic!("expected input");
        };
        assert!(prompt.is_some());
        assert!(*question);
        // Bare var(s).
        let StmtKind::Input { prompt, vars, .. } = &prog("INPUT A,B")[0].kind else {
            panic!("expected input");
        };
        assert!(prompt.is_none());
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn on_goto() {
        let StmtKind::On { kind, labels, .. } = &prog("ON X GOTO @A,@B,@C")[0].kind else {
            panic!("expected on");
        };
        assert_eq!(*kind, OnKind::Goto);
        assert_eq!(labels.len(), 3);
    }

    #[test]
    fn control_keywords() {
        assert!(matches!(prog("END")[0].kind, StmtKind::End));
        assert!(matches!(prog("STOP")[0].kind, StmtKind::Stop));
        assert!(matches!(prog("BREAK")[0].kind, StmtKind::Break));
        assert!(matches!(prog("CONTINUE")[0].kind, StmtKind::Continue));
        assert!(matches!(prog("GOTO @TOP")[0].kind, StmtKind::Goto(_)));
        assert!(matches!(prog("GOSUB @SUB")[0].kind, StmtKind::Gosub(_)));
        assert!(matches!(prog("OPTION STRICT")[0].kind, StmtKind::Option(_)));
    }

    #[test]
    fn label_definition() {
        let StmtKind::Label(name) = &prog("@LOOP")[0].kind else {
            panic!("expected label");
        };
        assert_eq!(name, "LOOP");
    }

    // ----- errors -----

    #[test]
    fn malformed_input_is_syntax_error() {
        for src in [
            "FOR I=0 TO", // missing limit expression
            "IF X",       // missing THEN
            "PRINT 1 2",  // missing separator between print expressions
            "1=2",        // literal target
            ")",          // can't start a statement
            // Real SB collapses these IF-block mismatches to generic Syntax error 3 (NOT the
            // table's 26/27/28) — hw_verified sb-oracle 2026-06-23.
            "IF 1 THEN\nPRINT 1",        // unterminated multi-line IF (no ENDIF)
            "IF 1 THEN\nA=1\nELSE\nB=2", // dangling ELSE, no ENDIF
            "PRINT 1\nENDIF",            // non-leading stray ENDIF
            "A=1\nENDIF",                // non-leading stray ENDIF
            "IF 1 THEN\nENDIF\nENDIF",   // second (non-leading) ENDIF after a closed block
            "ELSE",                      // stray ELSE
        ] {
            let err = parse(src).expect_err(&format!("`{src}` should fail"));
            assert_eq!(err.errnum, 3, "`{src}` should be Syntax error (3)");
        }
    }

    /// Block-structure mismatches carry their own SmileBASIC errnum (not the generic
    /// Syntax error 3) — `spec/reference/errors.yaml`, errnum 20..29.
    #[test]
    fn block_mismatch_has_structural_errnum() {
        for (src, want) in [
            ("NEXT", 21u32),               // NEXT without FOR
            ("WEND", 25),                  // WEND without WHILE
            ("UNTIL 1", 23),               // UNTIL without REPEAT
            ("WHILE X\nWEND\nWEND", 25),   // stray WEND after a closed WHILE
            ("FOR I=0 TO 3\nPRINT I", 20), // FOR without NEXT (EOF first)
            ("WHILE 1\nPRINT 1", 24),      // WHILE without WEND
            ("REPEAT\nPRINT 1", 22),       // REPEAT without UNTIL
            ("DEF F\nA=1", 29),            // DEF without END
            // A LEADING stray ENDIF (program's first statement) is "ENDIF without IF" (28);
            // hw_verified sb-oracle 2026-06-23. A non-leading one is generic 3 (below).
            ("ENDIF", 28),
            ("ENDIF\nPRINT 1", 28),
            ("ENDIF:PRINT 1", 28),
        ] {
            let err = parse(src).expect_err(&format!("`{src}` should fail"));
            assert_eq!(err.errnum, want, "`{src}` errnum");
        }
    }

    /// A statement-only command keyword used in expression position is a Syntax error
    /// (errnum 3), raised before any handler runs — NOT the undefined-call errnum 16 it
    /// would get if parsed as a `Call`. `A=LINPUT("X")`→3 is hw_verified (sb-oracle s_t5b
    /// 2026-06-22, linput.yaml); `INPUT` is the symmetric form (oracle-pending).
    #[test]
    fn command_keyword_in_expression_position_is_syntax_error() {
        for src in [
            r#"A=LINPUT("X")"#, // LINPUT as a function (hw_verified → 3, was 16)
            r#"A=INPUT("X")"#,  // INPUT as a function (symmetric)
            "A=PRINT",          // a bare statement keyword as a value
            "B=1+FOR",          // statement keyword mid-expression
        ] {
            let err = parse(src).expect_err(&format!("`{src}` should fail"));
            assert_eq!(err.errnum, 3, "`{src}` should be Syntax error (3)");
        }
    }

    /// The guard must not swallow legitimate expression forms that share a name with a
    /// command: `VAR(x)` is a by-reference expression, and the word operators are not
    /// statement keywords (`cur_starts_statement` excludes both).
    #[test]
    fn expression_lookalikes_still_parse() {
        for src in ["A=VAR(B)", "A=1 AND 2", "A=NOT 0", "A=3 MOD 2"] {
            parse(src).unwrap_or_else(|e| panic!("`{src}` should parse, got errnum {}", e.errnum));
        }
    }

    #[test]
    fn error_carries_location() {
        let err = parse("A=1\nIF X").unwrap_err();
        assert_eq!(err.errnum, 3);
        assert_eq!(err.loc.line, 2);
    }
}
