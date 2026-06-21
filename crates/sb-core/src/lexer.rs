//! Lexer — turns SmileBASIC source into a [`Token`] stream.
//!
//! A faithful port of `Lexical` in `osb/SMILEBASIC/parser.d`. SmileBASIC source is
//! UTF-16, so the scanner works over `&[u16]` (full-width characters in string
//! literals pass through untouched). The scanner handles:
//!
//! - decimal numbers and `.`-leading numbers (no exponent form — `1E5` lexes as
//!   `1` then the identifier `E5`, matching SB), `&H` hex and `&B` binary literals;
//! - identifiers (upper-cased, ASCII letters/digits/`_`) with a trailing `$`/`%`/`#`
//!   type suffix; `@label` (keeps the `@`) and `#constant` (drops the `#`);
//! - string literals, tolerating an unterminated string (closed by newline/EOF);
//! - `'` and `REM` line comments;
//! - two-char operators `== != <= >= << >> && ||` plus the single-char set;
//! - `TRUE`/`FALSE` folded to integer `1`/`0` (so they work as `DATA` constants).
//!
//! Names are ASCII-only here (matching osb's `std.ascii` gates); full-width
//! identifiers are a known osb limitation to revisit against the disassembly/oracle.

use crate::error::{ErrNum, SbError, SbResult};
use crate::token::{reserved_word, SourceLocation, Token, TokenType};

// --- character constants (UTF-16 code units) ---
const SPACE: u16 = b' ' as u16;
const CR: u16 = b'\r' as u16;
const LF: u16 = b'\n' as u16;
const SQUOTE: u16 = b'\'' as u16;
const DQUOTE: u16 = b'"' as u16;
const DOT: u16 = b'.' as u16;
const UNDERSCORE: u16 = b'_' as u16;
const AT: u16 = b'@' as u16;
const HASH: u16 = b'#' as u16;
const DOLLAR: u16 = b'$' as u16;
const PERCENT: u16 = b'%' as u16;
const AMP: u16 = b'&' as u16;
const PIPE: u16 = b'|' as u16;
const EQ: u16 = b'=' as u16;
const BANG: u16 = b'!' as u16;
const LT: u16 = b'<' as u16;
const GT: u16 = b'>' as u16;

fn is_digit(c: u16) -> bool {
    (b'0' as u16..=b'9' as u16).contains(&c)
}

fn is_alpha(c: u16) -> bool {
    (b'A' as u16..=b'Z' as u16).contains(&c) || (b'a' as u16..=b'z' as u16).contains(&c)
}

fn is_hex_digit(c: u16) -> bool {
    is_digit(c) || (b'A' as u16..=b'F' as u16).contains(&c)
}

/// ASCII upper-case (leaves non-letters, including suffix/punctuation, untouched).
fn to_upper(c: u16) -> u16 {
    if (b'a' as u16..=b'z' as u16).contains(&c) {
        c - 0x20
    } else {
        c
    }
}

fn is_suffix(c: u16) -> bool {
    c == DOLLAR || c == PERCENT || c == HASH
}

fn is_ident_char(c: u16) -> bool {
    is_alpha(c) || is_digit(c) || c == UNDERSCORE
}

/// Build a Rust `String` from an ASCII-only slice (digits/hex/binary text).
fn ascii_string(units: &[u16]) -> String {
    units.iter().map(|&u| u as u8 as char).collect()
}

/// The SmileBASIC lexer. Construct with [`Lexer::new`] (UTF-8) or
/// [`Lexer::from_utf16`], then drive with [`Lexer::tokenize`] (or [`Lexer::next_token`]).
pub struct Lexer {
    src: Vec<u16>,
    /// Index into `src` (UTF-16 code units).
    i: usize,
    /// Current 1-based line.
    line: u32,
}

impl Lexer {
    /// Build a lexer from a UTF-8 string (converted to SB's native UTF-16).
    pub fn new(src: &str) -> Self {
        Self::from_utf16(&src.encode_utf16().collect::<Vec<u16>>())
    }

    /// Build a lexer directly from UTF-16 code units (SmileBASIC's native form).
    pub fn from_utf16(src: &[u16]) -> Self {
        Self {
            src: src.to_vec(),
            i: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<u16> {
        self.src.get(self.i).copied()
    }

    fn peek_at(&self, n: usize) -> Option<u16> {
        self.src.get(self.i + n).copied()
    }

    /// Advance to just before the next CR/LF (used by `'` and `REM` comments).
    fn skip_to_eol(&mut self) {
        while let Some(c) = self.peek() {
            if c == CR || c == LF {
                break;
            }
            self.i += 1;
        }
    }

    /// Tokenize the whole input, ending with exactly one [`TokenType::Eof`].
    pub fn tokenize(mut self) -> SbResult<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            let t = self.next_token()?;
            let done = t.ty == TokenType::Eof;
            out.push(t);
            if done {
                return Ok(out);
            }
        }
    }

    /// Scan the next token. After end of input this keeps returning [`TokenType::Eof`].
    pub fn next_token(&mut self) -> SbResult<Token> {
        'scan: loop {
            // Whitespace between tokens is just the space character (matches osb).
            while self.peek() == Some(SPACE) {
                self.i += 1;
            }

            let start = self.i;
            let start_line = self.line;
            let loc = SourceLocation::new(start_line, start as u32);

            let c = match self.peek() {
                Some(c) => c,
                None => return Ok(Token::new(TokenType::Eof, loc)),
            };

            // `'` line comment: skip to EOL, then let the loop emit the newline/EOF.
            if c == SQUOTE {
                self.skip_to_eol();
                continue 'scan;
            }

            // Number (decimal / `.`-leading). `&H`/`&B` are handled in the `&` arm.
            if is_digit(c) || c == DOT {
                return self.scan_number(loc);
            }

            // Identifier / keyword / `TRUE`/`FALSE` / `REM` comment.
            if is_alpha(c) || c == UNDERSCORE {
                let mut ident: Vec<u16> = Vec::new();
                while let Some(ch) = self.peek() {
                    let up = to_upper(ch);
                    if !is_ident_char(up) {
                        break;
                    }
                    ident.push(up);
                    self.i += 1;
                }
                // A trailing type suffix makes this unconditionally an identifier
                // (e.g. `END$` is a variable, not the END keyword).
                if let Some(ch) = self.peek() {
                    if is_suffix(ch) {
                        ident.push(ch);
                        self.i += 1;
                        return Ok(Token::new(TokenType::Iden(ident), loc));
                    }
                }
                let name = ascii_string(&ident);
                match name.as_str() {
                    "TRUE" => return Ok(Token::new(TokenType::Integer(1), loc)),
                    "FALSE" => return Ok(Token::new(TokenType::Integer(0), loc)),
                    "REM" => {
                        self.skip_to_eol();
                        continue 'scan;
                    }
                    _ => {}
                }
                if let Some(kw) = reserved_word(&name) {
                    return Ok(Token::new(kw, loc));
                }
                return Ok(Token::new(TokenType::Iden(ident), loc));
            }

            // String literal (unterminated strings are tolerated, like SB).
            if c == DQUOTE {
                self.i += 1; // opening quote
                let mut s: Vec<u16> = Vec::new();
                while let Some(ch) = self.peek() {
                    if ch == DQUOTE {
                        self.i += 1; // closing quote
                        break;
                    }
                    if ch == CR || ch == LF {
                        break; // unterminated: stop before the newline (don't consume it)
                    }
                    s.push(ch);
                    self.i += 1;
                }
                return Ok(Token::new(TokenType::String(s), loc));
            }

            // `@label` (keeps `@`) or `#constant` (drops `#`).
            if c == AT || c == HASH {
                let is_label = c == AT;
                let mut name: Vec<u16> = Vec::new();
                if is_label {
                    name.push(AT);
                }
                self.i += 1; // skip the `@` / `#`
                while let Some(ch) = self.peek() {
                    if !is_ident_char(ch) {
                        break;
                    }
                    name.push(to_upper(ch));
                    self.i += 1;
                }
                let ty = if is_label {
                    TokenType::Label(name)
                } else {
                    TokenType::Constant(name)
                };
                return Ok(Token::new(ty, loc));
            }

            // Multi-char / special operators.
            let c2 = self.peek_at(1);
            match c {
                EQ => {
                    return Ok(self.op(loc, c2 == Some(EQ), TokenType::Equal, TokenType::Assign));
                }
                BANG => {
                    return Ok(self.op(
                        loc,
                        c2 == Some(EQ),
                        TokenType::NotEqual,
                        TokenType::LogicalNot,
                    ));
                }
                LT => {
                    if c2 == Some(EQ) {
                        return Ok(self.op2(loc, TokenType::LessEqual));
                    }
                    if c2 == Some(LT) {
                        return Ok(self.op2(loc, TokenType::LeftShift));
                    }
                    return Ok(self.op1(loc, TokenType::Less));
                }
                GT => {
                    if c2 == Some(EQ) {
                        return Ok(self.op2(loc, TokenType::GreaterEqual));
                    }
                    if c2 == Some(GT) {
                        return Ok(self.op2(loc, TokenType::RightShift));
                    }
                    return Ok(self.op1(loc, TokenType::Greater));
                }
                PIPE => {
                    if c2 == Some(PIPE) {
                        return Ok(self.op2(loc, TokenType::LogicalOr));
                    }
                    return Err(SbError::at(ErrNum::SyntaxError, start_line));
                }
                AMP => {
                    match c2.map(to_upper) {
                        Some(h) if h == b'H' as u16 => return self.scan_radix(loc, 16),
                        Some(b) if b == b'B' as u16 => return self.scan_radix(loc, 2),
                        _ => {}
                    }
                    if c2 == Some(AMP) {
                        return Ok(self.op2(loc, TokenType::LogicalAnd));
                    }
                    return Err(SbError::at(ErrNum::SyntaxError, start_line));
                }
                _ => {}
            }

            // Newline (CR, LF, or CRLF) — one token, then advance the line counter.
            if c == CR || c == LF {
                self.i += 1;
                if c == CR && self.peek() == Some(LF) {
                    self.i += 1;
                }
                let tok = Token::new(TokenType::NewLine, loc);
                self.line += 1;
                return Ok(tok);
            }

            // Single-character punctuation.
            let single = match c {
                _ if c == b'+' as u16 => TokenType::Plus,
                _ if c == b'-' as u16 => TokenType::Minus,
                _ if c == b'*' as u16 => TokenType::Mul,
                _ if c == b'/' as u16 => TokenType::Div,
                _ if c == b'(' as u16 => TokenType::LParen,
                _ if c == b')' as u16 => TokenType::RParen,
                _ if c == b'[' as u16 => TokenType::LBracket,
                _ if c == b']' as u16 => TokenType::RBracket,
                _ if c == b',' as u16 => TokenType::Comma,
                _ if c == b':' as u16 => TokenType::Colon,
                _ if c == b';' as u16 => TokenType::Semicolon,
                _ if c == b'?' as u16 => TokenType::Print,
                _ => return Err(SbError::at(ErrNum::SyntaxError, start_line)),
            };
            return Ok(self.op1(loc, single));
        }
    }

    /// Emit a 1-char token (consume one unit).
    fn op1(&mut self, loc: SourceLocation, ty: TokenType) -> Token {
        self.i += 1;
        Token::new(ty, loc)
    }

    /// Emit a 2-char token (consume two units).
    fn op2(&mut self, loc: SourceLocation, ty: TokenType) -> Token {
        self.i += 2;
        Token::new(ty, loc)
    }

    /// Pick the 2-char `yes` token (if `two`) else the 1-char `no` token.
    fn op(&mut self, loc: SourceLocation, two: bool, yes: TokenType, no: TokenType) -> Token {
        if two {
            self.op2(loc, yes)
        } else {
            self.op1(loc, no)
        }
    }

    /// Scan a decimal / `.`-leading number starting at the current position.
    fn scan_number(&mut self, loc: SourceLocation) -> SbResult<Token> {
        let start = self.i;
        let mut dot = false;
        while let Some(c) = self.peek() {
            if c == DOT {
                if dot {
                    break;
                }
                dot = true;
            } else if !is_digit(c) {
                break;
            }
            self.i += 1;
        }
        let text = ascii_string(&self.src[start..self.i]);
        // A lone "." (no digits) is not a number.
        if !text.bytes().any(|b| b.is_ascii_digit()) {
            return Err(SbError::at(ErrNum::SyntaxError, loc.line));
        }
        let num: f64 = text
            .parse()
            .map_err(|_| SbError::at(ErrNum::SyntaxError, loc.line))?;
        // Integer only when there was no dot and it fits i32 (matches osb's check).
        let ty = if !dot && num >= i32::MIN as f64 && num <= i32::MAX as f64 {
            TokenType::Integer(num as i32)
        } else {
            TokenType::Double(num)
        };
        Ok(Token::new(ty, loc))
    }

    /// Scan a `&H` (radix 16) or `&B` (radix 2) integer literal. `self.i` is on `&`.
    fn scan_radix(&mut self, loc: SourceLocation, radix: u32) -> SbResult<Token> {
        self.i += 2; // skip `&H` / `&B`
        let start = self.i;
        while let Some(c) = self.peek() {
            let ok = match radix {
                16 => is_hex_digit(to_upper(c)),
                _ => c == b'0' as u16 || c == b'1' as u16,
            };
            if !ok {
                break;
            }
            self.i += 1;
        }
        let text = ascii_string(&self.src[start..self.i]);
        if text.is_empty() {
            return Err(SbError::at(ErrNum::SyntaxError, loc.line));
        }
        // Parse as u32 then reinterpret as i32 (so `&HFFFFFFFF` == -1, like SB).
        let v = u32::from_str_radix(&text, radix)
            .map_err(|_| SbError::at(ErrNum::SyntaxError, loc.line))?;
        Ok(Token::new(TokenType::Integer(v as i32), loc))
    }
}

/// Convenience: tokenize a UTF-8 string in one call.
pub fn lex(src: &str) -> SbResult<Vec<Token>> {
    Lexer::new(src).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16s(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    /// Token *types* only (drops source locations), with the trailing Eof removed.
    fn types(src: &str) -> Vec<TokenType> {
        let mut t: Vec<TokenType> = lex(src).unwrap().into_iter().map(|t| t.ty).collect();
        assert_eq!(t.pop(), Some(TokenType::Eof), "stream must end with Eof");
        t
    }

    #[test]
    fn decimal_hex_binary_integers() {
        use TokenType::*;
        assert_eq!(types("10"), vec![Integer(10)]);
        assert_eq!(types("&HFF"), vec![Integer(255)]);
        assert_eq!(types("&hff"), vec![Integer(255)]); // case-insensitive prefix + digits
        assert_eq!(types("&HFFFFFFFF"), vec![Integer(-1)]); // u32 -> i32 reinterpret
        assert_eq!(types("&B1010"), vec![Integer(10)]);
        assert_eq!(types("&B0"), vec![Integer(0)]);
    }

    #[test]
    fn doubles_have_dots_or_overflow_i32() {
        use TokenType::*;
        assert_eq!(types("3.25"), vec![Double(3.25)]);
        assert_eq!(types(".5"), vec![Double(0.5)]);
        assert_eq!(types("5."), vec![Double(5.0)]);
        // No dot but out of i32 range -> Double.
        assert_eq!(types("9999999999"), vec![Double(9999999999.0)]);
        // i32::MAX stays integer; one more becomes a double.
        assert_eq!(types("2147483647"), vec![Integer(2147483647)]);
        assert_eq!(types("2147483648"), vec![Double(2147483648.0)]);
    }

    #[test]
    fn no_exponent_form() {
        use TokenType::*;
        // `1E5` is NOT scientific notation: `1` then identifier `E5`.
        assert_eq!(types("1E5"), vec![Integer(1), Iden(u16s("E5"))]);
    }

    #[test]
    fn true_false_fold_to_integers() {
        use TokenType::*;
        assert_eq!(types("TRUE"), vec![Integer(1)]);
        assert_eq!(types("false"), vec![Integer(0)]);
    }

    #[test]
    fn identifiers_uppercase_with_suffixes() {
        use TokenType::*;
        assert_eq!(types("abc"), vec![Iden(u16s("ABC"))]);
        assert_eq!(types("A$"), vec![Iden(u16s("A$"))]);
        assert_eq!(types("count%"), vec![Iden(u16s("COUNT%"))]);
        assert_eq!(types("x#"), vec![Iden(u16s("X#"))]);
        assert_eq!(types("_tmp1"), vec![Iden(u16s("_TMP1"))]);
        // A suffix defeats the keyword table.
        assert_eq!(types("END$"), vec![Iden(u16s("END$"))]);
    }

    #[test]
    fn labels_and_constants() {
        use TokenType::*;
        assert_eq!(types("@loop"), vec![Label(u16s("@LOOP"))]);
        assert_eq!(types("@Main_1"), vec![Label(u16s("@MAIN_1"))]);
        assert_eq!(types("#TWHITE"), vec![Constant(u16s("TWHITE"))]);
    }

    #[test]
    fn string_literals_and_unterminated() {
        use TokenType::*;
        assert_eq!(types("\"HELLO\""), vec![String(u16s("HELLO"))]);
        // Unterminated string runs to EOF.
        assert_eq!(types("\"abc"), vec![String(u16s("abc"))]);
        // Unterminated string stops at the newline (which still tokenizes).
        assert_eq!(
            types("\"abc\nXYZ"),
            vec![String(u16s("abc")), NewLine, Iden(u16s("XYZ"))]
        );
    }

    #[test]
    fn all_operators() {
        use TokenType::*;
        assert_eq!(
            types("+ - * / = == != < > <= >= << >> ! && || ( ) [ ] , : ; ?"),
            vec![
                Plus,
                Minus,
                Mul,
                Div,
                Assign,
                Equal,
                NotEqual,
                Less,
                Greater,
                LessEqual,
                GreaterEqual,
                LeftShift,
                RightShift,
                LogicalNot,
                LogicalAnd,
                LogicalOr,
                LParen,
                RParen,
                LBracket,
                RBracket,
                Comma,
                Colon,
                Semicolon,
                Print,
            ]
        );
    }

    #[test]
    fn word_operators_and_keywords() {
        use TokenType::*;
        assert_eq!(
            types("MOD DIV OR AND XOR NOT"),
            vec![Mod, IntDiv, Or, And, Xor, Not]
        );
        assert_eq!(
            types("if then else elseif endif"),
            vec![If, Then, Else, Elseif, Endif]
        );
        assert_eq!(
            types("FOR NEXT WHILE WEND REPEAT UNTIL"),
            vec![For, Next, While, WEnd, Repeat, Until]
        );
        assert_eq!(types("? X"), vec![Print, Iden(u16s("X"))]); // `?` is PRINT
                                                                // A builtin command (not a keyword) lexes as an identifier.
        assert_eq!(types("LOCATE"), vec![Iden(u16s("LOCATE"))]);
    }

    #[test]
    fn comments_are_dropped() {
        use TokenType::*;
        // `'` comment leaves only the line's newline + EOF.
        assert_eq!(
            types("A 'hello\nB"),
            vec![Iden(u16s("A")), NewLine, Iden(u16s("B"))]
        );
        // `REM` comment likewise; `REMARK` is a normal identifier.
        assert_eq!(types("REM nothing here\nB"), vec![NewLine, Iden(u16s("B"))]);
        assert_eq!(types("REMARK"), vec![Iden(u16s("REMARK"))]);
        // Comment at EOF with no trailing newline.
        assert_eq!(types("'just a comment"), Vec::<TokenType>::new());
    }

    #[test]
    fn precedence_sample_tokens() {
        use TokenType::*;
        assert_eq!(
            types("2+3*4"),
            vec![Integer(2), Plus, Integer(3), Mul, Integer(4)]
        );
    }

    #[test]
    fn line_numbers_track_colons_and_newlines() {
        // `:` keeps the line; LF and CRLF advance it.
        let toks = lex("A:B\nC\r\nD").unwrap();
        let lines: Vec<u32> = toks.iter().map(|t| t.loc.line).collect();
        let tys: Vec<&TokenType> = toks.iter().map(|t| &t.ty).collect();
        use TokenType::*;
        assert_eq!(
            tys,
            vec![
                &Iden(u16s("A")),
                &Colon,
                &Iden(u16s("B")),
                &NewLine,
                &Iden(u16s("C")),
                &NewLine,
                &Iden(u16s("D")),
                &Eof,
            ]
        );
        assert_eq!(lines, vec![1, 1, 1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn blank_lines_emit_newline_tokens() {
        use TokenType::*;
        assert_eq!(
            types("A\n\nB"),
            vec![Iden(u16s("A")), NewLine, NewLine, Iden(u16s("B"))]
        );
    }

    #[test]
    fn syntax_errors_for_bad_input() {
        // Stray characters and malformed literals are syntax errors (errnum 3).
        for bad in ["~", "&", "|", ".", "&H", "&B", "&HZ"] {
            let err = lex(bad).unwrap_err();
            assert_eq!(
                err.num,
                ErrNum::SyntaxError,
                "expected SyntaxError for {bad:?}"
            );
        }
        // Hex too large for u32 overflows -> syntax error.
        assert_eq!(lex("&H1FFFFFFFF").unwrap_err().num, ErrNum::SyntaxError);
    }

    #[test]
    fn from_utf16_matches_new() {
        let a = Lexer::new("PRINT 1").tokenize().unwrap();
        let b = Lexer::from_utf16(&u16s("PRINT 1")).tokenize().unwrap();
        assert_eq!(a, b);
    }
}
