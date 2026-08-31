// Declare the `lexer` and `parser` submodules.
// Rust looks for `src/lexer/mod.rs` (or `src/lexer.rs`) and `src/parser/mod.rs`.
mod lexer;
mod parser;

use crate::lexer::Lexer;
use crate::parser::Parser;

use std::env;
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("examples/basic.neko"));

    // ---- Phase 1: Lexical analysis ----
    let source = fs::read_to_string(&input_path)?;
    let lexer = Lexer::new(source);
    let lex_result = lexer.lex();

    lex_result.write_outputs("output")?;

    // ---- Phase 2: Syntactic analysis (only if lexer is clean) ----
    //
    // The rubric for this project says: "Tomar como entrada un código LIBRE
    // DE ERRORES LÉXICOS y verificar con base en su gramática, que se
    // cumplan las dos condiciones implícitas en su tarea."
    // Translation: skip syntactic analysis when the input has lexical
    // errors — they're a different problem class and we don't want
    // cascading noise from mis-aligned tokens.
    let (syn_errors, syn_ok) = if lex_result.errors.is_empty() {
        let parser = Parser::new(lex_result.tokens.clone());
        let syn_result = parser.parse();
        let count = syn_result.errors.len();
        syn_result.write_outputs("output")?;
        (count, true)
    } else {
        // Still write empty parse outputs so the directory shape stays
        // predictable for downstream tools.
        std::fs::write("output/parse_tree.txt", "")?;
        std::fs::write("output/parse_errors.txt", "(skipped: lexical errors present)\n")?;
        (0usize, false)
    };

    // ---- Terminal summary ----
    let tok_count = lex_result.tokens.len();
    let sym_count = lex_result.symbols.entries().len();
    let lex_errors = lex_result.errors.len();

    if syn_ok && syn_errors == 0 {
        println!(
            "OK — {} tokens, {} symbols, parse tree built with 0 syntactic errors.",
            tok_count, sym_count
        );
    } else {
        println!(
            "Errors found — lexical: {}, syntactic: {}. See output/errors.txt and output/parse_errors.txt.",
            lex_errors, syn_errors
        );
    }

    Ok(())
}
