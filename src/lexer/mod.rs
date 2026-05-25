// Each `pub mod NAME;` declares a submodule of `lexer`.
// Rust expects a file at `src/lexer/NAME.rs` for each one.
// In TS: each of these would be a separate file with `export` statements.
pub mod alphabet;
pub mod automata;
pub mod lexer;
pub mod symbol_table;
pub mod token;

// `pub use` re-exports items so external code can write:
//   use crate::lexer::Lexer;
// instead of the more verbose:
//   use crate::lexer::lexer::Lexer;
// (`#[allow(unused_imports)]` suppresses warnings if nothing imports these here.)
#[allow(unused_imports)]
pub use lexer::{LexicalAnalysis, Lexer};
#[allow(unused_imports)]
pub use symbol_table::SymbolTable;
#[allow(unused_imports)]
pub use token::{LexicalError, Token, TokenKind};
