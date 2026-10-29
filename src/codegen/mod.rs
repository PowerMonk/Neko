// Code generation — see docs/09-codegen-implementation.md.
//
// Submodule map:
//   symbol_table.rs — SymbolTable: nombre -> dirección (sección + offset)
//   runtime.rs      — rutinas del runtime (print_str/int/bool, read_line)
//   assembler.rs    — Assembler: Vec<Quadruple> -> texto ensamblador NASM

pub mod assembler;
pub mod runner;
pub mod runtime;
pub mod symbol_table;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use assembler::Assembler;
#[allow(unused_imports)]
pub use runner::RunOutcome;
#[allow(unused_imports)]
pub use symbol_table::{Section, Symbol, SymbolKind, SymbolTable};
