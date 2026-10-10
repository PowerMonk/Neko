// Intermediate Code Generation — see docs/08-icg-implementation.md.
//
// Submodule map:
//   quadruple.rs   — Quadruple struct
//   triple.rs      — Triple struct
//   emitter.rs     — CodeEmitter (shared counters + lists)
//   generator.rs   — ICGenerator (visitor over AST)

pub mod emitter;
pub mod generator;
pub mod quadruple;
pub mod triple;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use emitter::CodeEmitter;
#[allow(unused_imports)]
pub use generator::{GeneratedCode, ICGenerator};
#[allow(unused_imports)]
pub use quadruple::Quadruple;
#[allow(unused_imports)]
pub use triple::Triple;
