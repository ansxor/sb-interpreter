//! Parser (milestone M1).
//!
//! Recursive descent + precedence-climbing, with constant folding during parsing
//! (design reference: `osb/SMILEBASIC/parser.d` `Parser`, ranks in `getOPRank`).
//! Produces the AST consumed by [`crate::compiler`].

// TODO(M1): AST node types and the recursive-descent parser.
