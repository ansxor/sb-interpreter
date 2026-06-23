//! Token kinds produced by the SmileBASIC 3.6.0 lexer (`spec/concepts/execution-model.md`,
//! "Source text & lexing").
//!
//! The lexer is intentionally thin: it classifies literals, names and operators
//! and tracks source location. It does **not** resolve keywords — word operators
//! (`AND`/`OR`/`XOR`/`MOD`/`DIV`/`NOT`) and command names come out as
//! [`TokenKind::Ident`]; the parser (M1-T3) decides what they mean. The two
//! exceptions the docs pin at lex time are `TRUE`/`FALSE` (lex to integer `1`/`0`)
//! and comments (`'` and `REM`).

/// SmileBASIC variable type suffix on a name (`A$`, `A%`, `A#`).
///
/// The suffix is part of the variable's identity. No suffix means a
/// dynamically-typed numeric (Integer or Double depending on the assigned
/// value), per `execution-model.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suffix {
    /// no suffix — dynamically-typed numeric
    None,
    /// `%` — Integer (`i32`)
    Int,
    /// `#` — Real (`f64`)
    Real,
    /// `$` — String
    Str,
}

/// 1-based source location (line + column, both counted in characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceLoc {
    /// 1-based line number, incremented across newlines.
    pub line: u32,
    /// 1-based column (character index on the line).
    pub col: u32,
}

impl SourceLoc {
    pub fn new(line: u32, col: u32) -> Self {
        SourceLoc { line, col }
    }
}

/// The classified kind of a lexed token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ----- literals -----
    /// Integer literal (decimal that fits `i32`, or `&H`/`&B`). `TRUE`/`FALSE`
    /// also produce `Int(1)`/`Int(0)`.
    Int(i32),
    /// Real literal: a decimal containing `.`, or a decimal integer too large for
    /// `i32` (SmileBASIC promotes it to a Double).
    Real(f64),
    /// String literal `"…"` (the closing quote is optional — SmileBASIC tolerates
    /// an unterminated string to end of line).
    Str(String),

    // ----- names -----
    /// Identifier (variable / command / word-operator). The `name` is folded to
    /// uppercase (SmileBASIC names are case-insensitive) and excludes any type
    /// suffix, which is reported separately in `suffix`.
    Ident {
        name: String,
        suffix: Suffix,
    },
    /// `@label` — name folded to uppercase, without the leading `@`.
    Label(String),
    /// `#const` — name folded to uppercase, without the leading `#`.
    Const(String),

    // ----- operators -----
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Assign,    // =
    EqEq,      // ==
    NotEq,     // !=
    Less,      // <
    LessEq,    // <=
    Greater,   // >
    GreaterEq, // >=
    Shl,       // <<
    Shr,       // >>
    AndAnd,    // &&
    OrOr,      // ||
    Bang,      // !

    // ----- punctuation -----
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Question,  // ?  (the parser treats this as PRINT)

    // ----- structural -----
    /// End of a logical line (`\n`, `\r`, `\r\n`, or the end of a comment).
    Newline,
    /// End of input.
    Eof,

    /// A character the lexer could not classify. Carried through so the parser
    /// can raise a Syntax error (errnum 3) with a location, matching SmileBASIC's
    /// tolerant scan rather than panicking.
    Unknown(char),
}

/// A token plus the source location of its first character.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub loc: SourceLoc,
}

impl Token {
    pub fn new(kind: TokenKind, loc: SourceLoc) -> Self {
        Token { kind, loc }
    }
}
