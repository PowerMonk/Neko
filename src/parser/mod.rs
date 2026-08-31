// Parser module — see docs/05-parser.md for an overview.
//
// Submodule map:
//   ast.rs          — AST data types
//   error.rs        — SyntaxError struct
//   token_stream.rs — Cursor over Vec<Token>
//   parser.rs       — Recursive-descent parser (one fn per non-terminal)

pub mod ast;
pub mod error;
pub mod parser;
pub mod token_stream;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use ast::{
    AddOpChain, Arm, Chain, CmpOp, Expr, Literal, LogicalOpChain, MulOpChain, Pattern, Program,
    Stmt,
};
#[allow(unused_imports)]
pub use error::SyntaxError;
#[allow(unused_imports)]
pub use parser::{Parser, SyntacticAnalysis};
