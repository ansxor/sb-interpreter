//! The SmileBASIC 3.6.0 lexer (M1-T1).
//!
//! Implements the scan described in `spec/concepts/execution-model.md`
//! ("Source text & lexing"). The scan loop and token kinds follow osb's
//! `parser.d` `Lexical` *structurally only* — the behavior is the SmileBASIC 3.6.0
//! one. Identifiers are **ASCII-only**: a name starts with an ASCII letter `[A-Za-z]`
//! or `_` and continues with `[A-Za-z0-9_]`. Any non-ASCII char in a name — full-width
//! Latin `Ａ`, hiragana `あ`, katakana `ア`, kanji `愛`, or a full-width digit `１` — is
//! **not** an identifier char: it lexes as [`TokenKind::Unknown`], which the parser
//! rejects as **Syntax error (errnum 3)**. ASCII names are case-folded to upper
//! (`abc` == `ABC`). hw_verified via the sb-oracle skill (real SB 3.6.0, 2026-06-26):
//! `Ａ=1`/`あ=1`/`愛=1`/`１A=1`/`1A=1` all → errnum 3, while `_X=4`→4 and `abc`==`ABC`==11.
//! This **reverses** the earlier unverified assumption (repeated in osb's comments and the
//! manual's Japanese examples) that kana/full-width names are accepted — they are not.
//! Labels `@name` and `#const` names share the ASCII class, except a label may *start*
//! with a digit: `@1X` is legal where `1A` as a variable is not (both hw_verified). See
//! beads sb-interpreter-x01 / -29m.
//!
//! What the scan recognizes:
//! - decimal integers, `.`-leading and trailing-`.` reals;
//! - `&H…` hex and `&B…` binary integers (wrapping into `i32`);
//! - identifiers with `$`/`%`/`#` type suffixes;
//! - `@label` and `#const`;
//! - string literals (an unterminated `"…` is tolerated to end of line);
//! - `'` and `REM` comments (each ends the logical line);
//! - two-char operators `== != <= >= << >> && ||` and the single-char set;
//! - `TRUE`/`FALSE`, which lex to integer `1`/`0`.
//!
//! Line numbers / [`SourceLoc`] are tracked across both `:` and newlines.

use crate::token::{SourceLoc, Suffix, Token, TokenKind};

/// Lexes a SmileBASIC source string into a flat token list.
pub struct Lexer {
    /// Source as a char vector for O(1) indexed lookahead.
    chars: Vec<char>,
    /// Current index into `chars`.
    i: usize,
    /// 1-based current line.
    line: u32,
    /// Index into `chars` where the current line began (for column math).
    line_start: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            i: 0,
            line: 1,
            line_start: 0,
        }
    }

    /// Tokenize the whole source, returning tokens terminated by a single
    /// [`TokenKind::Eof`].
    pub fn tokenize(src: &str) -> Vec<Token> {
        let mut lx = Lexer::new(src);
        let mut out = Vec::new();
        loop {
            let tok = lx.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            out.push(tok);
            if is_eof {
                break;
            }
        }
        out
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.i + 1).copied()
    }

    /// Column (1-based) of the char at `self.i`.
    fn col(&self) -> u32 {
        (self.i - self.line_start + 1) as u32
    }

    fn loc(&self) -> SourceLoc {
        SourceLoc::new(self.line, self.col())
    }

    /// Consume a `\n`, `\r` or `\r\n` (caller has already confirmed the first is
    /// a line terminator) and advance the line counter.
    fn consume_newline(&mut self) {
        let c = self.chars[self.i];
        self.i += 1;
        if c == '\r' && self.peek() == Some('\n') {
            self.i += 1;
        }
        self.line += 1;
        self.line_start = self.i;
    }

    fn next_token(&mut self) -> Token {
        // Skip inline whitespace (space / tab). Newlines are significant.
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.i += 1;
            } else {
                break;
            }
        }

        let loc = self.loc();
        let c = match self.peek() {
            None => return Token::new(TokenKind::Eof, loc),
            Some(c) => c,
        };

        // ----- newline -----
        if c == '\n' || c == '\r' {
            self.consume_newline();
            return Token::new(TokenKind::Newline, loc);
        }

        // ----- comment: ' to end of line -----
        if c == '\'' {
            return self.end_comment(loc);
        }

        // ----- number: decimal / .-leading -----
        if c.is_ascii_digit() || c == '.' {
            return self.lex_number(loc);
        }

        // ----- identifier / keyword / word-operator -----
        // ASCII letters and `_` only — a non-ASCII char (full-width / kana / kanji) is
        // NOT a name start; it falls through to `lex_operator` → `Unknown` → Syntax
        // error 3 (hw_verified, sb-oracle 2026-06-26: `Ａ=1`/`あ=1`/`愛=1` → errnum 3).
        if c.is_ascii_alphabetic() || c == '_' {
            return self.lex_ident(loc);
        }

        // ----- string literal -----
        if c == '"' {
            return self.lex_string(loc);
        }

        // ----- @label / #const -----
        if c == '@' || c == '#' {
            return self.lex_label_or_const(loc, c == '@');
        }

        // ----- & : &H hex, &B binary, && logical-and -----
        if c == '&' {
            return self.lex_ampersand(loc);
        }

        // ----- multi/single-char operators -----
        self.lex_operator(loc, c)
    }

    /// Consume the rest of a comment line (`'` or `REM`) including its line
    /// terminator, and return a single `Newline` token. The comment body runs to
    /// the next `\n`/`\r`/EOF; the terminator is consumed so the comment collapses
    /// to one logical line break.
    fn end_comment(&mut self, loc: SourceLoc) -> Token {
        while let Some(c) = self.peek() {
            if c == '\n' || c == '\r' {
                break;
            }
            self.i += 1;
        }
        if matches!(self.peek(), Some('\n') | Some('\r')) {
            self.consume_newline();
        }
        Token::new(TokenKind::Newline, loc)
    }

    fn lex_number(&mut self, loc: SourceLoc) -> Token {
        let start = self.i;
        let mut seen_dot = false;
        while let Some(c) = self.peek() {
            if c == '.' {
                if seen_dot {
                    break; // a second dot ends the number (e.g. `1.2.3` → `1.2` `.3`)
                }
                seen_dot = true;
                self.i += 1;
            } else if c.is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }

        // Real SB accepts exponent notation on numeric literals: `1E10`,
        // `1.5E-3`, `.5E+2`. Only consume it when an `E` is followed by an
        // optional sign and at least one digit; otherwise `1E` stays `1`+`E`.
        //
        // Verified against SB 3.6.0: a trailing dot blocks the exponent slot,
        // so `5.E2` parses as `5.` followed by `E2` (syntax error in the parser),
        // while `.5E2` and `1.2E3` are valid.
        let mut seen_exp = false;
        if matches!(self.peek(), Some('e') | Some('E'))
            && self
                .chars
                .get(self.i.saturating_sub(1))
                .is_some_and(|c| c.is_ascii_digit())
        {
            let mut j = self.i + 1;
            if matches!(self.chars.get(j), Some('+') | Some('-')) {
                j += 1;
            }
            if self.chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
                seen_exp = true;
                self.i = j + 1;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        let text: String = self.chars[start..self.i].iter().collect();

        let kind = if seen_dot || seen_exp {
            // Real: rely on f64 parsing ("5.", ".5", "1.2", "1E10" all valid).
            TokenKind::Real(text.parse::<f64>().unwrap_or(0.0))
        } else {
            // Integer if it fits i32; otherwise SmileBASIC promotes to Double.
            match text.parse::<i64>() {
                Ok(v) if (i32::MIN as i64..=i32::MAX as i64).contains(&v) => {
                    TokenKind::Int(v as i32)
                }
                _ => TokenKind::Real(text.parse::<f64>().unwrap_or(0.0)),
            }
        };
        Token::new(kind, loc)
    }

    fn lex_ident(&mut self, loc: SourceLoc) -> Token {
        let start = self.i;
        while let Some(c) = self.peek() {
            // Continue chars are ASCII alphanumerics + `_` only; a non-ASCII char ends the
            // name and lexes as its own `Unknown` token (so `XＡ=9` → `X` then Unknown(Ａ)
            // → Syntax error 3, hw_verified sb-oracle 2026-06-26).
            if c.is_ascii_alphanumeric() || c == '_' {
                self.i += 1;
            } else {
                break;
            }
        }
        // SmileBASIC names are case-insensitive: fold ASCII to uppercase.
        let name: String = self.chars[start..self.i]
            .iter()
            .map(|c| c.to_ascii_uppercase())
            .collect();

        // `REM` introduces a line comment (only the bare word, not `REMARK`).
        if name == "REM" {
            return self.end_comment(loc);
        }

        // `TRUE` / `FALSE` are constants that lex straight to integers.
        match name.as_str() {
            "TRUE" => return Token::new(TokenKind::Int(1), loc),
            "FALSE" => return Token::new(TokenKind::Int(0), loc),
            _ => {}
        }

        // Optional type suffix. `#` attaches as a suffix only directly after a
        // name; a standalone `#name` is a constant, handled elsewhere.
        let suffix = match self.peek() {
            Some('$') => Some(Suffix::Str),
            Some('%') => Some(Suffix::Int),
            Some('#') => Some(Suffix::Real),
            _ => None,
        };
        let suffix = match suffix {
            Some(s) => {
                self.i += 1;
                s
            }
            None => Suffix::None,
        };
        Token::new(TokenKind::Ident { name, suffix }, loc)
    }

    fn lex_string(&mut self, loc: SourceLoc) -> Token {
        self.i += 1; // opening quote
        let start = self.i;
        while let Some(c) = self.peek() {
            if c == '"' {
                let s: String = self.chars[start..self.i].iter().collect();
                self.i += 1; // closing quote
                return Token::new(TokenKind::Str(s), loc);
            }
            if c == '\n' || c == '\r' {
                break; // unterminated string ends at the line (newline left intact)
            }
            self.i += 1;
        }
        let s: String = self.chars[start..self.i].iter().collect();
        Token::new(TokenKind::Str(s), loc)
    }

    fn lex_label_or_const(&mut self, loc: SourceLoc, is_label: bool) -> Token {
        self.i += 1; // consume @ or #
        let start = self.i;
        // Label/const names are ASCII-only too (hw_verified sb-oracle 2026-06-26: `@あ`/`@Ａ`
        // /`#あ` → errnum 3). A label MAY start with a digit (`@1X` is legal), so the start
        // char is not letter-gated here — only the ASCII alphanumeric+`_` run is scanned.
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.i += 1;
            } else {
                break;
            }
        }
        let name: String = self.chars[start..self.i]
            .iter()
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let kind = if is_label {
            TokenKind::Label(name)
        } else {
            TokenKind::Const(name)
        };
        Token::new(kind, loc)
    }

    fn lex_ampersand(&mut self, loc: SourceLoc) -> Token {
        match self.peek2() {
            Some('H') | Some('h') => {
                self.i += 2;
                let v = self.scan_radix(16, |c| c.is_ascii_hexdigit());
                Token::new(TokenKind::Int(v), loc)
            }
            Some('B') | Some('b') => {
                self.i += 2;
                let v = self.scan_radix(2, |c| c == '0' || c == '1');
                Token::new(TokenKind::Int(v), loc)
            }
            Some('&') => {
                self.i += 2;
                Token::new(TokenKind::AndAnd, loc)
            }
            // A lone `&` is not a SmileBASIC operator.
            _ => {
                self.i += 1;
                Token::new(TokenKind::Unknown('&'), loc)
            }
        }
    }

    /// Scan a run of digits accepted by `valid` and fold them into an `i32` with
    /// wrapping (so `&HFFFFFFFF` → `-1`, matching SmileBASIC's 32-bit literals).
    fn scan_radix(&mut self, radix: u32, valid: impl Fn(char) -> bool) -> i32 {
        let mut acc: u32 = 0;
        while let Some(c) = self.peek() {
            if valid(c) {
                let d = c.to_digit(radix).unwrap();
                acc = acc.wrapping_mul(radix).wrapping_add(d);
                self.i += 1;
            } else {
                break;
            }
        }
        acc as i32
    }

    fn lex_operator(&mut self, loc: SourceLoc, c: char) -> Token {
        let n = self.peek2();
        // Two-character operators first.
        let two = match (c, n) {
            ('=', Some('=')) => Some(TokenKind::EqEq),
            ('!', Some('=')) => Some(TokenKind::NotEq),
            ('<', Some('=')) => Some(TokenKind::LessEq),
            ('>', Some('=')) => Some(TokenKind::GreaterEq),
            ('<', Some('<')) => Some(TokenKind::Shl),
            ('>', Some('>')) => Some(TokenKind::Shr),
            ('|', Some('|')) => Some(TokenKind::OrOr),
            _ => None,
        };
        if let Some(kind) = two {
            self.i += 2;
            return Token::new(kind, loc);
        }

        let one = match c {
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '=' => TokenKind::Assign,
            '<' => TokenKind::Less,
            '>' => TokenKind::Greater,
            '!' => TokenKind::Bang,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '?' => TokenKind::Question,
            other => TokenKind::Unknown(other),
        };
        self.i += 1;
        Token::new(one, loc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tokenize and drop locations, dropping the trailing Eof for brevity.
    fn kinds(src: &str) -> Vec<TokenKind> {
        let mut v: Vec<TokenKind> = Lexer::tokenize(src).into_iter().map(|t| t.kind).collect();
        assert_eq!(v.pop(), Some(TokenKind::Eof));
        v
    }

    fn ident(name: &str, suffix: Suffix) -> TokenKind {
        TokenKind::Ident {
            name: name.to_string(),
            suffix,
        }
    }

    #[test]
    fn decimal_and_real_numbers() {
        assert_eq!(
            kinds("0 42 2147483647"),
            vec![
                TokenKind::Int(0),
                TokenKind::Int(42),
                TokenKind::Int(2147483647),
            ]
        );
        // .-leading and trailing-dot reals.
        assert_eq!(
            kinds(".5 5. 12.25"),
            vec![
                TokenKind::Real(0.5),
                TokenKind::Real(5.0),
                TokenKind::Real(12.25),
            ]
        );
        // i32 overflow promotes to a Double.
        assert_eq!(kinds("2147483648"), vec![TokenKind::Real(2147483648.0)]);
    }

    #[test]
    fn exponent_literals() {
        // Uppercase / lowercase, signed / unsigned, integer- and real-leading forms.
        assert_eq!(
            kinds("1E10 1e+5 1.5E-3 .5e2 5.0e2"),
            vec![
                TokenKind::Real(1e10),
                TokenKind::Real(1e5),
                TokenKind::Real(0.0015),
                TokenKind::Real(50.0),
                TokenKind::Real(500.0),
            ]
        );
        // Standalone `E` and an incomplete exponent are not part of the number.
        assert_eq!(
            kinds("1E E5"),
            vec![
                TokenKind::Int(1),
                ident("E", Suffix::None),
                ident("E5", Suffix::None),
            ]
        );
        // A trailing dot blocks the exponent slot in real SB 3.6.0; the lexer keeps
        // `5.` as a real and `e2` becomes the start of an identifier (`E2`).
        assert_eq!(
            kinds("5.e2"),
            vec![TokenKind::Real(5.0), ident("E2", Suffix::None),]
        );
    }

    #[test]
    fn second_dot_breaks_number() {
        assert_eq!(
            kinds("1.2.3"),
            vec![TokenKind::Real(1.2), TokenKind::Real(0.3),]
        );
    }

    #[test]
    fn hex_and_binary_literals() {
        assert_eq!(
            kinds("&H10 &HFF &B1010"),
            vec![
                TokenKind::Int(0x10),
                TokenKind::Int(0xFF),
                TokenKind::Int(0b1010),
            ]
        );
        // 32-bit wrap: &HFFFFFFFF == -1, lowercase accepted.
        assert_eq!(kinds("&hffffffff"), vec![TokenKind::Int(-1)]);
    }

    #[test]
    fn identifiers_with_suffixes() {
        assert_eq!(
            kinds("A B$ C% D#"),
            vec![
                ident("A", Suffix::None),
                ident("B", Suffix::Str),
                ident("C", Suffix::Int),
                ident("D", Suffix::Real),
            ]
        );
    }

    #[test]
    fn identifiers_are_case_folded() {
        assert_eq!(
            kinds("Foo foo FOO"),
            vec![
                ident("FOO", Suffix::None),
                ident("FOO", Suffix::None),
                ident("FOO", Suffix::None),
            ]
        );
    }

    #[test]
    fn non_ascii_is_not_an_identifier() {
        // SmileBASIC 3.6.0 identifiers are ASCII-only: a kana / full-width / kanji char is
        // NOT a name char — it lexes as `Unknown` (the parser then raises Syntax error 3).
        // hw_verified via the sb-oracle skill (real SB 3.6.0, 2026-06-26): `あ=1`/`Ａ=1`/
        // `愛=1` → errnum 3. This reverses the earlier unverified "Japanese names accepted"
        // assumption (beads sb-interpreter-x01 / -29m).
        assert_eq!(kinds("あ"), vec![TokenKind::Unknown('あ')]);
        // Full-width Latin `Ａ` (U+FF21) is likewise Unknown, not a fold of ASCII `A`.
        assert_eq!(kinds("Ａ"), vec![TokenKind::Unknown('Ａ')]);
        // A non-ASCII char ends an ASCII name run rather than extending it: `XＡ` → `X`
        // then `Unknown(Ａ)` (so `XＡ=9` is a Syntax error in the parser, per the oracle).
        assert_eq!(
            kinds("XＡ"),
            vec![ident("X", Suffix::None), TokenKind::Unknown('Ａ')]
        );
        // `_` IS a legal name char (start and continue): `_X` is one identifier.
        assert_eq!(kinds("_X"), vec![ident("_X", Suffix::None)]);
    }

    #[test]
    fn true_false_fold_to_int() {
        assert_eq!(
            kinds("TRUE FALSE true"),
            vec![TokenKind::Int(1), TokenKind::Int(0), TokenKind::Int(1),]
        );
    }

    #[test]
    fn labels_and_constants() {
        assert_eq!(
            kinds("@Loop #WHITE"),
            vec![
                TokenKind::Label("LOOP".to_string()),
                TokenKind::Const("WHITE".to_string()),
            ]
        );
        // `#` is a suffix when it follows a name, a const prefix when it leads.
        assert_eq!(
            kinds("X# #X"),
            vec![ident("X", Suffix::Real), TokenKind::Const("X".to_string()),]
        );
    }

    #[test]
    fn string_literals_terminated_and_not() {
        assert_eq!(kinds("\"hi\""), vec![TokenKind::Str("hi".to_string())]);
        // Unterminated string runs to end of line; the newline survives.
        assert_eq!(
            kinds("\"oops\n"),
            vec![TokenKind::Str("oops".to_string()), TokenKind::Newline,]
        );
    }

    #[test]
    fn two_char_operators() {
        assert_eq!(
            kinds("== != <= >= << >> && ||"),
            vec![
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::LessEq,
                TokenKind::GreaterEq,
                TokenKind::Shl,
                TokenKind::Shr,
                TokenKind::AndAnd,
                TokenKind::OrOr,
            ]
        );
    }

    #[test]
    fn single_char_operators_and_punctuation() {
        assert_eq!(
            kinds("+-*/=<>!()[],:;?"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Assign,
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::Bang,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Semicolon,
                TokenKind::Question,
            ]
        );
    }

    #[test]
    fn word_operators_stay_identifiers() {
        // AND/OR/XOR/MOD/DIV/NOT are left to the parser as identifiers.
        assert_eq!(
            kinds("A AND B MOD C"),
            vec![
                ident("A", Suffix::None),
                ident("AND", Suffix::None),
                ident("B", Suffix::None),
                ident("MOD", Suffix::None),
                ident("C", Suffix::None),
            ]
        );
    }

    #[test]
    fn comments_end_the_line() {
        assert_eq!(
            kinds("X=1 'comment here\nY=2"),
            vec![
                ident("X", Suffix::None),
                TokenKind::Assign,
                TokenKind::Int(1),
                TokenKind::Newline,
                ident("Y", Suffix::None),
                TokenKind::Assign,
                TokenKind::Int(2),
            ]
        );
        // REM as a bare word starts a comment; REMARK does not.
        assert_eq!(
            kinds("REM hello\nREMARK"),
            vec![TokenKind::Newline, ident("REMARK", Suffix::None),]
        );
    }

    #[test]
    fn line_numbers_track_across_newline_and_colon() {
        let toks = Lexer::tokenize("A=1:B=2\nC=3");
        // Find the three assignment-target identifiers and check their lines.
        let idents: Vec<&Token> = toks
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Ident { .. }))
            .collect();
        assert_eq!(idents.len(), 3);
        // A and B share line 1 (separated by `:`); C is on line 2.
        assert_eq!(idents[0].loc.line, 1); // A
        assert_eq!(idents[0].loc.col, 1);
        assert_eq!(idents[1].loc.line, 1); // B, after the colon
        assert_eq!(idents[1].loc.col, 5);
        assert_eq!(idents[2].loc.line, 2); // C, on the next line
        assert_eq!(idents[2].loc.col, 1);
    }

    #[test]
    fn crlf_is_one_newline() {
        let toks = Lexer::tokenize("A\r\nB");
        let newlines = toks.iter().filter(|t| t.kind == TokenKind::Newline).count();
        assert_eq!(newlines, 1);
        // B lands on line 2.
        let b = toks
            .iter()
            .find(|t| matches!(&t.kind, TokenKind::Ident { name, .. } if name == "B"))
            .unwrap();
        assert_eq!(b.loc.line, 2);
    }

    #[test]
    fn empty_source_is_just_eof() {
        assert_eq!(
            Lexer::tokenize(""),
            vec![Token::new(TokenKind::Eof, SourceLoc::new(1, 1))]
        );
    }

    #[test]
    fn unknown_char_is_carried() {
        assert_eq!(kinds("$"), vec![TokenKind::Unknown('$')]);
    }
}
