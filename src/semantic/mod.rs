// Semantic analysis — see docs/06-semantic-conventions.md and
// docs/07-semantic-implementation.md for an overview.
//
// Submodule map:
//   types.rs     — Type enum (Int, String, Bool, Fn)
//   scope.rs     — Symbol + Scope (parent-linked block scoping)
//   error.rs     — SemanticError
//   analyzer.rs  — SemanticAnalyzer (visitor over the AST)

pub mod analyzer;
pub mod error;
pub mod scope;
pub mod types;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use analyzer::{SemanticAnalysis, SemanticAnalyzer};
#[allow(unused_imports)]
pub use error::SemanticError;
#[allow(unused_imports)]
pub use scope::{Scope, Symbol};
#[allow(unused_imports)]
pub use types::Type;
