//! Tokens produced by the [lexer](crate::lexer).
//!
//! Mirrors `otya.smilebasic.token` in `osb/SMILEBASIC/token.d`: a `TokenType` tag
//! plus, for literal/name tokens, the carried value. We fold osb's separate
//! `TokenValue` union into the enum variants so a [`Token`] is self-describing.
//!
//! SmileBASIC source is UTF-16, so string/identifier payloads are stored as
//! `Vec<u16>` (matching [`crate::value::SbString`]). Identifiers, labels and
//! constants are upper-cased by the lexer (SB is case-insensitive for names).

use crate::value::SbString;

/// The kind of a lexical token (and its payload for literals/names).
///
/// Names mirror `TokenType` in `token.d`; punctuation/operator variants keep SB's
/// two distinct equals (`=` is [`Assign`](TokenType::Assign), `==` is
/// [`Equal`](TokenType::Equal) — disambiguated by the parser, not the lexer).
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // --- literals & names ---
    /// Integer literal (decimal in `i32` range, or `&H`/`&B` re-interpreted as `i32`).
    Integer(i32),
    /// Double literal (had a `.`, or magnitude outside the `i32` range).
    Double(f64),
    /// String literal (contents between quotes; unterminated strings are tolerated).
    String(SbString),
    /// Identifier, upper-cased, including any trailing `$`/`%`/`#` type suffix.
    Iden(SbString),
    /// `@label`, upper-cased, *including* the leading `@` (matches osb).
    Label(SbString),
    /// `#constant`, upper-cased, *without* the leading `#`.
    Constant(SbString),

    // --- operators ---
    Plus,         // +
    Minus,        // -
    Mul,          // *
    Div,          // /
    Mod,          // MOD
    IntDiv,       // DIV
    Assign,       // =
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    LeftShift,    // <<
    RightShift,   // >>
    LogicalNot,   // !
    LogicalAnd,   // &&
    LogicalOr,    // ||
    Or,           // OR
    And,          // AND
    Xor,          // XOR
    Not,          // NOT

    // --- punctuation ---
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    NewLine,   // end of a logical line (CR, LF, or CRLF)

    // --- keywords ---
    Print, // PRINT or ?
    If,
    Then,
    Else,
    Elseif,
    Endif,
    For,
    Next,
    While,
    WEnd,
    Repeat,
    Until,
    Goto,
    Gosub,
    Return,
    On,
    Def,
    End,
    Break,
    Continue,
    Var,
    Dim,
    Out,
    Swap,
    Inc,
    Dec,
    Data,
    Read,
    Restore,
    Input,
    Linput,
    Stop,
    Use,
    Exec,
    Call,
    Common,

    /// End of input. Emitted exactly once, as the final token.
    Eof,
}

/// Where a token starts in the source (for `ERRLINE` and diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// 1-based source line.
    pub line: u32,
    /// 0-based offset (in UTF-16 code units) of the token's first character.
    pub offset: u32,
}

impl SourceLocation {
    pub fn new(line: u32, offset: u32) -> Self {
        Self { line, offset }
    }
}

/// A token: its [`TokenType`] (with payload) and source [`SourceLocation`].
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub ty: TokenType,
    pub loc: SourceLocation,
}

impl Token {
    pub fn new(ty: TokenType, loc: SourceLocation) -> Self {
        Self { ty, loc }
    }
}

/// Look up a reserved word (already upper-cased) → its keyword [`TokenType`].
///
/// Mirrors `Lexical.initReservedWordsTable` in `parser.d`. Builtin *commands*
/// (LOCATE, COLOR, SPSET, …) are **not** here — they lex as [`TokenType::Iden`]
/// and are resolved at compile time. Only words with dedicated grammar are keywords.
///
/// `TRUE`/`FALSE` are deliberately absent: the lexer turns them into integer
/// literals `1`/`0` directly (so they work as `DATA` constants, per osb).
pub fn reserved_word(name: &str) -> Option<TokenType> {
    use TokenType::*;
    Some(match name {
        "OR" => Or,
        "AND" => And,
        "XOR" => Xor,
        "NOT" => Not,
        "PRINT" => Print,
        "GOTO" => Goto,
        "IF" => If,
        "THEN" => Then,
        "ELSE" => Else,
        "ELSEIF" => Elseif,
        "ENDIF" => Endif,
        "FOR" => For,
        "NEXT" => Next,
        "MOD" => Mod,
        "DIV" => IntDiv,
        "GOSUB" => Gosub,
        "RETURN" => Return,
        "END" => End,
        "BREAK" => Break,
        "CONTINUE" => Continue,
        "VAR" => Var,
        "DIM" => Dim,
        "DEF" => Def,
        "OUT" => Out,
        "WHILE" => While,
        "WEND" => WEnd,
        "INC" => Inc,
        "DEC" => Dec,
        "DATA" => Data,
        "READ" => Read,
        "RESTORE" => Restore,
        "ON" => On,
        "INPUT" => Input,
        "LINPUT" => Linput,
        "STOP" => Stop,
        "USE" => Use,
        "EXEC" => Exec,
        "CALL" => Call,
        "COMMON" => Common,
        "REPEAT" => Repeat,
        "UNTIL" => Until,
        "SWAP" => Swap,
        _ => return None,
    })
}
