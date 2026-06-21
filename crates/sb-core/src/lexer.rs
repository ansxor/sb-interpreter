//! Lexer (milestone M1).
//!
//! Design reference: `osb/SMILEBASIC/parser.d` `Lexical` — a forward scanner that
//! handles `'`/`REM` comments, decimal & `.`-prefixed numbers, `&H`/`&B` literals,
//! identifiers with `$`/`%`/`#` type suffixes, `@label`/`#const`, two-char operators
//! (`== != <= >= << >> && ||`), and `TRUE`/`FALSE` -> integer 1/0.
//!
//! SmileBASIC source is UTF-16; keep that in mind for full-width characters.

// TODO(M1): TokenType, Token, SourceLocation, and the scanner.
